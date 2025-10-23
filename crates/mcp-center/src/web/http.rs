use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    convert::Infallible,
    env,
    net::SocketAddr,
    path::PathBuf,
    sync::Arc,
    time::SystemTime,
};

use anyhow::Result;
use axum::{
    Json, Router,
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, HeaderName, Method, Request, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
};
use serde::{Deserialize, Serialize};
use specta::Type;
use tokio::{fs, net::TcpListener, task::JoinHandle};
use tower_http::cors::{Any, CorsLayer};
use tracing::{error, info, warn};

use crate::{
    CoreError, Layout,
    config::{ServerConfig, ServerDefinition, ServerProtocol},
    daemon::server_manager::{ServerManager, ServerSnapshot},
    project::{ProjectId, ProjectRecord, ProjectRegistry, ToolCustomization, ToolPermission},
};

#[derive(Clone)]
pub struct HttpState {
    pub manager: Arc<ServerManager>,
    pub registry: ProjectRegistry,
    pub layout: Layout,
    pub auth: HttpAuth,
}

#[derive(Debug)]
pub struct HttpServerHandle {
    addr: SocketAddr,
    task: JoinHandle<()>,
}

impl HttpServerHandle {
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn shutdown(self) {
        self.task.abort();
    }
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct McpListResponse {
    pub servers: Vec<ServerSnapshot>,
}

#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectListResponse {
    pub projects: Vec<ProjectSummary>,
}

#[derive(Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ToolSummary {
    pub name: String,
    pub description: Option<String>,
    pub server_id: String,
    pub server_name: String,
}

#[derive(Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct ToolListResponse {
    pub tools: Vec<ToolSummary>,
}

#[derive(Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct ServerDetail {
    pub id: String,
    pub name: String,
    pub protocol: ServerProtocol,
    pub enabled: bool,
    pub tool_count: usize,
    pub command: Option<String>,
    pub args: Vec<String>,
    pub url: Option<String>,
    pub env: BTreeMap<String, String>,
    pub headers: BTreeMap<String, String>,
    pub created_at: Option<u64>,
    pub last_seen: Option<u64>,
}

#[derive(Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct ServerDetailResponse {
    pub server: ServerDetail,
    pub tools: Vec<ToolSummary>,
}

#[derive(Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct ServerToggleResponse {
    pub server: ServerSnapshot,
    pub warning: Option<String>,
}

#[derive(Serialize, Type)]
#[serde(rename_all = "snake_case")]
pub struct ProjectSummary {
    pub id: String,
    pub path: String,
    pub display_name: Option<String>,
    pub agent: Option<String>,
    pub allowed_server_ids: Vec<String>,
    pub created_at: u64,
    pub last_seen_at: u64,
}

impl From<ProjectRecord> for ProjectSummary {
    fn from(record: ProjectRecord) -> Self {
        ProjectSummary {
            id: record.id,
            path: record.path.to_string_lossy().to_string(),
            display_name: record.display_name,
            agent: record.agent,
            allowed_server_ids: record.allowed_server_ids,
            created_at: record.created_at,
            last_seen_at: record.last_seen_at,
        }
    }
}

#[derive(Debug, Serialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Debug)]
enum ApiError {
    NotFound(String),
    BadRequest(String),
    Unauthorized(String),
    Internal(String),
}

impl ApiError {
    fn not_found(message: impl Into<String>) -> Self {
        ApiError::NotFound(message.into())
    }

    fn bad_request(message: impl Into<String>) -> Self {
        ApiError::BadRequest(message.into())
    }

    fn unauthorized(message: impl Into<String>) -> Self {
        ApiError::Unauthorized(message.into())
    }

    fn internal(message: impl Into<String>) -> Self {
        ApiError::Internal(message.into())
    }
}

