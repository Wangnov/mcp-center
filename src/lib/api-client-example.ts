// Example: Using generated type definitions
// 此文件展示如何使用 api-types.generated.ts 中的类型

import type {
  ServerSnapshot,
  McpListResponse,
  ProjectSummary,
  CreateMcpRequest,
  ServerProtocol,
} from "./api-types.generated";

//  示例 1：使用响应类型
async function fetchServers(): Promise<ServerSnapshot[]> {
  const response = await fetch("http://127.0.0.1:8787/api/mcp");
  const data: McpListResponse = await response.json();

  //  完整的类型安全！
  return data.servers.map((server) => ({
    ...server,
    // toolCount 是 number 类型（自动从 Rust 的 usize 转换）
    displayToolCount: `${server.toolCount} 个工具`,
  }));
}

//  示例 2：使用请求类型
export async function createServer(
  name: string,
  protocol: ServerProtocol,
  command?: string,
): Promise<ServerSnapshot> {
  const request: CreateMcpRequest = {
    name,
    protocol,
    command: command ?? null,
    args: null,
    endpoint: null,
    env: null,
    headers: null,
  };

  const response = await fetch("http://127.0.0.1:8787/api/mcp", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(request),
  });

  return response.json();
}

//  示例 3：类型守卫
export function isStdioServer(server: ServerSnapshot): boolean {
  //  protocol 是联合类型，TypeScript 会检查拼写错误
  return server.protocol === "stdio";
}

//  示例 4：处理可空类型
export function formatProject(project: ProjectSummary): string {
  //  displayName 是 string | null，TypeScript 强制检查
  const name = project.displayName ?? project.path;
  const agent = project.agent ?? "未知";

  //  createdAt 是 number（从 u64 自动转换）
  const date = new Date(project.createdAt * 1000);

  return `${name} (${agent}) - ${date.toLocaleDateString()}`;
}

//  示例 5：React Hook 使用
import { useQuery } from "@tanstack/react-query";

export function useServers() {
  return useQuery<ServerSnapshot[], Error>({
    queryKey: ["servers"],
    queryFn: fetchServers,
    //  完整的类型推导
  });
}

//  示例 6：Zustand Store 使用
import { create } from "zustand";

interface ServerStore {
  servers: ServerSnapshot[];
  setServers: (servers: ServerSnapshot[]) => void;
  toggleServer: (id: string) => void;
}

export const useServerStore = create<ServerStore>((set) => ({
  servers: [],
  setServers: (servers) => set({ servers }),
  toggleServer: (id) =>
    set((state) => ({
      servers: state.servers.map((s) =>
        s.id === id ? { ...s, enabled: !s.enabled } : s,
      ),
    })),
}));
