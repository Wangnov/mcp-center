import { describe, expect, it, beforeEach, vi } from "vitest";
import { screen, waitFor, act } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { renderWithQueryClient } from "@/test/test-utils";
import { ProjectsPage } from "./Projects";
import {
  listMcpServers,
  listProjects,
  allowProjectServers,
  denyProjectServers,
  allowProjectTools,
  denyProjectTools,
  setProjectToolDescription,
  resetProjectToolDescription,
  type McpServer,
  type ProjectSummary,
} from "@/lib/api";

vi.mock("@/lib/api", async () => {
  const actual = await vi.importActual<typeof import("@/lib/api")>("@/lib/api");
  return {
    ...actual,
    listMcpServers: vi.fn(),
    listProjects: vi.fn(),
    allowProjectServers: vi.fn(),
    denyProjectServers: vi.fn(),
    allowProjectTools: vi.fn(),
    denyProjectTools: vi.fn(),
    setProjectToolDescription: vi.fn(),
    resetProjectToolDescription: vi.fn(),
  };
});

describe("ProjectsPage", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    const servers: McpServer[] = [
      {
        id: "srv-1",
        name: "Alpha",
        protocol: "stdio",
        enabled: true,
        toolCount: 2,
      },
      {
        id: "srv-2",
        name: "Beta",
        protocol: "stdio",
        enabled: false,
        toolCount: 0,
      },
    ];
    vi.mocked(listMcpServers).mockResolvedValue(servers);

    const projects: ProjectSummary[] = [
      {
        id: "proj-1",
        path: "/workspace/demo",
        displayName: null,
        agent: null,
        allowedServerIds: ["srv-1"],
        createdAt: Date.now(),
        lastSeenAt: Date.now(),
      } as ProjectSummary,
    ].map((project) =>
      ({
        ...project,
        allowed_server_ids: project.allowedServerIds,
      } as unknown as ProjectSummary),
    );
    vi.mocked(listProjects).mockResolvedValue(projects);

    vi.mocked(allowProjectServers).mockResolvedValue(null);
    vi.mocked(denyProjectServers).mockResolvedValue(null);
    vi.mocked(allowProjectTools).mockResolvedValue(null);
    vi.mocked(denyProjectTools).mockResolvedValue(null);
    vi.mocked(setProjectToolDescription).mockResolvedValue(null);
    vi.mocked(resetProjectToolDescription).mockResolvedValue(null);
  });

  it("saves updated permissions and tool assignments", async () => {
    const user = userEvent.setup();

    renderWithQueryClient(<ProjectsPage />);

    await screen.findByText("/workspace/demo");

    await user.click(screen.getByRole("button", { name: "edit_permissions" }));

    const allowCheckbox = await screen.findByLabelText("Beta");
    await user.click(allowCheckbox);

    const denyCheckbox = await screen.findByLabelText("Alpha");
    await user.click(denyCheckbox);

    const allowTextarea = screen.getByLabelText("project_tool_allow_label");
    await user.type(allowTextarea, "Alpha::analyze, Beta::inspect");

    await user.click(screen.getByRole("button", { name: "save_changes" }));

    await waitFor(() => {
      expect(allowProjectServers).toHaveBeenCalled();
      expect(denyProjectServers).toHaveBeenCalled();
    });

    const allowCall = vi.mocked(allowProjectServers).mock.calls.at(-1);
    expect(allowCall?.[0]).toBe("proj-1");
    expect(allowCall?.[1]).toEqual(["srv-2"]);

    const denyCall = vi.mocked(denyProjectServers).mock.calls.at(-1);
    expect(denyCall?.[0]).toBe("proj-1");
    expect(denyCall?.[1]).toEqual(["srv-1"]);

    await waitFor(() => {
      expect(allowProjectTools).toHaveBeenCalledWith(
        "proj-1",
        ["Alpha::analyze", "Beta::inspect"],
      );
    });

    expect(denyProjectTools).not.toHaveBeenCalled();
  });

  it("sets and resets tool description", async () => {
    const user = userEvent.setup();

    renderWithQueryClient(<ProjectsPage />);

    await screen.findByText("/workspace/demo");
    await user.click(screen.getByRole("button", { name: "edit_permissions" }));

    const toolInput = screen.getByPlaceholderText(
      "project_tool_description_tool_placeholder",
    );
    const descTextarea = screen.getByPlaceholderText(
      "project_tool_description_text_placeholder",
    );

    await user.type(toolInput, "resolve-library-id");
    await user.type(descTextarea, "Resolve repositories");

    await user.click(screen.getByRole("button", { name: "set_description" }));

    await waitFor(() => {
      expect(setProjectToolDescription).toHaveBeenCalledWith(
        "proj-1",
        "resolve-library-id",
        "Resolve repositories",
      );
    });

    await user.click(screen.getByRole("button", { name: "reset_description" }));

    await waitFor(() => {
      expect(resetProjectToolDescription).toHaveBeenCalledWith(
        "proj-1",
        "resolve-library-id",
      );
    });
  });

  it("submits deny list without allow entries", async () => {
    const user = userEvent.setup();

    renderWithQueryClient(<ProjectsPage />);

    await screen.findByText("/workspace/demo");
    await user.click(screen.getByRole("button", { name: "edit_permissions" }));

    const denyTextarea = screen.getByLabelText("project_tool_deny_label");
    await user.type(denyTextarea, "Alpha::inspect\nBeta::fetch");

    await user.click(screen.getByRole("button", { name: "save_changes" }));

    await waitFor(() => {
      expect(denyProjectTools).toHaveBeenCalledWith("proj-1", [
        "Alpha::inspect",
        "Beta::fetch",
      ]);
    });

    expect(allowProjectTools).not.toHaveBeenCalled();
  });

  it("skips mutations when no changes are made", async () => {
    const user = userEvent.setup();

    renderWithQueryClient(<ProjectsPage />);

    await screen.findByText("/workspace/demo");
    await user.click(screen.getByRole("button", { name: "edit_permissions" }));

    await user.click(screen.getByRole("button", { name: "save_changes" }));

    await waitFor(() => {
      expect(allowProjectServers).not.toHaveBeenCalled();
      expect(denyProjectServers).not.toHaveBeenCalled();
      expect(allowProjectTools).not.toHaveBeenCalled();
      expect(denyProjectTools).not.toHaveBeenCalled();
    });

    await waitFor(() => {
      expect(
        screen.queryByRole("dialog", { name: "edit_permissions" }),
      ).not.toBeInTheDocument();
    });
  });

  it("shows fallback when no servers can be assigned", async () => {
    const user = userEvent.setup();

    vi.mocked(listMcpServers).mockResolvedValueOnce([]);
    vi.mocked(listProjects).mockResolvedValueOnce([
      {
        id: "proj-empty",
        path: "/workspace/empty",
        displayName: null,
        agent: null,
        allowedServerIds: [],
        createdAt: Date.now(),
        lastSeenAt: Date.now(),
      } as ProjectSummary,
    ].map(
      (project) =>
        ({
          ...project,
          allowed_server_ids: project.allowedServerIds,
        }) as unknown as ProjectSummary,
    ));

    renderWithQueryClient(<ProjectsPage />);

    await screen.findByText("/workspace/empty");
    await user.click(screen.getByRole("button", { name: "edit_permissions" }));

    expect(
      await screen.findByText("no_servers_available"),
    ).toBeInTheDocument();
  });

  it("renders auth required hint when loading projects fails", async () => {
    vi.mocked(listProjects).mockRejectedValueOnce(
      new Error("AUTH_REQUIRED"),
    );

    renderWithQueryClient(<ProjectsPage />);

    const errorLine = await screen.findByText(/error_loading_projects/i);
    expect(errorLine.textContent).toContain("auth_required");
  });

  it("renders base url missing hint when API base is unset", async () => {
    vi.mocked(listProjects).mockRejectedValueOnce(
      new Error("API_BASE_URL_UNSET"),
    );

    renderWithQueryClient(<ProjectsPage />);

    const errorLine = await screen.findByText(/error_loading_projects/i);
    expect(errorLine.textContent).toContain("api_base_missing");
  });

  it("renders raw error message when project loading fails unexpectedly", async () => {
    vi.mocked(listProjects).mockRejectedValueOnce(new Error("network gone"));

    renderWithQueryClient(<ProjectsPage />);

    const errorLine = await screen.findByText(/error_loading_projects/i);
    expect(errorLine.textContent).toContain("network gone");
  });

  it("logs errors when permission mutations fail", async () => {
    const user = userEvent.setup();
    const errorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});

    vi.mocked(allowProjectServers).mockRejectedValueOnce(new Error("nope"));

    renderWithQueryClient(<ProjectsPage />);

    await screen.findByText("/workspace/demo");
    await user.click(screen.getByRole("button", { name: "edit_permissions" }));

    const allowCheckbox = await screen.findByLabelText("Beta");
    await user.click(allowCheckbox);

    await user.click(screen.getByRole("button", { name: "save_changes" }));

    await waitFor(() => {
      expect(errorSpy).toHaveBeenCalledWith(
        "Failed to update project permissions",
        expect.any(Error),
      );
    });

    expect(denyProjectServers).not.toHaveBeenCalled();
    expect(
      screen.getByRole("button", { name: "save_changes" }),
    ).toBeInTheDocument();

    errorSpy.mockRestore();
  });

  it("logs tool description mutation failures", async () => {
    const user = userEvent.setup();
    const errorSpy = vi
      .spyOn(console, "error")
      .mockImplementation(() => {});

    vi.mocked(setProjectToolDescription).mockRejectedValueOnce(
      new Error("set fails"),
    );
    vi.mocked(resetProjectToolDescription).mockRejectedValueOnce(
      new Error("reset fails"),
    );

    renderWithQueryClient(<ProjectsPage />);

    await screen.findByText("/workspace/demo");
    await user.click(screen.getByRole("button", { name: "edit_permissions" }));

    const toolInput = screen.getByPlaceholderText(
      "project_tool_description_tool_placeholder",
    );
    const descTextarea = screen.getByPlaceholderText(
      "project_tool_description_text_placeholder",
    );

    await user.type(toolInput, "resolve-library-id");
    await user.type(descTextarea, "Docs");

    await user.click(screen.getByRole("button", { name: "set_description" }));

    await waitFor(() => {
      expect(errorSpy).toHaveBeenCalledWith(
        "Failed to set tool description",
        expect.any(Error),
      );
    });

    await user.click(screen.getByRole("button", { name: "reset_description" }));

    await waitFor(() => {
      expect(errorSpy).toHaveBeenCalledWith(
        "Failed to reset tool description",
        expect.any(Error),
      );
    });

    errorSpy.mockRestore();
  });

  it("shows server id when the display name is missing", async () => {
    const user = userEvent.setup();

    vi.mocked(listMcpServers).mockResolvedValueOnce([
      {
        id: "srv-known",
        name: "Known",
        protocol: "stdio",
        enabled: true,
        toolCount: 1,
      },
    ]);
    vi.mocked(listProjects).mockResolvedValueOnce([
      {
        id: "proj-fallback",
        path: "/workspace/fallback",
        displayName: null,
        agent: null,
        allowedServerIds: ["srv-known", "srv-missing"],
        allowed_server_ids: ["srv-known", "srv-missing"],
        createdAt: Date.now(),
        lastSeenAt: Date.now(),
      } as unknown as ProjectSummary,
    ]);

    renderWithQueryClient(<ProjectsPage />);

    await screen.findByText("/workspace/fallback");
    await user.click(screen.getByRole("button", { name: "edit_permissions" }));

    expect(await screen.findByLabelText("Known")).toBeInTheDocument();
    expect(screen.getByText("srv-missing")).toBeInTheDocument();
  });

  it("closes dialog when escape key is pressed", async () => {
    const user = userEvent.setup();

    renderWithQueryClient(<ProjectsPage />);

    await screen.findByText("/workspace/demo");
    await user.click(screen.getByRole("button", { name: "edit_permissions" }));

    await screen.findByRole("dialog", { name: "edit_permissions" });
    await user.keyboard("{Escape}");

    await waitFor(() => {
      expect(
        screen.queryByRole("dialog", { name: "edit_permissions" }),
      ).toBeNull();
    });
  });

  it("shows loading label while permissions are saving", async () => {
    const user = userEvent.setup();
    let resolveAllow: ((value: unknown) => void) | undefined;
    vi.mocked(allowProjectServers).mockImplementationOnce(
      () =>
        new Promise((resolve) => {
          resolveAllow = resolve;
        }),
    );

    renderWithQueryClient(<ProjectsPage />);

    await screen.findByText("/workspace/demo");
    await user.click(screen.getByRole("button", { name: "edit_permissions" }));

    const betaCheckbox = await screen.findByLabelText("Beta");
    await user.click(betaCheckbox);

    const saveButton = screen.getByRole("button", { name: "save_changes" });
    await user.click(saveButton);

    await waitFor(() => {
      expect(allowProjectServers).toHaveBeenCalled();
    });

    const loadingButton = await screen.findByRole("button", { name: "loading" });
    expect(loadingButton).toBeDisabled();

    await act(async () => {
      resolveAllow?.(null);
    });
  });
});