impl From<CoreError> for ApiError {
    fn from(err: CoreError) -> Self {
        match err {
            CoreError::ServerConfigNotFound { id } => {
                ApiError::not_found(format!("server with id '{id}' not found"))
            }
            CoreError::ServerConfigNotFoundByName { name } => {
                ApiError::not_found(format!("server '{name}' not found"))
            }
            CoreError::ProjectConfigNotFound { id } => {
                ApiError::not_found(format!("project '{id}' not found"))
            }
            CoreError::ProjectRead { .. }
            | CoreError::ProjectParse { .. }
            | CoreError::ProjectSerialise { .. }
            | CoreError::ProjectWrite { .. }
            | CoreError::CreateDirectory { .. }
            | CoreError::ReadDirectory { .. }
            | CoreError::ReadConfig { .. }
            | CoreError::ParseJson { .. }
            | CoreError::ParseToml { .. }
            | CoreError::SerialiseToml { .. }
            | CoreError::RemoveFile { .. }
            | CoreError::HomeDirectoryUnknown => ApiError::internal(err.to_string()),
            CoreError::ServerNameEmpty { .. }
            | CoreError::UnsupportedProtocol { .. }
            | CoreError::ServerEndpointMissing { .. }
            | CoreError::ServerEndpointInvalid { .. } => ApiError::bad_request(err.to_string()),
            other => ApiError::internal(other.to_string()),
        }
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(err: anyhow::Error) -> Self {
        match err.downcast::<CoreError>() {
            Ok(core) => ApiError::from(core),
            Err(other) => ApiError::internal(other.to_string()),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::NotFound(message) => (StatusCode::NOT_FOUND, message),
            ApiError::BadRequest(message) => (StatusCode::BAD_REQUEST, message),
            ApiError::Unauthorized(message) => (StatusCode::UNAUTHORIZED, message),
            ApiError::Internal(message) => (StatusCode::INTERNAL_SERVER_ERROR, message),
        };
        let body = Json(ErrorResponse { error: message });
        (status, body).into_response()
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub enum ClientKind {
    #[default]
    Unknown,
    Web,
    Tauri,
}

impl ClientKind {
    fn from_headers(headers: &HeaderMap) -> Self {
        let value = headers.get("x-mcp-client").and_then(|val| val.to_str().ok());
        match value.map(|v| v.trim().to_ascii_lowercase()) {
            Some(ref value) if value == "web" => ClientKind::Web,
            Some(ref value) if value == "tauri" => ClientKind::Tauri,
            _ => ClientKind::Unknown,
        }
    }
}

async fn attach_client_kind(mut req: Request<Body>, next: Next) -> Result<Response, Infallible> {
    let kind = ClientKind::from_headers(req.headers());
    req.extensions_mut().insert(kind);
    Ok(next.run(req).await)
}

#[derive(Clone, Default)]
pub struct HttpAuth {
    token: Option<String>,
}

impl HttpAuth {
    pub fn new(token: Option<String>) -> Self {
        Self { token: token.map(|t| t.trim().to_string()).filter(|t| !t.is_empty()) }
    }

    fn verify(&self, kind: ClientKind, req: &Request<Body>) -> Result<(), ApiError> {
        let Some(expected) = self.token.as_deref() else {
            return Ok(());
        };

        let matches_authorization = req
            .headers()
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.trim())
            .filter(|value| value.starts_with("Bearer "))
            .map(|value| value.trim_start_matches("Bearer ").trim())
            .map(|value| value == expected)
            .unwrap_or(false);

        let matches_custom = req
            .headers()
            .get("x-mcp-token")
            .and_then(|value| value.to_str().ok())
            .map(|value| value.trim())
            .map(|value| value == expected)
            .unwrap_or(false);

        if matches_authorization || matches_custom {
            return Ok(());
        }

        let client = match kind {
            ClientKind::Web => "Web",
            ClientKind::Tauri => "Tauri",
            ClientKind::Unknown => "Unknown",
        };

        Err(ApiError::unauthorized(format!(
            "{client} client missing valid authentication token"
        )))
    }
}

pub fn build_router(state: HttpState) -> Router {
    let cors = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::OPTIONS])
        .allow_headers([
            header::ACCEPT,
            header::CONTENT_TYPE,
            HeaderName::from_static("x-mcp-client"),
            header::AUTHORIZATION,
            HeaderName::from_static("x-mcp-token"),
        ])
        .allow_origin(Any);

    let auth_state = state.auth.clone();

