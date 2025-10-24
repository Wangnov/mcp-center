use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    env, fs,
    io::{self, SeekFrom, Write},
    path::PathBuf,
    process,
    time::Duration as StdDuration,
};

use anyhow::{Context, Result, anyhow, bail};
use clap::{ArgAction, Args, CommandFactory, FromArgMatches, Parser, Subcommand, ValueEnum};
use interprocess::local_socket::traits::tokio::Stream as _;
use interprocess::local_socket::{GenericFilePath, ToFsName, tokio::prelude::LocalSocketStream};
use mcp_center::cli_i18n as i18n;
use mcp_center::daemon::{
    logging::{self, LogEntry, LogFileMeta, LogLevel},
    rpc::{DaemonRequest, DaemonResponse, ResponseData},
};
use mcp_center::project::{ToolCustomization, ToolPermission};
use mcp_center::{
    Layout, ProjectId, ProjectRecord, ProjectRegistry, ServerConfig, ServerDefinition,
    ServerProtocol, default_root,
};
use serde_json::json;
use time::OffsetDateTime;
use tokio::{
    fs::OpenOptions,
    io::{AsyncBufReadExt, AsyncSeekExt, AsyncWriteExt, BufReader},
    time::sleep,
};

#[derive(Parser, Debug)]
#[command(
    name = "mcp-center",
    version,
    about = "i18n:cli.about",
    disable_version_flag = true
)]
struct Cli {
    #[arg(
        short = 'v',
        long = "version",
        global = true,
        action = ArgAction::SetTrue,
        help = "i18n:cli.version_flag_help"
    )]
    show_version: bool,
    #[arg(long, global = true, value_name = "DIR", help = "i18n:cli.root_help")]
    root: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    #[command(about = "i18n:command.init.about")]
    Init,

    #[command(about = "i18n:command.serve.about")]
    Serve(mcp_center::daemon::serve::ServeArgs),

    #[command(about = "i18n:command.connect.about")]
    Connect(mcp_center::bridge::connect::ConnectArgs),

    #[command(about = "i18n:command.mcp.about")]
    Mcp {
        #[command(subcommand)]
        command: McpCommand,
    },

    #[command(about = "i18n:command.project.about")]
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },

    #[command(about = "i18n:command.logs.about")]
    Logs {
        #[command(subcommand)]
        command: LogsCommand,
    },
}

#[derive(Subcommand, Debug)]
enum McpCommand {
    #[command(about = "i18n:command.mcp.add.about")]
    Add(McpAddArgs),
    #[command(about = "i18n:command.mcp.list.about")]
    List,
    #[command(about = "i18n:command.mcp.list_tools.about")]
    ListTools(McpListToolsArgs),
    #[command(about = "i18n:command.mcp.info.about")]
    Info(McpNameArgs),
    #[command(about = "i18n:command.mcp.remove.about")]
    Remove(McpRemoveArgs),
    #[command(about = "i18n:command.mcp.enable.about")]
    Enable(McpNameArgs),
    #[command(about = "i18n:command.mcp.disable.about")]
    Disable(McpNameArgs),
}

#[derive(Subcommand, Debug)]
enum ProjectCommand {
    #[command(about = "i18n:command.project.add.about")]
    Add(ProjectAddArgs),
    #[command(about = "i18n:command.project.remove.about")]
    Remove(ProjectRemoveArgs),
    #[command(about = "i18n:command.project.list.about")]
    List,
    #[command(about = "i18n:command.project.allow.about")]
    Allow(ProjectAssignArgs),
    #[command(about = "i18n:command.project.deny.about")]
    Deny(ProjectAssignArgs),
    #[command(about = "i18n:command.project.allow_tools.about")]
    AllowTools(ProjectToolsArgs),
    #[command(about = "i18n:command.project.deny_tools.about")]
    DenyTools(ProjectToolsArgs),
    #[command(about = "i18n:command.project.set_tool_desc.about")]
    SetToolDesc(ProjectToolDescArgs),
    #[command(about = "i18n:command.project.reset_tool_desc.about")]
    ResetToolDesc(ProjectResetToolDescArgs),
}

#[derive(Subcommand, Debug)]
enum LogsCommand {
    #[command(about = "i18n:command.logs.list.about")]
    List(LogsListArgs),
    #[command(about = "i18n:command.logs.show.about")]
    Show(LogsShowArgs),
    #[command(about = "i18n:command.logs.tail.about")]
    Tail(LogsTailArgs),
}

#[derive(Args, Debug)]
struct McpAddArgs {
    #[arg(value_name = "NAME_OR_PATH", help = "i18n:args.mcp_add.name_or_path")]
    name_or_path: String,

    #[arg(long, help = "i18n:args.mcp_add.name")]
    name: Option<String>,

    #[arg(long, value_enum, default_value_t = ProtocolArg::StdIo, help = "i18n:args.mcp_add.protocol")]
    protocol: ProtocolArg,

    #[arg(long, value_name = "URL", help = "i18n:args.mcp_add.url")]
    url: Option<String>,

    #[arg(
        long = "env",
        value_name = "KEY=VALUE",
        value_parser = parse_env_pair,
        action = ArgAction::Append,
        help = "i18n:args.mcp_add.env"
    )]
    env: Vec<(String, String)>,

    #[arg(
        value_name = "COMMAND",
        trailing_var_arg = true,
        help = "i18n:args.mcp_add.command"
    )]
    command: Vec<String>,
}

#[derive(Args, Debug)]
struct McpNameArgs {
    #[arg(help = "i18n:args.mcp_name")]
    name: String,
}

#[derive(Args, Debug)]
struct McpRemoveArgs {
    #[arg(help = "i18n:args.mcp_remove.name")]
    name: String,
    #[arg(short, long, help = "i18n:args.mcp_remove.yes")]
    yes: bool,
}

