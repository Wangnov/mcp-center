// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tracing::info;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod commands;

fn main() {
    // 初始化日志系统
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    info!("启动 MCP Center 应用...");

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::greet,
            commands::get_app_version,
            commands::list_mcp_servers,
            commands::toggle_server_enabled,
            commands::get_backend_base_url,
            commands::get_backend_auth_token,
        ])
        .run(tauri::generate_context!())
        .expect("运行 Tauri 应用时出错");
}
