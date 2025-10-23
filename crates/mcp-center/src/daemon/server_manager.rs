use std::{
    borrow::Cow,
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, RwLock as SyncRwLock,
        atomic::{AtomicBool, Ordering},
    },
};

use crate::{Layout, ServerDefinition, ServerProtocol};
use anyhow::{Context, Result, anyhow};
use rmcp::{
    ErrorData as McpError,
    model::{
        CallToolRequestMethod, CallToolRequestParam, CallToolResult, ClientResult,
        ServerNotification, ServerRequest, Tool, ToolListChangedNotification,
    },
    service::{RoleClient, Service, ServiceError, ServiceExt},
    transport::{
        SseClientTransport, StreamableHttpClientTransport,
        child_process::{ConfigureCommandExt, TokioChildProcess},
    },
};
use tokio::{
    fs::{self, OpenOptions},
    sync::{Mutex, RwLock},
};
use tracing::{debug, info, warn};

use serde::Serialize;
use serde_json::to_vec;
use specta::Type;
use tokio::io::AsyncWriteExt;

#[derive(Clone, Debug)]
pub struct ToolEntry {
    pub server_id: String,
    pub server_name: String,
    pub tool: Tool,
}

impl ToolEntry {
    pub fn into_tool(self) -> Tool {
        let mut tool = self.tool.clone();
        let provider_note = format!(
            "\n[provided by {name} (id: {id})]",
            name = self.server_name,
            id = self.server_id
        );

        let merged = match tool.description.as_ref() {
            Some(desc) => {
                let mut base = desc.to_string();
                if !base.contains("[provided by") {
                    base.push_str(&provider_note);
                }
                base
            }
            None => format!(
                "Provided by {name} (id: {id}).",
                name = self.server_name,
                id = self.server_id
            ),
        };

        tool.description = Some(Cow::Owned(merged));
        tool
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ManagedServerKind {
    LocalProcess,
    Remote,
}

pub struct ServerManager {
    layout: Layout,
    servers: SyncRwLock<HashMap<String, Arc<ManagedServer>>>,
    tool_cache: RwLock<Vec<ToolEntry>>,
    tool_index: RwLock<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ServerSnapshot {
    pub id: String,
    pub name: String,
    pub protocol: ServerProtocol,
    pub enabled: bool,
    #[specta(type = u32)]
    pub tool_count: usize,
    pub created_at: Option<u64>,
    pub last_seen: Option<u64>,
}

impl ServerManager {
    pub async fn start(layout: Layout) -> Result<Self> {
        let mut servers = HashMap::new();
        let configs =
            layout.list_server_configs().context("failed to list server configurations")?;
        let enabled: Vec<_> = configs.into_iter().filter(|cfg| cfg.definition().enabled).collect();

        // Allow daemon to start even with no enabled servers
        // Users can add servers later through CLI, Desktop, or WebUI
        if enabled.is_empty() {
            info!("no enabled MCP servers found; daemon will start with empty server list");
        } else {
            for config in enabled {
                let handle = ManagedServer::launch(&layout, config.definition().clone()).await?;
                servers.insert(config.definition().id.clone(), handle);
            }
        }

        let manager = Self {
            layout,
            servers: SyncRwLock::new(servers),
            tool_cache: RwLock::new(Vec::new()),
            tool_index: RwLock::new(HashMap::new()),
        };

        // Refresh tool cache (will be empty if no servers)
        manager.force_refresh_tool_cache().await?;

        Ok(manager)
    }

    pub fn server_count(&self) -> usize {
        self.servers.read().unwrap().len()
    }

    pub fn list_server_ids(&self) -> Vec<String> {
        self.servers.read().unwrap().keys().cloned().collect()
    }

    pub fn list_server_names(&self) -> Vec<String> {
        self.servers
            .read()
            .unwrap()
            .values()
            .map(|server| server.display_name())
            .collect()
    }

    pub async fn list_servers(&self) -> Vec<ServerSnapshot> {
        let handles = {
            let guard = self.servers.read().unwrap();
            guard.values().cloned().collect::<Vec<_>>()
        };
        let mut snapshots = Vec::with_capacity(handles.len());
        for server in handles {
            snapshots.push(server.snapshot().await);
        }
        snapshots
    }

    pub async fn list_tools(&self) -> Result<Vec<ToolEntry>> {
        self.ensure_tool_cache().await?;
        let snapshot = self.tool_cache.read().await;
        Ok(snapshot.clone())
    }

    /// Ensure the given server is running; returns true if it was started.
    pub async fn ensure_server_running(&self, server_id: &str) -> Result<bool> {
        if self.servers.read().unwrap().contains_key(server_id) {
            return Ok(false);
        }

        let config = self.layout.load_server_config(server_id)?;
        let definition = config.definition().clone();
        if !definition.enabled {
            debug!(server_id, "requested to start server that is disabled in config");
        }

        let handle = ManagedServer::launch(&self.layout, definition).await?;
        {
            let mut guard = self.servers.write().unwrap();
            guard.insert(server_id.to_string(), handle);
        }

        self.force_refresh_tool_cache().await?;
        Ok(true)
    }

    /// Stop a running server; returns true if a server instance was stopped.
    pub async fn disable_server(&self, server_id: &str) -> Result<bool> {
        let handle = {
            let mut guard = self.servers.write().unwrap();
            guard.remove(server_id)
        };

        if let Some(server) = handle {
            if let Err(err) = server.shutdown().await {
                warn!(error = ?err, server_id, "failed to shutdown server cleanly");
            }
            self.force_refresh_tool_cache().await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub async fn tool_count_for(&self, server_id: &str) -> Option<usize> {
        let server = {
            let guard = self.servers.read().unwrap();
            guard.get(server_id).cloned()
        };
        if let Some(server) = server {
            let tools = server.tools.read().await;
            Some(tools.len())
        } else {
            None
        }
    }

    pub async fn call_tool(
        &self,
        params: CallToolRequestParam,
    ) -> Result<CallToolResult, McpError> {
        self.ensure_tool_cache()
            .await
            .map_err(|err| McpError::internal_error(err.to_string(), None))?;

        debug!("=== DEBUG: ServerManager routing tool call ===");
        debug!("  tool_name: {}", params.name);

        let server_id = {
            let index = self.tool_index.read().await;
            index.get(params.name.as_ref()).cloned()
        };

        let Some(server_id) = server_id else {
            debug!("  tool not found in index");
            return Err(McpError::method_not_found::<CallToolRequestMethod>());
        };

        debug!("  routed to server_id: {}", server_id);

        let server = {
            let guard = self.servers.read().unwrap();
            guard.get(&server_id).cloned()
        }
        .ok_or_else(|| {
            McpError::internal_error(format!("tool mapped to missing server {server_id}"), None)
        })?;

        debug!("  server_name: {}", server.display_name());

        server
            .call_tool(params)
            .await
            .map_err(|err| McpError::internal_error(err.to_string(), None))
    }

    /// 获取指定tool所属的server名称（用于权限检查）
    pub async fn get_server_for_tool(&self, tool_name: &str) -> Result<String, McpError> {
        self.ensure_tool_cache()
            .await
            .map_err(|err| McpError::internal_error(err.to_string(), None))?;

        let server_id = {
            let index = self.tool_index.read().await;
            index.get(tool_name).cloned()
        };

        let Some(server_id) = server_id else {
            return Err(McpError::method_not_found::<CallToolRequestMethod>());
        };

        Ok(server_id)
    }

    pub async fn shutdown(&self) {
        let handles = {
            let guard = self.servers.read().unwrap();
            guard.values().cloned().collect::<Vec<_>>()
        };
        for server in handles {
            if let Err(err) = server.shutdown().await {
                warn!(error = ?err, id = server.id(), "failed to shutdown server cleanly");
            }
        }
    }

    async fn ensure_tool_cache(&self) -> Result<()> {
        let needs_refresh = {
            let guard = self.servers.read().unwrap();
            guard.values().any(|server| server.needs_refresh())
        };

        if needs_refresh || self.tool_cache.read().await.is_empty() {
            self.force_refresh_tool_cache().await?;
        }
        Ok(())
    }

    async fn force_refresh_tool_cache(&self) -> Result<()> {
        let mut new_index = HashMap::new();
        let mut new_entries = Vec::new();

        let servers = {
            let guard = self.servers.read().unwrap();
            guard
                .iter()
                .map(|(id, server)| (id.clone(), server.clone()))
                .collect::<Vec<_>>()
        };

        for (server_id, server) in servers {
            let tools = server.refresh_tools().await?;
            let server_name = server.display_name();
            for tool in tools.iter() {
                let tool_name = tool.name.clone().into_owned();
                if let Some(existing) = new_index.insert(tool_name.clone(), server_id.clone()) {
                    warn!(
                        tool = %tool_name,
                        first_server = %existing,
                        second_server = %server_id,
                        "duplicate tool name detected; latest definition wins"
                    );
                }
                new_entries.push(ToolEntry {
                    server_id: server_id.clone(),
                    server_name: server_name.clone(),
                    tool: tool.clone(),
                });
            }
        }

        *self.tool_index.write().await = new_index;
        *self.tool_cache.write().await = new_entries;

        Ok(())
    }
}

struct ManagedServer {
    definition: ServerDefinition,
    runtime: Mutex<ServerRuntime>,
    tools: RwLock<Vec<Tool>>,
    needs_refresh: Arc<AtomicBool>,
}

struct ServerRuntime {
    client: Option<rmcp::service::RunningService<RoleClient, ServerAdapter>>,
    pid_path: Option<PathBuf>,
    kind: ManagedServerKind,
}

impl ManagedServer {
    async fn launch(layout: &Layout, definition: ServerDefinition) -> Result<Arc<Self>> {
        let log_path = layout.server_log_path(&definition.id);
        fs::write(&log_path, &[])
            .await
            .with_context(|| format!("failed to prepare log file {}", log_path.display()))?;

        let needs_refresh = Arc::new(AtomicBool::new(true));
        let adapter = ServerAdapter::new(
            definition.name.clone().unwrap_or_else(|| definition.id.clone()),
            log_path.clone(),
            needs_refresh.clone(),
        )
        .await?;

        match definition.protocol {
            ServerProtocol::StdIo => {
                let (client, pid_path) =
                    Self::spawn_local(layout, &definition, adapter.clone()).await?;
                let server = Arc::new(Self {
                    definition,
                    runtime: Mutex::new(ServerRuntime {
                        client: Some(client),
                        pid_path: Some(pid_path),
                        kind: ManagedServerKind::LocalProcess,
                    }),
                    tools: RwLock::new(Vec::new()),
                    needs_refresh,
                });
                Ok(server)
            }
            ServerProtocol::Sse | ServerProtocol::Http => {
                let client = Self::connect_remote(&definition, adapter.clone()).await?;
                let pid_path = layout.server_pid_path(&definition.id);
                if let Err(err) = fs::remove_file(&pid_path).await {
                    if err.kind() != std::io::ErrorKind::NotFound {
                        warn!(error = ?err, path = %pid_path.display(), "failed to clean remote pid file");
                    }
                }
                let server = Arc::new(Self {
                    definition,
                    runtime: Mutex::new(ServerRuntime {
                        client: Some(client),
                        pid_path: None,
                        kind: ManagedServerKind::Remote,
                    }),
                    tools: RwLock::new(Vec::new()),
                    needs_refresh,
                });
                Ok(server)
            }
            ServerProtocol::Unknown => Err(anyhow!("unsupported protocol: unknown")),
        }
    }

    async fn spawn_local(
        layout: &Layout,
        definition: &ServerDefinition,
        adapter: ServerAdapter,
    ) -> Result<(rmcp::service::RunningService<RoleClient, ServerAdapter>, PathBuf)> {
        let mut command = tokio::process::Command::new(&definition.command);
        command.args(&definition.args);
        if !definition.env.is_empty() {
            command.envs(&definition.env);
        }
        command.kill_on_drop(true);

        let transport = TokioChildProcess::new(command.configure(|cmd| {
            cmd.stdin(std::process::Stdio::piped());
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::inherit());
        }))
        .with_context(|| format!("failed to spawn process '{}'", definition.command))?;

        let pid = transport.id();
        let pid_path = layout.server_pid_path(&definition.id);
        if let Some(pid) = pid {
            fs::write(&pid_path, pid.to_string().as_bytes())
                .await
                .with_context(|| format!("failed to write pid file {}", pid_path.display()))?;
            info!(server_id = %definition.id, pid, path = %pid_path.display(), "local MCP server spawned");
        }

        let client = adapter
            .clone()
            .serve(transport)
            .await
            .context("failed to initialise MCP transport for local server")?;

        Ok((client, pid_path))
    }

    async fn connect_remote(
        definition: &ServerDefinition,
        adapter: ServerAdapter,
    ) -> Result<rmcp::service::RunningService<RoleClient, ServerAdapter>> {
        let endpoint = definition
            .endpoint
            .as_ref()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow!("missing endpoint for remote server"))?;

        info!(server_id = %definition.id, endpoint, "connecting remote MCP server");

        match definition.protocol {
            ServerProtocol::Sse => {
                let transport =
                    SseClientTransport::start(endpoint.clone()).await.map_err(|err| {
                        anyhow!("failed to connect to SSE endpoint {endpoint}: {err}")
                    })?;
                adapter
                    .clone()
                    .serve(transport)
                    .await
                    .context("failed to initialise SSE transport")
            }
            ServerProtocol::Http => {
                let transport = StreamableHttpClientTransport::from_uri(endpoint.clone());
                adapter
                    .clone()
                    .serve(transport)
                    .await
                    .context("failed to initialise HTTP transport")
            }
            _ => Err(anyhow!("protocol mismatch for remote connection")),
        }
    }

    fn id(&self) -> &str {
        &self.definition.id
    }

    fn display_name(&self) -> String {
        self.definition.name.clone().unwrap_or_else(|| self.definition.id.clone())
    }

    async fn snapshot(&self) -> ServerSnapshot {
        let tools = self.tools.read().await;
        ServerSnapshot {
            id: self.definition.id.clone(),
            name: self.display_name(),
            protocol: self.definition.protocol.clone(),
            enabled: self.definition.enabled,
            tool_count: tools.len(),
            created_at: None, // 由 HTTP 层填充
            last_seen: None,  // 未实现
        }
    }

    fn needs_refresh(&self) -> bool {
        self.needs_refresh.load(Ordering::SeqCst)
    }

    async fn refresh_tools(&self) -> Result<Vec<Tool>> {
        let peer = {
            let runtime = self.runtime.lock().await;
            runtime
                .client
                .as_ref()
                .map(|client| client.peer().clone())
                .ok_or_else(|| anyhow!("server connection is shutting down"))?
        };
        let tools = peer
            .list_all_tools()
            .await
            .map_err(|err| anyhow!("failed to list tools: {err}"))?;

        {
            let mut guard = self.tools.write().await;
            *guard = tools.clone();
        }
        self.needs_refresh.store(false, Ordering::SeqCst);
        Ok(tools)
    }

    async fn call_tool(
        &self,
        params: CallToolRequestParam,
    ) -> Result<CallToolResult, ServiceError> {
        let peer = {
            let runtime = self.runtime.lock().await;
            runtime
                .client
                .as_ref()
                .map(|client| client.peer().clone())
                .ok_or_else(|| ServiceError::TransportClosed)?
        };
        peer.call_tool(params).await
    }

    async fn shutdown(&self) -> Result<()> {
        let mut runtime = self.runtime.lock().await;
        if let Some(client) = runtime.client.take() {
            if let Err(err) = client.cancel().await {
                warn!(error = ?err, server_id = %self.definition.id, "error while cancelling MCP connection");
            }
        }

        if let (ManagedServerKind::LocalProcess, Some(pid_path)) =
            (runtime.kind, runtime.pid_path.clone())
        {
            if let Err(err) = fs::remove_file(&pid_path).await {
                if err.kind() != std::io::ErrorKind::NotFound {
                    warn!(error = ?err, path = %pid_path.display(), "failed to remove pid file");
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone)]
struct ServerAdapter {
    inner: Arc<ServerAdapterInner>,
}

struct ServerAdapterInner {
    server_name: String,
    log: tokio::sync::Mutex<tokio::fs::File>,
    needs_refresh: Arc<AtomicBool>,
}

impl ServerAdapter {
    async fn new(
        server_name: String,
        log_path: PathBuf,
        needs_refresh: Arc<AtomicBool>,
    ) -> Result<Self> {
        let log_file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&log_path)
            .await
            .with_context(|| format!("failed to open log file {}", log_path.display()))?;

        Ok(Self {
            inner: Arc::new(ServerAdapterInner {
                server_name,
                log: tokio::sync::Mutex::new(log_file),
                needs_refresh,
            }),
        })
    }
}

impl Service<RoleClient> for ServerAdapter {
    async fn handle_request(
        &self,
        request: <RoleClient as rmcp::service::ServiceRole>::PeerReq,
        _context: rmcp::service::RequestContext<RoleClient>,
    ) -> Result<ClientResult, McpError> {
        match request {
            ServerRequest::PingRequest(_) => Ok(ClientResult::empty(())),
            other => {
                warn!(?other, "unsupported server-initiated request");
                Err(McpError::internal_error("unsupported server request", None))
            }
        }
    }

    async fn handle_notification(
        &self,
        notification: <RoleClient as rmcp::service::ServiceRole>::PeerNot,
        _context: rmcp::service::NotificationContext<RoleClient>,
    ) -> Result<(), McpError> {
        match notification {
            ServerNotification::LoggingMessageNotification(message) => {
                if let Err(err) = self.inner.write_log(&message).await {
                    warn!(error = ?err, server = %self.inner.server_name, "failed to write log entry");
                }
            }
            ServerNotification::ToolListChangedNotification(ToolListChangedNotification {
                ..
            }) => {
                self.inner.needs_refresh.store(true, Ordering::SeqCst);
            }
            _ => {}
        }
        Ok(())
    }

    fn get_info(&self) -> <RoleClient as rmcp::service::ServiceRole>::Info {
        Default::default()
    }
}

impl ServerAdapterInner {
    async fn write_log(&self, message: &rmcp::model::LoggingMessageNotification) -> Result<()> {
        let mut file = self.log.lock().await;
        let payload = to_vec(message)?;
        file.write_all(&payload).await?;
        file.write_all(b"\n").await?;
        Ok(())
    }
}
