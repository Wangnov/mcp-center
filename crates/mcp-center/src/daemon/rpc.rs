//! Daemon RPC interface for CLI communication via Unix Socket

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, error, info};

use interprocess::local_socket::traits::tokio::Listener as _;
use interprocess::local_socket::{
    GenericFilePath, ListenerOptions, ToFsName, tokio::prelude::LocalSocketStream,
};

use crate::daemon::server_manager::ServerManager;

/// RPC request from CLI to daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", content = "params")]
pub enum DaemonRequest {
    /// List all tools from a specific server or all servers
    ListTools { server_name: Option<String> },
    /// Get detailed info about a specific tool
    GetToolInfo { tool_name: String },
    /// Ping to check if daemon is alive
    Ping,
}

/// RPC response from daemon to CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum DaemonResponse {
    #[serde(rename = "ok")]
    Success { data: ResponseData },
    #[serde(rename = "error")]
    Error { message: String },
}

/// Response data variants
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResponseData {
    ToolList(Vec<ToolInfo>),
    ToolInfo(ToolInfo),
    Pong(String),
}

/// Tool information for CLI display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub name: String,
    pub description: String,
    pub server_name: String,
}

/// RPC server that listens on Unix Socket
pub struct RpcServer {
    manager: Arc<ServerManager>,
    socket_path: std::path::PathBuf,
}

impl RpcServer {
    pub fn new(manager: Arc<ServerManager>, socket_path: std::path::PathBuf) -> Self {
        Self { manager, socket_path }
    }

    /// Start the RPC server
    pub async fn start(self) -> anyhow::Result<()> {
        // Remove existing socket file if present
        #[cfg(unix)]
        {
            if self.socket_path.exists() {
                std::fs::remove_file(&self.socket_path)?;
            }

            // Ensure parent directory exists
            if let Some(parent) = self.socket_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let socket_display = self.socket_path.to_string_lossy().into_owned();
        let listener_name = socket_display.as_str().to_fs_name::<GenericFilePath>()?;
        let listener = ListenerOptions::new().name(listener_name).create_tokio()?;
        info!("RPC server listening on {}", socket_display);

        loop {
            match listener.accept().await {
                Ok(stream) => {
                    let manager = self.manager.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, manager).await {
                            error!("Error handling RPC connection: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Error accepting RPC connection: {}", e);
                }
            }
        }
    }
}

/// Handle a single RPC connection
async fn handle_connection(
    stream: LocalSocketStream,
    manager: Arc<ServerManager>,
) -> anyhow::Result<()> {
    let (reader, mut writer) = tokio::io::split(stream);
    let mut reader = BufReader::new(reader);
    let mut line = String::new();

    while reader.read_line(&mut line).await? > 0 {
        let request: DaemonRequest = match serde_json::from_str(line.trim()) {
            Ok(req) => req,
            Err(e) => {
                let response = DaemonResponse::Error { message: format!("Invalid request: {e}") };
                let response_json = serde_json::to_string(&response)? + "\n";
                writer.write_all(response_json.as_bytes()).await?;
                line.clear();
                continue;
            }
        };

        debug!("Received RPC request: {:?}", request);

        let response = handle_request(request, &manager).await;
        let response_json = serde_json::to_string(&response)? + "\n";
        writer.write_all(response_json.as_bytes()).await?;

        line.clear();
    }

    Ok(())
}

/// Handle a single RPC request
async fn handle_request(request: DaemonRequest, manager: &ServerManager) -> DaemonResponse {
    match request {
        DaemonRequest::ListTools { server_name } => match list_tools(manager, server_name).await {
            Ok(tools) => DaemonResponse::Success { data: ResponseData::ToolList(tools) },
            Err(e) => DaemonResponse::Error { message: format!("Failed to list tools: {e}") },
        },
        DaemonRequest::GetToolInfo { tool_name } => {
            match get_tool_info(manager, &tool_name).await {
                Ok(info) => DaemonResponse::Success { data: ResponseData::ToolInfo(info) },
                Err(e) => {
                    DaemonResponse::Error { message: format!("Failed to get tool info: {e}") }
                }
            }
        }
        DaemonRequest::Ping => {
            DaemonResponse::Success { data: ResponseData::Pong("pong".to_string()) }
        }
    }
}

/// List tools from specified server or all servers
async fn list_tools(
    manager: &ServerManager,
    server_name: Option<String>,
) -> anyhow::Result<Vec<ToolInfo>> {
    let entries = manager.list_tools().await?;

    let mut tools = Vec::new();
    for entry in entries {
        // Filter by server name if specified
        if let Some(ref name) = server_name {
            if entry.server_name != *name {
                continue;
            }
        }

        tools.push(ToolInfo {
            name: entry.tool.name.to_string(),
            description: entry.tool.description.clone().unwrap_or_default().to_string(),
            server_name: entry.server_name.clone(),
        });
    }

    Ok(tools)
}

/// Get detailed info about a specific tool
async fn get_tool_info(manager: &ServerManager, tool_name: &str) -> anyhow::Result<ToolInfo> {
    let entries = manager.list_tools().await?;

    for entry in entries {
        if entry.tool.name == tool_name {
            return Ok(ToolInfo {
                name: entry.tool.name.to_string(),
                description: entry.tool.description.clone().unwrap_or_default().to_string(),
                server_name: entry.server_name.clone(),
            });
        }
    }

    Err(anyhow::anyhow!("Tool '{tool_name}' not found"))
}