    Router::new()
        .route("/api/health", get(get_health))
        .route("/api/mcp", get(list_mcp).post(create_mcp))
        .route("/api/mcp/:id", get(get_mcp_detail).delete(delete_mcp))
        .route("/api/mcp/:id/enabled", patch(update_mcp_enabled))
        .route("/api/mcp/:id/tools", get(get_mcp_tools))
        .route("/api/project", get(list_projects))
        .route("/api/project/allow", post(project_allow))
        .route("/api/project/deny", post(project_deny))
        .route("/api/project/tools/allow", post(project_allow_tools))
        .route("/api/project/tools/deny", post(project_deny_tools))
        .route("/api/project/tool/description", post(project_set_tool_desc))
        .route("/api/project/tool/description/reset", post(project_reset_tool_desc))
        .layer(middleware::from_fn_with_state(auth_state, authenticate))
        .layer(middleware::from_fn(attach_client_kind))
        .layer(cors)
        .with_state(state)
}

async fn authenticate(
    State(auth): State<HttpAuth>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, ApiError> {
    let kind = req.extensions().get::<ClientKind>().copied().unwrap_or_default();
    auth.verify(kind, &req)?;
    Ok(next.run(req).await)
}

pub async fn spawn_http_server(state: HttpState, addr: SocketAddr) -> Result<HttpServerHandle> {
    let router = build_router(state);
    let listener = TcpListener::bind(addr).await?;
    let local_addr = listener.local_addr()?;
    info!("HTTP server listening on {}", local_addr);

    let task = tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, router).await {
            error!("HTTP server terminated with error: {err}");
        }
    });

    Ok(HttpServerHandle { addr: local_addr, task })
}

async fn get_health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn list_mcp(State(state): State<HttpState>) -> Result<Json<McpListResponse>, ApiError> {
    let configs = state.layout.list_server_configs().map_err(ApiError::from)?;
    let running = state.manager.list_servers().await;
    let mut running_map: HashMap<String, ServerSnapshot> =
        running.into_iter().map(|snapshot| (snapshot.id.clone(), snapshot)).collect();

    let mut servers = Vec::with_capacity(configs.len().max(running_map.len()));
    for config in configs {
        let definition = config.definition();
        let id = definition.id.clone();
        let created_at = server_config_timestamp(config.source()).await;

        let mut snapshot = running_map.remove(&id).unwrap_or_else(|| ServerSnapshot {
            id: id.clone(),
            name: definition.name.clone().unwrap_or_else(|| id.clone()),
            protocol: definition.protocol.clone(),
            enabled: definition.enabled,
            tool_count: 0,
            created_at,
            last_seen: None,
        });
        snapshot.enabled = definition.enabled;
        snapshot.name = definition.name.clone().unwrap_or_else(|| id.clone());
        snapshot.created_at = created_at; // 确保使用配置文件的时间戳
        servers.push(snapshot);
    }

    Ok(Json(McpListResponse { servers }))
}

#[derive(Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CreateMcpRequest {
    pub name: String,
    pub protocol: ServerProtocol,
    pub command: Option<String>,
    pub args: Option<String>,
    pub endpoint: Option<String>,
    pub env: Option<BTreeMap<String, String>>,
    pub headers: Option<BTreeMap<String, String>>,
}

