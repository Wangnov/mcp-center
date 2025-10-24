import type { ReactNode } from "react";
import { describe, expect, it, vi, beforeEach } from "vitest";
import userEvent from "@testing-library/user-event";
import { screen, waitFor } from "@testing-library/react";
import { renderWithQueryClient } from "@/test/test-utils";
import { AddServerDialog } from "./AddServerDialog";
import { addMcpServer } from "@/lib/api";
import { toast } from "sonner";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>(
    "@/lib/api",
  );
  return {
    ...actual,
    addMcpServer: vi.fn().mockResolvedValue(null),
  };
});

vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: vi.fn(),
  },
}));

vi.mock("@/components/ui/dialog", () => {
  const Wrapper = ({ children }: { children: ReactNode }) => <>{children}</>;
  return {
    Dialog: Wrapper,
    DialogTrigger: Wrapper,
    DialogContent: Wrapper,
    DialogDescription: Wrapper,
    DialogHeader: Wrapper,
    DialogTitle: Wrapper,
    DialogFooter: Wrapper,
    DialogClose: Wrapper,
  };
});

describe("AddServerDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("submits stdio server and splits command", async () => {
    const user = userEvent.setup();

    renderWithQueryClient(
      <AddServerDialog open onOpenChange={() => {}}>
        <span>noop</span>
      </AddServerDialog>,
    );

    await user.type(
      screen.getByPlaceholderText("My Awesome Server"),
      "Demo Server",
    );
    await user.type(
      screen.getByPlaceholderText("npx -y my-mcp-server"),
      "node server.js --flag",
    );

    await user.click(screen.getByRole("button", { name: "save_server" }));

    await waitFor(() => {
      expect(addMcpServer).toHaveBeenCalledWith(
        expect.objectContaining({
          name: "Demo Server",
          protocol: "stdio",
          command: "node",
          args: "server.js --flag",
        }),
      );
    });
    expect(toast.success).toHaveBeenCalled();
  });

  it("shows error toast on failure", async () => {
    vi.mocked(addMcpServer).mockRejectedValueOnce(new Error("boom"));
    const user = userEvent.setup();

    renderWithQueryClient(
      <AddServerDialog open onOpenChange={() => {}}>
        <span>noop</span>
      </AddServerDialog>,
    );
    await user.type(
      screen.getByPlaceholderText("My Awesome Server"),
      "Broken Server",
    );
    await user.type(
      screen.getByPlaceholderText("npx -y my-mcp-server"),
      "node server.js",
    );

    await user.click(screen.getByRole("button", { name: "save_server" }));

    await waitFor(() => {
      expect(toast.error).toHaveBeenCalled();
    });
  });
});
