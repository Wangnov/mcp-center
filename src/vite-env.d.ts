/// <reference types="vite/client" />

import type * as TauriAPI from "@tauri-apps/api";

interface ImportMetaEnv {
  readonly VITE_APP_TITLE: string;
  readonly DEV: boolean;
  readonly PROD: boolean;
  readonly MODE: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

// Tauri API 类型扩展
interface Window {
  __TAURI__?: typeof TauriAPI;
}