async fn create_mcp(
    State(state): State<HttpState>,
    Json(body): Json<CreateMcpRequest>,
) -> Result<(StatusCode, Json<ServerSnapshot>), ApiError> {
    state.layout.ensure().map_err(ApiError::from)?;

    let configs = state.layout.list_server_configs().map_err(ApiError::from)?;

    let CreateMcpRequest { name, protocol, command, args, endpoint, env, headers } = body;
    let display_name = name.trim();
    if display_name.is_empty() {
        return Err(ApiError::bad_request("server name cannot be empty"));
    }

    ensure_unique_server_name_from_configs(&configs, display_name, None)?;

    let mut definition = ServerDefinition {
        id: String::new(),
        name: Some(display_name.to_string()),
        protocol: protocol.clone(),
        command: String::new(),
        args: Vec::new(),
        env: env.unwrap_or_default(),
        endpoint: None,
        headers: headers.unwrap_or_default(),
        enabled: false,
    };

    match protocol {
        ServerProtocol::StdIo => {
            let executable = command
                .as_ref()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .ok_or_else(|| ApiError::bad_request("command is required for stdio servers"))?;
            definition.command = executable.to_string();
            definition.args = parse_command_args(args);
        }
        ServerProtocol::Sse | ServerProtocol::Http => {
            if command.as_ref().map(|value| !value.trim().is_empty()).unwrap_or(false) {
                return Err(ApiError::bad_request("command is not allowed for remote MCP servers"));
            }
            let endpoint = endpoint
                .as_ref()
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
                .ok_or_else(|| ApiError::bad_request("endpoint is required for remote servers"))?;
            definition.endpoint = Some(endpoint.to_string());
        }
        ServerProtocol::Unknown => {
            return Err(ApiError::bad_request("unsupported server protocol"));
        }
    }

    let mut config = ServerConfig::new(definition).map_err(ApiError::from)?;
    config.definition_mut().id.clear();

    let existing_ids: HashSet<String> =
        configs.iter().map(|cfg| cfg.definition().id.clone()).collect();
    config.assign_unique_id(&existing_ids);

    let destination = state.layout.server_config_toml_path(config.definition().id.as_str());
    if destination.exists() {
        return Err(ApiError::internal(format!(
            "server id '{}' already exists at {}",
            config.definition().id,
            destination.display()
        )));
    }

    let toml = config.to_toml_string().map_err(ApiError::from)?;
    fs::write(&destination, toml)
        .await
        .map_err(|err| ApiError::internal(format!("failed to write server config: {err}")))?;

    // 获取刚创建的配置文件的时间戳
    let created_at = server_config_timestamp(Some(&destination)).await;

    let snapshot = ServerSnapshot {
        id: config.definition().id.clone(),
        name: config
            .definition()
            .name
            .clone()
            .unwrap_or_else(|| config.definition().id.clone()),
        protocol: config.definition().protocol.clone(),
        enabled: config.definition().enabled,
        tool_count: 0,
        created_at,
        last_seen: None,
    };

    Ok((StatusCode::CREATED, Json(snapshot)))
}

async fn get_mcp_detail(
    State(state): State<HttpState>,
    Path(id): Path<String>,
) -> Result<Json<ServerDetailResponse>, ApiError> {
    let config = state.layout.load_server_config(&id).map_err(ApiError::from)?;
    let tools = collect_server_tools(&state.manager, &id).await?;
    let tool_count = state.manager.tool_count_for(&id).await.unwrap_or(tools.len());
    let created_at = server_config_timestamp(config.source()).await;

    let definition = config.definition();
    let server = ServerDetail {
        id: definition.id.clone(),
        name: definition.name.clone().unwrap_or_else(|| definition.id.clone()),
        protocol: definition.protocol.clone(),
        enabled: definition.enabled,
        tool_count,
        command: (!definition.command.trim().is_empty()).then(|| definition.command.clone()),
        args: definition.args.clone(),
        url: definition.endpoint.clone(),
        env: definition.env.clone(),
        headers: definition.headers.clone(),
        created_at,
        last_seen: None,
    };

    Ok(Json(ServerDetailResponse { server, tools }))
}

async fn get_mcp_tools(
    State(state): State<HttpState>,
    Path(id): Path<String>,
) -> Result<Json<ToolListResponse>, ApiError> {
    let tools = collect_server_tools(&state.manager, &id).await?;
    Ok(Json(ToolListResponse { tools }))
}

async fn delete_mcp(
    State(state): State<HttpState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    state.layout.ensure().map_err(ApiError::from)?;
    let config = state.layout.load_server_config(&id).map_err(ApiError::from)?;
    let definition = config.definition();

    if let Err(err) = state.manager.disable_server(&definition.id).await {
        warn!(server_id = %definition.id, error = ?err, "failed to stop server before deletion");
    }

    state.layout.remove_server_config(&definition.id).map_err(ApiError::from)?;

    Ok(StatusCode::NO_CONTENT)
}

