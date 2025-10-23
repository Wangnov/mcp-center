use std::sync::{Arc, RwLock};

use crate::project::ToolPermission;
use crate::{Layout, ProjectId, ProjectRegistry};
use rmcp::{
    ErrorData as McpError,
    model::{
        CallToolRequest, CallToolRequestParam, ClientRequest, CompleteRequestMethod,
        GetPromptRequestMethod, InitializeResult, ListPromptsRequestMethod,
        ListResourceTemplatesRequestMethod, ListResourcesRequestMethod, ListToolsResult,
        ProtocolVersion, ReadResourceRequestMethod, ServerCapabilities, ServerResult,
        SetLevelRequestMethod, SubscribeRequestMethod, Tool, UnsubscribeRequestMethod,
    },
    service::{NotificationContext, RequestContext, RoleServer, Service},
};
use tracing::{debug, warn};

use crate::daemon::server_manager::ServerManager;

pub struct HostService {
    manager: Arc<ServerManager>,
    layout: Layout,
    project_id: Arc<RwLock<ProjectId>>,
    registry: ProjectRegistry,
}

impl HostService {
    pub fn new(
        manager: Arc<ServerManager>,
        layout: Layout,
        project_id: Arc<RwLock<ProjectId>>,
        registry: ProjectRegistry,
    ) -> Self {
        Self { manager, layout, project_id, registry }
    }

    /// Check if a specific tool is allowed for the current project
    /// Priority: allowed_server_tools > allowed_server_ids
    fn is_tool_allowed(&self, tool_name: &str, server_id: &str) -> bool {
        let project_id = self.project_id.read().unwrap();
        match self.registry.load(&project_id) {
            Ok(record) => {
                // 1. Check tool-level permissions first (highest priority)
                if let Some(permission) = record.allowed_server_tools.get(server_id) {
                    let allowed = match permission {
                        ToolPermission::All => true,
                        ToolPermission::AllowList { tools } => {
                            tools.contains(&tool_name.to_string())
                        }
                        ToolPermission::DenyList { tools } => {
                            !tools.contains(&tool_name.to_string())
                        }
                    };
                    debug!(
                        "Tool-level permission check: tool='{}', server='{}', permission={:?}, allowed={}",
                        tool_name, server_id, permission, allowed
                    );
                    return allowed;
                }

                // 2. Fallback to server-level permissions
                if record.allowed_server_ids.is_empty() {
                    // Empty list = allow all
                    true
                } else {
                    record.allowed_server_ids.contains(&server_id.to_string())
                }
            }
            Err(_) => {
                // No project record = allow all
                true
            }
        }
    }

    /// Get custom description for a tool if configured
    fn get_custom_description(&self, tool_name: &str) -> Option<String> {
        let project_id = self.project_id.read().unwrap();
        match self.registry.load(&project_id) {
            Ok(record) => {
                for customization in &record.tool_customizations {
                    if customization.tool_name == tool_name {
                        return customization.description.clone();
                    }
                }
                None
            }
            Err(_) => None,
        }
    }

    fn server_info(&self) -> InitializeResult {
        let capabilities = ServerCapabilities::builder().enable_tools().build();

        let instructions = {
            let servers = self.manager.list_server_names();
            if servers.is_empty() {
                format!(
                    "MCP Center bridge (root: {}). No servers currently available.",
                    self.layout.root().display()
                )
            } else {
                format!(
                    "MCP Center bridge (root: {}). Managed servers: {}.",
                    self.layout.root().display(),
                    servers.join(", ")
                )
            }
        };

        InitializeResult {
            protocol_version: ProtocolVersion::default(),
            capabilities,
            server_info: rmcp::model::Implementation {
                name: "mcp-center".to_string(),
                title: Some("MCP Center Aggregator".to_string()),
                version: env!("CARGO_PKG_VERSION").to_string(),
                icons: None,
                website_url: None,
            },
            instructions: Some(instructions),
        }
    }

