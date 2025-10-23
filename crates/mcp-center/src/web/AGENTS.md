# Web 模块速览（AI 助手用）

## 模块总览

- 位置：`crates/mcp-center/src/web/http.rs`
- 角色：提供 Axum 驱动的 HTTP API，供桌面端/WebUI/研发工具访问后端守护进程能力。
- 依赖核心对象：
  - `HttpState`：封装 `ServerManager`、`ProjectRegistry`、`Layout` 与鉴权配置。
  - `HttpServerHandle`：封装监听 socket 与后台任务，统一管理启动 / 停止。
  - `ServerManager` / `ProjectRegistry`：来自其他模块的业务核心，用于实际读写服务器、项目状态。

## 路由与接口

- 入口函数：`build_router(state: HttpState) -> Router`
  - 绑定所有 API、注入中间件（鉴权、CORS、日志）。
  - 主要路由前缀：`/api`.
- 重要端点：
  - `GET /api/health`：健康检查。
  - `GET /api/mcp`、`POST /api/mcp`、`PATCH /api/mcp/:id/enabled`、`DELETE /api/mcp/:id` 等：管理 MCP Server。
  - `GET /api/project`、`POST /api/project/allow|deny|allow-tools|deny-tools|set-tool-desc|reset-tool-desc` 等：操控项目权限。
  - `GET /api/mcp/:id/tools`、`GET /api/mcp/:id`：查询服务器详情与工具列表。
- Handler 均以 `async fn` 形式实现，并返回 `Json<T>` 或 `Result<Json<T>, ApiError>`，Axum 自动完成序列化。

## 数据模型与 Specta

- 所有对外响应 / 请求体结构都在同一文件定义，并同时派生：
  - `serde::{Serialize, Deserialize}`（依场景而定），确保 HTTP 正常收发。
  - `specta::Type`，用于 Specta 输出 TypeScript 类型。
- TypeScript 类型生成脚本：`crates/mcp-center/src/bin/export-types.rs`
  - 运行 `npm run export-types` 或 `cargo run --bin export-types` 自动生成 `src/lib/api-types.generated.ts`。
  - 生成文件是前端查看契约的首选单一来源。

## 鉴权与中间件

- Token 鉴权：
  - `HttpAuth` 结构读取配置，`auth::auth_layer` 中间件验证 `Authorization: Bearer <token>`。
  - 鉴权失败返回 `401 Unauthorized`；`health` 等少数接口可跳过。
- CORS：
  - 使用 `tower_http::cors::CorsLayer`，默认允许常用动词与任意来源（参见 `build_router` 中配置）。
- 日志：
  - `tracing` 在 handler 内输出调试信息，方便排查。

## 文件结构速览

- `HttpState` / `HttpServerHandle` / `HttpAuth`：模块顶端定义。
- Type 定义区域（Specta/Serde 模型）。
- `ApiError` 与错误映射：统一将 `CoreError` 转换为 HTTP 错误码与消息。
- 辅助函数：
  - `collect_server_tools`、`load_or_create_project`、`parse_tool_spec` 等封装重复逻辑。
  - `normalize_project_path` 负责展开 `~`、转换绝对路径，并应用 `ProjectId`.

## 测试与调试

- 端到端测试位于 `crates/mcp-center/tests/http_api.rs`：
  - 使用临时目录 / `tokio` runtime 构建 Router，调用实际 HTTP handler 验证完整流程。
  - 覆盖项目权限、服务器增删、鉴权等主流程。
- 本地调试建议：
  1. 运行守护进程：`cargo run --bin mcp-center serve`.
  2. 或直接在测试中通过 `build_router` 生成 `Router`，用 `oneshot` 发送 Mock 请求。
  3. 设置 `RUST_LOG=debug` 观测详细日志。

## 注意事项

- 字段命名：当前多数响应使用 `snake_case`（与 CLI / 测试一致），Specta 会导出对应风格；若未来改为 `camelCase` 需同步前端、测试、Specta 类型。
- 项目路径解析：严格依赖 `ProjectRegistry` 与 `Layout`，路径展开/规范化失败会直接返回 `400`/`500`。修改时需同步测试场景。
- BigInt 类型：`u64`/`usize` 通过 Specta 脚本强制导出为 TypeScript `number`，请确认值不超过 `Number.MAX_SAFE_INTEGER`。

## 常用命令

- 启动 HTTP 服务（开发模式）：

  ```bash
  cargo run --bin mcp-center -- serve
  ```

- 生成最新 TypeScript 类型：

  ```bash
  npm run export-types
  ```

- 运行 HTTP 相关测试：

  ```bash
  cargo test --test http_api
  ```

如需扩展接口，建议先在 `http.rs` 中添加模型与 handler，补充 Specta 派生，然后更新 `export-types.rs` 中的 `EXPORT_TARGETS` 常量并重生成前端类型。