fn ensure_unique_server_name_from_configs(
    configs: &[ServerConfig],
    candidate: &str,
    skip_id: Option<&str>,
) -> Result<(), ApiError> {
    for config in configs {
        let existing = config
            .definition()
            .name
            .as_ref()
            .map(|value| value.trim())
            .filter(|value| !value.is_empty());

        if let Some(existing) = existing {
            if existing == candidate {
                if skip_id.map(|id| id == config.definition().id).unwrap_or(false) {
                    continue;
                }
                return Err(ApiError::bad_request(format!(
                    "server name '{candidate}' already exists"
                )));
            }
        }
    }
    Ok(())
}

fn parse_command_args(raw: Option<String>) -> Vec<String> {
    raw.map(|value| {
        value
            .split_whitespace()
            .filter(|segment| !segment.is_empty())
            .map(|segment| segment.to_string())
            .collect()
    })
    .unwrap_or_default()
}

async fn server_config_timestamp(path: Option<&std::path::Path>) -> Option<u64> {
    let path = path?;
    let metadata = fs::metadata(path).await.ok()?;
    metadata
        .created()
        .or_else(|_| metadata.modified())
        .ok()
        .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs())
}

async fn collect_server_tools(
    manager: &ServerManager,
    server_id: &str,
) -> Result<Vec<ToolSummary>, ApiError> {
    let entries = manager.list_tools().await.map_err(ApiError::from)?;
    let mut tools = Vec::new();
    for entry in entries.into_iter() {
        if entry.server_id != server_id {
            continue;
        }
        let name = entry.tool.name.clone().into_owned();
        let description = entry.tool.description.as_ref().map(|desc| desc.to_string());
        tools.push(ToolSummary {
            name,
            description,
            server_id: entry.server_id,
            server_name: entry.server_name,
        });
    }
    Ok(tools)
}

async fn list_projects(
    State(state): State<HttpState>,
) -> Result<Json<ProjectListResponse>, StatusCode> {
    let projects = state
        .registry
        .list()
        .map_err(|err| {
            error!("failed to list projects: {err}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .into_iter()
        .map(ProjectSummary::from)
        .collect();

    Ok(Json(ProjectListResponse { projects }))
}

#[derive(Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMcpEnabled {
    pub enabled: bool,
}

async fn update_mcp_enabled(
    State(state): State<HttpState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateMcpEnabled>,
) -> Result<Json<ServerToggleResponse>, ApiError> {
    let mut config = state.layout.load_server_config(&id).map_err(ApiError::from)?;
    config.definition_mut().enabled = body.enabled;
    let path = config
        .source()
        .ok_or_else(|| ApiError::internal("server config path is unknown"))?
        .to_path_buf();
    let toml = config.to_toml_string().map_err(ApiError::from)?;
    fs::write(&path, toml)
        .await
        .map_err(|err| ApiError::internal(format!("failed to write server config: {err}")))?;

    let mut warning = None;
    if body.enabled {
        if let Err(err) = state.manager.ensure_server_running(&id).await {
            warn!(server_id = %id, error = ?err, "failed to start MCP server after enabling");
            warning = Some(format!("failed to start MCP server: {err}"));
        }
    } else if let Err(err) = state.manager.disable_server(&id).await {
        warn!(server_id = %id, error = ?err, "failed to stop MCP server after disabling");
        warning = Some(format!("failed to stop MCP server: {err}"));
    }

    let tool_count = state.manager.tool_count_for(&id).await.unwrap_or_default();
    let created_at = server_config_timestamp(config.source()).await;

    let snapshot = ServerSnapshot {
        id: config.definition().id.clone(),
        name: config
            .definition()
            .name
            .clone()
            .unwrap_or_else(|| config.definition().id.clone()),
        protocol: config.definition().protocol.clone(),
        enabled: config.definition().enabled,
        tool_count,
        created_at,
        last_seen: None,
    };

    Ok(Json(ServerToggleResponse { server: snapshot, warning }))
}

#[derive(Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectAssignRequest {
    pub target: String,
    pub servers: Vec<String>,
}

async fn project_allow(
    State(state): State<HttpState>,
    Json(body): Json<ProjectAssignRequest>,
) -> Result<Json<ProjectSummary>, ApiError> {
    state.registry.ensure().map_err(ApiError::from)?;

    let (mut record, canonical) = load_or_create_project(&state.registry, &body.target)?;

    let configs = state.layout.list_server_configs().map_err(ApiError::from)?;
    let mut existing_ids = HashSet::new();
    for cfg in &configs {
        existing_ids.insert(cfg.definition().id.clone());
    }

    let mut allowed: BTreeSet<String> = record.allowed_server_ids.iter().cloned().collect();
    for server in body.servers.iter() {
        let trimmed = server.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !existing_ids.contains(trimmed) {
            return Err(ApiError::bad_request(format!("unknown server id '{trimmed}'")));
        }
        allowed.insert(trimmed.to_string());
    }

    record.allowed_server_ids = allowed.into_iter().collect();
    record.path = canonical;
    record.touch();
    state.registry.store(&record).map_err(ApiError::from)?;

    Ok(Json(ProjectSummary::from(record)))
}

async fn project_deny(
    State(state): State<HttpState>,
    Json(body): Json<ProjectAssignRequest>,
) -> Result<Json<ProjectSummary>, ApiError> {
    state.registry.ensure().map_err(ApiError::from)?;

    let (mut record, canonical) = load_existing_project_with_path(&state.registry, &body.target)?;

    let mut allowed: BTreeSet<String> = record.allowed_server_ids.iter().cloned().collect();
    for server in body.servers.iter() {
        let trimmed = server.trim();
        if trimmed.is_empty() {
            continue;
        }
        allowed.remove(trimmed);
    }

    record.allowed_server_ids = allowed.into_iter().collect();
    record.path = canonical;
    record.touch();
    state.registry.store(&record).map_err(ApiError::from)?;

    Ok(Json(ProjectSummary::from(record)))
}

#[derive(Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectToolsRequest {
    pub target: String,
    pub tools: Vec<String>,
}

