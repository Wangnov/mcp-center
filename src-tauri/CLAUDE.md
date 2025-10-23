# Tauri 后端概览（AI 助手用）

## 当前状态

- 依赖 `tauri 2.0` + `tokio`，仅实现基础框架与若干占位命令，尚未接入 Rust 守护进程或核心逻辑。
- `commands.rs` 中除 `greet` / `get_app_version` / 环境变量查询外，其余 API（列表、启停服务器）均直接返回错误提示，前端实际仍通过 HTTP API 调用后端。
- `tauri-plugin-shell` 已启用，便于从桌面侧运行命令。
- `tauri.conf.json`/`build.rs`/`icons/` 等已就绪，可通过 `npm run tauri:dev` 启动基本窗口。

## 构建运行

- 开发模式：`npm run tauri:dev`（先确保后端 HTTP API 已运行，否则前端请求失败）。
- 目前 Tauri 侧主要用于：
  1. 在 `main.tsx` 中读取 `get_backend_base_url` / `get_backend_auth_token`。
  2. 提供 `greet` 等示例命令。
- 尚未注册菜单、托盘、窗口多实例等功能。

## 下一步建议

- 将实际的 `mcp-center` 守护进程/HTTP API 启动逻辑移动至 Tauri（或提供显式按钮），然后在命令中调用核心 crate。
- 用 `tauri::async_runtime::spawn` 或 `tokio` runtime 管理后台任务，并暴露状态/日志给前端。
- 丰富命令（list/toggle 等），或干脆复用 HTTP API（例如请求本地 CLI 端点）以减轻桌面端负担。
