use std::{
    path::PathBuf,
    sync::{Arc, RwLock},
};

use crate::{Layout, ProjectId, ProjectRecord, ProjectRegistry};
use anyhow::{Context, Result};
use rmcp::{ServiceExt as _, service::RoleServer, transport::async_rw::AsyncRwTransport};
use serde::{Deserialize, Serialize};
use serde_json::{self, Value};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    task::JoinHandle,
};
use tracing::{debug, error, info, warn};

use crate::daemon::{host::HostService, server_manager::ServerManager};

/// Message sent by `mcp-center-bridge` upon establishing a control connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeHello {
    pub project_path: PathBuf,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub bridge_pid: Option<u32>,
    #[serde(default)]
    pub metadata: Value,
}

/// Response returned by the daemon after registering the bridge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeReady {
    pub project_id: String,
    pub allowed_server_ids: Vec<String>,
    #[serde(default)]
    pub project_path: PathBuf,
}

/// Line-oriented control protocol messages.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlMessage {
    BridgeHello(BridgeHello),
    BridgeReady(BridgeReady),
    Error { message: String },
}

impl ControlMessage {
    #[allow(dead_code)]
    pub fn hello(
        project_path: PathBuf,
        agent: Option<String>,
        bridge_pid: Option<u32>,
        metadata: Value,
    ) -> Self {
        ControlMessage::BridgeHello(BridgeHello { project_path, agent, bridge_pid, metadata })
    }

    pub fn ready(
        project_id: String,
        project_path: PathBuf,
        allowed_server_ids: Vec<String>,
    ) -> Self {
        ControlMessage::BridgeReady(BridgeReady { project_id, allowed_server_ids, project_path })
    }

    pub fn error(message: impl Into<String>) -> Self {
        ControlMessage::Error { message: message.into() }
    }
}

pub enum ControlServerHandle {
    Active {
        task: JoinHandle<()>,
        socket_path: PathBuf,
    },
    #[cfg(not(unix))]
    Disabled,
}

impl ControlServerHandle {
    pub async fn shutdown(self) {
        match self {
            ControlServerHandle::Active { task, socket_path } => {
                task.abort();
                let _ = task.await;
                #[cfg(unix)]
                {
                    if let Err(err) = tokio::fs::remove_file(&socket_path).await {
                        if err.kind() != std::io::ErrorKind::NotFound {
                            warn!(error = ?err, path = %socket_path.display(), "failed to remove control socket");
                        }
                    }
                }
            }
            #[cfg(not(unix))]
            ControlServerHandle::Disabled => {}
        }
    }
}

pub async fn spawn_control_server(
    layout: Layout,
    registry: ProjectRegistry,
    manager: Arc<ServerManager>,
) -> Result<ControlServerHandle> {
    #[cfg(unix)]
    {
        use tokio::net::UnixListener;

        let socket_path = layout.daemon_socket_path();
        if tokio::fs::metadata(&socket_path).await.is_ok() {
            tokio::fs::remove_file(&socket_path).await.with_context(|| {
                format!("failed to remove stale control socket {}", socket_path.display())
            })?;
        }

        let listener = UnixListener::bind(&socket_path)
            .with_context(|| format!("failed to bind control socket {}", socket_path.display()))?;
        info!(path = %socket_path.display(), "control socket listening");

        let task = tokio::spawn(run_listener(listener, layout, registry, manager));
        Ok(ControlServerHandle::Active { task, socket_path })
    }

    #[cfg(not(unix))]
    {
        warn!("control socket disabled: unix domain sockets not supported on this platform");
        Ok(ControlServerHandle::Disabled)
    }
}

#[cfg(unix)]
async fn run_listener(
    listener: tokio::net::UnixListener,
    layout: Layout,
    registry: ProjectRegistry,
    manager: Arc<ServerManager>,
) {
    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let layout = layout.clone();
                let registry = registry.clone();
                let manager = manager.clone();
                tokio::spawn(async move {
                    if let Err(err) = handle_stream(stream, layout, registry, manager).await {
                        warn!(error = ?err, "control session ended with error");
                    }
                });
            }
            Err(err) => {
                error!(error = ?err, "control socket accept failed");
                break;
            }
        }
    }
}

