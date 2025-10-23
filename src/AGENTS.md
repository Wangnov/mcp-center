# 前端概览（AI 助手用）

## 技术栈

- **React 19 + TypeScript + Vite 6**：`vite.config.ts` 启用 `@tailwindcss/vite` 插件与 Lightning CSS。
- **Tailwind CSS v4**：采用 `@import "tailwindcss";`、`@theme`、`@utility` 等新语法，主题变量在 `app.css` 定义，并额外引入 `tw-animate-css`。
- **shadcn/ui（Radix UI）**：`src/components/ui` 目录收录复制版组件，作为原子 UI 基础。
- **状态/数据层**：
  - `@tanstack/react-query`：请求缓存与同步。
  - `zustand`（带 `devtools`/`persist` 中间件）：全局状态存储于 `src/stores/mcp-store.ts`。
  - `i18next`：多语言（中/英/日）文本；`src/i18n.ts` 初始化。
- **Tauri 集成**：`main.tsx` 检测 `window.__TAURI_IPC__`，借助 `@tauri-apps/api/core` 拉取后台地址与 token。

## 目录导读

- `App.tsx`：ThemeProvider、React Router、Toaster、TooltipProvider 的根布局（默认重定向 `/mcp`）。
- `components/`：布局、对话框与业务组件（详见 `components/AGENTS.md`）。
- `pages/`：按路由划分的页面（Servers/Projects/Settings）。
- `lib/`：与后端交互的 API 封装、Specta 生成的类型 (`api-types.generated.ts`)、工具函数。
- `stores/`：zustand store 定义与 actions。
- `hooks/`：自定义 Hook，如键盘快捷键。
- `app.css`：Tailwind v4 主题变量、自定义工具类，注意不存在传统 `tailwind.config.js`。
- `i18n.ts`：加载多语言资源；新增文案时需更新对应 JSON，保持简/繁/英/日四种语言同步。

## 关键依赖

- `@tanstack/react-query`, `zustand`, `sonner`, `lucide-react`, `@radix-ui` 系列。
- `tailwind-merge`, `clsx`：组合 class 工具。
- `@tauri-apps/api`, `vite` 环境变量 `VITE_*` / `TAURI_*`。

## 开发提示

- Tailwind v4 无需手写 config；自定义主题均放在 CSS 中，如需扩展请继续使用 `@theme` / `@utility`。
- UI 组件采用 shadcn 模式（复制到本地）；新增组件保持目录结构与导出方式一致。
- React Query 使用 `queryKey` 进行缓存更新，页面内操作后务必 `invalidateQueries`。
- 与后端通信通过 `src/lib/api.ts`，优先复用现有函数并同步 Specta 类型。
