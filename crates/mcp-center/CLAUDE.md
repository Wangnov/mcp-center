# MCP Center 概览（AI 助手用）

## 仓库定位

- 单一仓库提供 CLI、守护进程、桥接层与 HTTP API 的 MCP 管控能力。
- 通过项目哈希 (Blake3) 进行权限隔离，并使用 Specta 将类型契约同步给前端 / 桌面端。

## 顶层结构

```text
crates/mcp-center/
├── src/
│   ├── bin/        # CLI 主入口、Specta 导出脚本
│   ├── bridge/     # connect 子命令与控制协议
│   ├── cli/        # i18n 支持
│   ├── config/     # ServerDefinition/Config 读写
│   ├── daemon/     # 守护进程、控制/RPC/Host
│   ├── project/    # ProjectId/Registry/权限模型
│   ├── web/        # Axum HTTP API
│   ├── error.rs    # CoreError 统一错误
│   └── paths.rs    # Layout 目录结构
├── tests/          # CLI & HTTP 集成测试
├── docs/           # Specta 指南等开发文档
└── src-tauri/…     # GUI 工程（另行维护）
```

各子目录都有 `AGENTS.md`，包含更细节的职责、交互与注意事项，请在深入修改前先查阅。

## 核心流程（概述）

1. **CLI (`src/bin/mcp-center.rs`)**：暴露 `init / serve / connect / mcp / project` 等命令，直接操作 `ServerConfig`、`ProjectRegistry` 或调用 daemon RPC。
2. **Bridge (`src/bridge`)**：`mcp-center connect` 探测项目路径、连接/启动 daemon，并通过控制 socket 建立 MCP 会话。
3. **Daemon (`src/daemon`)**：`serve` 启动控制 socket、RPC socket、可选 HTTP；`server_manager` 管理 MCP 服务；`host` 基于项目权限过滤工具。
4. **Web (`src/web/http.rs`)**：提供 Axum 路由及 REST API，所有请求/响应结构均派生 `specta::Type` 以导出契约。
5. **项目/配置 (`src/project`, `src/config`)**：维护 TOML 记录与验证逻辑，供 CLI/Daemon/Web 复用。
6. **测试客户端 (`crates/mcp-center-test-client`)**：独立 MCP client/CLI，用于端到端验证工具调用与调试（详见该目录 `AGENTS.md`）。

## Specta 契约同步

- 所有对外结构在对应模块 `#[derive(Type)]`。
- `src/bin/export-types.rs` 中的 `EXPORT_TARGETS` 统一列出导出模型；运行 `npm run export-types` 或 `cargo run --bin export-types` 会生成 `src/lib/api-types.generated.ts`。
- 更详细的生成流程见 `docs/type-generation-guide.md` 与 `src/web/AGENTS.md`。

## 关键依赖（摘自 Cargo.toml）

- `tokio`, `axum`, `tower-http`：异步运行时与 HTTP 服务。
- `rmcp` (git 版 SDK)：提供 MCP client/server primitives 与多种 transport。
- `specta`, `specta-typescript`：类型导出工具链。
- `serde`, `toml_edit`, `serde_json`：序列化与配置读写。
- `tracing`, `tracing-subscriber`：结构化日志。
- `clap`：命令行解析；配合 `locale_config` 提供多语言提示。
- `blake3`, `rand`, `time`, `url`：分别用于项目 ID 哈希、随机 ID、时间戳、URL 校验。
- `assert_cmd`, `tempfile`, `mcp-center-test-client` 等测试工具。

## 开发提示

- 共享结构 (`ProjectRecord`, `ServerDefinition`, Specta 模型等) 的变更需同步更新 CLI/Web/Daemon/测试；具体细节见各模块 `AGENTS.md`。
- 桥接控制 socket 已迁移到 `interprocess` 跨平台本地 socket；Unix 平台负责清理 socket 文件，Windows 使用命名管道自动清理。
- 文件 IO 及目录管理建议统一通过 `paths::Layout`，新增资源时请在 `paths.rs` 补充接口。
- 任何前端/接口变更记得重新运行 `npm run export-types` 并提交生成文件。
