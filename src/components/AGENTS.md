# components 目录速览

## 角色

- 汇集页面可复用的布局、弹窗、徽章等 UI 组件。
- 将 shadcn/ui 原子组件进行二次封装（位于 `ui/`）。
- 负责与业务状态/API 输出直接交互的展示型组件，例如服务器详情抽屉、健康状态提示。

## 子目录/文件

- `Layout.tsx`：顶层壳组件（侧边栏导航 + `<Outlet />`）。依赖 `TooltipProvider`、`HealthStatusBadge`、`McpCenterLogo`。
- `AddServerDialog.tsx`：新增服务器对话框，组合 shadcn `Dialog`、`Form`、`Select` 等组件。
- `HealthStatusBadge.tsx`：调用 `getHealth` API 并根据状态显示动画徽章。
- `ErrorFallback.tsx`：全局错误边界的回退 UI，配合 `react-error-boundary` 使用，提供重试与刷新操作。
- `theme-provider.tsx`：包装 `next-themes` 风格的 ThemeProvider，控制 `class` attribute。
- `icons/`：目前仅包含 `McpCenterLogo.tsx`。
- `servers/`：
  - `ServerDetailDrawer.tsx`：查看服务器详情（React Query 查询 + Drawer UI）。
  - `ToolDetailDialog.tsx`：展示单个工具信息。
- `ui/`：从 shadcn/ui 拷贝的基础组件（alert-dialog、button、table 等），已适配 Tailwind v4。

## 注意

- 所有样式依赖 Tailwind v4 变量，避免在组件中直接引用旧式 `@apply`。
- 交互型组件通常依赖 React Query（`useQuery`, `useMutation`），记得传入正确的 `queryKey`。
- 添加新 Radix/shadcn 组件时请保持 `ui/` 内的命名与官方结构一致，方便升级。
