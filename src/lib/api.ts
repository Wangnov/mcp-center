import { invoke } from "@tauri-apps/api/core";
import { version as webVersion } from "../../package.json";

declare global {
  interface Window {
    __TAURI_IPC__?: unknown;
    __MCP_CENTER_HTTP_BASE__?: string;
    __MCP_CENTER_HTTP_TOKEN__?: string;
  }
}

const isTauri = typeof window !== "undefined" && !!window.__TAURI_IPC__;
const apiBaseUrl =
  (typeof window !== "undefined" && window.__MCP_CENTER_HTTP_BASE__) ||
  import.meta.env.VITE_API_BASE_URL;
const apiAuthToken =
  (typeof window !== "undefined" && window.__MCP_CENTER_HTTP_TOKEN__) ||
  import.meta.env.VITE_API_AUTH_TOKEN;

const CLIENT_HEADER = "X-MCP-Client";

function resolveUrl(path: string): string | null {
  // 如果没有配置 apiBaseUrl 或为空字符串，使用相对路径（Vite 代理会处理）
  if (!apiBaseUrl || apiBaseUrl.trim() === "") {
    return path.startsWith("/") ? path : `/${path}`;
  }

  // 有配置 apiBaseUrl，拼接完整 URL
  const base = apiBaseUrl.replace(/\/$/, "");
  const suffix = path.startsWith("/") ? path : `/${path}`;
  return `${base}${suffix}`;
}

async function requestJson<T>(
  path: string,
  init: RequestInit = {},
  expectBody = true,
): Promise<T | null> {
  const url = resolveUrl(path);

  // resolveUrl 现在总是返回有效路径，不会返回 null
  if (!url) {
    throw new Error("API_BASE_URL_UNSET");
  }

  const headers: Record<string, string> = {
    Accept: "application/json",
    [CLIENT_HEADER]: isTauri ? "tauri" : "web",
    ...(apiAuthToken ? { Authorization: `Bearer ${apiAuthToken}` } : {}),
    ...(init.headers as Record<string, string> | undefined),
  };

  const response = await fetch(url, {
    ...init,
    headers,
  });

  if (!response.ok) {
    const body = await response.text();
    if (response.status === 401 || response.status === 403) {
      throw new Error("AUTH_REQUIRED");
    }
    throw new Error(
      `HTTP ${response.status}: ${body || "Unknown error calling " + path}`,
    );
  }

  if (!expectBody) {
    return null;
  }

  const text = await response.text();
  if (!text) {
    return null;
  }
  return JSON.parse(text) as T;
}

async function getJson<T>(path: string): Promise<T | null> {
  return requestJson<T>(path, { method: "GET" });
}

async function patchJson<T>(path: string, body: unknown): Promise<T | null> {
  return requestJson<T>(path, {
    method: "PATCH",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(body),
  });
}

async function postJson<T>(path: string, body: unknown): Promise<T | null> {
  return requestJson<T>(path, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(body),
  });
}

export const getAppVersion = async (): Promise<string> => {
  if (isTauri) {
    return await invoke("get_app_version");
  } else {
    return Promise.resolve(webVersion);
  }
};

// ===== 健康检查接口 =====
export interface HealthResponse {
  status: string;
}

export const getHealth = async (): Promise<HealthResponse | null> => {
  try {
    return await getJson<HealthResponse>("/api/health");
  } catch (error) {
    console.error("Health check failed:", error);
    return null;
  }
};

export interface McpServer {
  id: string;
  name: string;
  protocol: "stdio" | "sse" | "http";
  enabled: boolean;
  toolCount: number; // 列表接口返回驼峰命名
  // 扩展字段（来自详情接口）
  tool_count?: number; // 详情接口使用蛇形命名
  command?: string;
  args?: string[];
  url?: string;
  env?: Record<string, string>;
  headers?: Record<string, string>;
  // 时间戳字段 - 支持两种命名
  createdAt?: number; // 列表接口使用驼峰命名
  lastSeen?: number; // 列表接口使用驼峰命名
  created_at?: number; // 详情接口使用蛇形命名
  last_seen?: number; // 详情接口使用蛇形命名
}

interface ServerListResponse {
  servers: McpServer[];
}

