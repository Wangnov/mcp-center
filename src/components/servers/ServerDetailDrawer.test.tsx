import type { ReactNode } from "react";
import { describe, expect, it, beforeEach, vi } from "vitest";
import userEvent from "@testing-library/user-event";
import { screen, waitFor } from "@testing-library/react";
import { renderWithQueryClient } from "@/test/test-utils";
import { ServerDetailDrawer } from "./ServerDetailDrawer";
import { getMcpServerDetail } from "@/lib/api";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>(
    "@/lib/api",
  );
  return {
    ...actual,
    getMcpServerDetail: vi.fn(),
  };
});

vi.mock("@/components/ui/drawer", () => {
  const Wrapper = ({ children }: { children: ReactNode }) => <>{children}</>;
  return {
    Drawer: Wrapper,
    DrawerContent: Wrapper,
    DrawerHeader: Wrapper,
    DrawerFooter: Wrapper,
    DrawerTitle: ({ children }: { children: ReactNode }) => <>{children}</>,
    DrawerDescription: Wrapper,
    DrawerClose: ({ children }: { children: ReactNode }) => <>{children}</>,
  };
});

vi.mock("@/components/ui/tooltip", () => {
  const Wrapper = ({ children }: { children: ReactNode }) => <>{children}</>;
  return {
    Tooltip: Wrapper,
    TooltipTrigger: Wrapper,
    TooltipContent: Wrapper,
  };
});

vi.mock("@/components/servers/ToolDetailDialog", () => ({
  ToolDetailDialog: ({ open, tool }: any) =>
    open && tool ? <div>{`tool:${tool.name}`}</div> : null,
}));

describe("ServerDetailDrawer", () => {
  const server = {
    id: "srv-1",
    name: "Alpha Server",
    protocol: "stdio" as const,
    enabled: true,
    toolCount: 2,
    command: "node",
    args: ["server.js"],
    createdAt: 1730000000,
    lastSeen: 1730001000,
  };

  beforeEach(() => {
    vi.mocked(getMcpServerDetail).mockResolvedValue({
      server,
      tools: [
        { name: "resolve", serverName: "Alpha", serverId: "srv-1" },
      ],
    });
  });

  it("loads server detail and triggers callbacks", async () => {
    const onEdit = vi.fn();
    const onDelete = vi.fn();
    const user = userEvent.setup();

    renderWithQueryClient(
      <ServerDetailDrawer
        open
        onOpenChange={() => undefined}
        server={server}
        onEdit={onEdit}
        onDelete={onDelete}
      />,
    );

    await screen.findByText("Alpha Server");

    await waitFor(() => {
      expect(getMcpServerDetail).toHaveBeenCalledWith("srv-1");
    });

    await user.click(screen.getByText("edit_server"));
    expect(onEdit).toHaveBeenCalledWith(server);

    await user.click(screen.getByText("delete_server"));
    expect(onDelete).toHaveBeenCalledWith(server);

    await user.click(screen.getByText("resolve"));
    await screen.findByText("tool:resolve");
  });

  it("renders remote configuration, env vars and headers when provided", async () => {
    const longValue = "abcdefghijklmnopqrstuvwxyz1234567890";
    vi.mocked(getMcpServerDetail).mockResolvedValueOnce({
      server: {
        ...server,
        protocol: "http",
        url: "https://example.com",
        env: {
          VERY_LONG_KEY: longValue,
        },
        headers: {
          Authorization: "Bearer secret",
        },
        lastSeen: null,
      },
      tools: [],
    });

    renderWithQueryClient(
      <ServerDetailDrawer
        open
        onOpenChange={() => undefined}
        server={{ ...server, protocol: "http" }}
      />,
    );

    await screen.findByText("https://example.com");
    expect(screen.getByText("VERY_LONG_KEY")).toBeInTheDocument();
    expect(
      screen.getByText(longValue.substring(0, 30) + "..."),
    ).toBeInTheDocument();
    expect(screen.getByText("Authorization")).toBeInTheDocument();
    expect(screen.getByText("Bearer secret")).toBeInTheDocument();
  });

  it("shows loading spinner while fetching detail", () => {
    vi.mocked(getMcpServerDetail).mockImplementationOnce(
      () => new Promise(() => undefined) as Promise<any>,
    );

    renderWithQueryClient(
      <ServerDetailDrawer
        open
        onOpenChange={() => undefined}
        server={server}
      />,
    );

    expect(document.querySelector(".animate-spin")).not.toBeNull();
  });
});
