use std::{borrow::Cow, fmt, path::PathBuf, sync::Arc};

use anyhow::{Context, Result, anyhow};
use http::header::{AUTHORIZATION, HeaderMap, HeaderName, HeaderValue};
use reqwest::Client as ReqwestClient;
use rmcp::{
    ErrorData as McpError, RoleClient,
    model::{
        CallToolRequestParam, CallToolResult, ClientResult, InitializeResult, JsonObject,
        ListToolsResult, ServerNotification, ServerRequest, Tool,
    },
    service::{
        NotificationContext, Peer, QuitReason, RequestContext, RunningService, Service, ServiceExt,
    },
    transport::{
        ConfigureCommandExt, SseClientTransport, StreamableHttpClientTransport,
        child_process::TokioChildProcess, sse_client::SseClientConfig,
        streamable_http_client::StreamableHttpClientTransportConfig,
    },
};
use serde::Serialize;
use serde_json::Value;
use tokio::sync::broadcast;
use tracing::debug;

/// Events emitted by [`TestClient`] while handling MCP traffic.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", content = "payload")]
pub enum ClientEvent {
    Initialized(InitializeResult),
    Notification(ServerNotification),
    Request(ServerRequest),
    Warning { message: String },
}

/// Lightweight MCP client used for testing and debugging `mcp-center`.
pub struct TestClient {
    runtime: Option<RunningService<RoleClient, ClientService>>,
    events: broadcast::Sender<ClientEvent>,
}

impl fmt::Debug for TestClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TestClient")
            .field("events_buf_len", &self.events.receiver_count())
            .finish()
    }
}

impl TestClient {
    /// Establishes a connection using the provided [`ConnectRequest`].
    pub async fn connect(request: ConnectRequest) -> Result<Self> {
        match request {
            ConnectRequest::StdIo(config) => Self::connect_stdio(config).await,
            ConnectRequest::Sse(config) => Self::connect_sse(config).await,
            ConnectRequest::StreamHttp(config) => Self::connect_stream_http(config).await,
        }
    }

    /// Connects to an MCP server exposed over stdio.
    pub async fn connect_stdio(config: StdIoConfig) -> Result<Self> {
        let (service, events) = ClientService::with_channel();
        let mut command = tokio::process::Command::new(&config.command);
        command.args(&config.args);
        if !config.env.is_empty() {
            command.envs(config.env.iter().map(|(k, v)| (k.as_str(), v.as_str())));
        }
        command.kill_on_drop(true);

        let transport = TokioChildProcess::new(command.configure(|cmd| {
            cmd.stdin(std::process::Stdio::piped());
            cmd.stdout(std::process::Stdio::piped());
            cmd.stderr(std::process::Stdio::inherit());
        }))
        .with_context(|| format!("failed to spawn process {:?}", config.command))?;

        Self::from_transport(service, events, transport).await
    }

    /// Connects to a remote SSE MCP endpoint.
    pub async fn connect_sse(config: SseConfig) -> Result<Self> {
        let (service, events) = ClientService::with_channel();
        let headers = build_header_map(&config.headers, config.auth_token.as_deref())?;
        let client = ReqwestClient::builder()
            .default_headers(headers)
            .build()
            .context("failed to build reqwest client for SSE transport")?;

        let mut sse_config =
            SseClientConfig { sse_endpoint: config.endpoint.into(), ..Default::default() };
        sse_config.use_message_endpoint = config.message_endpoint;

        let transport = SseClientTransport::start_with_client(client, sse_config)
            .await
            .context("failed to start SSE transport")?;

        Self::from_transport(service, events, transport).await
    }

    /// Connects to an MCP server that exposes the streaming HTTP protocol.
    pub async fn connect_stream_http(config: StreamHttpConfig) -> Result<Self> {
        let (service, events) = ClientService::with_channel();
        let headers = build_header_map(&config.headers, config.auth_token.as_deref())?;
        let client = ReqwestClient::builder()
            .default_headers(headers)
            .build()
            .context("failed to build reqwest client for streaming HTTP transport")?;

        let mut transport_config = StreamableHttpClientTransportConfig::with_uri(config.endpoint);
        transport_config.allow_stateless = config.allow_stateless;
        transport_config.auth_header = config.auth_token;

        let transport = StreamableHttpClientTransport::with_client(client, transport_config);

        Self::from_transport(service, events, transport).await
    }