#[derive(Args, Debug)]
struct ProjectAddArgs {
    #[arg(value_name = "PATH", help = "i18n:args.project_add.path")]
    path: Option<PathBuf>,
}

#[derive(Args, Debug)]
struct ProjectRemoveArgs {
    #[arg(value_name = "PATH_OR_ID", help = "i18n:args.project_remove.target")]
    target: String,
    #[arg(short, long, help = "i18n:args.project_remove.yes")]
    yes: bool,
}

#[derive(Args, Debug)]
struct ProjectAssignArgs {
    #[arg(value_name = "PATH_OR_ID", help = "i18n:args.project.target")]
    target: String,
    #[arg(value_name = "SERVER", num_args = 1.., help = "i18n:args.project.servers")]
    servers: Vec<String>,
}

#[derive(Args, Debug)]
struct McpListToolsArgs {
    #[arg(value_name = "SERVER", help = "i18n:args.mcp_list_tools.server")]
    server: Option<String>,
}

#[derive(Args, Debug)]
struct ProjectToolsArgs {
    #[arg(value_name = "PATH_OR_ID", help = "i18n:args.project_tools.target")]
    target: String,
    #[arg(value_name = "SERVER::TOOL", num_args = 1.., help = "i18n:args.project_tools.tools")]
    tools: Vec<String>,
}

#[derive(Args, Debug)]
struct ProjectToolDescArgs {
    #[arg(value_name = "PATH_OR_ID", help = "i18n:args.project_tool_desc.target")]
    target: String,
    #[arg(
        value_name = "TOOL_NAME",
        help = "i18n:args.project_tool_desc.tool_name"
    )]
    tool_name: String,
    #[arg(
        value_name = "DESCRIPTION",
        help = "i18n:args.project_tool_desc.description"
    )]
    description: String,
}

#[derive(Args, Debug)]
struct ProjectResetToolDescArgs {
    #[arg(
        value_name = "PATH_OR_ID",
        help = "i18n:args.project_reset_tool_desc.target"
    )]
    target: String,
    #[arg(
        value_name = "TOOL_NAME",
        help = "i18n:args.project_reset_tool_desc.tool_name"
    )]
    tool_name: String,
}

#[derive(Args, Debug)]
struct LogsListArgs {
    #[arg(long, value_name = "SERVER", help = "i18n:args.logs.server")]
    server: Option<String>,
}

#[derive(Args, Debug)]
struct LogsShowArgs {
    #[arg(value_name = "SERVER", help = "i18n:args.logs.server")]
    server: String,
    #[arg(long, value_name = "FILE", help = "i18n:args.logs.file")]
    file: Option<String>,
    #[arg(
        long,
        value_name = "LIMIT",
        default_value_t = 200,
        help = "i18n:args.logs.limit"
    )]
    limit: usize,
    #[arg(long, action = ArgAction::SetTrue, help = "i18n:args.logs.json")]
    json: bool,
}

#[derive(Args, Debug)]
struct LogsTailArgs {
    #[arg(value_name = "SERVER", help = "i18n:args.logs.server")]
    server: String,
    #[arg(long, value_name = "FILE", help = "i18n:args.logs.file")]
    file: Option<String>,
    #[arg(long, action = ArgAction::SetTrue, help = "i18n:args.logs.from_start")]
    from_start: bool,
}

#[derive(Copy, Clone, Debug, ValueEnum)]
enum ProtocolArg {
    #[value(name = "stdio")]
    StdIo,
    #[value(name = "sse")]
    Sse,
    #[value(name = "http")]
    Http,
}

impl From<ProtocolArg> for ServerProtocol {
    fn from(value: ProtocolArg) -> Self {
        match value {
            ProtocolArg::StdIo => ServerProtocol::StdIo,
            ProtocolArg::Sse => ServerProtocol::Sse,
            ProtocolArg::Http => ServerProtocol::Http,
        }
    }
}

#[tokio::main]
async fn main() {
    let messages = i18n::messages();
    let command = i18n::localize_command(Cli::command(), messages);

    let mut matches = command.get_matches();
    let cli = Cli::from_arg_matches_mut(&mut matches).unwrap_or_else(|err| err.exit());

    if cli.show_version {
        if let Some(version) = Cli::command().get_version() {
            println!("{version}");
        }
        return;
    }

    if let Err(err) = run(cli).await {
        let rendered = messages.render_anyhow(&err);
        eprintln!("{} {}", messages.error_prefix(), rendered);
        process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Serve(args) => mcp_center::daemon::serve::run(args).await,
        Command::Connect(args) => mcp_center::bridge::connect::run(args).await,
        Command::Init => {
            let layout = resolve_layout(cli.root.clone())?;
            handle_init(&layout)
        }
        Command::Mcp { command } => {
            let layout = resolve_layout(cli.root.clone())?;
            handle_mcp_command(&layout, command).await
        }
        Command::Project { command } => {
            let layout = resolve_layout(cli.root.clone())?;
            handle_project_command(&layout, command)
        }
        Command::Logs { command } => {
            let layout = resolve_layout(cli.root.clone())?;
            handle_logs_command(&layout, command).await
        }
    }
}

fn resolve_layout(root_override: Option<PathBuf>) -> Result<Layout> {
    let root = match root_override {
        Some(path) => expand_tilde(path)?,
        None => default_root()?,
    };
    Ok(Layout::new(root))
}

