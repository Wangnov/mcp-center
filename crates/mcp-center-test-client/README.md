# mcp-center-test-client

轻量级 MCP 客户端，用于调试和端到端测试 MCP Center 或任意 MCP 服务器。

## 功能

- **库 (`lib.rs`)**
  - `TestClient::connect_stdio / connect_sse / connect_stream_http`
  - `list_all_tools`, `call_tool`, `subscribe`, `shutdown`
  - 广播 MCP 事件，便于测试中观察通知与请求
- **CLI (`src/bin/main.rs`)**
  - `list-tools`：列出目标 MCP server 的工具
  - `call-tool`：直接调用工具，支持 JSON 参数
  - `watch`：实时查看 MCP 事件
  - `info`：只完成握手并输出 `InitializeResult`

## 使用示例

```bash
# 通过 stdio 连接本地 mcp-center bridge
cargo run -p mcp-center-test-client -- \
  list-tools stdio --cmd ./target/debug/mcp-center --arg connect

# 调用远程 SSE server 的工具
cargo run -p mcp-center-test-client -- \
  call-tool sse --url https://example.com/mcp/sse \
  --name "resolve-library-id" \
  --args-json '{"query":"tokio"}'
```

在测试中可直接引入 `mcp_center_test_client::TestClient` 作为库使用。

## 依赖提醒

- 核心依赖 `rmcp 0.8.1`，与主后端保持一致。
- SSE / streaming HTTP 需启用 `reqwest` 的 `json`、`stream`、`rustls` 特性。

更多细节可参考同目录下的 `AGENTS.md` / `CLAUDE.md`。
