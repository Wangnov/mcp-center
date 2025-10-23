# MCP Center

MCP Center 是一个统一的 MCP（Model Context Protocol） 管理平台，同时提供：

- **命令行工具**：管理服务器、项目权限，启动守护进程或桥接到 IDE/Agent。
- **守护进程 & HTTP API**：聚合多个 MCP server，并基于项目进行权限控制。
- **Web/Tauri 界面**：React + Tailwind v4 前端，可打包为桌面应用。
- **测试客户端**：用于端到端调试 MCP 工具调用。

> 📌 说明：仓库中存在大量 `AGENTS.md`/`CLAUDE.md` 文件，为 AI 助手准备的结构化说明。  
> 面向人类的开发指南请阅读本文及各子目录的 `README.md`。

---

## 仓库结构

```text
crates/
├── mcp-center/             # Rust 后端：CLI、守护进程、HTTP、桥接
├── mcp-center-test-client/ # 轻量 MCP client，便于端到端测试
src/                        # React 19 + Vite 6 WebUI
src-tauri/                  # Tauri 2.0 桌面壳
docs/                       # 设计文档、调研记录
```

常见工作流：

- 后端：`cargo fmt`, `cargo fix-all`, `cargo lint-fix`, `cargo test-all`, `cargo run --bin mcp-center serve`
- 前端：`npm run export-types`, `npm run dev`, `npm run lint`, `npm run type-check`
- 桌面：`npm run tauri:dev`（需先启动后端 HTTP 服务）
- Specta：后端模型变化后执行 `npm run export-types`

---

## 快速开始

1. **克隆仓库并安装依赖**

   ```bash
   npm install
   ```

2. **启动后端守护进程**

   ```bash
   cargo run --bin mcp-center serve
   ```

3. **启动 Web 前端**

   ```bash
   npm run dev
   ```

4. **（可选）启动 Tauri 桌面端**

   ```bash
   npm run tauri:dev
   ```

5. **验证 Specta 类型是否同步**

   ```bash
   npm run export-types
   git status src/lib/api-types.generated.ts
   ```

---

## 贡献指南

1. 阅读根目录 `AGENTS.md` 获得整体约定；子目录的 `README.md` 提供人类开发者视角的说明。
2. 提交前确保格式化与测试全部通过：
   - `cargo fmt && cargo fix-all && cargo lint-fix && cargo test-all`
   - `npm run export-types && npm run lint && npm run type-check`
3. 若新增 API / 数据结构，务必同步更新前端 `api.ts` 与相关文档。
4. 保持多语言（CLI 与前端）的翻译完整：后端更新 `crates/mcp-center/src/cli/i18n.rs`，前端更新 `src/i18n` 资源。

---

## 进一步阅读

- `docs/`：历史设计、调研文档。
- `crates/mcp-center/README.md`：后端结构与命令。
- `src/README.md`：前端工程说明。
- `src-tauri/README.md`：桌面壳集成说明。
- `crates/mcp-center-test-client/README.md`：测试客户端使用指南。

如需与 AI 助手协作，可查阅相应目录下的 `AGENTS.md`/`CLAUDE.md` 获取更加结构化的提示信息。祝开发顺利！