async fn project_allow_tools(
    State(state): State<HttpState>,
    Json(body): Json<ProjectToolsRequest>,
) -> Result<Json<ProjectSummary>, ApiError> {
    state.registry.ensure().map_err(ApiError::from)?;

    let (mut record, _) = load_existing_project_with_path(&state.registry, &body.target)?;

    let mut server_tools: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for spec in &body.tools {
        let (server, tool) = parse_tool_spec(spec)?;
        server_tools.entry(server).or_default().push(tool);
    }

    for (server, tools) in server_tools {
        record
            .allowed_server_tools
            .insert(server.clone(), ToolPermission::AllowList { tools: tools.clone() });
    }

    record.touch();
    state.registry.store(&record).map_err(ApiError::from)?;

    Ok(Json(ProjectSummary::from(record)))
}

async fn project_deny_tools(
    State(state): State<HttpState>,
    Json(body): Json<ProjectToolsRequest>,
) -> Result<Json<ProjectSummary>, ApiError> {
    state.registry.ensure().map_err(ApiError::from)?;

    let (mut record, _) = load_existing_project_with_path(&state.registry, &body.target)?;

    let mut server_tools: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for spec in &body.tools {
        let (server, tool) = parse_tool_spec(spec)?;
        server_tools.entry(server).or_default().push(tool);
    }

    for (server, tools) in server_tools {
        record
            .allowed_server_tools
            .insert(server.clone(), ToolPermission::DenyList { tools: tools.clone() });
    }

    record.touch();
    state.registry.store(&record).map_err(ApiError::from)?;

    Ok(Json(ProjectSummary::from(record)))
}

#[derive(Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectToolDescRequest {
    pub target: String,
    pub tool: String,
    pub description: String,
}

async fn project_set_tool_desc(
    State(state): State<HttpState>,
    Json(body): Json<ProjectToolDescRequest>,
) -> Result<Json<ProjectSummary>, ApiError> {
    state.registry.ensure().map_err(ApiError::from)?;

    let (mut record, _) = load_existing_project_with_path(&state.registry, &body.target)?;
    record.tool_customizations.retain(|c| c.tool_name != body.tool);
    record.tool_customizations.push(ToolCustomization {
        tool_name: body.tool.clone(),
        description: Some(body.description.clone()),
    });
    record.touch();
    state.registry.store(&record).map_err(ApiError::from)?;

    Ok(Json(ProjectSummary::from(record)))
}

