//! MCP Center bridge connection implementation

use std::{
    env,
    io::ErrorKind,
    path::{Path, PathBuf},
    process::{Command as StdCommand, Stdio},
    time::Duration,
};

use crate::{Layout, ProjectId, bridge::control::ControlMessage, default_root};
use anyhow::{Context, Result, bail};
use clap::Args;
use interprocess::local_socket::traits::tokio::Stream as _;
use interprocess::local_socket::{GenericFilePath, ToFsName, tokio::prelude::LocalSocketStream};
use serde_json::{Value, json};
use tokio::{
    io::{self, AsyncBufReadExt, AsyncWriteExt},
    process::Command as TokioCommand,
};
use tracing::{debug, info, warn};

#[derive(Args, Debug)]
pub struct ConnectArgs {
    #[arg(
        long,
        value_name = "DIR",
        help = "Override the MCP Center root directory."
    )]
    pub root: Option<PathBuf>,

    #[arg(
        long,
        value_name = "PATH",
        help = "Path to the mcp-center executable (for spawning serve mode)."
    )]
    pub daemon: Option<PathBuf>,
}

pub async fn run(args: ConnectArgs) -> Result<()> {
    init_tracing();

    run_impl(args).await
}

async fn run_impl(args: ConnectArgs) -> Result<()> {
    let layout = resolve_layout(args.root.clone())?;
    layout.ensure()?;

    let project_path = detect_project_path().await?;
    let project_id = ProjectId::from_path(&project_path);
    debug!(project = %project_id.as_str(), path = %project_path.display(), "detected project path");

    let mut stream = connect_or_launch(&layout, &args).await?;

    perform_handshake(&mut stream, &project_path).await?;

    tunnel_stdio(stream).await
}

async fn perform_handshake(stream: &mut LocalSocketStream, project_path: &Path) -> Result<()> {
    let metadata = gather_metadata().await?;
    let hello = ControlMessage::hello(
        project_path.to_path_buf(),
        detect_agent_name(),
        Some(std::process::id()),
        metadata,
    );

    let mut payload = serde_json::to_vec(&hello)?;
    payload.push(b'\n');
    stream.write_all(&payload).await?;

    let mut reader = tokio::io::BufReader::new(&mut *stream);
    let mut response = String::new();
    let read = reader.read_line(&mut response).await?;
    if read == 0 {
        bail!("daemon closed control channel before responding");
    }
    let message: ControlMessage =
        serde_json::from_str(response.trim()).context("invalid response from daemon")?;

    match message {
        ControlMessage::BridgeReady(ready) => {
            info!(project = ready.project_id, servers = ?ready.allowed_server_ids, "connected to daemon");
            Ok(())
        }
        ControlMessage::Error { message } => bail!("daemon rejected connection: {message}"),
        other => bail!("unexpected control response: {other:?}"),
    }
}

async fn tunnel_stdio(stream: LocalSocketStream) -> Result<()> {
    use tokio::io::{AsyncReadExt, BufReader};

    let (read_half, mut write_half) = tokio::io::split(stream);
    let mut stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(read_half);

    let upstream = tokio::spawn(async move {
        let mut buffer = [0u8; 8192];
        loop {
            let read = stdin.read(&mut buffer).await?;
            if read == 0 {
                write_half.shutdown().await?;
                break;
            }
            write_half.write_all(&buffer[..read]).await?;
        }
        io::Result::Ok(())
    });

    let downstream = tokio::spawn(async move {
        tokio::io::copy(&mut reader, &mut stdout).await?;
        stdout.flush().await?;
        io::Result::Ok(())
    });

    tokio::select! {
        result = upstream => result??,
        result = downstream => result??,
        _ = tokio::signal::ctrl_c() => {
            warn!("received Ctrl+C, closing bridge");
        }
    }

    Ok(())
}