    async fn from_transport<T, E, A>(
        service: ClientService,
        events: broadcast::Sender<ClientEvent>,
        transport: T,
    ) -> Result<Self>
    where
        T: rmcp::transport::IntoTransport<RoleClient, E, A>,
        E: std::error::Error + Send + Sync + 'static,
    {
        let runtime = service
            .clone()
            .serve(transport)
            .await
            .context("failed to initialise MCP client")?;

        if let Some(info) = runtime.peer().peer_info().cloned() {
            let _ = events.send(ClientEvent::Initialized(info));
        }

        Ok(Self { runtime: Some(runtime), events })
    }

    fn peer(&self) -> &Peer<RoleClient> {
        self.runtime.as_ref().expect("client runtime should be available").peer()
    }

    /// Subscribes to live events produced by the client.
    pub fn subscribe(&self) -> broadcast::Receiver<ClientEvent> {
        self.events.subscribe()
    }

    /// Returns the cached initialize result (if the handshake completed successfully).
    pub fn initialize_result(&self) -> Option<InitializeResult> {
        self.runtime.as_ref().and_then(|runtime| runtime.peer().peer_info().cloned())
    }

    /// Fetches all tools exposed by the connected MCP server.
    pub async fn list_all_tools(&self) -> Result<Vec<Tool>> {
        self.peer().list_all_tools().await.context("failed to list tools")
    }

    /// Runs a single `list_tools` request and returns the raw response.
    pub async fn list_tools_page(&self) -> Result<ListToolsResult> {
        self.peer().list_tools(None).await.context("failed to execute list_tools")
    }

    /// Calls a tool by name using the provided JSON arguments.
    pub async fn call_tool(
        &self,
        name: impl Into<String>,
        arguments: Option<Value>,
    ) -> Result<CallToolResult> {
        let arguments =
            arguments.map(json_to_object).transpose().context("invalid tool arguments")?;
        let params = CallToolRequestParam { name: Cow::Owned(name.into()), arguments };
        self.peer().call_tool(params).await.context("tool invocation failed")
    }

    /// Attempts to gracefully shut down the underlying transport task.
    pub async fn shutdown(mut self) -> Result<()> {
        if let Some(runtime) = self.runtime.take() {
            let result = runtime.cancel().await;
            match result {
                Ok(QuitReason::Cancelled) | Ok(QuitReason::Closed) => Ok(()),
                Ok(QuitReason::JoinError(err)) => Err(anyhow!(err)),
                Err(err) => Err(anyhow!(err)),
            }
        } else {
            Ok(())
        }
    }
}

impl Drop for TestClient {
    fn drop(&mut self) {
        if let Some(runtime) = self.runtime.take() {
            debug!("dropping TestClient; cancelling MCP runtime");
            runtime.cancellation_token().cancel();
            // we intentionally detach here; callers should use `shutdown` when they
            // need deterministic shutdown semantics.
        }
    }
}

#[derive(Clone)]
struct ClientService {
    inner: Arc<ClientServiceInner>,
}

struct ClientServiceInner {
    events: broadcast::Sender<ClientEvent>,
}

impl ClientService {
    fn with_channel() -> (Self, broadcast::Sender<ClientEvent>) {
        let (tx, _) = broadcast::channel(256);
        (Self::new(tx.clone()), tx)
    }

    fn new(events: broadcast::Sender<ClientEvent>) -> Self {
        Self { inner: Arc::new(ClientServiceInner { events }) }
    }

    fn emit(&self, event: ClientEvent) {
        let _ = self.inner.events.send(event);
    }
}

impl Service<RoleClient> for ClientService {
    async fn handle_request(
        &self,
        request: <RoleClient as rmcp::service::ServiceRole>::PeerReq,
        _context: RequestContext<RoleClient>,
    ) -> Result<ClientResult, McpError> {
        self.emit(ClientEvent::Request(request.clone()));
        match request {
            ServerRequest::PingRequest(_) => Ok(ClientResult::empty(())),
            other => {
                self.emit(ClientEvent::Warning {
                    message: format!("unsupported server request: {other:?}"),
                });
                Err(McpError::internal_error("unsupported server-initiated request", None))
            }
        }
    }

