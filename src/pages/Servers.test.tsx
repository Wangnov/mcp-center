import type { ReactNode } from "react";
import { describe, expect, it, beforeEach, vi } from "vitest";
import userEvent from "@testing-library/user-event";
import { screen, waitFor, within } from "@testing-library/react";
import { renderWithQueryClient } from "@/test/test-utils";
import { ServersPage } from "./Servers";
import {
  listMcpServers,
  toggleMcpEnabled,
  deleteMcpServer,
  getMcpServerTools,
  type McpServer,
} from "@/lib/api";
import { toast } from "sonner";

const { mockServerDetailDrawer, mockToolDetailDialog } = vi.hoisted(() => ({
  mockServerDetailDrawer: vi.fn(() => null),
  mockToolDetailDialog: vi.fn(() => null),
}));

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>(
    "@/lib/api",
  );
  return {
    ...actual,
    listMcpServers: vi.fn(),
    toggleMcpEnabled: vi.fn(),
    deleteMcpServer: vi.fn(),
    getMcpServerTools: vi.fn(),
  };
});

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
    warning: vi.fn(),
  },
}));

vi.mock("@/components/servers/ServerDetailDrawer", () => ({
  ServerDetailDrawer: mockServerDetailDrawer,
}));

vi.mock("@/components/servers/ToolDetailDialog", () => ({
  ToolDetailDialog: mockToolDetailDialog,
}));

vi.mock("@/components/ui/tooltip", () => {
  const Wrapper = ({ children }: { children: ReactNode }) => <>{children}</>;
  return {
    Tooltip: Wrapper,
    TooltipTrigger: Wrapper,
    TooltipContent: Wrapper,
  };
});

vi.mock("@/components/ui/dropdown-menu", () => {
  const Wrapper = ({ children }: { children: ReactNode }) => <>{children}</>;
  return {
    DropdownMenu: Wrapper,
    DropdownMenuTrigger: Wrapper,
    DropdownMenuContent: Wrapper,
    DropdownMenuItem: ({ children, onClick }: any) => (
      <button type="button" onClick={onClick}>
        {children}
      </button>
    ),
    DropdownMenuLabel: Wrapper,
    DropdownMenuSeparator: Wrapper,
  };
});

vi.mock("@/components/ui/alert-dialog", () => {
  const Wrapper = ({ children }: { children: ReactNode }) => <>{children}</>;
  return {
    AlertDialog: Wrapper,
    AlertDialogContent: Wrapper,
    AlertDialogHeader: Wrapper,
    AlertDialogTitle: Wrapper,
    AlertDialogDescription: Wrapper,
    AlertDialogFooter: Wrapper,
    AlertDialogCancel: ({ children, ...rest }: any) => (
      <button type="button" {...rest}>
        {children}
      </button>
    ),
    AlertDialogAction: ({ children, onClick, disabled }: any) => (
      <button type="button" onClick={onClick} disabled={disabled}>
        {children}
      </button>
    ),
  };
});

