# mcp-center（后端）

该 crate 是 MCP Center 的核心后端，实现以下功能：

- `mcp-center` CLI（`src/bin/mcp-center.rs`）：
  - `init / serve / connect / mcp / project` 等子命令
  - 多语言提示（见 `src/cli/i18n.rs`）
- 守护进程与服务（`src/daemon`）：
  - 控制 socket、RPC socket、ServerManager、HostService
  - 负责 MCP server 生命周期与权限过滤
- HTTP API（`src/web/http.rs`）：
  - Axum 路由，所有模型派生 `specta::Type` 以导出前端类型
- 项目/配置（`src/project`, `src/config`）：
  - 项目 ID（Blake3）、权限记录与服务器配置的持久化
- Specta 类型导出工具（`src/bin/export-types.rs`）：
  - 生成 `src/lib/api-types.generated.ts`

## 常用命令

```bash
# 格式化 + 代码修复 + lint + 测试
cargo fmt
cargo fix-all
cargo lint-fix
cargo test-all

# 启动守护进程
cargo run --bin mcp-center serve

# 启动桥接（IDE/Agent 连接）
cargo run --bin mcp-center connect

# 导出前端 Specta 类型
npm run export-types   # 在仓库根执行
```

## 多语言

- CLI 文案位于 `src/cli/i18n.rs`，新增命令时请同步更新四种语言版本。
- HTTP 接口返回错误信息尽量用 `CoreError`，由 CLI/前端统一翻译。

## 进一步阅读

- `AGENTS.md` / `CLAUDE.md`：提供给 AI 助手的模块级说明。
- `tests/`：CLI + HTTP 集成测试示例。
- `docs/`：Specta 使用手册、架构设计。
