import React, { Suspense } from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./app.css";
import "./i18n";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";

const queryClient = new QueryClient();

async function bootstrap() {
  if (typeof window !== "undefined" && window.__TAURI_IPC__) {
    try {
      const base = await invoke<string | null>("get_backend_base_url");
      if (base) {
        window.__MCP_CENTER_HTTP_BASE__ = base;
        console.debug("[MCP Center] 使用 Tauri 后端地址:", base);
      }
      const token = await invoke<string | null>("get_backend_auth_token");
      if (token) {
        window.__MCP_CENTER_HTTP_TOKEN__ = token;
      }
    } catch (error) {
      console.warn("[MCP Center] 获取后端地址失败，继续使用默认策略", error);
    }
  }

  ReactDOM.createRoot(document.getElementById("root")!).render(
    <React.StrictMode>
      <QueryClientProvider client={queryClient}>
        <Suspense fallback="Loading...">
          <App />
        </Suspense>
      </QueryClientProvider>
    </React.StrictMode>,
  );
}

bootstrap().catch((error) => {
  console.error("[MCP Center] 启动前端失败", error);
});
