# components/servers 目录速览

## 目的

- 专门承载 MCP 服务器相关的详情/工具查看组件，供 `ServersPage` 复用。

## 组件

- `ServerDetailDrawer.tsx`
  - 使用 React Query 调用 `getMcpServerDetail`，展示基础信息、协议配置、工具列表。
  - 支持 stdio/SSE/HTTP 三种协议显示差异化字段。
  - 通过 `ToolDetailDialog` 打开单个工具的 JSON 详情。
- `ServerDetailDrawer.test.tsx` 覆盖抽屉加载工具列表、编辑/删除按钮和工具弹窗。
- `ToolDetailDialog.tsx`
  - 展示 `ToolInfo`（描述、input schema 等），支持复制命令或 JSON。
- `ToolDetailDialog.test.tsx` 验证打开/关闭行为与描述渲染。

## 注意

- Drawer open 状态由外部控制；组件内部在 `open && server` 条件下 refetch。
- HTTP API 字段已统一为 camelCase，组件无需额外的大小写兼容代码。
- 所有按钮/徽章均调用 `components/ui` 下的基础组件，确保主题一致。
