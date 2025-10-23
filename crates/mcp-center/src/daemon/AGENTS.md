# Daemon 模块速览（AI 助手用）

## 目标

- 在后台统一管理 MCP Server 生命周期、桥接客户端、HTTP/RPC 接口。
- 与项目权限系统协同，为 List/Call Tool 提供过滤。
- 提供 CLI (`serve`) 与 bridge (`connect`) 的运行支撑。

## 组成

- `serve.rs`：`ServeArgs` CLI 参数 + `run()` 入口。
  - 初始化 tracing、解析 root (`Layout`)、拉起 `ServerManager`。
  - 启动组件：
    - 控制 socket (`control::spawn_control_server`) —— 供 `mcp-center connect` 交互。
    - RPC socket (`RpcServer`) —— CLI 端工具/列表命令。
    - 可选 HTTP API (`web::http::spawn_http_server`)。
  - 监听 Ctrl+C 后执行清理（shutdown servers、关闭 socket、删除 pid/uds）。
- `control.rs`：控制通道协议。
  - 接受 `BridgeHello`，计算 `ProjectId`，持久化/迁移 `ProjectRecord`（从 `ProjectRegistry`）。
  - 构建 `HostService` + `AsyncRwTransport` 代理 remote MCP 协议。
  - 连接后尝试 `peer.list_roots()`，以真实 root 更新 project_id 并做记录迁移。
  - 维护 `Arc<RwLock<ProjectId>>`，供 HostService 访问最新项目权限。
- `host.rs`：MCP Host 实现（代理层）。
  - `HostService` 实现 `Service<RoleServer>`。
  - `list_tools`/`call_tool` 前检查 `ProjectRegistry` 中的 `ToolPermission` / `allowed_server_ids`；同时应用 `ToolCustomization` 覆盖描述。
  - 其余 MCP 方法目前返回 `method_not_found`。
- `server_manager.rs`：Server 生命周期与工具索引。
  - `ServerManager::start` 读取 `Layout::list_server_configs`，按协议启动本地子进程或远程传输。
  - 维护 `tool_cache` + `tool_index`（`HashMap<tool_name, server_id>`），并在 `ToolListChanged` 通知时置脏。
  - `ManagedServer` 封装 spawn/connect、日志写入、工具刷新、Call tool。
- `rpc.rs`：简单 JSON-RPC（行分隔）供 CLI 使用；支持 ListTools/GetToolInfo/Ping。

## 交互关系

- 依赖 `Layout` 管理目录、socket、日志文件。
- 项目数据通过 `ProjectRegistry` 读写，`control.rs` 调整记录后 `HostService` 即可获取。
- Web 层通过共享的 `ServerManager` / `ProjectRegistry` 提供 HTTP API。
- Bridge (`connect`) 使用 `ControlMessage` 与 daemon 建立 session。

## 注意事项

- 控制 socket 改为使用 `interprocess` 跨平台本地 socket，Unix/Windows 行为一致（Unix 仍需删除残留文件）。
- `server_manager` 使用 tokio + rmcp；处理 async 错误时多返回 `anyhow` 或 `McpError`，上层需充分日志。
- 工具缓存：调用 `list_tools`/`call_tool` 前先 `ensure_tool_cache()`，倚赖 `needs_refresh` 标志；若新增通知类型需同步设置。
- 日志写入：`ServerAdapterInner::write_log` 将服务端 log JSON 行追加到 `<logs>/<id>.log`。
- 清理：daemon 停止后需删除 rpc socket/控制 socket/pid 文件；`serve` 里已有基础处理，新增资源时别忘记清理。
