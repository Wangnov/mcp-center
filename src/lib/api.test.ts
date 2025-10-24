import {
  beforeEach,
  afterEach,
  afterAll,
  describe,
  expect,
  it,
  vi,
  type Mock,
} from "vitest";
import {
  addMcpServer,
  listMcpServers,
  listProjects,
  toggleMcpEnabled,
  allowProjectServers,
  denyProjectServers,
  allowProjectTools,
  denyProjectTools,
  setProjectToolDescription,
  resetProjectToolDescription,
  getMcpServerDetail,
  getMcpServerTools,
  deleteMcpServer,
  getHealth,
  getAppVersion,
  listServerLogs,
  getLogEntries,
  type AddServerPayload,
} from "./api";

const originalFetch = global.fetch;
const originalEventSource = (globalThis as any).EventSource;

describe("api client", () => {
  beforeEach(() => {
    vi.restoreAllMocks();
    global.fetch = vi.fn();
    Object.assign(window, {
      __MCP_CENTER_HTTP_BASE__: undefined,
      __MCP_CENTER_HTTP_TOKEN__: undefined,
    });
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  afterAll(() => {
    global.fetch = originalFetch;
    if (originalEventSource) {
      (globalThis as any).EventSource = originalEventSource;
    }
  });

  it("uses relative url when base is not configured", async () => {
    const responsePayload = { servers: [] };
    (global.fetch as unknown as Mock).mockResolvedValue(
      new Response(JSON.stringify(responsePayload), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    );

    await listMcpServers();

    expect(global.fetch).toHaveBeenCalledWith("/api/mcp", expect.any(Object));
  });

  it("throws AUTH_REQUIRED when server returns 401", async () => {
    (global.fetch as unknown as Mock).mockResolvedValue(
      new Response("Unauthorized", { status: 401 }),
    );

    await expect(listMcpServers()).rejects.toThrowError("AUTH_REQUIRED");
  });

  it("throws when listing projects without authorization", async () => {
    (global.fetch as unknown as Mock).mockResolvedValue(
      new Response("Unauthorized", { status: 401 }),
    );

    await expect(listProjects()).rejects.toThrowError("AUTH_REQUIRED");
  });

  it("sends POST payload to add server", async () => {
    (global.fetch as unknown as Mock).mockResolvedValue(
      new Response(JSON.stringify({ id: "id", name: "demo" }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    );

    const payload: AddServerPayload = {
      name: "Demo",
      protocol: "stdio",
      command: "node server.js",
    };

    await addMcpServer(payload);

    expect(global.fetch).toHaveBeenCalledWith(
      "/api/mcp",
      expect.objectContaining({
        method: "POST",
        headers: expect.objectContaining({ "Content-Type": "application/json" }),
      }),
    );
  });

  it("sends PATCH request for toggle", async () => {
    (global.fetch as unknown as vi.Mock).mockResolvedValue(
      new Response(JSON.stringify({ server: { id: "demo", enabled: true } }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    );

    await toggleMcpEnabled("demo", true);

    expect(global.fetch).toHaveBeenCalledWith(
      "/api/mcp/demo/enabled",
      expect.objectContaining({ method: "PATCH" }),
    );
  });

  it("uses Tauri base url and auth header when present", async () => {
    vi.resetModules();
    window.__TAURI_IPC__ = {};
    window.__MCP_CENTER_HTTP_BASE__ = "https://api.local";
    window.__MCP_CENTER_HTTP_TOKEN__ = "token-123";
    (global.fetch as unknown as Mock).mockResolvedValue(
      new Response(JSON.stringify({ servers: [] }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    );

    const api = await import("./api");
    await api.listMcpServers();

    expect(global.fetch).toHaveBeenCalledWith(
      "https://api.local/api/mcp",
      expect.objectContaining({
        headers: expect.objectContaining({ Authorization: "Bearer token-123" }),
      }),
    );

    window.__TAURI_IPC__ = undefined;
  });

  it("posts allow/deny project payloads", async () => {
    (global.fetch as unknown as Mock)
      .mockResolvedValueOnce(
        new Response(JSON.stringify({}), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({}), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({}), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({}), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      );

    await allowProjectServers("proj-1", ["srv-1"]);
    expect(global.fetch).toHaveBeenCalledWith(
      "/api/project/allow",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ target: "proj-1", servers: ["srv-1"] }),
      }),
    );

    await denyProjectServers("proj-1", ["srv-2"]);
    expect(global.fetch).toHaveBeenCalledWith(
      "/api/project/deny",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ target: "proj-1", servers: ["srv-2"] }),
      }),
    );

    await allowProjectTools("proj-1", ["srv::tool"]);
    expect(global.fetch).toHaveBeenCalledWith(
      "/api/project/tools/allow",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ target: "proj-1", tools: ["srv::tool"] }),
      }),
    );

    await denyProjectTools("proj-1", ["srv::tool"]);
    expect(global.fetch).toHaveBeenCalledWith(
      "/api/project/tools/deny",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ target: "proj-1", tools: ["srv::tool"] }),
      }),
    );
  });

  it("requests server log summaries with optional filtering", async () => {
    (global.fetch as unknown as Mock).mockResolvedValue(
      new Response(JSON.stringify({ servers: [] }), {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    );

    await listServerLogs({ serverId: "demo" });

    expect(global.fetch).toHaveBeenCalledWith(
      "/api/logs/servers?serverId=demo",
      expect.objectContaining({ method: "GET" }),
    );
  });

  it("returns empty server list when log summary body is empty", async () => {
    (global.fetch as unknown as Mock).mockResolvedValue(
      new Response("", {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    );

    const logs = await listServerLogs();
    expect(logs).toEqual({ servers: [] });
  });

  it("requests log entries with cursor and limit", async () => {
    (global.fetch as unknown as Mock).mockResolvedValue(
      new Response(
        JSON.stringify({
          serverId: "demo",
          file: "20250108.log",
          entries: [],
          nextCursor: null,
          hasMore: false,
        }),
        {
          status: 200,
          headers: { "Content-Type": "application/json" },
        },
      ),
    );

    await getLogEntries({
      serverId: "demo",
      file: "20250108.log",
      cursor: 42,
      limit: 100,
    });

    expect(global.fetch).toHaveBeenCalledWith(
      "/api/logs/entries?serverId=demo&file=20250108.log&cursor=42&limit=100",
      expect.objectContaining({ method: "GET" }),
    );
  });

  it("throws when log entries response is empty", async () => {
    (global.fetch as unknown as Mock).mockResolvedValue(
      new Response("", {
        status: 200,
        headers: { "Content-Type": "application/json" },
      }),
    );

    await expect(
      getLogEntries({
        serverId: "demo",
        file: null,
      }),
    ).rejects.toThrowError("AUTH_REQUIRED");
  });

  it("opens log stream with token and client hint", async () => {
    vi.resetModules();
    window.__TAURI_IPC__ = {};
    window.__MCP_CENTER_HTTP_BASE__ = "https://api.local";
    window.__MCP_CENTER_HTTP_TOKEN__ = "secret";

    const streamSpy = vi.fn();
    class MockEventSource {
      url: string;
      constructor(url: string) {
        this.url = url;
        streamSpy(url);
      }
      close() {
        /* noop */
      }
    }

    (globalThis as any).EventSource = MockEventSource as unknown as typeof EventSource;

    const api = await import("./api");
    const source = api.openLogStream("gitlab");

    expect(streamSpy).toHaveBeenCalled();
    const url = streamSpy.mock.calls[0][0] as string;
    expect(url).toContain("https://api.local/api/logs/tail/gitlab");
    expect(url).toContain("client=tauri");
    expect(url).toContain("token=secret");

    source.close();

    window.__TAURI_IPC__ = undefined;
    window.__MCP_CENTER_HTTP_BASE__ = undefined;
    window.__MCP_CENTER_HTTP_TOKEN__ = undefined;
  });

  it("sets and resets custom tool descriptions", async () => {
    (global.fetch as unknown as Mock)
      .mockResolvedValueOnce(
        new Response(JSON.stringify({}), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({}), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      );

    await setProjectToolDescription(
      "proj-1",
      "resolve",
      "Resolve description",
    );
    expect(global.fetch).toHaveBeenCalledWith(
      "/api/project/tool/description",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({
          target: "proj-1",
          tool: "resolve",
          description: "Resolve description",
        }),
      }),
    );

    await resetProjectToolDescription("proj-1", "resolve");
    expect(global.fetch).toHaveBeenCalledWith(
      "/api/project/tool/description/reset",
      expect.objectContaining({
        method: "POST",
        body: JSON.stringify({ target: "proj-1", tool: "resolve" }),
      }),
    );
  });

  it("fetches server detail, tools and deletes server", async () => {
    (global.fetch as unknown as Mock)
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ server: {}, tools: [] }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      )
      .mockResolvedValueOnce(
        new Response(JSON.stringify({ tools: [] }), {
          status: 200,
          headers: { "Content-Type": "application/json" },
        }),
      )
      .mockResolvedValueOnce(
        new Response(null, {
          status: 204,
          headers: { "Content-Type": "application/json" },
        }),
      );

    await getMcpServerDetail("srv-1");
    expect(global.fetch).toHaveBeenCalledWith(
      "/api/mcp/srv-1",
      expect.objectContaining({ method: "GET" }),
    );

    await getMcpServerTools("srv-2");
    expect(global.fetch).toHaveBeenCalledWith(
      "/api/mcp/srv-2/tools",
      expect.objectContaining({ method: "GET" }),
    );

    const deleteResult = await deleteMcpServer("srv-3");
    expect(global.fetch).toHaveBeenCalledWith(
      "/api/mcp/srv-3",
      expect.objectContaining({ method: "DELETE" }),
    );
    expect(deleteResult).toBeUndefined();
  });

  it("returns null health response on error", async () => {
    (global.fetch as unknown as Mock).mockRejectedValue(new Error("offline"));

    const result = await getHealth();
    expect(result).toBeNull();
  });

  it("skips network when there is nothing to allow or deny", async () => {
    const allowServers = await allowProjectServers("proj-1", []);
    const denyServers = await denyProjectServers("proj-1", []);
    const allowToolsResult = await allowProjectTools("proj-1", []);
    const denyToolsResult = await denyProjectTools("proj-1", []);

    expect(allowServers).toBeNull();
    expect(denyServers).toBeNull();
    expect(allowToolsResult).toBeNull();
    expect(denyToolsResult).toBeNull();
    expect(global.fetch).not.toHaveBeenCalled();
  });

  it("throws descriptive error for non-auth failures", async () => {
    (global.fetch as unknown as Mock).mockResolvedValue(
      new Response("Boom", { status: 500 }),
    );

    await expect(listMcpServers()).rejects.toThrow(
      "HTTP 500: Boom",
    );
  });

  it("resolves app version via tauri invoke when running in tauri", async () => {
    vi.resetModules();
    const invokeMock = vi.fn().mockResolvedValue("9.9.9");
    vi.doMock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

    window.__TAURI_IPC__ = {};
    const api = await import("./api");
    const version = await api.getAppVersion();

    expect(invokeMock).toHaveBeenCalledWith("get_app_version");
    expect(version).toBe("9.9.9");

    window.__TAURI_IPC__ = undefined;
    vi.doUnmock("@tauri-apps/api/core");
    vi.resetModules();
  });

  it("returns web package version when not in tauri", async () => {
    const version = await getAppVersion();
    expect(version).toBeDefined();
  });
});

afterAll(() => {
  global.fetch = originalFetch;
});
