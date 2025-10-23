# MCP Center 后端扩展设计

## 1. 背景

- WebUI 与 Tauri 桌面端已共享 React 组件，但数据访问方式不同：WebUI 目前使用假数据，Tauri 通过 `invoke` 调用模拟命令。
- 核心守护进程（`mcp-center serve`）仅暴露 Unix Socket 控制通道与 CLI RPC，缺少 HTTP 接口，导致浏览器无法直接访问。
- 需要一个统一的服务层，让 WebUI、Tauri 以及未来可能出现的其它集成共用同一套 API，并沿用现有 `ServerManager`、`ProjectRegistry` 等业务逻辑。

## 2. 目标

1. 在守护进程进程内提供基于 Axum 的 HTTP API，覆盖服务器、项目、权限、日志等查询与操作能力。
2. 让 WebUI 与 Tauri 前端均通过 HTTP API 获取/提交数据，最大化代码复用，减少环境分支逻辑。
3. 引入中间件区分请求来源（Web、Tauri、本地内部接口等），为权限、鉴权、跨域、速率限制打好基础。
4. 保持守护进程原有控制通道与 CLI 功能不受影响，逐步迁移功能而非一次性替换。

## 3. 目标架构概览

```text
          ┌───────────────────────────┐
          │        前端客户端         │
          │  - Web 浏览器 (fetch)     │
          │  - Tauri WebView (fetch)  │
          └────────────┬──────────────┘
                       │ HTTP(S)
             ┌─────────▼─────────┐
             │  Axum Router 层   │
             │  /api/...         │
             └─────────┬─────────┘
                       │ Tower Layer
             ┌─────────▼─────────┐
             │ AppState (Arc<>)  │
             │ - ServerManager    │
             │ - ProjectRegistry  │
             │ - Layout           │
             └─────────┬─────────┘
                       │
          ┌────────────▼────────────┐
          │ 核心逻辑 (现有模块)      │
          │ - MCP 服务器管理         │
          │ - 权限过滤 / Host        │
          │ - Bridge / Control 流程  │
          └─────────────────────────┘
```

## 4. Axum HTTP 服务设计

### 4.1 路由与资源