async fn connect_or_launch(layout: &Layout, args: &ConnectArgs) -> Result<LocalSocketStream> {
    let socket_path = layout.daemon_socket_path();
    let socket_name = socket_path.to_string_lossy().into_owned();

    match LocalSocketStream::connect(socket_name.as_str().to_fs_name::<GenericFilePath>()?).await {
        Ok(stream) => return Ok(stream),
        Err(err) if matches!(err.kind(), ErrorKind::NotFound | ErrorKind::ConnectionRefused) => {
            #[cfg(unix)]
            if err.kind() == ErrorKind::ConnectionRefused {
                warn!("control socket present but no listener, removing stale socket");
                let _ = std::fs::remove_file(&socket_path);
            }
            spawn_daemon(layout, args)?;
        }
        Err(err) => return Err(err.into()),
    }

    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    loop {
        match LocalSocketStream::connect(socket_name.as_str().to_fs_name::<GenericFilePath>()?)
            .await
        {
            Ok(stream) => return Ok(stream),
            Err(err)
                if matches!(err.kind(), ErrorKind::NotFound | ErrorKind::ConnectionRefused) =>
            {
                if tokio::time::Instant::now() > deadline {
                    bail!("timed out while waiting for daemon socket");
                }
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
            Err(err) => return Err(err.into()),
        }
    }
}

fn spawn_daemon(layout: &Layout, args: &ConnectArgs) -> Result<()> {
    // 默认使用当前可执行文件自身（单一二进制模式）
    let daemon_path = args
        .daemon
        .clone()
        .or_else(|| env::var_os("MCP_CENTER_DAEMON").map(PathBuf::from))
        .or_else(|| env::current_exe().ok())
        .unwrap_or_else(|| PathBuf::from("mcp-center"));

    let mut command = StdCommand::new(&daemon_path);

    // 添加 "serve" 子命令
    command.arg("serve");

    // 保留 stderr 用于调试,但重定向到临时日志文件
    let log_dir = layout.logs_dir();
    std::fs::create_dir_all(log_dir)?;
    let log_file = log_dir.join("daemon-startup.log");
    let stderr_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file)
        .with_context(|| format!("failed to open log file {}", log_file.display()))?;

    command.stdout(Stdio::null());
    command.stderr(stderr_file);
    command.stdin(Stdio::null());

    if let Some(root) = args.root.as_ref() {
        command.arg("--root").arg(root);
    }

    configure_detached_process(&mut command)?;

    command
        .spawn()
        .with_context(|| format!("failed to spawn daemon using {}", daemon_path.display()))?;
    info!(path = %daemon_path.display(), log = %log_file.display(), "spawned mcp-center serve");

    Ok(())
}

#[cfg(unix)]
fn configure_detached_process(command: &mut StdCommand) -> Result<()> {
    use std::os::unix::process::CommandExt;

    unsafe {
        command.pre_exec(|| {
            libc::setsid();
            Ok(())
        });
    }

    Ok(())
}

#[cfg(not(unix))]
fn configure_detached_process(_command: &mut StdCommand) -> Result<()> {
    Ok(())
}

async fn detect_project_path() -> Result<PathBuf> {
    debug!("=== DEBUG: Starting project path detection ===");

    // Log ALL environment variables for complete diagnosis
    debug!("ALL environment variables:");
    let mut env_vars: Vec<_> = env::vars().collect();
    env_vars.sort_by(|a, b| a.0.cmp(&b.0));
    for (key, value) in env_vars {
        debug!("  {} = {}", key, value);
    }

    debug!("---");

    // Log all potentially relevant environment variables with focus
    let interesting_env_vars = [
        "MCP_CENTER_PROJECT_PATH",
        "PROJECT_ROOT",
        "WORKSPACE_ROOT",
        "PWD",
        "OLDPWD",
        "HOME",
        "USER",
        "CURSOR_AGENT",
        "WINDSURF_AGENT",
        "MCP_AGENT_NAME",
        "VSCODE_CWD",
        "IDEA_INITIAL_DIRECTORY",
        "EDITOR",
        "VISUAL",
        "CURSOR_WORKSPACE",
        "VSCODE_WORKSPACE",
    ];

    debug!("Key environment variables:");
    for var in &interesting_env_vars {
        if let Ok(value) = env::var(var) {
            debug!("  {} = {}", var, value);
        } else {
            debug!("  {} = <not set>", var);
        }
    }

    // Log current process info
    debug!("Process info:");
    debug!("  pid = {}", std::process::id());
    if let Ok(exe) = env::current_exe() {
        debug!("  exe = {}", exe.display());
    }
    if let Ok(cwd) = env::current_dir() {
        debug!("  cwd = {}", cwd.display());
    }

    // Check for MCP_CENTER_PROJECT_PATH override
    if let Some(env_override) = env::var_os("MCP_CENTER_PROJECT_PATH") {
        let path = PathBuf::from(&env_override);
        debug!("Found MCP_CENTER_PROJECT_PATH override: {}", path.display());
        let canonical = canonicalize_best_effort(&path).await.unwrap_or_else(|| path.clone());
        debug!("Resolved to: {}", canonical.display());
        return Ok(canonical);
    }

    let cwd = env::current_dir().context("failed to determine current directory")?;
    debug!("Base CWD: {}", cwd.display());

    // Try marker-based detection
    debug!("Probing for project markers...");
    if let Some(marker_dir) = probe_markers(&cwd).await? {
        debug!("Found project via marker: {}", marker_dir.display());
        return Ok(marker_dir);
    }
    debug!("No project markers found");

    // Try git toplevel
    debug!("Checking for git repository...");
    if let Some(git_root) = git_toplevel(&cwd).await? {
        debug!("Found git root: {}", git_root.display());
        return Ok(git_root);
    }
    debug!("Not in a git repository");

    let final_path = canonicalize_best_effort(&cwd).await.unwrap_or_else(|| cwd.clone());
    debug!("Falling back to CWD: {}", final_path.display());
    debug!("=== DEBUG: Project path detection complete ===");
    Ok(final_path)
}