fn handle_init(layout: &Layout) -> Result<()> {
    let messages = i18n::messages();
    layout.ensure()?;

    let had_configs_before = !layout.list_server_configs()?.is_empty();

    let samples = vec![
        ServerDefinition {
            id: String::new(),
            name: Some("Context7".to_string()),
            protocol: ServerProtocol::StdIo,
            command: "npx".to_string(),
            args: vec![
                "-y".to_string(),
                "@upstash/context7-mcp".to_string(),
                "--api-key".to_string(),
                "YOUR_API_KEY".to_string(),
            ],
            env: BTreeMap::new(),
            endpoint: None,
            headers: BTreeMap::new(),
            enabled: false,
        },
        ServerDefinition {
            id: String::new(),
            name: Some("DeepWiki".to_string()),
            protocol: ServerProtocol::Sse,
            command: String::new(),
            args: Vec::new(),
            env: BTreeMap::new(),
            endpoint: Some("https://mcp.deepwiki.com/sse".to_string()),
            headers: BTreeMap::new(),
            enabled: false,
        },
    ];

    let mut created_paths = Vec::new();
    for definition in samples {
        let mut config = ServerConfig::new(definition)?;
        config.definition_mut().id.clear();
        assign_server_id(layout, &mut config)?;
        let destination = layout.server_config_toml_path(config.definition().id.as_str());
        if destination.exists() {
            continue;
        }
        let toml = config.to_toml_string()?;
        fs::write(&destination, toml)
            .with_context(|| messages.sample_write_failed(&destination))?;
        created_paths.push(destination);
    }

    if created_paths.is_empty() {
        println!("{}", messages.workspace_already_initialized(layout.root()));
    } else if had_configs_before {
        for path in &created_paths {
            println!("{}", messages.sample_added(path));
        }
    } else {
        println!("{}", messages.workspace_initialized(layout.root(), &created_paths[0]));
        for path in created_paths.iter().skip(1) {
            println!("{}", messages.sample_added(path));
        }
    }
    Ok(())
}

async fn handle_mcp_command(layout: &Layout, command: McpCommand) -> Result<()> {
    match command {
        McpCommand::Add(args) => handle_mcp_add(layout, args),
        McpCommand::List => handle_mcp_list(layout),
        McpCommand::ListTools(args) => handle_mcp_list_tools(layout, args).await,
        McpCommand::Info(args) => handle_mcp_info(layout, args),
        McpCommand::Remove(args) => handle_mcp_remove(layout, args),
        McpCommand::Enable(args) => handle_mcp_enable(layout, args),
        McpCommand::Disable(args) => handle_mcp_disable(layout, args),
    }
}

fn handle_mcp_add(layout: &Layout, args: McpAddArgs) -> Result<()> {
    layout.ensure()?;

    if args.command.is_empty() && looks_like_config_path(&args.name_or_path) {
        add_from_file(layout, &args.name_or_path)
    } else {
        add_inline(layout, args)
    }
}

fn add_from_file(layout: &Layout, input: &str) -> Result<()> {
    let messages = i18n::messages();
    let path = expand_tilde(PathBuf::from(input))?;
    let mut config = ServerConfig::from_file(&path)?;
    config.definition_mut().id.clear();
    assign_server_id(layout, &mut config)?;
    let name = definition_name(config.definition())?;
    ensure_unique_name(layout, &name, None)?;
    config.definition_mut().name = Some(name.clone());
    config.definition_mut().enabled = false;

    let destination = layout.server_config_toml_path(config.definition().id.as_str());
    if destination.exists() {
        bail!("{}", messages.config_id_exists(config.definition().id.as_str(), &destination));
    }

    let toml = config.to_toml_string()?;
    fs::write(&destination, toml).with_context(|| messages.copy_definition_failed(&destination))?;

    println!(
        "{}",
        messages.registered_server(&name, config.definition().id.as_str(), &destination)
    );
    Ok(())
}

fn add_inline(layout: &Layout, args: McpAddArgs) -> Result<()> {
    let messages = i18n::messages();
    let McpAddArgs { name_or_path, name, protocol, env, url, command } = args;

    let endpoint = match protocol {
        ProtocolArg::StdIo => {
            if url.is_some() {
                bail!("{}", messages.url_not_allowed_for_stdio());
            }
            None
        }
        ProtocolArg::Sse | ProtocolArg::Http => {
            let raw = url.ok_or_else(|| anyhow!(messages.inline_url_required()))?;
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                bail!("{}", messages.inline_url_required());
            }
            Some(trimmed.to_string())
        }
    };

    let (executable, command_args) = match protocol {
        ProtocolArg::StdIo => {
            if command.is_empty() {
                bail!("{}", messages.inline_command_required());
            }
            let mut iter = command.into_iter();
            let executable =
                iter.next().ok_or_else(|| anyhow!(messages.inline_command_required()))?;
            let args = iter.collect();
            (executable, args)
        }
        ProtocolArg::Sse | ProtocolArg::Http => {
            if !command.is_empty() {
                bail!("{}", messages.remote_command_forbidden());
            }
            (String::new(), Vec::new())
        }
    };

    let env = env.into_iter().collect::<BTreeMap<String, String>>();
    let raw_display_name = name.unwrap_or_else(|| name_or_path.clone());
    let display_name = normalize_name(&raw_display_name)?;
    ensure_unique_name(layout, &display_name, None)?;

    let definition = ServerDefinition {
        id: String::new(),
        name: Some(display_name.clone()),
        protocol: protocol.into(),
        command: executable,
        args: command_args,
        env,
        endpoint,
        headers: BTreeMap::new(),
        enabled: false,
    };
    let mut config = ServerConfig::new(definition)?;
    config.definition_mut().id.clear();
    assign_server_id(layout, &mut config)?;
    let destination = layout.server_config_toml_path(config.definition().id.as_str());

    if destination.exists() {
        bail!("{}", messages.config_id_exists(config.definition().id.as_str(), &destination));
    }

    let toml = config.to_toml_string()?;
    fs::write(&destination, toml)
        .with_context(|| messages.write_definition_failed(&destination))?;

    println!(
        "{}",
        messages.added_server(&display_name, config.definition().id.as_str(), &destination)
    );
    Ok(())
}

fn assign_server_id(layout: &Layout, config: &mut ServerConfig) -> Result<()> {
    let existing_ids: HashSet<String> = layout
        .list_server_configs()?
        .into_iter()
        .map(|cfg| cfg.definition().id.clone())
        .collect();
    config.assign_unique_id(&existing_ids);
    Ok(())
}