    async fn handle_notification(
        &self,
        notification: <RoleClient as rmcp::service::ServiceRole>::PeerNot,
        _context: NotificationContext<RoleClient>,
    ) -> Result<(), McpError> {
        self.emit(ClientEvent::Notification(notification));
        Ok(())
    }

    fn get_info(&self) -> <RoleClient as rmcp::service::ServiceRole>::Info {
        Default::default()
    }
}

/// Connection request variants supported by [`TestClient`].
#[derive(Debug, Clone)]
pub enum ConnectRequest {
    StdIo(StdIoConfig),
    Sse(SseConfig),
    StreamHttp(StreamHttpConfig),
}

/// Configuration for stdio transports.
#[derive(Debug, Clone)]
pub struct StdIoConfig {
    pub command: PathBuf,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

impl StdIoConfig {
    pub fn new(command: impl Into<PathBuf>) -> Self {
        Self { command: command.into(), args: Vec::new(), env: Vec::new() }
    }

    pub fn with_args(mut self, args: Vec<String>) -> Self {
        self.args = args;
        self
    }

    pub fn with_env(mut self, env: Vec<(String, String)>) -> Self {
        self.env = env;
        self
    }
}

/// Configuration for SSE transports.
#[derive(Debug, Clone)]
pub struct SseConfig {
    pub endpoint: String,
    pub message_endpoint: Option<String>,
    pub headers: Vec<(String, String)>,
    pub auth_token: Option<String>,
}

impl SseConfig {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            message_endpoint: None,
            headers: Vec::new(),
            auth_token: None,
        }
    }

    pub fn with_headers(mut self, headers: Vec<(String, String)>) -> Self {
        self.headers = headers;
        self
    }

    pub fn with_message_endpoint(mut self, endpoint: Option<String>) -> Self {
        self.message_endpoint = endpoint;
        self
    }

    pub fn with_auth_token(mut self, token: Option<String>) -> Self {
        self.auth_token = token;
        self
    }
}

/// Configuration for streaming HTTP transports.
#[derive(Debug, Clone)]
pub struct StreamHttpConfig {
    pub endpoint: String,
    pub headers: Vec<(String, String)>,
    pub auth_token: Option<String>,
    pub allow_stateless: bool,
}

impl StreamHttpConfig {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            headers: Vec::new(),
            auth_token: None,
            allow_stateless: true,
        }
    }

    pub fn with_headers(mut self, headers: Vec<(String, String)>) -> Self {
        self.headers = headers;
        self
    }

    pub fn with_auth_token(mut self, token: Option<String>) -> Self {
        self.auth_token = token;
        self
    }

    pub fn allow_stateless(mut self, allow: bool) -> Self {
        self.allow_stateless = allow;
        self
    }
}

fn build_header_map(pairs: &[(String, String)], auth_token: Option<&str>) -> Result<HeaderMap> {
    let mut map = HeaderMap::new();
    for (name, value) in pairs {
        let header_name = HeaderName::from_bytes(name.as_bytes())
            .with_context(|| format!("invalid header name '{name}'"))?;
        let header_value = HeaderValue::from_str(value)
            .with_context(|| format!("invalid header value for '{name}'"))?;
        map.insert(header_name, header_value);
    }
    if let Some(token) = auth_token {
        let header_value =
            HeaderValue::from_str(&format!("Bearer {token}")).context("invalid auth token")?;
        map.insert(AUTHORIZATION, header_value);
    }
    Ok(map)
}

fn json_to_object(value: Value) -> Result<JsonObject> {
    match value {
        Value::Null => Ok(JsonObject::new()),
        Value::Object(map) => Ok(map),
        other => Err(anyhow!("tool arguments must be a JSON object, got {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_header_map_adds_authorization() {
        let headers = vec![("x-test".to_string(), "value".to_string())];
        let map = build_header_map(&headers, Some("token-123")).expect("header map");
        assert_eq!(map.get("x-test").unwrap(), "value");
        assert_eq!(map.get(AUTHORIZATION).unwrap(), "Bearer token-123");
    }
}
