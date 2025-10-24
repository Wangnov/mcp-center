# pages 目录速览

## Servers.tsx

- 页面级管理界面：列表、过滤、快捷键（Ctrl+N 新增）、批量操作。
- React Query：`listMcpServers` + `toggleMcpEnabled` + `deleteMcpServer`。
- 结合 `ServerDetailDrawer`、`AddServerDialog`、`ToolDetailDialog`。
- 使用 `sonner` 提示、`lucide-react` 图标、Table/Switch 等 shadcn 组件。
- 测试：`Servers.test.tsx` 覆盖加载渲染、过滤/快捷键交互、启用切换与删除确认、工具描述操作、时间格式化分支与工具加载中的提示等完整流程。

## Projects.tsx

- 展示项目列表、允许配置允许的服务器/工具，调用 `listProjects`（以及相关 API）。
- 复用 `components/ui` 与 `zustand` store 更新 UI 状态。
- 单测：`Projects.test.ts`（工具字符串解析）、`Projects.interactions.test.tsx`（对话框提交流程、无服务器可选提示、权限/描述提交的错误分支）。

## Settings.tsx

- 系统设置占位页，通常用于主题/语言切换或调试。
- 测试：`Settings.test.tsx` 覆盖语言下拉切换与版本加载状态。

## servers-columns.tsx

- 提取 `@tanstack/react-table` 列定义（若引用 React Table），保持列配置与 UI 解耦。

## 注意事项

- 页面层负责组合 hooks/store 与业务组件；数据写入后务必 `invalidateQueries`。
- 当新增页面时，记得在 `App.tsx` 的 React Router 配置中注册路由。