fn parse_env_pair(raw: &str) -> Result<(String, String)> {
    let messages = i18n::messages();
    let (key, value) = raw.split_once('=').ok_or_else(|| anyhow!(messages.env_pair_format()))?;
    if key.trim().is_empty() {
        bail!("{}", messages.env_key_empty());
    }
    Ok((key.trim().to_string(), value.to_string()))
}

fn expand_tilde(path: PathBuf) -> Result<PathBuf> {
    if let Some(str_path) = path.to_str() {
        if let Some(stripped) = str_path.strip_prefix("~") {
            let messages = i18n::messages();
            let home = dirs_home().context(messages.expand_home_missing())?;
            if stripped.is_empty() {
                return Ok(home);
            }
            let stripped = stripped.strip_prefix('/').unwrap_or(stripped);
            return Ok(home.join(stripped));
        }
    }
    Ok(path)
}

fn dirs_home() -> Option<PathBuf> {
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return Some(PathBuf::from(home));
        }
    }
    if let Ok(profile) = std::env::var("USERPROFILE") {
        if !profile.is_empty() {
            return Some(PathBuf::from(profile));
        }
    }
    None
}

fn looks_like_config_path(input: &str) -> bool {
    input.ends_with(".toml") || input.ends_with(".json")
}

fn handle_mcp_list(layout: &Layout) -> Result<()> {
    let messages = i18n::messages();
    layout.ensure()?;
    let configs = layout.list_server_configs()?;
    if configs.is_empty() {
        println!("{}", messages.no_servers_registered());
        return Ok(());
    }

    let (name_header, enabled_header, proto_header, endpoint_header, id_header, command_header) =
        messages.list_headers();
    println!(
        "{name_header:<20} {enabled_header:<7} {proto_header:<8} {endpoint_header:<35} {id_header:<20} {command_header}"
    );
    for config in configs {
        let definition = config.definition();
        let name = definition_name(definition)?;
        let proto = match definition.protocol {
            ServerProtocol::StdIo => "stdio",
            ServerProtocol::Sse => "sse",
            ServerProtocol::Http => "http",
            ServerProtocol::Unknown => "unknown",
        };
        let enabled = messages.enabled_label(definition.enabled);
        let args = definition.args.join(" ");
        let endpoint_display =
            definition.endpoint.as_deref().filter(|s| !s.is_empty()).unwrap_or("-");
        let command_display = if definition.command.trim().is_empty() && args.is_empty() {
            "-".to_string()
        } else if args.is_empty() {
            definition.command.clone()
        } else {
            format!("{} {}", definition.command, args)
        };
        println!(
            "{:<20} {:<7} {:<8} {:<35} {:<20} {}",
            name, enabled, proto, endpoint_display, definition.id, command_display
        );
    }
    Ok(())
}

fn handle_mcp_info(layout: &Layout, args: McpNameArgs) -> Result<()> {
    layout.ensure()?;
    let config = layout.load_server_config_by_name(&args.name)?;
    let definition = config.definition();
    let name = definition_name(definition)?;
    let doc = json!({
        "id": definition.id,
        "name": name,
        "protocol": match definition.protocol {
            ServerProtocol::StdIo => "stdio",
            ServerProtocol::Sse => "sse",
            ServerProtocol::Http => "http",
            ServerProtocol::Unknown => "unknown",
        },
        "command": definition.command,
        "args": definition.args,
        "env": definition.env,
        "endpoint": definition.endpoint.clone(),
        "headers": definition.headers,
        "enabled": definition.enabled,
        "configPath": config.source().map(|p| p.display().to_string()),
        "logPath": layout.server_log_path(&definition.id).display().to_string(),
        "pidPath": layout.server_pid_path(&definition.id).display().to_string(),
    });
    println!("{}", serde_json::to_string_pretty(&doc)?);
    Ok(())
}

fn handle_mcp_remove(layout: &Layout, args: McpRemoveArgs) -> Result<()> {
    let messages = i18n::messages();
    layout.ensure()?;
    let config = layout.load_server_config_by_name(&args.name)?;
    let definition = config.definition();
    let name = definition_name(definition)?;
    if !args.yes && !confirm_removal(&name)? {
        println!("{}", messages.removal_aborted());
        return Ok(());
    }
    layout.remove_server_config(&definition.id)?;
    println!("{}", messages.removal_done(&name, definition.id.as_str()));
    Ok(())
}

fn confirm_removal(name: &str) -> Result<bool> {
    let messages = i18n::messages();
    print!("{}", messages.confirm_removal_prompt(name));
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let answer = input.trim().to_ascii_lowercase();
    Ok(matches!(answer.as_str(), "y" | "yes"))
}

fn handle_mcp_enable(layout: &Layout, args: McpNameArgs) -> Result<()> {
    let messages = i18n::messages();
    layout.ensure()?;
    let mut config = layout.load_server_config_by_name(&args.name)?;
    let name = definition_name(config.definition())?;
    if config.definition().enabled {
        println!("{}", messages.server_already_enabled(&name));
        return Ok(());
    }
    config.definition_mut().enabled = true;
    persist_server_config(&config)?;
    println!("{}", messages.server_enabled(&name));
    Ok(())
}

fn handle_mcp_disable(layout: &Layout, args: McpNameArgs) -> Result<()> {
    let messages = i18n::messages();
    layout.ensure()?;
    let mut config = layout.load_server_config_by_name(&args.name)?;
    let name = definition_name(config.definition())?;
    if !config.definition().enabled {
        println!("{}", messages.server_already_disabled(&name));
        return Ok(());
    }
    config.definition_mut().enabled = false;
    persist_server_config(&config)?;
    println!("{}", messages.server_disabled(&name));
    Ok(())
}