describe("ServersPage", () => {
  const servers: McpServer[] = [
    {
      id: "srv-1",
      name: "Alpha Server",
      protocol: "stdio",
      enabled: true,
      toolCount: 2,
      createdAt: 1730000000,
      lastSeen: 1730001000,
      command: "/bin/alpha",
      args: ["--flag", "--verbose"],
      env: {
        API_KEY: "ABCDEFGHIJKLMNOPQRSTUVWX",
      },
    },
    {
      id: "srv-3",
      name: "Gamma Server",
      protocol: "sse",
      enabled: true,
      toolCount: 1,
      createdAt: 1730003000,
      lastSeen: 1730003100,
    },
    {
      id: "srv-2",
      name: "Beta Server",
      protocol: "http",
      enabled: false,
      toolCount: 0,
      createdAt: 1730002000,
      lastSeen: null,
      url: "https://beta.example.com",
      env: {
        PORT: "3000",
      },
      headers: {
        Authorization: "Bearer token",
      },
    },
  ];

  beforeEach(() => {
    vi.clearAllMocks();
    mockServerDetailDrawer.mockClear();
    mockToolDetailDialog.mockClear();
    vi.mocked(listMcpServers).mockResolvedValue(servers);
    vi.mocked(toggleMcpEnabled).mockResolvedValue({
      server: { ...servers[0], enabled: false },
    } as never);
    vi.mocked(deleteMcpServer).mockResolvedValue(undefined);
    vi.mocked(getMcpServerTools).mockResolvedValue([]);
  });

  it("renders server list and toggles enabled state", async () => {
    const user = userEvent.setup();

    renderWithQueryClient(<ServersPage />);

    await screen.findByText("Alpha Server");

    const switches = screen.getAllByRole("switch");
    await user.click(switches[0]);

    await waitFor(() => {
      expect(toggleMcpEnabled).toHaveBeenCalledWith("srv-1", false);
    });
  });

  it("deletes a server after confirmation", async () => {
    const user = userEvent.setup();

    renderWithQueryClient(<ServersPage />);

    await screen.findByText("Alpha Server");

    await user.click(screen.getAllByRole("button", { name: "actions" })[0]);
    const deleteMenuItem = screen.getAllByText("delete_server")[0];
    await user.click(deleteMenuItem);

    const confirmButton = screen.getAllByText("delete").at(-1);
    await user.click(confirmButton!);

    await waitFor(() => {
      expect(deleteMcpServer).toHaveBeenCalledWith("srv-1");
    });
  });

  it("filters servers by search, protocol, and status", async () => {
    const user = userEvent.setup();

    renderWithQueryClient(<ServersPage />);

    await screen.findByText("Alpha Server");

    const searchInput = screen.getByPlaceholderText("search_servers");
    await user.type(searchInput, "Beta");

    expect(await screen.findByText("Beta Server")).toBeInTheDocument();
    expect(screen.queryByText("Alpha Server")).toBeNull();

    await user.clear(searchInput);
    await user.click(screen.getByRole("button", { name: "http" }));

    expect(await screen.findByText("Beta Server")).toBeInTheDocument();
    expect(screen.queryByText("Alpha Server")).toBeNull();

    await user.click(screen.getByRole("button", { name: "all" }));
    await user.click(screen.getByRole("button", { name: "sse" }));

    expect(await screen.findByText("Gamma Server")).toBeInTheDocument();
    expect(screen.queryByText("Alpha Server")).toBeNull();
    expect(screen.queryByText("Beta Server")).toBeNull();

    await user.click(screen.getByRole("button", { name: "all" }));
    await user.click(screen.getByRole("button", { name: "stdio" }));

    expect(await screen.findByText("Alpha Server")).toBeInTheDocument();
    expect(screen.queryByText("Beta Server")).toBeNull();

    await user.click(screen.getByRole("button", { name: "all" }));
    await user.click(screen.getByRole("button", { name: "enabled" }));

    expect(await screen.findByText("Alpha Server")).toBeInTheDocument();
    expect(screen.queryByText("Beta Server")).toBeNull();

    await user.click(screen.getByRole("button", { name: "disabled" }));

    expect(await screen.findByText("Beta Server")).toBeInTheDocument();
  });

  it("expands a row to load tools", async () => {
    const user = userEvent.setup();
    vi.mocked(getMcpServerTools).mockResolvedValueOnce([
      { name: "resolve", serverName: "Alpha", serverId: "srv-1" },
    ]);

    renderWithQueryClient(<ServersPage />);

    const betaRow = await screen.findByText("Beta Server");
    const rowElement = betaRow.closest("tr");
    expect(rowElement).not.toBeNull();

    const expandButton = rowElement!.querySelector("button");
    expect(expandButton).not.toBeNull();
    await user.click(expandButton!);

    await waitFor(() => {
      expect(getMcpServerTools).toHaveBeenCalledWith("srv-2");
    });

    expect(await screen.findByText("resolve")).toBeInTheDocument();

    const initialCallCount = mockToolDetailDialog.mock.calls.length;
    await user.click(screen.getByText("resolve"));

    await waitFor(() => {
      expect(mockToolDetailDialog.mock.calls.length).toBeGreaterThan(
        initialCallCount,
      );
    });

    const lastCall = mockToolDetailDialog.mock.calls.at(-1);
    expect(lastCall?.[0]).toMatchObject({
      open: true,
      tool: expect.objectContaining({ name: "resolve" }),
    });

    await user.click(expandButton!);

    await waitFor(() => {
      expect(screen.queryByText("resolve")).toBeNull();
    });
  });

  it("shows error message when loading fails", async () => {
    vi.mocked(listMcpServers).mockRejectedValueOnce(new Error("AUTH_REQUIRED"));

    renderWithQueryClient(<ServersPage />);

    expect(
      await screen.findByText(/error_loading_servers/i),
    ).toBeInTheDocument();
  });

  it("renders base url missing hint when API base is unset", async () => {
    vi.mocked(listMcpServers).mockRejectedValueOnce(
      new Error("API_BASE_URL_UNSET"),
    );

    renderWithQueryClient(<ServersPage />);

    const message = await screen.findByText(/error_loading_servers/i);
    expect(message.textContent).toContain("api_base_missing");
  });

  it("renders server configuration details and clears filters", async () => {
    const user = userEvent.setup();
    vi.mocked(getMcpServerTools).mockResolvedValue([]);

    renderWithQueryClient(<ServersPage />);

    const alphaRow = await screen.findByText("Alpha Server");
    const alphaRowElement = alphaRow.closest("tr");
    expect(alphaRowElement).not.toBeNull();

    const alphaExpand = alphaRowElement!.querySelector("button");
    expect(alphaExpand).not.toBeNull();
    await user.click(alphaExpand!);

    await waitFor(() => {
      expect(getMcpServerTools).toHaveBeenCalledWith("srv-1");
    });

    expect(screen.getByText("/bin/alpha")).toBeInTheDocument();
    expect(screen.getByText("--flag --verbose")).toBeInTheDocument();
    expect(screen.getByText("API_KEY")).toBeInTheDocument();
    expect(
      screen.getByText("ABCDEFGHIJKLMNOPQRST***"),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        (text) => text === "no_tools_available" || text === "暂无工具",
      ),
    ).toBeInTheDocument();

    const betaRow = await screen.findByText("Beta Server");
    const betaRowElement = betaRow.closest("tr");
    expect(betaRowElement).not.toBeNull();
    expect(within(betaRowElement!).getByText("-")).toBeInTheDocument();

    const betaExpand = betaRowElement!.querySelector("button");
    expect(betaExpand).not.toBeNull();
    await user.click(betaExpand!);

    await waitFor(() => {
      expect(getMcpServerTools).toHaveBeenCalledWith("srv-2");
    });

    expect(
      screen.getByText("https://beta.example.com"),
    ).toBeInTheDocument();
    expect(screen.getByText("Authorization")).toBeInTheDocument();
    expect(screen.getByText("Bearer token")).toBeInTheDocument();

    const searchInput = screen.getByPlaceholderText("search_servers");
    await user.clear(searchInput);
    await user.type(searchInput, "Nonexistent");

    expect(
      await screen.findByText(
        (text) => text === "no_servers_found" || text.includes("未找到"),
      ),
    ).toBeInTheDocument();

    const clearButton = screen.getByRole("button", {
      name: /clear_filters|清除过滤器/,
    });
    await user.click(clearButton);

    expect(await screen.findByText("Alpha Server")).toBeInTheDocument();
    expect(await screen.findByText("Beta Server")).toBeInTheDocument();
  });

  it("opens server detail drawer when view action is triggered", async () => {
    const user = userEvent.setup();

    renderWithQueryClient(<ServersPage />);

    await screen.findByText("Alpha Server");
    await user.click(screen.getAllByRole("button", { name: "actions" })[0]);
    await user.click(screen.getAllByText("view_details")[0]!);

    await waitFor(() => {
      const lastCall = mockServerDetailDrawer.mock.calls.at(-1);
      expect(lastCall?.[0]).toMatchObject({
        open: true,
        server: expect.objectContaining({ id: "srv-1" }),
      });
    });
  });

  it("logs edit action when edit is triggered", async () => {
    const user = userEvent.setup();
    const logSpy = vi.spyOn(console, "log").mockImplementation(() => {});

    renderWithQueryClient(<ServersPage />);

    await screen.findByText("Alpha Server");
    await user.click(screen.getAllByRole("button", { name: "actions" })[0]);
    await user.click(screen.getAllByText("edit_server")[0]!);

    expect(logSpy).toHaveBeenCalledWith(
      "Edit server:",
      expect.objectContaining({ id: "srv-1" }),
    );

    logSpy.mockRestore();
  });

  it("responds to keyboard shortcuts", async () => {
    const user = userEvent.setup();

    renderWithQueryClient(<ServersPage />);

    await screen.findByText("Alpha Server");
    const searchInput = screen.getByPlaceholderText("search_servers");

    await user.keyboard("{Control>}n{/Control}");

    await waitFor(() => {
      expect(screen.getByText("add_server_desc")).toBeInTheDocument();
    });

    await user.keyboard("{Escape}");

    await waitFor(() => {
      expect(screen.queryByText("add_server_desc")).toBeNull();
    });

    await user.keyboard("/");

    await waitFor(() => {
      expect(document.activeElement).toBe(searchInput);
    });

    await user.click(screen.getAllByRole("button", { name: "actions" })[0]);
    await user.click(screen.getAllByText("view_details")[0]!);

    await waitFor(() => {
      expect(mockServerDetailDrawer.mock.calls.at(-1)?.[0]?.open).toBe(true);
    });

    await user.keyboard("{Escape}");

    await waitFor(() => {
      expect(mockServerDetailDrawer.mock.calls.at(-1)?.[0]?.open).toBe(false);
    });
  });

  it("shows warning toast when toggle mutation returns warning", async () => {
    const user = userEvent.setup();
    vi.mocked(toggleMcpEnabled).mockResolvedValueOnce({
      warning: "partial failure",
    } as never);

    renderWithQueryClient(<ServersPage />);

    const switches = await screen.findAllByRole("switch");
    await user.click(switches[0]);

    await waitFor(() => {
      expect(toast.warning).toHaveBeenCalledWith("mcp_toggle_warning");
    });
  });

  it("shows error toast when toggle mutation fails", async () => {
    const user = userEvent.setup();
    vi.mocked(toggleMcpEnabled).mockRejectedValueOnce(new Error("boom"));

    renderWithQueryClient(<ServersPage />);

    const switches = await screen.findAllByRole("switch");
    await user.click(switches[0]);

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith("mcp_toggle_error", {
        description: "boom",
      });
    });
  });

  it("shows error toast when delete mutation fails", async () => {
    const user = userEvent.setup();
    vi.mocked(deleteMcpServer).mockRejectedValueOnce(new Error("nope"));

    renderWithQueryClient(<ServersPage />);

    await screen.findByText("Alpha Server");
    await user.click(screen.getAllByRole("button", { name: "actions" })[0]);
    await user.click(screen.getAllByText("delete_server")[0]);
    const confirmButton = screen.getAllByText("delete").at(-1);
    await user.click(confirmButton!);

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalledWith("mcp_delete_error", {
        description: "nope",
      });
    });
  });

  it("formats timestamps across recency buckets", async () => {
    const base = Math.floor(Date.now() / 1000);
    vi.mocked(listMcpServers).mockResolvedValueOnce([
      {
        id: "srv-now",
        name: "Now Server",
        protocol: "stdio",
        enabled: true,
        toolCount: 0,
        createdAt: base,
        lastSeen: base,
      },
      {
        id: "srv-min",
        name: "Minute Server",
        protocol: "sse",
        enabled: true,
        toolCount: 0,
        createdAt: base - 5 * 60,
        lastSeen: base - 5 * 60,
      },
      {
        id: "srv-hour",
        name: "Hour Server",
        protocol: "http",
        enabled: false,
        toolCount: 0,
        createdAt: base - 3 * 3600,
        lastSeen: base - 3 * 3600,
      },
      {
        id: "srv-day",
        name: "Day Server",
        protocol: "stdio",
        enabled: true,
        toolCount: 0,
        createdAt: base - 2 * 86400,
        lastSeen: base - 2 * 86400,
      },
      {
        id: "srv-old",
        name: "Old Server",
        protocol: "ws" as unknown as McpServer["protocol"],
        enabled: true,
        toolCount: 0,
        createdAt: base - 10 * 86400,
        lastSeen: base - 10 * 86400,
      },
      {
        id: "srv-none",
        name: "No Activity Server",
        protocol: "stdio",
        enabled: false,
        toolCount: 0,
        createdAt: base,
      },
    ]);

    renderWithQueryClient(<ServersPage />);

    const nowRow = (await screen.findByText("Now Server")).closest("tr");
    expect(nowRow).not.toBeNull();
    expect(within(nowRow!).getAllByText("just_now")).toHaveLength(2);

    const minutesRow = screen.getByText("Minute Server").closest("tr");
    expect(minutesRow).not.toBeNull();
    expect(within(minutesRow!).getAllByText("5minutes_ago")).toHaveLength(2);

    const hoursRow = screen.getByText("Hour Server").closest("tr");
    expect(hoursRow).not.toBeNull();
    expect(within(hoursRow!).getAllByText("3hours_ago")).toHaveLength(2);

    const daysRow = screen.getByText("Day Server").closest("tr");
    expect(daysRow).not.toBeNull();
    expect(within(daysRow!).getAllByText("2days_ago")).toHaveLength(2);

    const oldRow = screen.getByText("Old Server").closest("tr");
    expect(oldRow).not.toBeNull();
    const formattedOld = new Date(
      (base - 10 * 86400) * 1000,
    ).toLocaleDateString();
    expect(within(oldRow!).getAllByText(formattedOld)).toHaveLength(2);

    const idleRow = screen.getByText("No Activity Server").closest("tr");
    expect(idleRow).not.toBeNull();
    expect(within(idleRow!).getAllByText("-").length).toBeGreaterThan(0);
  });

  it("shows loading indicator while fetching tools", async () => {
    const user = userEvent.setup();
    let resolveTools: ((value: never[]) => void) | undefined;
    vi.mocked(getMcpServerTools).mockImplementationOnce(() => {
      return new Promise<never[]>((resolve) => {
        resolveTools = resolve;
      });
    });

    renderWithQueryClient(<ServersPage />);

    const alphaRow = await screen.findByText("Alpha Server");
    const alphaRowElement = alphaRow.closest("tr");
    expect(alphaRowElement).not.toBeNull();

    const expand = alphaRowElement!.querySelector("button");
    expect(expand).not.toBeNull();
    await user.click(expand!);

    await waitFor(() => {
      expect(document.querySelector(".animate-spin")).not.toBeNull();
    });

    resolveTools?.([]);

    await waitFor(() => {
      expect(document.querySelector(".animate-spin")).toBeNull();
    });
  });
});