#[cfg(unix)]
async fn handle_stream(
    stream: tokio::net::UnixStream,
    layout: Layout,
    registry: ProjectRegistry,
    manager: Arc<ServerManager>,
) -> Result<()> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    let bytes = reader
        .read_line(&mut line)
        .await
        .context("failed to read handshake from bridge")?;
    if bytes == 0 {
        anyhow::bail!("bridge closed connection during handshake");
    }

    let message: ControlMessage =
        serde_json::from_str(line.trim()).context("invalid control message")?;

    let BridgeHello { project_path: raw_path, agent, bridge_pid, metadata } = match message {
        ControlMessage::BridgeHello(hello) => hello,
        other => {
            let err = ControlMessage::error("expected bridge_hello message");
            let payload = serde_json::to_vec(&err)?;
            reader.get_mut().write_all(&payload).await?;
            reader.get_mut().write_all(b"\n").await?;
            anyhow::bail!("unexpected control message: {other:?}");
        }
    };

    debug!("=== DEBUG: Received BridgeHello ===");
    debug!("  raw_path: {}", raw_path.display());
    debug!("  agent: {:?}", agent);
    debug!("  bridge_pid: {:?}", bridge_pid);
    debug!(
        "  metadata: {}",
        serde_json::to_string_pretty(&metadata).unwrap_or_else(|_| "{}".to_string())
    );

    let project_path = normalize_project_path(&raw_path).await.unwrap_or_else(|| {
        debug!("Failed to normalize path, using raw path");
        raw_path.clone()
    });
    debug!("  normalized_path: {}", project_path.display());

    // 先用初始路径生成临时 project_id（可能不准确）
    let initial_project_id = ProjectId::from_path(&project_path);
    debug!("  initial project_id: {}", initial_project_id.as_str());

    let mut record = match registry.load(&initial_project_id) {
        Ok(record) => {
            // 已存在的项目，保留用户设置
            record
        }
        Err(_) => {
            // 新项目，默认允许所有服务器
            let mut new_record =
                ProjectRecord::new(initial_project_id.clone(), project_path.clone());
            new_record.allowed_server_ids = manager.list_server_ids();
            new_record
        }
    };
    record.touch();
    if let Some(agent) = agent {
        record.set_agent(Some(agent.clone()));
        record.metadata.insert("agent".to_string(), agent);
    }
    if let Some(pid) = bridge_pid {
        record.metadata.insert("bridge_pid".to_string(), pid.to_string());
    }
    if let Value::Object(entries) = metadata {
        for (key, value) in entries {
            record.metadata.insert(format!("meta_{key}"), value.to_string());
        }
    }
    if record.path != project_path {
        record.path = project_path.clone();
    }
    registry.store(&record)?;

    let ready = ControlMessage::ready(
        record.id.clone(),
        project_path.clone(),
        record.allowed_server_ids.clone(),
    );
    let payload = serde_json::to_vec(&ready)?;
    let mut stream = reader.into_inner();
    stream.write_all(&payload).await?;
    stream.write_all(b"\n").await?;

    let (read_half, write_half) = stream.into_split();
    let transport = AsyncRwTransport::<RoleServer, _, _>::new_server(read_half, write_half);

    // 使用 Arc<RwLock> 包装 project_id，允许后续更新
    let project_id_lock = Arc::new(RwLock::new(initial_project_id.clone()));
    let host_service = HostService::new(
        manager.clone(),
        layout.clone(),
        project_id_lock.clone(),
        registry.clone(),
    );
    let project_id_str = record.id.clone();

    match host_service.serve(transport).await {
        Ok(running) => {
            let project = project_id_str.clone();
            let peer = running.peer().clone();

            // IMPORTANT: Synchronously fetch roots before spawn to determine the real project_id
            debug!("=== DEBUG: Attempting to list roots from client ===");
            match peer.list_roots().await {
                Ok(roots_result) => {
                    debug!("  Successfully got roots from client!");
                    debug!("  Number of roots: {}", roots_result.roots.len());
                    if let Some(root) = roots_result.roots.first() {
                        debug!("  Root URI: {}", root.uri);
                        debug!("  Root name: {:?}", root.name);

                        // 解析file:// URI
                        if let Some(real_path) = parse_file_uri(&root.uri) {
                            debug!("  Parsed real path: {}", real_path.display());

                            // 基于真实路径重新计算 project_id
                            let real_project_id = ProjectId::from_path(&real_path);
                            debug!("  Real project_id: {}", real_project_id.as_str());

                            // Update the project_id in HostService
                            {
                                let mut pid = project_id_lock.write().unwrap();
                                *pid = real_project_id.clone();
                                info!(
                                    "Updated HostService project_id: {} -> {}",
                                    initial_project_id.as_str(),
                                    real_project_id.as_str()
                                );
                            }

                            // 如果真实 ID 和初始 ID 不同，需要迁移记录
                            if real_project_id.as_str() != initial_project_id.as_str() {
                                info!(
                                    "Project ID mismatch: initial={}, real={}. Migrating...",
                                    initial_project_id.as_str(),
                                    real_project_id.as_str()
                                );

                                // 检查真实 ID 是否已存在
                                match registry.load(&real_project_id) {
                                    Ok(mut existing_record) => {
                                        // 真实 ID 已存在，更新它
                                        info!("Real project ID already exists, updating it");
                                        existing_record.path = real_path.clone();
                                        existing_record.touch();
                                        // 保留现有的服务器权限配置
                                        if let Err(e) = registry.store(&existing_record) {
                                            warn!("Failed to update real project record: {}", e);
                                        } else {
                                            info!("Updated real project record");
                                        }
                                    }
                                    Err(_) => {
                                        // 真实 ID 不存在，创建新记录
                                        info!("Creating new record with real project ID");
                                        let mut new_record = ProjectRecord::new(
                                            real_project_id.clone(),
                                            real_path.clone(),
                                        );
                                        // 继承初始记录的服务器权限配置
                                        new_record.allowed_server_ids =
                                            record.allowed_server_ids.clone();
                                        new_record.agent = record.agent.clone();
                                        new_record.metadata = record.metadata.clone();
                                        new_record.touch();
                                        if let Err(e) = registry.store(&new_record) {
                                            warn!("Failed to create real project record: {}", e);
                                        } else {
                                            info!("Created real project record");
                                        }
                                    }
                                }

                                // 删除错误的初始记录（如果它是因为路径检测失败创建的）
                                // 只在初始路径是父目录时删除
                                if initial_project_id.as_str() != real_project_id.as_str()
                                    && real_path.starts_with(&project_path)
                                {
                                    info!(
                                        "Deleting incorrect initial record {}",
                                        initial_project_id.as_str()
                                    );
                                    if let Err(e) = registry.delete(&initial_project_id) {
                                        warn!("Failed to delete incorrect record: {}", e);
                                    } else {
                                        info!("Deleted incorrect initial record");
                                    }
                                }
                            } else {
                                // ID 相同，只需要更新路径（如果不同）
                                if let Ok(mut record) = registry.load(&real_project_id) {
                                    if record.path != real_path {
                                        let old_path = record.path.clone();
                                        record.path = real_path.clone();
                                        if let Err(e) = registry.store(&record) {
                                            warn!("Failed to update project path: {}", e);
                                        } else {
                                            info!(
                                                "Updated project path: {} -> {}",
                                                old_path.display(),
                                                record.path.display()
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    debug!("  Failed to list roots: {:?}", e);
                    debug!("  Will use the path from BridgeHello");
                }
            }
            debug!("=== DEBUG: Roots listing complete ===");

            // 现在spawn session处理
            tokio::spawn(async move {
                match running.waiting().await {
                    Ok(reason) => info!(project = %project, ?reason, "bridge session closed"),
                    Err(err) => {
                        warn!(project = %project, error = ?err, "bridge session join error")
                    }
                }
            });
        }
        Err(err) => {
            warn!(project = %project_id_str, error = ?err, "failed to establish MCP session for bridge");
        }
    }

    Ok(())
}

#[cfg(unix)]
async fn normalize_project_path(path: &PathBuf) -> Option<PathBuf> {
    match tokio::fs::canonicalize(path).await {
        Ok(canonical) => Some(canonical),
        Err(err) => {
            warn!(path = %path.display(), error = ?err, "failed to canonicalize project path");
            None
        }
    }
}

/// Parse a file:// URI to a local filesystem path
/// Examples:
///   file:///Users/wangnov/project -> /Users/wangnov/project
///   file://localhost/Users/wangnov/project -> /Users/wangnov/project
fn parse_file_uri(uri: &str) -> Option<PathBuf> {
    let uri = uri.trim();

    // Handle file:// scheme
    if let Some(stripped) = uri.strip_prefix("file://") {
        // Remove optional localhost
        let path_part = stripped.strip_prefix("localhost").unwrap_or(stripped);

        // The path should start with /
        if path_part.starts_with('/') {
            return Some(PathBuf::from(path_part));
        }
    }

    None
}

#[cfg(not(unix))]
async fn normalize_project_path(path: &PathBuf) -> Option<PathBuf> {
    Some(path.clone())
}