参考 [Axum 官方路由示例](https://github.com/tokio-rs/axum/)（资料来自 Context7 文档检索），服务示例：

```rust
use axum::{routing::get, Router};
use std::sync::Arc;

#[derive(Clone)]
struct AppState {
    manager: Arc<ServerManager>,
    registry: ProjectRegistry,
}

fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/api/mcp", get(list_mcp))
        .route("/api/mcp/:name/enabled", patch(update_mcp_enabled))
        .route("/api/project", get(list_projects))
        .route("/api/project/allow", post(project_allow))
        .route("/api/project/deny", post(project_deny))
        .with_state(state)
}
```

规划的首批路由：

- `GET /api/mcp`：返回服务器列表（启用状态、协议、工具数量等）。
- `PATCH /api/mcp/:name/enabled`：切换启用状态。
- `GET /api/project`：列出所有项目及权限。
- `POST /api/project/allow`、`POST /api/project/deny`：更新项目的服务器权限。
- `POST /api/project/tools/allow`、`POST /api/project/tools/deny`：调整工具级权限。
- `POST /api/project/tool/description`、`POST /api/project/tool/description/reset`：设置或清除自定义工具描述。
- `GET /api/logs/tail`（可选）：按需提供日志订阅或最新记录。

### 4.2 中间件

依据 Tower 的 `Layer` / `Service` 抽象（参照 Context7 中 Tower 文档），构建以下中间件管线：

1. **ClientKindLayer**：根据请求头 `X-MCP-Client`、来源 IP、User-Agent 区分 `ClientKind::{Web,Tauri,Local}`，写入 `RequestExtensions`。
2. **CorsLayer**（参考 `tower_http` 示例）：仅对 `ClientKind::Web` 启用跨域；`ClientKind::Tauri`/`Local` 允许无 CORS。
3. **AuthLayer**（预留）：当前可放行，将来对 Web 请求验证 Token，对 Tauri 请求做本地口令校验。
4. **TracingLayer**：统一打印访问日志，结合现有 `tracing` 配置。

### 4.3 状态共享

- `AppState` 结构体持有 `Arc<ServerManager>`、`ProjectRegistry`、`Layout`、`tokio::watch::Sender`（如需推送）。
- 通过 Axum 的 `State` 提取器（Context7 Axum 文档中的 `with_state` 用法）在 handler 中访问核心对象。
- 对长耗时操作（如扫描工具列表）考虑引入 Tower 的 `Timeout` 中间件，防止阻塞。

### 4.4 错误映射

- Handler 返回 `Result<impl IntoResponse, CoreError>`。
- 定义 `impl IntoResponse for CoreError`：映射到标准 JSON 错误结构 `{code, message, details}`，保持与 CLI 统一。
- 对外错误统一写入 i18n key，前端由已有国际化模块渲染。

## 5. 客户端访问策略

### 5.1 WebUI

- Vite 环境通过环境变量 `VITE_API_BASE_URL` 指向 Axum 服务。
- `src/lib/api.ts` 重构为基于 `fetch` 的轻量客户端：统一封装基础请求、错误处理、`X-MCP-Client: web` 头部。
- 当前实现已支持 `VITE_API_BASE_URL` 与 `window.__MCP_CENTER_HTTP_BASE__` 双通道配置（缺省时直接报错提示未配置），统一通过 HTTP API 获取数据。
- 若启用鉴权，前端需在环境变量 `VITE_API_AUTH_TOKEN` 或 `window.__MCP_CENTER_HTTP_TOKEN__` 中注入 Token，并在请求头自动携带 `Authorization: Bearer <TOKEN>`。
- 使用 React Query 的 `QueryClient` 做缓存，同步处理 Loading/Error 状态。
- 开发阶段可在 `vite.config.ts` 配置代理，避免跨域；生产部署依赖 Axum 的 CORS。

### 5.2 Tauri

- `src-tauri/src/main.rs` 在启动时 `spawn` Axum HTTP 服务，监听 `127.0.0.1:0`，获取实际端口。
- 新增 `#[tauri::command] fn get_backend_base_url() -> String` 返回 `http://127.0.0.1:{port}/api`。
- 前端启动时先调用该命令，设置全局 `API_BASE_URL`，之后所有请求走 `fetch`，并携带 `X-MCP-Client: tauri`。
- 若后续需要无网络场景，Axum 仍运行在本地进程内，不受影响。
- 目前实现提供了环境变量覆盖（`MCP_CENTER_HTTP_BASE_URL`/`MCP_CENTER_HTTP_BASE`），未配置时将回退到默认 `http://127.0.0.1:8787/api`，同时在前端启动流程中自动写入 `window.__MCP_CENTER_HTTP_BASE__`。
- 若启用 HTTP 鉴权，Tauri 会通过新命令 `get_backend_auth_token` 自动注入 `window.__MCP_CENTER_HTTP_TOKEN__`，请求头将使用同一 Token。

### 5.3 兼容旧逻辑

- 保留现有 `invoke` 命令一段时间，逐步迁移调用方，便于回滚。
- `src/lib/api.ts` 在检测到 `API_BASE_URL` 未配置时可回退到 `invoke`，作为安全网。

## 6. 配置与运行

- `mcp-center serve` 增加参数：`--http-bind 127.0.0.1:8787`、`--http-public false`、`--http-tls-cert` 等。
- 默认行为：若不传参则仍只启动控制通道；Tauri 模式下由桌面端决定监听端口。
- 将绑定地址、CORS 白名单、认证 Token 等写入配置文件（`~/.mcp-center/config/http.toml`），方便部署。
- 新增 `--http-auth-token`（或环境变量 `MCP_CENTER_HTTP_TOKEN`）用于启用 Bearer Token 鉴权；前端/Tauri 需在请求头中附带 `Authorization: Bearer <TOKEN>` 或 `X-MCP-Token`。

## 7. 实施计划

1. **阶段一：基础 HTTP API**
   - [x] 引入 `axum`、`tower`, `tower-http` 依赖，并在守护进程中构建最小可用服务（`GET /api/health`）。
   - [x] 封装 `AppState`、共享 `ServerManager`。
   - [x] 实现服务器列表、项目列表两个只读接口。
2. **阶段二：前端切换**
   - [x] 调整 `src/lib/api.ts` 使用统一 `fetch` 客户端。
   - [x] WebUI/Tauri 分别配置 Base URL，替换现有 mock/invoke。
   - [ ] 保持旧命令 fallback，完成端到端联调。
3. **阶段三：权限操作与安全**
   - [ ] 增加 `PATCH/POST` 写操作，复用 CLI 业务逻辑。
   - [ ] 完成 CORS、ClientKind、中间件组合。
   - [ ] 设计鉴权策略（Token/本地 ACL），为未来公网部署预留空间。
4. **阶段四：增强与文档**
   - [ ] 整理 OpenAPI/文档，可使用 `utoipa`/手写 YAML。
   - [ ] 增加日志订阅、WebSocket 推送等增强功能。

## 8. 风险与对策

- **业务逻辑重复**：所有 handler 必须调用现有核心模块，避免复制代码；必要时在核心 crate 中补充复用函数。
- **性能影响**：HTTP 接口与守护进程共享 async runtime，需要评估高并发场景；可结合 Tower 中的 `BufferLayer`、`ConcurrencyLimitLayer` 做保护。
- **跨环境差异**：浏览器存在 CORS/认证需求；Tauri/本地工具需保留无网络情况下的本地访问能力，`ClientKind` 中间件可区别处理。
- **安全性**：对公网开放时需要身份认证、TLS、速率限制；设计中已预留对应层级。

## 9. 测试策略

- 单元测试：为每个 handler 编写 Axum `Router` 级别测试，使用 `tower::ServiceExt::oneshot` 驱动请求。
- 集成测试：在 `crates/mcp-center-test-client` 中新增 HTTP 端到端场景，覆盖服务器列表、权限更新。
- 前端联调：使用 Mock Service Worker 或实机守护进程模拟环境；确保 React Query 与 Zustand 状态更新正确。
- 性能测试：基于 `wrk`/`hey` 做基础压测，检查守护进程资源占用。

## 10. 后续扩展

- WebSocket/SSE：为权限变更、日志推送提供实时能力，可利用 Axum + `tokio::sync::broadcast`.
- OpenAPI 生成：自动化生成 API 文档，供外部团队集成。
- 插件化中间件：将 `ClientKindLayer`、鉴权层抽象为独立 Crate，供其他守护进程复用。

---

本文档中关于 Axum、Tower、tower-http 的用法均基于 Context7 最新文档检索结果，以确保第三方依赖设计的准确性。后续引入其他第三方库时需按同样流程获取资料。