fn ensure_unique_name(layout: &Layout, name: &str, skip_id: Option<&str>) -> Result<()> {
    let candidate = normalize_name(name)?;
    for config in layout.list_server_configs()? {
        let existing_name = definition_name(config.definition())?;
        if existing_name == candidate {
            if skip_id.map(|id| id == config.definition().id).unwrap_or(false) {
                continue;
            }
            bail!("{}", i18n::messages().server_name_duplicate(&candidate));
        }
    }
    Ok(())
}

fn handle_project_command(layout: &Layout, command: ProjectCommand) -> Result<()> {
    match command {
        ProjectCommand::Add(args) => handle_project_add(layout, args),
        ProjectCommand::Remove(args) => handle_project_remove(layout, args),
        ProjectCommand::List => handle_project_list(layout),
        ProjectCommand::Allow(args) => handle_project_allow(layout, args),
        ProjectCommand::Deny(args) => handle_project_deny(layout, args),
        ProjectCommand::AllowTools(args) => handle_project_allow_tools(layout, args),
        ProjectCommand::DenyTools(args) => handle_project_deny_tools(layout, args),
        ProjectCommand::SetToolDesc(args) => handle_project_set_tool_desc(layout, args),
        ProjectCommand::ResetToolDesc(args) => handle_project_reset_tool_desc(layout, args),
    }
}

async fn handle_logs_command(layout: &Layout, command: LogsCommand) -> Result<()> {
    match command {
        LogsCommand::List(args) => handle_logs_list(layout, args).await,
        LogsCommand::Show(args) => handle_logs_show(layout, args).await,
        LogsCommand::Tail(args) => handle_logs_tail(layout, args).await,
    }
}

fn handle_project_list(layout: &Layout) -> Result<()> {
    let messages = i18n::messages();
    let registry = ProjectRegistry::new(layout);
    registry.ensure()?;
    let mut records = registry.list()?;
    let server_lookup = layout
        .list_server_configs()?
        .into_iter()
        .map(|cfg| {
            let definition = cfg.definition();
            let label = definition.name.clone().unwrap_or_else(|| definition.id.clone());
            (definition.id.clone(), label)
        })
        .collect::<HashMap<_, _>>();

    if records.is_empty() {
        println!("{}", messages.project_empty());
        return Ok(());
    }

    records.sort_by(|a, b| b.last_seen_at.cmp(&a.last_seen_at));
    let (project_header, agent_header, servers_header, seen_header) = messages.project_headers();
    println!("{project_header:<40}  {agent_header:<18}  {servers_header:<30}  {seen_header}");

    for record in records {
        let home_dir = env::var("HOME").ok().map(PathBuf::from);
        let is_home = home_dir.as_ref().is_some_and(|home| record.path == *home);

        let project_display = if is_home {
            format!("~ (global) ({})", record.id)
        } else {
            format!("{} ({})", record.path.display(), record.id)
        };
        let agent = record.agent.as_deref().filter(|s| !s.is_empty()).unwrap_or("-");
        let servers = if record.allowed_server_ids.is_empty() {
            "-".to_string()
        } else {
            record
                .allowed_server_ids
                .iter()
                .map(|id| server_lookup.get(id).cloned().unwrap_or_else(|| id.clone()))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let last_seen = format_timestamp(record.last_seen_at);
        println!("{project_display:<40}  {agent:<18}  {servers:<30}  {last_seen}");
    }
    Ok(())
}

fn handle_project_allow(layout: &Layout, args: ProjectAssignArgs) -> Result<()> {
    let messages = i18n::messages();
    let registry = ProjectRegistry::new(layout);
    registry.ensure()?;

    let canonical = normalize_project_path(&args.target)?;

    // 使用 find_by_path 查找是否已存在该路径的记录，防止重复
    let mut record = if let Some(existing) = registry.find_by_path(&canonical)? {
        existing
    } else {
        // 路径不存在，创建新记录
        let project_id = ProjectId::from_path(&canonical);
        ProjectRecord::new(project_id, canonical.clone())
    };

    let existing_ids: HashSet<String> = layout
        .list_server_configs()?
        .into_iter()
        .map(|cfg| cfg.definition().id.clone())
        .collect();

    let mut allowed =
        record.allowed_server_ids.into_iter().collect::<std::collections::BTreeSet<_>>();
    let mut added = Vec::new();

    for server in args.servers {
        let trimmed = server.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !existing_ids.contains(trimmed) {
            bail!(messages.project_server_unknown(trimmed));
        }
        if allowed.insert(trimmed.to_string()) {
            added.push(trimmed.to_string());
        }
    }

    record.allowed_server_ids = allowed.into_iter().collect();
    record.path = canonical.clone();
    record.touch();

    registry.store(&record)?;

    if added.is_empty() {
        println!("{}", messages.project_allow_unchanged(&canonical));
    } else {
        println!("{}", messages.project_allow_done(&canonical, &added));
    }
    Ok(())
}

fn handle_project_deny(layout: &Layout, args: ProjectAssignArgs) -> Result<()> {
    let messages = i18n::messages();
    let registry = ProjectRegistry::new(layout);
    registry.ensure()?;

    let canonical = normalize_project_path(&args.target)?;

    // 使用 find_by_path 查找记录
    let mut record = if let Some(existing) = registry.find_by_path(&canonical)? {
        existing
    } else {
        // 如果路径不存在任何记录，返回错误
        return Err(anyhow!(messages.project_record_missing(&canonical)));
    };

    let mut allowed =
        record.allowed_server_ids.into_iter().collect::<std::collections::BTreeSet<_>>();
    let mut removed = Vec::new();
    let mut missing = Vec::new();

    for server in args.servers {
        let trimmed = server.trim();
        if trimmed.is_empty() {
            continue;
        }
        if allowed.remove(trimmed) {
            removed.push(trimmed.to_string());
        } else {
            missing.push(trimmed.to_string());
        }
    }

    if !missing.is_empty() {
        eprintln!("{}", messages.project_deny_missing(&canonical, &missing));
    }

    record.allowed_server_ids = allowed.into_iter().collect();
    record.path = canonical.clone();
    record.touch();
    registry.store(&record)?;

    if removed.is_empty() {
        println!("{}", messages.project_deny_unchanged(&canonical));
    } else {
        println!("{}", messages.project_deny_done(&canonical, &removed));
    }
    Ok(())
}

fn handle_project_add(layout: &Layout, args: ProjectAddArgs) -> Result<()> {
    let registry = ProjectRegistry::new(layout);
    registry.ensure()?;

    // 确定要添加的路径：参数提供的路径或当前目录
    let path = if let Some(provided) = args.path {
        normalize_project_path(&provided.display().to_string())?
    } else {
        env::current_dir().context("Failed to get current directory")?
    };

    // 检查是否已存在该路径的 project
    if let Some(existing) = registry.find_by_path(&path)? {
        println!("Project already exists at {} (ID: {})", path.display(), existing.id);
        return Ok(());
    }

    // 创建新的 project record
    let project_id = ProjectId::from_path(&path);
    let record = ProjectRecord::new(project_id.clone(), path.clone());
    registry.store(&record)?;

    println!("Added project: {} (ID: {})", path.display(), project_id.as_str());
    Ok(())
}

fn handle_project_remove(layout: &Layout, args: ProjectRemoveArgs) -> Result<()> {
    let registry = ProjectRegistry::new(layout);
    registry.ensure()?;

    // 首先尝试按路径查找
    let path_result = normalize_project_path(&args.target);
    let record = if let Ok(path) = path_result {
        registry.find_by_path(&path)?
    } else {
        // 如果不是有效路径，尝试按ID查找（直接从文件名加载）
        registry.load_from_id_str(&args.target).ok()
    };

    let Some(record) = record else {
        bail!("Project not found: {}", args.target);
    };

    // 确认删除
    if !args.yes {
        print!("Remove project {} (ID: {})? [y/N] ", record.path.display(), record.id);
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let answer = input.trim().to_ascii_lowercase();
        if !matches!(answer.as_str(), "y" | "yes") {
            println!("Aborted");
            return Ok(());
        }
    }

    // 删除 project record - 使用从路径生成的 ProjectId
    let id = ProjectId::from_path(&record.path);
    registry.delete(&id)?;

    println!("Removed project: {} (ID: {})", record.path.display(), record.id);
    Ok(())
}

fn normalize_project_path(raw: &str) -> Result<PathBuf> {
    let path = PathBuf::from(raw);
    let absolute = if path.is_absolute() {
        path
    } else {
        env::current_dir()?.join(path)
    };
    Ok(fs::canonicalize(&absolute).unwrap_or(absolute))
}

fn format_timestamp(secs: u64) -> String {
    use time::UtcOffset;

    let timestamp =
        OffsetDateTime::from_unix_timestamp(secs as i64).unwrap_or(OffsetDateTime::UNIX_EPOCH);

    // 尝试获取本地时区偏移
    let local_timestamp = if let Ok(local_offset) = UtcOffset::current_local_offset() {
        timestamp.to_offset(local_offset)
    } else {
        // 如果无法获取本地时区，使用 UTC
        timestamp
    };

    // 使用更友好的格式：2025-10-16 17:30:45
    let format =
        time::format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second]").unwrap();

    local_timestamp.format(&format).unwrap_or_else(|_| secs.to_string())
}

