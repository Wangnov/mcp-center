# stores 目录速览

- `mcp-store.ts`：唯一的 Zustand store。
  - `servers`, `projects`, `ui`, `isLoading`, `error` 等状态。
  - `setServers`/`addServer`/`updateServer`/`removeServer`/`toggleServerEnabled` 等 actions。
  - `persist` middleware 默认使用 `localStorage`，key 由 Zustand 自动生成；注意数据结构变更需考虑迁移。
  - `devtools` middleware 已启用，可在浏览器扩展观察状态。
  - 自 2025-10-24 起新增 `selectEnabledServers` / `selectSelectedServer` / `selectProjects` 等纯函数选择器，用于组件、测试共享逻辑；`useEnabledServers` 等 Hook 基于这些 selector 封装，并通过 `shallow` 比较保持引用稳定。

使用建议：

- 读写时使用 `useMcpStore` 的 selector（例如 `useMcpStore((s) => s.servers)`）避免不必要的 re-render。
- `reset()` 可在用户登出或清除缓存时调用。
- 由于 store 中也存储 UI state（主题、语言等），若引入 next-themes/i18next，请保持同步更新逻辑一致。
