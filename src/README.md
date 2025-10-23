# Web 前端（React + Vite）

基于 React 19、Vite 6、Tailwind CSS v4、shadcn/ui 构建的 MCP Center WebUI。可在浏览器中运行，也可由 Tauri 打包为桌面应用。

## 主要依赖

- React 19 + Suspense
- Tailwind CSS v4（`@import "tailwindcss";`，主题维护在 `app.css`）
- Zustand（全局状态）与 `@tanstack/react-query`（数据请求）
- shadcn/ui + Radix UI + lucide 图标
- i18next 多语言（简/繁/英/日）

## 常用脚本

```bash
npm run export-types   # 生成 Specta 类型
npm run dev            # Vite Dev Server
npm run lint           # ESLint
npm run lint:fix       # ESLint 自动修复
npm run format         # Prettier
npm run type-check     # TypeScript 检查
npm run build          # 生产构建（含 tsc -b）
```

## 目录结构

- `App.tsx`：应用入口与路由配置
- `components/`：布局、对话框、shadcn 组件
- `pages/`：按路由划分的页面（Servers/Projects/Settings）
- `stores/`：Zustand store
- `lib/api.ts`：与后端 HTTP API 通信（需 Specta 类型支持）
- `hooks/`：自定义 Hook（如键盘快捷键）
- `i18n.ts`：多语言初始化（新增文案时同步更新资源）

## 注意事项

- Tailwind v4 不再使用 `@apply`，自定义类写到 `app.css` 的 `@utility`。
- HTTP 返回 snake_case，前端使用 camelCase 类型，封装层需兼容字段命名差异。
- 打包为桌面应用时，Tauri 会调用 `get_backend_base_url` / `get_backend_auth_token` 命令注入 API 地址。

更多细节请查看本目录下的 `AGENTS.md` / `CLAUDE.md`。