export interface ProjectSummary {
  id: string;
  path: string;
  display_name: string | null;
  agent: string | null;
  allowed_server_ids: string[];
  created_at: number;
  last_seen_at: number;
}

interface ProjectListResponse {
  projects: ProjectSummary[];
}

export type AddServerPayload = {
  name: string;
  protocol: "stdio" | "sse" | "http";
  command?: string;
  endpoint?: string;
  // args can be a single string that will be split
  args?: string;
};

export const addMcpServer = async (
  payload: AddServerPayload,
): Promise<McpServer | null> => {
  return postJson<McpServer>("/api/mcp", payload);
};

export const listMcpServers = async (): Promise<McpServer[]> => {
  const httpResult = await getJson<ServerListResponse>("/api/mcp");
  if (httpResult) {
    return httpResult.servers;
  }

  throw new Error("AUTH_REQUIRED");
};

export const toggleMcpEnabled = async (
  id: string,
  enabled: boolean,
): Promise<ToggleServerResponse | null> => {
  return patchJson<ToggleServerResponse>(
    `/api/mcp/${encodeURIComponent(id)}/enabled`,
    {
      enabled,
    },
  );
};

export const listProjects = async (): Promise<ProjectSummary[]> => {
  const httpResult = await getJson<ProjectListResponse>("/api/project");
  if (httpResult) {
    return httpResult.projects;
  }

  throw new Error("AUTH_REQUIRED");
};

export const allowProjectServers = async (
  target: string,
  servers: string[],
): Promise<ProjectSummary | null> => {
  if (servers.length === 0) {
    return null;
  }
  return postJson<ProjectSummary>("/api/project/allow", { target, servers });
};

export const denyProjectServers = async (
  target: string,
  servers: string[],
): Promise<ProjectSummary | null> => {
  if (servers.length === 0) {
    return null;
  }
  return postJson<ProjectSummary>("/api/project/deny", { target, servers });
};

export const allowProjectTools = async (
  target: string,
  tools: string[],
): Promise<ProjectSummary | null> => {
  if (tools.length === 0) {
    return null;
  }
  return postJson<ProjectSummary>("/api/project/tools/allow", {
    target,
    tools,
  });
};

export const denyProjectTools = async (
  target: string,
  tools: string[],
): Promise<ProjectSummary | null> => {
  if (tools.length === 0) {
    return null;
  }
  return postJson<ProjectSummary>("/api/project/tools/deny", { target, tools });
};

export const setProjectToolDescription = async (
  target: string,
  tool: string,
  description: string,
): Promise<ProjectSummary | null> => {
  return postJson<ProjectSummary>("/api/project/tool/description", {
    target,
    tool,
    description,
  });
};

export const resetProjectToolDescription = async (
  target: string,
  tool: string,
): Promise<ProjectSummary | null> => {
  return postJson<ProjectSummary>("/api/project/tool/description/reset", {
    target,
    tool,
  });
};

// ===== MCP Server 扩展接口 =====

export interface ToolInfo {
  name: string;
  description?: string;
  serverName: string; // 后端使用驼峰命名
  serverId: string; // 后端使用驼峰命名
}

interface ServerDetailResponse {
  server: McpServer;
  tools: ToolInfo[];
}

export interface ToggleServerResponse {
  server: McpServer;
  warning?: string;
}

// 获取服务器详情（包括工具列表）
export const getMcpServerDetail = async (
  serverId: string,
): Promise<ServerDetailResponse | null> => {
  return getJson<ServerDetailResponse>(
    `/api/mcp/${encodeURIComponent(serverId)}`,
  );
};

// 获取服务器的工具列表
export const getMcpServerTools = async (
  serverId: string,
): Promise<ToolInfo[]> => {
  const result = await getJson<{ tools: ToolInfo[] }>(
    `/api/mcp/${encodeURIComponent(serverId)}/tools`,
  );
  return result?.tools || [];
};

// 删除服务器
export const deleteMcpServer = async (serverId: string): Promise<void> => {
  await requestJson(
    `/api/mcp/${encodeURIComponent(serverId)}`,
    {
      method: "DELETE",
    },
    false,
  );
};
