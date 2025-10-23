use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use clap::{ArgAction, Args, Parser, Subcommand};
use mcp_center_test_client::{
    ClientEvent, ConnectRequest, SseConfig, StdIoConfig, StreamHttpConfig, TestClient,
};
use rmcp::model::InitializeResult;
use serde_json::Value;
use tokio::signal;
use tracing::subscriber;
use tracing::{error, info};
use tracing_subscriber::{EnvFilter, fmt};

#[derive(Parser)]
#[command(author, version, about = "Utility MCP client for testing mcp-center")]
struct Cli {
    /// Sets the log level (TRACE, DEBUG, INFO, WARN, ERROR)
    #[arg(long, default_value = "info")]
    log_level: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Lists tools exposed by the target MCP server.
    ListTools {
        #[command(subcommand)]
        transport: TransportCommand,

        /// Print tools as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Invokes a tool once with optional JSON arguments.
    CallTool {
        #[command(subcommand)]
        transport: TransportCommand,

        /// Tool name.
        #[arg(long)]
        name: String,

        /// JSON object passed as tool arguments.
        #[arg(long = "args-json")]
        args_json: Option<String>,

        /// Pretty-print JSON output.
        #[arg(long)]
        pretty: bool,
    },
    /// Streams MCP events until interrupted.
    Watch {
        #[command(subcommand)]
        transport: TransportCommand,

        /// Output events as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Shows handshake information and exits.
    Info {
        #[command(subcommand)]
        transport: TransportCommand,

        /// Output in JSON.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Clone)]
enum TransportCommand {
    /// Connect via stdio (child process).
    Stdio(StdIoCliArgs),
    /// Connect via Server-Sent Events endpoint.
    Sse(SseCliArgs),
    /// Connect via streaming HTTP endpoint.
    StreamHttp(StreamHttpCliArgs),
}

#[derive(Args, Clone)]
struct StdIoCliArgs {
    /// Command to spawn (defaults to `mcp-center`).
    #[arg(long = "cmd", default_value = "mcp-center")]
    command: PathBuf,

    /// Arguments forwarded to the command (defaults to `connect` when omitted).
    #[arg(long = "arg", value_name = "ARG", action = ArgAction::Append)]
    args: Vec<String>,

    /// Environment variables in KEY=VALUE form.
    #[arg(long = "env", value_name = "KEY=VALUE", value_parser = parse_key_val)]
    env: Vec<KeyVal>,
}

#[derive(Args, Clone)]
struct SseCliArgs {
    /// SSE endpoint (e.g. https://example.com/mcp/sse).
    #[arg(long)]
    url: String,

    /// Optional fixed message endpoint (`POST`) path.
    #[arg(long = "message-endpoint")]
    message_endpoint: Option<String>,

    /// Additional HTTP headers.
    #[arg(long = "header", value_name = "KEY=VALUE", value_parser = parse_key_val)]
    headers: Vec<KeyVal>,

    /// Bearer token used for Authorization header.
    #[arg(long = "auth-token")]
    auth_token: Option<String>,
}

#[derive(Args, Clone)]
struct StreamHttpCliArgs {
    /// Streaming HTTP endpoint (base URL).
    #[arg(long)]
    url: String,

    /// Additional HTTP headers.
    #[arg(long = "header", value_name = "KEY=VALUE", value_parser = parse_key_val)]
    headers: Vec<KeyVal>,

    /// Bearer token added to the Authorization header.
    #[arg(long = "auth-token")]
    auth_token: Option<String>,

    /// Require servers to create a stateful session.
    #[arg(long = "require-session", action = ArgAction::SetTrue)]
    require_session: bool,
}

#[derive(Debug, Clone)]
struct KeyVal {
    key: String,
    value: String,
}

fn parse_key_val(input: &str) -> std::result::Result<KeyVal, String> {
    let (key, value) = input
        .split_once('=')
        .ok_or_else(|| format!("invalid KEY=VALUE pair: {input}"))?;
    if key.is_empty() {
        return Err("header/environment key cannot be empty".into());
    }
    Ok(KeyVal { key: key.trim().to_string(), value: value.to_string() })
}

impl TransportCommand {
    fn into_request(self) -> Result<ConnectRequest> {
        match self {
            TransportCommand::Stdio(args) => {
                let command =
                    resolve_command(args.command).context("failed to locate mcp-center binary")?;
                let mut config = StdIoConfig::new(command);
                let arg_list = if args.args.is_empty() {
                    vec!["connect".to_string()]
                } else {
                    args.args
                };
                config = config.with_args(arg_list);
                let env = args.env.into_iter().map(|kv| (kv.key, kv.value)).collect();
                config = config.with_env(env);
                Ok(ConnectRequest::StdIo(config))
            }
            TransportCommand::Sse(args) => {
                let headers = args.headers.into_iter().map(|kv| (kv.key, kv.value)).collect();
                let config = SseConfig::new(args.url)
                    .with_headers(headers)
                    .with_message_endpoint(args.message_endpoint)
                    .with_auth_token(args.auth_token);
                Ok(ConnectRequest::Sse(config))
            }
            TransportCommand::StreamHttp(args) => {
                let headers = args.headers.into_iter().map(|kv| (kv.key, kv.value)).collect();
                let config = StreamHttpConfig::new(args.url)
                    .with_headers(headers)
                    .with_auth_token(args.auth_token)
                    .allow_stateless(!args.require_session);
                Ok(ConnectRequest::StreamHttp(config))
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(&cli.log_level)?;

    match cli.command {
        Command::ListTools { transport, json } => {
            let request = transport.into_request()?;
            let client = TestClient::connect(request).await?;
            if let Some(info) = client.initialize_result() {
                print_info(&info, json)?;
            }
            let tools = client.list_all_tools().await?;
            if json {
                println!("{}", serde_json::to_string_pretty(&tools)?);
            } else if tools.is_empty() {
                println!("No tools returned by server.");
            } else {
                println!("Tools ({}):", tools.len());
                for tool in tools {
                    let description = tool
                        .description
                        .as_ref()
                        .map(|d| d.trim())
                        .filter(|d| !d.is_empty())
                        .unwrap_or("—");
                    println!("  - {} :: {}", tool.name, description);
                }
            }
            client.shutdown().await?;
        }
        Command::CallTool { transport, name, args_json, pretty } => {
            let request = transport.into_request()?;
            let client = TestClient::connect(request).await?;
            let arguments = match args_json {
                Some(data) => Some(parse_json_arg(&data)?),
                None => None,
            };
            let result = client.call_tool(name, arguments).await?;
            if pretty {
                println!("{}", serde_json::to_string_pretty(&result)?);
            } else {
                println!("{}", serde_json::to_string(&result)?);
            }
            client.shutdown().await?;
        }
        Command::Watch { transport, json } => {
            let request = transport.into_request()?;
            let client = TestClient::connect(request).await?;
            let mut receiver = client.subscribe();
            if let Some(info) = client.initialize_result() {
                print_info(&info, json)?;
            }
            info!("watching events; press Ctrl+C to exit");

            loop {
                tokio::select! {
                    res = receiver.recv() => match res {
                        Ok(event) => print_event(&event, json)?,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                            eprintln!("skipped {skipped} events (channel lag)");
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                            println!("event stream closed");
                            break;
                        }
                    },
                    _ = signal::ctrl_c() => {
                        println!("received Ctrl+C, stopping watch");
                        break;
                    }
                }
            }
            client.shutdown().await?;
        }
        Command::Info { transport, json } => {
            let request = transport.into_request()?;
            let client = TestClient::connect(request).await?;
            if let Some(info) = client.initialize_result() {
                print_info(&info, json)?;
            } else {
                error!("server did not provide initialize result");
            }
            client.shutdown().await?;
        }
    }

    Ok(())
}

fn init_tracing(level: &str) -> Result<()> {
    let filter = EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("info"));
    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .finish();
    let _ = subscriber::set_global_default(subscriber);
    Ok(())
}

fn print_event(event: &ClientEvent, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(event)?);
    } else {
        println!("{event:#?}");
    }
    Ok(())
}

fn print_info(info: &InitializeResult, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(info)?);
    } else {
        println!(
            "Connected: {} v{} — {}",
            info.server_info.name,
            info.server_info.version.as_str(),
            info.instructions.as_deref().unwrap_or("no instructions")
        );
    }
    Ok(())
}

fn parse_json_arg(raw: &str) -> Result<Value> {
    if let Some(path) = raw.strip_prefix('@') {
        let content =
            std::fs::read_to_string(path).with_context(|| format!("failed to read {path}"))?;
        Ok(serde_json::from_str(&content)
            .with_context(|| format!("invalid JSON in file {path}"))?)
    } else {
        Ok(serde_json::from_str(raw).context("invalid JSON argument")?)
    }
}

fn resolve_command(path: PathBuf) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path);
    }
    if path.exists() {
        return Ok(path);
    }

    for key in ["CARGO_BIN_EXE_mcp-center", "CARGO_BIN_EXE_mcp_center"] {
        if let Some(value) = std::env::var_os(key) {
            let candidate = PathBuf::from(value);
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }

    if let Some(path_env) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_env) {
            let candidate = dir.join(&path);
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }

    Err(anyhow!("command '{}' not found in PATH; specify with --cmd", path.display()))
}
