# lib 目录速览

## 文件

- `api.ts`：前端与后端通信的主要入口（fetch + React Query）。
  - 自动从 `window.__MCP_CENTER_HTTP_*` 或 `VITE_` 环境变量读取 base/token。
  - 暴露 `listMcpServers`, `toggleMcpEnabled`, `getMcpServerDetail`, `listProjects`, `getHealth` 等函数。
  - `requestJson` 统一处理 401/403 → 抛出 `AUTH_REQUIRED`。
- `api-types.generated.ts`：由 Specta 生成的 TypeScript 类型，反映后端 HTTP 契约（禁止手改）。
- `api-client-example.ts`：示例代码，演示如何使用生成类型调用 API。
- `utils.ts`：`cn()` 工具，封装 `clsx` + `tailwind-merge`。

## 使用指引

- 新增 API 时先在后端添加 Specta 类型 → 运行 `npm run export-types` → 在此文件夹增添封装函数。
- 与 React Query 结合时，统一定义 `queryKey`，并在写操作后 `invalidateQueries`。
- 若前端运行于 Tauri，需要确保 `@tauri-apps/api` 的调用守住 `window.__TAURI_IPC__` 宏。