fn normalize_name(raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        bail!("{}", i18n::messages().server_name_empty());
    }
    Ok(trimmed.to_string())
}

fn definition_name(definition: &ServerDefinition) -> Result<String> {
    definition
        .name
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(|value| value.to_string())
        .ok_or_else(|| anyhow!(i18n::messages().definition_missing_display_name()))
}

fn persist_server_config(config: &ServerConfig) -> Result<()> {
    let messages = i18n::messages();
    let path = config.source().ok_or_else(|| anyhow!(messages.persist_path_unknown()))?;
    let toml = config.to_toml_string()?;
    fs::write(path, toml).with_context(|| messages.update_definition_failed(path))?;
    Ok(())
}

// ============= RPC Client Helper =============

async fn send_rpc_request(layout: &Layout, request: DaemonRequest) -> Result<DaemonResponse> {
    let socket_path = layout.daemon_rpc_socket_path();
    #[cfg(unix)]
    if !socket_path.exists() {
        bail!("{}", i18n::messages().daemon_not_running());
    }

    let socket_name = socket_path.to_string_lossy().to_string();
    let stream_name = socket_name.as_str().to_fs_name::<GenericFilePath>()?;
    let stream = match LocalSocketStream::connect(stream_name).await {
        Ok(stream) => stream,
        Err(err)
            if matches!(err.kind(), io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused) =>
        {
            bail!("{}", i18n::messages().daemon_not_running());
        }
        Err(err) => return Err(err.into()),
    };
    let (reader, mut writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);

    // Send request
    let request_json = serde_json::to_string(&request)? + "\n";
    writer.write_all(request_json.as_bytes()).await?;

    // Read response
    let mut response_line = String::new();
    reader.read_line(&mut response_line).await?;

    let response: DaemonResponse = serde_json::from_str(response_line.trim())?;
    Ok(response)
}

// ============= MCP list-tools Command =============

async fn handle_mcp_list_tools(layout: &Layout, args: McpListToolsArgs) -> Result<()> {
    let messages = i18n::messages();

    let request = DaemonRequest::ListTools { server_name: args.server.clone() };

    let response = send_rpc_request(layout, request).await?;

    match response {
        DaemonResponse::Success { data } => {
            if let ResponseData::ToolList(tools) = data {
                if tools.is_empty() {
                    println!("{}", messages.no_tools_found());
                    return Ok(());
                }

                if let Some(server) = args.server {
                    println!("{}", messages.tools_from_server(&server));
                } else {
                    println!("{}", messages.all_tools());
                }

                for tool in tools {
                    println!("  {} ({})", tool.name, tool.server_name);
                    if !tool.description.is_empty() {
                        println!("    {}", tool.description);
                    }
                }
            } else {
                bail!("{}", messages.unexpected_response());
            }
        }
        DaemonResponse::Error { message } => {
            bail!("{}: {}", messages.rpc_error(), message);
        }
    }
    Ok(())
}

