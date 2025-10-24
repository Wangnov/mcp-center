//! MCP Center daemon service implementation

use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use crate::{Layout, ProjectRegistry, default_root};
use anyhow::{Context, Result};
use clap::Args;
use tokio::signal;
use tracing::{debug, error, info, warn};
use tracing_appender::{non_blocking::WorkerGuard, rolling};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use crate::{
    daemon::{control, rpc::RpcServer, server_manager::ServerManager},
    web::http::{self, HttpState},
};

#[derive(Args, Debug)]
pub struct ServeArgs {
    /// Override the workspace root directory.
    #[arg(long)]
    pub root: Option<PathBuf>,
    /// Bind address for the HTTP API (e.g. 127.0.0.1:8787).
    #[arg(long, value_name = "ADDR", help = "i18n:args.serve.http_bind")]
    pub http_bind: Option<SocketAddr>,
    /// Authentication token required for HTTP API (fallback env MCP_CENTER_HTTP_TOKEN).
    #[arg(long, value_name = "TOKEN")]
    pub http_auth_token: Option<String>,
}

pub async fn run(mut args: ServeArgs) -> Result<()> {
    let layout = resolve_layout(args.root.clone())?;
    layout.ensure()?;
    let _tracing_guard = init_tracing(&layout)?;

    if let Err(err) = run_impl(layout, &mut args).await {
        error!(error = ?err, "daemon terminated with error");
        return Err(err);
    }
    info!("daemon exited cleanly");
    Ok(())
}

async fn run_impl(layout: Layout, args: &mut ServeArgs) -> Result<()> {
    let ServeArgs { http_bind, http_auth_token, .. } = args;
    let http_bind = *http_bind;
    let mut http_auth_token = http_auth_token.take();
    if http_auth_token.is_none() {
        http_auth_token = std::env::var("MCP_CENTER_HTTP_TOKEN").ok();
    }

    let registry = ProjectRegistry::new(&layout);
    registry.ensure()?;

    let manager = Arc::new(ServerManager::start(layout.clone()).await?);
    let control_handle =
        control::spawn_control_server(layout.clone(), registry.clone(), manager.clone()).await?;

    // Start RPC server for CLI communication
    let rpc_socket_path = layout.daemon_rpc_socket_path();
    let rpc_server = RpcServer::new(manager.clone(), rpc_socket_path.clone());
    let rpc_handle = tokio::spawn(async move {
        if let Err(e) = rpc_server.start().await {
            error!("RPC server error: {}", e);
        }
    });

    let http_handle = if let Some(addr) = http_bind {
        let state = HttpState {
            manager: manager.clone(),
            registry: registry.clone(),
            layout: layout.clone(),
            auth: http::HttpAuth::new(http_auth_token.clone()),
        };
        Some(http::spawn_http_server(state, addr).await?)
    } else {
        None
    };

    info!(
        servers = manager.server_count(),
        rpc_socket = %rpc_socket_path.display(),
        http_addr = http_handle.as_ref().map(|h| h.addr().to_string()),
        "daemon ready, control socket and RPC socket listening"
    );

    // 在 bridge 模式下,daemon 只监听 control socket,不在 stdin/stdout 上建立会话
    // 等待 Ctrl+C 信号来优雅关闭
    let ctrl_c = signal::ctrl_c();
    tokio::pin!(ctrl_c);

    match (&mut ctrl_c).await {
        Ok(()) => {
            info!("received Ctrl+C, shutting down daemon");
        }
        Err(err) => {
            warn!(error = ?err, "failed to listen for Ctrl+C");
        }
    }

    manager.shutdown().await;
    control_handle.shutdown().await;
    rpc_handle.abort();

    if let Some(handle) = http_handle {
        handle.shutdown();
    }

    // Clean up RPC socket
    if rpc_socket_path.exists() {
        let _ = std::fs::remove_file(&rpc_socket_path);
    }

    info!("daemon stopped cleanly");
    Ok(())
}

fn resolve_layout(root_override: Option<PathBuf>) -> Result<Layout> {
    let root = match root_override {
        Some(path) => expand_tilde(path)?,
        None => default_root()?,
    };
    debug!(root = %root.display(), "resolved workspace root");
    Ok(Layout::new(root))
}

fn expand_tilde(path: PathBuf) -> Result<PathBuf> {
    if let Some(str_path) = path.to_str() {
        if let Some(stripped) = str_path.strip_prefix('~') {
            let home = dirs_home().context("cannot expand '~', HOME unset")?;
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

fn init_tracing(layout: &Layout) -> Result<WorkerGuard> {
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    let stderr_layer = fmt::layer()
        .with_target(true)
        .with_file(true)
        .with_line_number(true)
        .with_writer(std::io::stderr);

    let daemon_log_dir = layout.logs_dir().join("daemon");
    std::fs::create_dir_all(&daemon_log_dir).with_context(|| {
        format!("failed to create daemon log directory {}", daemon_log_dir.display())
    })?;
    let file_appender = rolling::hourly(daemon_log_dir, "daemon.log");
    let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

    let file_layer = fmt::layer()
        .with_target(true)
        .with_file(true)
        .with_line_number(true)
        .json()
        .with_writer(file_writer);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(stderr_layer)
        .with(file_layer)
        .init();

    Ok(guard)
}