    async fn list_tools(&self) -> std::result::Result<Vec<Tool>, McpError> {
        let entries = self
            .manager
            .list_tools()
            .await
            .map_err(|err| McpError::internal_error(err.to_string(), None))?;

        // Permission control: Use tool-level permission check
        debug!("Filtering tools with tool-level permissions");

        let mut filtered: Vec<Tool> = Vec::new();
        for entry in entries {
            if self.is_tool_allowed(&entry.tool.name, &entry.server_id) {
                let mut tool = entry.tool;

                // Apply custom description if configured
                if let Some(custom_desc) = self.get_custom_description(&tool.name) {
                    debug!("Applying custom description for tool '{}'", tool.name);
                    tool.description = Some(custom_desc.into());
                }

                filtered.push(tool);
            } else {
                debug!(
                    "  Filtered out tool '{}' from server '{}'",
                    entry.tool.name, entry.server_id
                );
            }
        }

        debug!("Returning {} tools (filtered from original list)", filtered.len());
        Ok(filtered)
    }

    async fn call_tool(
        &self,
        params: CallToolRequestParam,
    ) -> std::result::Result<ServerResult, McpError> {
        debug!("=== DEBUG: Tool Call Received ===");
        debug!("  tool_name: {}", params.name);
        debug!(
            "  arguments: {}",
            serde_json::to_string_pretty(&params.arguments).unwrap_or_else(|_| "{}".to_string())
        );

        // Permission control: Use tool-level permission check
        if let Ok(tool_server_id) = self.manager.get_server_for_tool(&params.name).await {
            if !self.is_tool_allowed(&params.name, &tool_server_id) {
                warn!(
                    "Tool '{}' from server '{}' not allowed for this project",
                    params.name, tool_server_id
                );
                return Err(McpError::invalid_params(
                    format!(
                        "Tool '{}' from server '{}' is not allowed for this project",
                        params.name, tool_server_id
                    ),
                    None,
                ));
            }
            debug!("  server: {} (allowed)", tool_server_id);
        }

        let result = self.manager.call_tool(params).await?;

        debug!(
            "  result: {}",
            serde_json::to_string(&result).unwrap_or_else(|_| "error".to_string())
        );
        debug!("=== DEBUG: Tool Call Complete ===");

        Ok(ServerResult::CallToolResult(result))
    }
}

impl Service<RoleServer> for HostService {
    async fn handle_request(
        &self,
        request: <RoleServer as rmcp::service::ServiceRole>::PeerReq,
        _context: RequestContext<RoleServer>,
    ) -> Result<ServerResult, McpError> {
        match request {
            ClientRequest::InitializeRequest(_) => {
                Ok(ServerResult::InitializeResult(self.server_info()))
            }
            ClientRequest::PingRequest(_) => Ok(ServerResult::empty(())),
            ClientRequest::ListToolsRequest(_) => {
                let tools = self.list_tools().await?;
                Ok(ServerResult::ListToolsResult(ListToolsResult::with_all_items(tools)))
            }
            ClientRequest::CallToolRequest(CallToolRequest { params, .. }) => {
                self.call_tool(params).await
            }
            ClientRequest::CompleteRequest(_) => {
                Err(McpError::method_not_found::<CompleteRequestMethod>())
            }
            ClientRequest::SetLevelRequest(_) => {
                Err(McpError::method_not_found::<SetLevelRequestMethod>())
            }
            ClientRequest::GetPromptRequest(_) => {
                Err(McpError::method_not_found::<GetPromptRequestMethod>())
            }
            ClientRequest::ListPromptsRequest(_) => {
                Err(McpError::method_not_found::<ListPromptsRequestMethod>())
            }
            ClientRequest::ListResourcesRequest(_) => {
                Err(McpError::method_not_found::<ListResourcesRequestMethod>())
            }
            ClientRequest::ListResourceTemplatesRequest(_) => {
                Err(McpError::method_not_found::<ListResourceTemplatesRequestMethod>())
            }
            ClientRequest::ReadResourceRequest(_) => {
                Err(McpError::method_not_found::<ReadResourceRequestMethod>())
            }
            ClientRequest::SubscribeRequest(_) => {
                Err(McpError::method_not_found::<SubscribeRequestMethod>())
            }
            ClientRequest::UnsubscribeRequest(_) => {
                Err(McpError::method_not_found::<UnsubscribeRequestMethod>())
            }
        }
    }

    async fn handle_notification(
        &self,
        _notification: <RoleServer as rmcp::service::ServiceRole>::PeerNot,
        _context: NotificationContext<RoleServer>,
    ) -> Result<(), McpError> {
        Ok(())
    }

    fn get_info(&self) -> <RoleServer as rmcp::service::ServiceRole>::Info {
        self.server_info()
    }
}
