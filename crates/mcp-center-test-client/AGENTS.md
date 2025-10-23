# MCP Center Test Client 概览（AI 助手用）

## 目标

- 提供一个轻量级、独立的 MCP 客户端，用于调试/测试 `mcp-center` 或任意 MCP 服务器。
- 既可作为库在 `mcp-center` 测试中复用，也可通过 `cargo run -p mcp-center-test-client -- ...` 直接使用 CLI 工具完成端到端调用。

## 结构

```text
src/
├── lib.rs           # TestClient 库，支持 stdio/SSE/streaming HTTP 三种连接方式
└── bin/main.rs      # 命令行入口（list-tools / call-tool / watch / info）
```

## 主要能力

- `TestClient::connect_*`：根据配置连接 MCP 服务，返回可复用的运行时实例。
- `list_all_tools` / `call_tool`：封装常见操作，内部通过 rmcp `Peer<RoleClient>` 调度。
- `subscribe()`：广播 MCP 事件（初始化、通知、警告）到 `tokio::broadcast::Receiver`，便于实时调试。
- `shutdown()`：显式关闭底层 transport，防止测试中残留任务。
- `bin/main.rs` CLI：
  - `list-tools`：列举工具列表，支持 JSON 输出。
  - `call-tool`：一次性调用指定工具，可从命令行或文件读取 JSON 参数。
  - `watch`：持续打印事件流，适合观察 streaming 行为。
  - `info`：只做握手并输出 `InitializeResult`。
  - `TransportCommand` 支持 `stdio`（默认执行 `mcp-center connect`）、`sse`、`stream-http` 三种方式。

## 关键依赖

- `rmcp` 0.8.1：与主仓相同的 MCP SDK，用于连接远程/本地服务。
- `reqwest`（`json`/`stream`/`rustls` 特性）：SSE 与 streaming HTTP 需要。
- `tokio`：异步运行时，CLI/库共用。
- `clap`：命令行解析。
- `serde_json`：解析 CLI 传入的工具参数 / 输出结果。

## 与主仓库的协作

- `mcp-center` 集成测试可使用 `mcp_center_test_client::TestClient` 编写更高阶用例。
- CLI 默认通过 `--cmd mcp-center` 调用主仓的 `connect` 子命令，可根据测试需要替换为任意 MCP 实现。

## 提示

- 运行 CLI 示例：`cargo run -p mcp-center-test-client -- list-tools stdio --cmd ./target/debug/mcp-center`.
- 测试工具调用时建议显式调用 `shutdown()`，避免后台任务影响后续测试。
- 若扩展新的传输协议，需同时在 `ConnectRequest`/`TransportCommand` 中添加分支。