// ============= Project Tool Permission Commands =============

fn parse_tool_spec(spec: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = spec.split("::").collect();
    if parts.len() != 2 {
        bail!("{}", i18n::messages().invalid_tool_spec(spec));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

fn handle_project_allow_tools(layout: &Layout, args: ProjectToolsArgs) -> Result<()> {
    let messages = i18n::messages();
    let registry = ProjectRegistry::new(layout);
    registry.ensure()?;

    let mut record = load_project_record(&registry, &args.target)?;

    // Parse tool specs and group by server
    let mut server_tools: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for spec in &args.tools {
        let (server, tool) = parse_tool_spec(spec)?;
        server_tools.entry(server).or_default().push(tool);
    }

    // Update allowed_server_tools
    for (server, tools) in server_tools {
        record
            .allowed_server_tools
            .insert(server.clone(), ToolPermission::AllowList { tools: tools.clone() });
        println!("{}", messages.project_tools_allowed(&server, &tools.join(", ")));
    }

    registry.store(&record)?;
    println!("{}", messages.project_config_updated(&record.path.display().to_string()));
    Ok(())
}

fn handle_project_deny_tools(layout: &Layout, args: ProjectToolsArgs) -> Result<()> {
    let messages = i18n::messages();
    let registry = ProjectRegistry::new(layout);
    registry.ensure()?;

    let mut record = load_project_record(&registry, &args.target)?;

    // Parse tool specs and group by server
    let mut server_tools: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for spec in &args.tools {
        let (server, tool) = parse_tool_spec(spec)?;
        server_tools.entry(server).or_default().push(tool);
    }

    // Update allowed_server_tools
    for (server, tools) in server_tools {
        record
            .allowed_server_tools
            .insert(server.clone(), ToolPermission::DenyList { tools: tools.clone() });
        println!("{}", messages.project_tools_denied(&server, &tools.join(", ")));
    }

    registry.store(&record)?;
    println!("{}", messages.project_config_updated(&record.path.display().to_string()));
    Ok(())
}

// ============= Project Tool Customization Commands =============

fn handle_project_set_tool_desc(layout: &Layout, args: ProjectToolDescArgs) -> Result<()> {
    let messages = i18n::messages();
    let registry = ProjectRegistry::new(layout);
    registry.ensure()?;

    let mut record = load_project_record(&registry, &args.target)?;

    // Remove existing customization for this tool
    record.tool_customizations.retain(|c| c.tool_name != args.tool_name);

    // Add new customization
    record.tool_customizations.push(ToolCustomization {
        tool_name: args.tool_name.clone(),
        description: Some(args.description.clone()),
    });

    registry.store(&record)?;
    println!("{}", messages.tool_desc_set(&args.tool_name));
    println!("{}", messages.project_config_updated(&record.path.display().to_string()));
    Ok(())
}

fn handle_project_reset_tool_desc(layout: &Layout, args: ProjectResetToolDescArgs) -> Result<()> {
    let messages = i18n::messages();
    let registry = ProjectRegistry::new(layout);
    registry.ensure()?;

    let mut record = load_project_record(&registry, &args.target)?;

    // Remove customization for this tool
    let before_len = record.tool_customizations.len();
    record.tool_customizations.retain(|c| c.tool_name != args.tool_name);
    let after_len = record.tool_customizations.len();

    if before_len == after_len {
        println!("{}", messages.tool_desc_not_customized(&args.tool_name));
        return Ok(());
    }

    registry.store(&record)?;
    println!("{}", messages.tool_desc_reset(&args.tool_name));
    println!("{}", messages.project_config_updated(&record.path.display().to_string()));
    Ok(())
}

async fn handle_logs_list(layout: &Layout, args: LogsListArgs) -> Result<()> {
    let messages = i18n::messages();
    layout.ensure()?;

    let mut server_ids = BTreeSet::new();
    if let Some(server) = args.server.clone() {
        server_ids.insert(server);
    } else {
        for config in layout.list_server_configs()? {
            server_ids.insert(config.definition().id.clone());
        }
        if let Ok(mut entries) = tokio::fs::read_dir(layout.server_logs_dir()).await {
            while let Some(entry) = entries.next_entry().await? {
                if entry.file_type().await?.is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        server_ids.insert(name.to_string());
                    }
                }
            }
        }
    }

    let mut rows: Vec<(String, LogFileMeta)> = Vec::new();
    for server_id in &server_ids {
        let metas = logging::list_server_log_files(layout, server_id).await?;
        if metas.is_empty() && args.server.is_some() {
            println!("{}", messages.logs_no_files_for(server_id));
        }
        for meta in metas {
            rows.push((server_id.clone(), meta));
        }
    }

    if rows.is_empty() {
        if args.server.is_none() {
            println!("{}", messages.logs_no_files());
        }
        return Ok(());
    }

    let (server_header, file_header, size_header, lines_header, range_header) =
        messages.logs_list_header();
    println!(
        "{server_header:<20}  {file_header:<16}  {size_header:>10}  {lines_header:>10}  {range_header}"
    );
    for (server_id, meta) in rows {
        println!(
            "{:<20}  {:<16}  {:>10}  {:>10}  {}",
            server_id,
            meta.file_name,
            format_size(meta.file_size),
            meta.line_count,
            format_range(&meta)
        );
    }
    Ok(())
}

