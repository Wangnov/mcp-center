# MCP Center 项目指南（AI 助手总览）

> 使命：构建一个“有图形界面、可脚本化、跨平台”的 MCP 管控中心。  
> 哲学：遵循 Linus 原则——写代码、跑测试、迭代小步快跑（Talk is cheap, show me the code）。

---

## 0. 如何使用本指南

- 本文件聚焦战略层面：技术栈、工具链、协作风格、跨端契约。
- **根目录和每个子目录**（后端、前端、Tauri、测试等）都存在独立的 `AGENTS.md` 项目记忆文档，执行具体任务前请先阅读对应文件，遵循既定惯例。
- 每次作出可能影响项目记忆的改动（新增文件、组件、流程调整等）后，必须更新相关的一个或多个 `AGENTS.md` 项目记忆文档；若无需更新，请在总结中说明原因。

---

## 1. 代码版图

```text
crates/
├── mcp-center/             # Rust 后端：CLI + 守护进程 + HTTP API + 桥接层
│   ├── src/bin/            # CLI 主入口、Specta 导出工具
│   ├── src/bridge/         # connect 子命令（控制通道）
│   ├── src/config|project  # 服务器配置 & 项目权限持久化
│   ├── src/daemon/         # ServerManager / HostService / RPC / control socket
│   ├── src/web/http.rs     # Axum HTTP API（Specta 类型）
│   ├── tests/              # CLI & HTTP 集成测试
│   └── docs/               # Specta 使用指南等
├── mcp-center-test-client/ # 轻量 MCP client（库 + CLI），端到端测试工具

src/                        # React 19 + Vite 6 + Tailwind v4 + shadcn/ui WebUI
src-tauri/                  # Tauri 2.0 桌面壳（命令待接入核心逻辑）
```

---

## 2. 技术栈 & 工具链

### Rust 后端

- Toolchain：`rust-toolchain.toml` ⇒ stable + `rustfmt` + `clippy`
- 核心依赖：
  - Runtime/HTTP：`tokio`, `axum`, `tower-http`
  - MCP SDK：`rmcp` (git 版，含 stdio/SSE/HTTP transport)
  - 契约：`specta`, `specta-typescript`
  - 序列化：`serde`, `serde_json`, `toml_edit`
  - 权限/配置：`blake3`, `rand`, `time`, `url`
  - CLI & I18N：`clap`, `locale_config`
  - 日志：`tracing`, `tracing-subscriber`
- 集成测试：`assert_cmd`, `tempfile`, `mcp-center-test-client`

### TypeScript / React 前端

- Vite 6 + React 19 (Suspense) + TypeScript 5.x
- Tailwind CSS **v4**：`@import "tailwindcss";`，主题采用 `@theme` / `@utility`
- UI：shadcn/ui（Radix UI）复制版、`lucide-react`, `sonner`
- 状态：`@tanstack/react-query`, `zustand`（带 `devtools` & `persist`）
- 多语言：`i18next`
- 工具：`eslint`, `tailwind-merge`, `clsx`

### Tauri 桌面

- `tauri 2.0`, `tauri-plugin-shell`
- 当前命令多数为占位（返回错误提醒前端走 HTTP）；后续需要接入守护进程启动与系统能力
- CLI 入口 `npm run tauri:dev`

### 旁系工具

- `mcp-center-test-client`：可以 `cargo run -p mcp-center-test-client -- ...` 验证任意 MCP server
- Specta 导出：`npm run export-types`（在 `crates/mcp-center/src/bin/export-types.rs` 中列出类型）

---

## 3. 开发哲学 & 风格

### “Linus 风格”最小准则

1. **代码优先**：出现需求或 bug，先写实现/测试，文档同步更新。
2. **小步快跑**：每次提交或 PR 聚焦一个责任单元，易读、易回滚。
3. **保持可运行**：主分支时刻可编译/可测试；不要留下半残实现。

### Rust 编码规范

- 统一 `cargo fmt`；`cargo lint-fix` 保证 `clippy -D warnings`。
- 错误通过 `CoreError`、`ApiError` 表达；在 CLI/Tauri/Web 层翻译为用户提示。
- 共享逻辑放在 `Layout`/`ServerManager`/`ProjectRegistry` 等模块，避免重复实现。
- 任何访问文件系统的逻辑优先走 `paths::Layout`，统一目录结构。

### TypeScript / React 规范

- 函数组件 + Hooks；跨页状态放 `zustand`，请求缓存用 React Query。
- Tailwind v4：
  - 不使用 `@apply`；自定义类写到 `app.css` 的 `@utility` 中。
  - 使用 CSS 变量 (`var(--color-xxx)`) 替代 `theme()`.
  - 主题、动画统一放在 `@theme`。
- shadcn/ui：保持接口与官方同步；若升级请按官方 CLI diff。
- ESLint (`npm run lint`) 必须零告警。

### 前后端契约

