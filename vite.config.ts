import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import path from "path";

// https://vite.dev/config/
export default defineConfig({
  plugins: [
    react({
      // React 19 + React Compiler (未来启用)
      babel: {
        plugins: [
          // ['babel-plugin-react-compiler', {}]  // React Compiler (可选)
        ],
      },
    }),
    tailwindcss(),
  ],

  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },

  // Tauri 特定配置
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: true,
    watch: {
      // 监听 src-tauri 变化
      ignored: ["**/src-tauri/**"],
    },
    // API 代理配置（开发模式）
    proxy: {
      "/api": {
        target: "http://localhost:8787",
        changeOrigin: true,
        secure: false,
        // 不需要 rewrite，直接转发整个路径
      },
    },
  },

  envPrefix: ["VITE_", "TAURI_"],

  build: {
    // Tauri 使用 Chromium 内核，可以使用现代 ES 特性
    target: ["es2021", "chrome110", "safari16"],
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_DEBUG,

    // 代码分割优化
    rollupOptions: {
      output: {
        manualChunks: {
          "react-vendor": ["react", "react-dom"],
          "radix-ui": [
            "@radix-ui/react-dialog",
            "@radix-ui/react-dropdown-menu",
            "@radix-ui/react-label",
            "@radix-ui/react-select",
            "@radix-ui/react-slot",
            "@radix-ui/react-switch",
            "@radix-ui/react-tabs",
            "@radix-ui/react-toast",
          ],
        },
      },
    },
  },

  // CSS 优化 (Tailwind v4 使用 Lightning CSS)
  css: {
    transformer: "lightningcss",
  },
});
