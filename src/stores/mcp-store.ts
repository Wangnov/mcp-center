/**
 * MCP Center 全局状态管理 - Zustand Store
 *
 * 使用 Zustand 管理应用全局状态，包括：
 * - MCP 服务器列表
 * - 项目配置
 * - UI 状态（主题、语言等）
 */

import { create } from "zustand";
import { devtools, persist } from "zustand/middleware";

/* ========================================
   类型定义
   ======================================== */

export interface McpServer {
  id: string;
  name: string;
  protocol: "stdio" | "sse" | "http";
  enabled: boolean;
  command?: string;
  args?: string[];
  url?: string;
  env?: Record<string, string>;
  toolCount: number;
  lastSeen?: number;
}

export interface Project {
  id: string;
  path: string;
  name: string;
  allowedServers: string[];
  createdAt: number;
  lastSeen: number;
}

export interface UiState {
  theme: "light" | "dark" | "system";
  language: "zh-CN" | "zh-TW" | "en" | "ja";
  sidebarCollapsed: boolean;
}

/* ========================================
   Store 定义
   ======================================== */

interface McpStoreState {
  // MCP 服务器
  servers: McpServer[];
  selectedServerId: string | null;

  // 项目
  projects: Project[];
  selectedProjectId: string | null;

  // UI 状态
  ui: UiState;

  // 加载状态
  isLoading: boolean;
  error: string | null;
}

interface McpStoreActions {
  // 服务器操作
  setServers: (servers: McpServer[]) => void;
  addServer: (server: McpServer) => void;
  updateServer: (id: string, updates: Partial<McpServer>) => void;
  removeServer: (id: string) => void;
  toggleServerEnabled: (id: string) => void;
  selectServer: (id: string | null) => void;

  // 项目操作
  setProjects: (projects: Project[]) => void;
  addProject: (project: Project) => void;
  updateProject: (id: string, updates: Partial<Project>) => void;
  removeProject: (id: string) => void;
  selectProject: (id: string | null) => void;

  // UI 操作
  setTheme: (theme: UiState["theme"]) => void;
  setLanguage: (language: UiState["language"]) => void;
  toggleSidebar: () => void;

  // 通用操作
  setLoading: (isLoading: boolean) => void;
  setError: (error: string | null) => void;
  reset: () => void;
}

type McpStore = McpStoreState & McpStoreActions;

/* ========================================
   初始状态
   ======================================== */

const initialState: McpStoreState = {
  servers: [],
  selectedServerId: null,
  projects: [],
  selectedProjectId: null,
  ui: {
    theme: "system",
    language: "zh-CN",
    sidebarCollapsed: false,
  },
  isLoading: false,
  error: null,
};

/* ========================================
   Store 创建
   ======================================== */

export const useMcpStore = create<McpStore>()(
  devtools(
    persist(
      (set) => ({
        ...initialState,

        // ===== 服务器操作 =====
        setServers: (servers) => set({ servers }),

        addServer: (server) =>
          set((state) => ({
            servers: [...state.servers, server],
          })),

        updateServer: (id, updates) =>
          set((state) => ({
            servers: state.servers.map((s) =>
              s.id === id ? { ...s, ...updates } : s,
            ),
          })),

        removeServer: (id) =>
          set((state) => ({
            servers: state.servers.filter((s) => s.id !== id),
            selectedServerId:
              state.selectedServerId === id ? null : state.selectedServerId,
          })),

        toggleServerEnabled: (id) =>
          set((state) => ({
            servers: state.servers.map((s) =>
              s.id === id ? { ...s, enabled: !s.enabled } : s,
            ),
          })),

        selectServer: (id) => set({ selectedServerId: id }),

        // ===== 项目操作 =====
        setProjects: (projects) => set({ projects }),

        addProject: (project) =>
          set((state) => ({
            projects: [...state.projects, project],
          })),

        updateProject: (id, updates) =>
          set((state) => ({
            projects: state.projects.map((p) =>
              p.id === id ? { ...p, ...updates } : p,
            ),
          })),

        removeProject: (id) =>
          set((state) => ({
            projects: state.projects.filter((p) => p.id !== id),
            selectedProjectId:
              state.selectedProjectId === id ? null : state.selectedProjectId,
          })),

        selectProject: (id) => set({ selectedProjectId: id }),

        // ===== UI 操作 =====
        setTheme: (theme) =>
          set((state) => ({
            ui: { ...state.ui, theme },
          })),

        setLanguage: (language) =>
          set((state) => ({
            ui: { ...state.ui, language },
          })),

        toggleSidebar: () =>
          set((state) => ({
            ui: { ...state.ui, sidebarCollapsed: !state.ui.sidebarCollapsed },
          })),

        // ===== 通用操作 =====
        setLoading: (isLoading) => set({ isLoading }),

        setError: (error) => set({ error }),

        reset: () => set(initialState),
      }),
      {
        name: "mcp-center-storage",
        // 只持久化必要的状态
        partialize: (state) => ({
          ui: state.ui,
          selectedServerId: state.selectedServerId,
          selectedProjectId: state.selectedProjectId,
        }),
      },
    ),
    {
      name: "MCP Center Store",
      enabled: import.meta.env.DEV, // 仅开发环境启用 devtools
    },
  ),
);

/* ========================================
   选择器 Hooks（性能优化）
   ======================================== */

// 仅订阅服务器列表
export const useServers = () => useMcpStore((state) => state.servers);

// 仅订阅已启用的服务器
export const useEnabledServers = () =>
  useMcpStore((state) => state.servers.filter((s) => s.enabled));

// 仅订阅选中的服务器
export const useSelectedServer = () =>
  useMcpStore((state) => {
    const id = state.selectedServerId;
    return id ? state.servers.find((s) => s.id === id) : null;
  });

// 仅订阅项目列表
export const useProjects = () => useMcpStore((state) => state.projects);

// 仅订阅选中的项目
export const useSelectedProject = () =>
  useMcpStore((state) => {
    const id = state.selectedProjectId;
    return id ? state.projects.find((p) => p.id === id) : null;
  });

// 仅订阅 UI 状态
export const useUiState = () => useMcpStore((state) => state.ui);

// 仅订阅主题
export const useTheme = () => useMcpStore((state) => state.ui.theme);

// 仅订阅语言
export const useLanguage = () => useMcpStore((state) => state.ui.language);