- HTTP JSON 响应统一使用 **camelCase**，Specta 导出的 TS 类型与后端保持一致。
- `api.ts` 封装层直接透传 camelCase 响应，不再做字段重命名。
- 导出的 Specta 文件 (`api-types.generated.ts`) 禁止手改，新增接口后必须重新导出。

---

## 4. 常用工作流

### 后端

```bash
cargo fmt
cargo fix-all        # cargo alias：fix --workspace --all-targets
cargo lint-fix        # clippy
cargo test-all
cargo run --bin mcp-center serve   # 守护进程
cargo run --bin mcp-center connect # 桥接命令
```

### 前端 / Tauri

```bash
npm install
npm run export-types
npm run dev           # 浏览器开发模式
npm run lint          # ESLint
npm run type-check    # TypeScript 检查
npm run lint:fix      # ESLint 自动修复（必要时）
npm run format        # Prettier（谨慎使用，与 eslint 配合）
npm run tauri:dev     # 桌面模式（依赖后端 HTTP 服务）
```

### Specta 类型同步

1. 后端结构 `#[derive(Type)]`
2. 更新 `EXPORT_TARGETS`（如新增类型）
3. `npm run export-types`
4. 提交生成的 `api-types.generated.ts`

---

## 5. 协作注意事项

- **ProjectRegistry**：桥接、守护进程、HTTP API 三端共享，权限逻辑变更需同步更新。
- **ServerManager**：负责工具缓存；修改 MCP server 行为后记得 `force_refresh_tool_cache`.
- **HTTP API**：将 `Config`/`Project`/`Server` 类型暴露给前端；Specta 代码必须覆盖所有公开结构。
- **测试客户端**：`TestClient` & CLI 提供端到端验证工具，比手写请求更方便。
- **Tauri**：目前命令为空壳，计划用来启动/管理守护进程；别忘了与 Web API 的交互方式要一致。
- **前端**：
  - React Query key 结构需统一（`["servers"]`, `["server-detail", id]` 等）。
  - Zustand store 使用 selector 减少重新渲染。
  - Tailwind 主题变更要考虑亮/暗模式变量。
- **多语言**：
  - CLI 层新增字符串 ⇒ 更新 `crates/mcp-center/src/cli/i18n.rs` 四种语言分支。
  - Web UI 新增文案 ⇒ 更新 `src/i18n.ts` 所加载的 JSON 资源，保持与后端提示一致。

## 代码质量工具清单

- Rust：`cargo fmt`, `cargo fix-all`, `cargo lint-fix`, `cargo check-all`, `cargo test-all`（均在 `.cargo/config.toml` 定义 alias）。
- 前端：`npm run lint`, `npm run lint:fix`, `npm run format`, `npm run type-check`, `npm run build`（其中 build 会先 `export-types` 再 `tsc -b`）。
- Specta：`npm run export-types` —— 任何改变后端模型后必须运行。

---

## 6. 测试策略

- `crates/mcp-center/tests`：真实 CLI + HTTP 交互（使用临时目录、Spawn 子进程）。
- `mcp-center-test-client`：可作为库或 CLI 用于端到端测试工具调用。
- 前端目前缺少自动测试，可根据需要添加 React Testing Library / Playwright。
- Tauri 命令接入守护进程后，推荐使用 `tauri::test` 或 CLI 脚本验证。

---

## 7. 契约与命名指南

- **HTTP 命名**：后端与前端一致采用 camelCase；`api.ts` 不再做大小写转换。
- **文件结构**：Rust 模块按需求拆分；前端保持 “页面（pages）→ 复用组件（components）→ 基础组件（components/ui）” 层次。
- **Tailwind**：`app.css` 作为单一来源，禁止在组件中引入独立 Tailwind 配置文件。
- **多语言**：i18n key 使用 `snake_case`，翻译字符串在 `src/i18n.ts` 所指向的资源中维护。

---

## 8. 扩展路线图（供未来参考）

1. **Tauri 深度集成**：由桌面端启动/管理 `mcp-center serve`，并在 UI 中显示状态/日志。
2. **权限配置 UI**：完善项目信息、工具授权的交互和可视化。
3. **日志/监控**：将守护进程日志流暴露给前端查看。
4. **跨平台桥接**：控制 socket 已迁移至 `interprocess`，统一支持 Unix/Windows；后续需持续验证多平台兼容性。
5. **API 命名迁移**：长期目标将 HTTP 响应迁移到 camelCase（涉及后端、Specta、前端同步更新）。

---

## 9. 开发者 Checklist

1. 阅读根目录与相关子目录的 `AGENTS.md`。
2. 跑通开发环境（后端 `cargo run --bin mcp-center serve` + 前端 `npm run dev`）。
3. 熟悉 Specta 导出流程及前后端命名约束。
4. 提交前执行：
   - `cargo fmt && cargo lint-fix && cargo test-all`
   - `npm run export-types && npm run lint && npm run type-check`
5. 文档同步更新：如流程或契约变更，请更新本指南及对应子模块说明。
