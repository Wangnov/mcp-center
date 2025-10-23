use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Message sent by `mcp-center-bridge` upon establishing a control connection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeHello {
    pub project_path: PathBuf,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub bridge_pid: Option<u32>,
    #[serde(default)]
    pub metadata: serde_json::Value,
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
    pub fn hello(
        project_path: PathBuf,
        agent: Option<String>,
        bridge_pid: Option<u32>,
        metadata: serde_json::Value,
    ) -> Self {
        ControlMessage::BridgeHello(BridgeHello { project_path, agent, bridge_pid, metadata })
    }

    #[allow(dead_code)]
    pub fn ready(
        project_id: String,
        project_path: PathBuf,
        allowed_server_ids: Vec<String>,
    ) -> Self {
        ControlMessage::BridgeReady(BridgeReady { project_id, project_path, allowed_server_ids })
    }

    #[allow(dead_code)]
    pub fn error(message: impl Into<String>) -> Self {
        ControlMessage::Error { message: message.into() }
    }
}