async fn handle_logs_show(layout: &Layout, args: LogsShowArgs) -> Result<()> {
    let messages = i18n::messages();
    layout.ensure()?;

    let files = logging::list_server_log_files(layout, &args.server).await?;
    if files.is_empty() {
        println!("{}", messages.logs_no_files_for(&args.server));
        return Ok(());
    }

    let target = select_log_file(&files, args.file.as_deref(), &args.server)?;
    let limit = args.limit.max(1);
    let offset = target.line_count.saturating_sub(limit as u64);
    let page = logging::read_log_entries(&target.path, offset, limit).await?;

    if page.entries.is_empty() {
        println!("{}", messages.logs_show_no_entries());
        return Ok(());
    }

    println!("{}", messages.logs_show_header(page.entries.len(), &target.file_name));
    for entry in page.entries {
        print_log_entry(&entry, args.json);
    }
    Ok(())
}

async fn handle_logs_tail(layout: &Layout, args: LogsTailArgs) -> Result<()> {
    let messages = i18n::messages();
    layout.ensure()?;

    let files = logging::list_server_log_files(layout, &args.server).await?;
    if files.is_empty() {
        println!("{}", messages.logs_no_files_for(&args.server));
        return Ok(());
    }

    let target = select_log_file(&files, args.file.as_deref(), &args.server)?;
    let mut file = OpenOptions::new()
        .read(true)
        .open(&target.path)
        .await
        .with_context(|| format!("failed to open log file {}", target.path.display()))?;

    if args.from_start {
        file.seek(SeekFrom::Start(0)).await?;
    } else {
        file.seek(SeekFrom::End(0)).await?;
    }

    let mut reader = BufReader::new(file);
    let mut buffer = String::new();

    println!("{}", messages.logs_tail_following(&args.server, &target.file_name));

    loop {
        buffer.clear();
        let bytes = reader.read_line(&mut buffer).await?;
        if bytes == 0 {
            sleep(StdDuration::from_millis(500)).await;
            continue;
        }
        if buffer.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<LogEntry>(buffer.trim_end()) {
            Ok(entry) => {
                print_log_entry(&entry, false);
                let _ = io::stdout().flush();
            }
            Err(err) => {
                eprintln!("{} {}", messages.error_prefix(), err);
            }
        }
    }
}

// Helper function to load project record by path or ID
fn load_project_record(registry: &ProjectRegistry, target: &str) -> Result<ProjectRecord> {
    let messages = i18n::messages();

    // Try as path first
    let path = expand_tilde(PathBuf::from(target))?;
    if path.exists() {
        if let Ok(Some(record)) = registry.find_by_path(&path) {
            return Ok(record);
        }
    }

    // Try as project ID
    match registry.load_from_id_str(target) {
        Ok(record) => Ok(record),
        Err(_) => bail!("{}", messages.project_not_found(target)),
    }
}

fn select_log_file<'a>(
    files: &'a [LogFileMeta],
    requested: Option<&str>,
    server: &str,
) -> Result<&'a LogFileMeta> {
    if let Some(name) = requested {
        files
            .iter()
            .find(|meta| meta.file_name == name)
            .ok_or_else(|| anyhow!(i18n::messages().logs_file_not_found(server, name)))
    } else {
        files.last().ok_or_else(|| anyhow!(i18n::messages().logs_no_files_for(server)))
    }
}

fn format_size(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;

    let value = bytes as f64;
    if value >= GB {
        format!("{:.2} GB", value / GB)
    } else if value >= MB {
        format!("{:.2} MB", value / MB)
    } else if value >= KB {
        format!("{:.2} KB", value / KB)
    } else {
        format!("{bytes} B")
    }
}

fn format_range(meta: &LogFileMeta) -> String {
    match (&meta.first_timestamp, &meta.last_timestamp) {
        (Some(start), Some(end)) if start == end => start.clone(),
        (Some(start), Some(end)) => format!("{start} → {end}"),
        (Some(start), None) => start.clone(),
        (None, Some(end)) => end.clone(),
        _ => "-".to_string(),
    }
}

fn print_log_entry(entry: &LogEntry, as_json: bool) {
    if as_json {
        match serde_json::to_string(entry) {
            Ok(line) => println!("{line}"),
            Err(err) => eprintln!("{} {}", i18n::messages().error_prefix(), err),
        }
        return;
    }

    let level = format_level(entry.level);
    let mut context_parts = Vec::new();
    if let Some(server) = entry.server.as_ref() {
        context_parts.push(format!("server={}", server.id));
    }
    if let Some(tool) = entry.tool.as_ref() {
        context_parts.push(format!("tool={}", tool.name));
        context_parts.push(format!("call={}", truncate_call_id(&tool.call_id)));
    }
    if let Some(duration) = entry.duration_ms {
        context_parts.push(format!("duration={}", format_duration(duration)));
    }

    if context_parts.is_empty() {
        println!("{} [{}] {}", entry.timestamp, level, entry.message);
    } else {
        println!(
            "{} [{}] {} ({})",
            entry.timestamp,
            level,
            entry.message,
            context_parts.join(", ")
        );
    }

    if let Some(details) = entry.details.as_ref() {
        if !details.is_null() {
            println!("    details:");
            match serde_json::to_string_pretty(details) {
                Ok(json) => {
                    for line in json.lines() {
                        println!("      {line}");
                    }
                }
                Err(_) => println!("      {details}"),
            }
        }
    }
}

fn format_level(level: LogLevel) -> &'static str {
    match level {
        LogLevel::Trace => "TRACE",
        LogLevel::Debug => "DEBUG",
        LogLevel::Info => "INFO",
        LogLevel::Warn => "WARN",
        LogLevel::Error => "ERROR",
    }
}

fn truncate_call_id(call_id: &str) -> String {
    const MAX_LEN: usize = 8;
    if call_id.len() <= MAX_LEN {
        call_id.to_string()
    } else {
        format!("{}…", &call_id[..MAX_LEN])
    }
}

fn format_duration(ms: u128) -> String {
    if ms >= 1000 {
        let seconds = ms as f64 / 1000.0;
        format!("{seconds:.2}s")
    } else {
        format!("{ms}ms")
    }
}
