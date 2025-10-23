/**
 * Tauri Commands
 *
 * 这些函数可以从前端 JavaScript/TypeScript 调用
 */

use std::env;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/* ========================================
   数据类型定义
   ======================================== */

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub id: String,
    pub name: String,
    pub protocol: String,
    pub enabled: bool,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub url: Option<String>,
    pub tool_count: usize,
}

/* ========================================
   命令实现
   ======================================== */

/// 测试命令 - 欢迎消息
#[tauri::command]
pub fn greet(name: &str) -> String {
    info!("收到 greet 命令，name: {}", name);
    format!("你好, {}! 欢迎使用 MCP Center!", name)
}

/// 获取应用版本
#[tauri::command]
pub fn get_app_version() -> String {
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    debug!("获取应用版本: {}", VERSION);
    VERSION.to_string()
}

/// 获取 MCP 服务器列表
///
/// TODO: 接入真实的 MCP Center 核心库
#[tauri::command]
pub async fn list_mcp_servers() -> Result<Vec<McpServer>, String> {
    Err("list_mcp_servers is not available via Tauri command. Please ensure HTTP API is configured.".into())
}

/// 切换服务器启用状态
///
/// TODO: 接入真实的 MCP Center 核心库
#[tauri::command]
pub async fn toggle_server_enabled(server_id: String) -> Result<(), String> {
    Err(format!(
        "toggle_server_enabled currently requires the HTTP API (server: {server_id})."
    ))
}

/// 获取后端 HTTP 基础地址
#[tauri::command]
pub async fn get_backend_base_url() -> Result<Option<String>, String> {
    const CANDIDATE_ENV_VARS: [&str; 2] = ["MCP_CENTER_HTTP_BASE_URL", "MCP_CENTER_HTTP_BASE"];

    for key in CANDIDATE_ENV_VARS {
        if let Ok(value) = env::var(key) {
            let trimmed = value.trim().trim_end_matches('/');
            if !trimmed.is_empty() {
                info!("使用环境变量 {} 提供的 HTTP 基础地址: {}", key, trimmed);
                return Ok(Some(trimmed.to_string()));
            }
        }
    }

    // 兜底使用默认本地端口
    let default_base = "http://127.0.0.1:8787/api";
    warn!(
        "未检测到 MCP_CENTER_HTTP_BASE_URL 环境变量，使用默认地址 {}",
        default_base
    );
    Ok(Some(default_base.to_string()))
}

/// 获取后端 HTTP 鉴权 Token（可选）
#[tauri::command]
pub async fn get_backend_auth_token() -> Result<Option<String>, String> {
    match env::var("MCP_CENTER_HTTP_TOKEN") {
        Ok(value) if !value.trim().is_empty() => Ok(Some(value.trim().to_string())),
        Ok(_) => Ok(None),
        Err(_) => Ok(None),
    }
}