async fn probe_markers(base: &Path) -> Result<Option<PathBuf>> {
    const FILE_MARKERS: &[&str] = &[".cursor/settings.json", "cursor.json", ".windsurfrc"];

    const DIR_MARKERS: &[&str] = &[".cursor", ".windsurf"];

    for ancestor in base.ancestors() {
        for marker in FILE_MARKERS {
            if ancestor.join(marker).exists() {
                return Ok(canonicalize_best_effort(ancestor)
                    .await
                    .or_else(|| Some(ancestor.to_path_buf())));
            }
        }
        for marker in DIR_MARKERS {
            if ancestor.join(marker).exists() {
                return Ok(canonicalize_best_effort(ancestor)
                    .await
                    .or_else(|| Some(ancestor.to_path_buf())));
            }
        }
    }
    Ok(None)
}

async fn git_toplevel(base: &Path) -> Result<Option<PathBuf>> {
    let output = TokioCommand::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(base)
        .output()
        .await;

    match output {
        Ok(result) if result.status.success() => {
            let raw = String::from_utf8_lossy(&result.stdout).trim().to_string();
            let path = PathBuf::from(raw);
            Ok(Some(canonicalize_best_effort(&path).await.unwrap_or(path)))
        }
        Ok(_) => Ok(None),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err.into()),
    }
}

async fn canonicalize_best_effort(path: &Path) -> Option<PathBuf> {
    match tokio::fs::canonicalize(path).await {
        Ok(canonical) => Some(canonical),
        Err(err) => {
            debug!(path = %path.display(), error = ?err, "failed to canonicalize path");
            None
        }
    }
}

fn detect_agent_name() -> Option<String> {
    let env_keys = ["MCP_AGENT_NAME", "CURSOR_AGENT", "WINDSURF_AGENT"];

    for key in env_keys {
        if let Some(value) = env::var_os(key) {
            let trimmed = value.to_string_lossy().trim().to_string();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
    }
    None
}

async fn gather_metadata() -> Result<Value> {
    let cwd = env::current_dir().ok();
    let exe = env::current_exe().ok();
    let mut meta = json!({
        "pid": std::process::id(),
    });

    if let Some(cwd) = cwd {
        meta["cwd"] = Value::String(cwd.display().to_string());
    }
    if let Some(exe) = exe {
        meta["exe"] = Value::String(exe.display().to_string());
    }

    Ok(meta)
}

fn resolve_layout(root_override: Option<PathBuf>) -> Result<Layout> {
    let root = match root_override {
        Some(path) => path,
        None => default_root()?,
    };
    Ok(Layout::new(root))
}

fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt, prelude::*};

    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("warn"))
        .unwrap();

    // IMPORTANT: Force log output to stderr to avoid interfering with MCP protocol on stdout
    let fmt_layer = fmt::layer().with_target(true).with_writer(std::io::stderr);

    tracing_subscriber::registry().with(env_filter).with(fmt_layer).init();
}
