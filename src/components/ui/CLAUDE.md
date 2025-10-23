# components/ui 目录速览

## 来源

- 基于 **shadcn/ui** 官方组件（React + Radix UI + Tailwind v4）手动复制并按需调整。
- 所有组件都是“可组合”版本，无默认样式逻辑，因此 Tailwind 类名来源于 `app.css` 中的主题变量。

## 主要组件

- 反馈与弹层：`alert-dialog`, `dialog`, `drawer`, `dropdown-menu`, `tooltip`, `sonner`（Toast）。
- 表单类：`form`, `input`, `select`, `checkbox`, `switch`, `label`.
- 基础 UI：`button`, `badge`, `table`, `avatar`.

## 使用约定

- 组件均导出 React 组件 + 命名导出（与 shadcn 原版一致），直接从 `@/components/ui/<component>` 引入。
- Tailwind v4 无 `@apply`，组件内部样式使用 className 与 CSS 变量；扩展时请继续沿用。
- 若升级 shadcn/ui，请参考官方 `npx shadcn@latest add <component>` 输出，并手动同步差异。