#[derive(Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProjectToolResetRequest {
    pub target: String,
    pub tool: String,
}

async fn project_reset_tool_desc(
    State(state): State<HttpState>,
    Json(body): Json<ProjectToolResetRequest>,
) -> Result<Json<ProjectSummary>, ApiError> {
    state.registry.ensure().map_err(ApiError::from)?;

    let (mut record, _) = load_existing_project_with_path(&state.registry, &body.target)?;
    let before = record.tool_customizations.len();
    record.tool_customizations.retain(|c| c.tool_name != body.tool);
    if before == record.tool_customizations.len() {
        return Err(ApiError::not_found(format!("tool customization not found for {}", body.tool)));
    }
    record.touch();
    state.registry.store(&record).map_err(ApiError::from)?;

    Ok(Json(ProjectSummary::from(record)))
}

fn load_or_create_project(
    registry: &ProjectRegistry,
    target: &str,
) -> Result<(ProjectRecord, PathBuf), ApiError> {
    match load_existing_project_with_path(registry, target) {
        Ok((record, path)) => Ok((record, path)),
        Err(ApiError::NotFound(_)) => {
            let path = normalize_project_path(target)?;
            let record =
                registry.find_by_path(&path).map_err(ApiError::from)?.unwrap_or_else(|| {
                    let id = ProjectId::from_path(&path);
                    ProjectRecord::new(id, path.clone())
                });
            Ok((record, path))
        }
        Err(err) => Err(err),
    }
}

fn load_existing_project_with_path(
    registry: &ProjectRegistry,
    target: &str,
) -> Result<(ProjectRecord, PathBuf), ApiError> {
    if let Ok(path) = normalize_project_path(target) {
        if let Some(record) = registry.find_by_path(&path).map_err(ApiError::from)? {
            return Ok((record, path));
        }
    }
    let record =
        registry
            .load_from_id_str(target)
            .map_err(|err| match err.downcast::<CoreError>() {
                Ok(CoreError::ProjectConfigNotFound { .. }) => {
                    ApiError::not_found(format!("project not found: {target}"))
                }
                Ok(core) => ApiError::from(core),
                Err(other) => ApiError::internal(other.to_string()),
            })?;
    let canonical = record.path.clone();
    Ok((record, canonical))
}

fn normalize_project_path(raw: &str) -> Result<PathBuf, ApiError> {
    let path = expand_tilde(raw)?;
    let absolute = if path.is_absolute() {
        path
    } else {
        env::current_dir()
            .map_err(|err| ApiError::internal(format!("failed to get current dir: {err}")))?
            .join(path)
    };
    Ok(std::fs::canonicalize(&absolute).unwrap_or(absolute))
}

fn expand_tilde(path: &str) -> Result<PathBuf, ApiError> {
    if let Some(stripped) = path.strip_prefix('~') {
        let home = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .map_err(|_| ApiError::bad_request("cannot expand '~' without HOME"))?;
        let mut base = PathBuf::from(home);
        let remainder = stripped.trim_start_matches(['/', '\\']);
        if !remainder.is_empty() {
            base = base.join(remainder);
        }
        Ok(base)
    } else {
        Ok(PathBuf::from(path))
    }
}

fn parse_tool_spec(spec: &str) -> Result<(String, String), ApiError> {
    let mut parts = spec.split("::");
    let server = parts
        .next()
        .ok_or_else(|| ApiError::bad_request(format!("invalid tool spec: {spec}")))?;
    let tool = parts
        .next()
        .ok_or_else(|| ApiError::bad_request(format!("invalid tool spec: {spec}")))?;
    if parts.next().is_some() {
        return Err(ApiError::bad_request(format!("invalid tool spec: {spec}")));
    }
    let server = server.trim();
    let tool = tool.trim();
    if server.is_empty() || tool.is_empty() {
        return Err(ApiError::bad_request(format!("invalid tool spec: {spec}")));
    }
    Ok((server.to_string(), tool.to_string()))
}
