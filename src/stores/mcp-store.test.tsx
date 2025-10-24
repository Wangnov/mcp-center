import { describe, expect, beforeEach, it } from "vitest";
import { render, waitFor } from "@testing-library/react";
import {
  selectEnabledServers,
  selectLanguage,
  selectProjects,
  selectSelectedProject,
  selectSelectedServer,
  selectTheme,
  useEnabledServers,
  useLanguage,
  useMcpStore,
  useProjects,
  useSelectedProject,
  useSelectedServer,
  useServers,
  useTheme,
  useUiState,
} from "./mcp-store";

beforeEach(() => {
  useMcpStore.getState().reset();
  localStorage.clear();
});

describe("useMcpStore", () => {
  it("manages server collection and toggles enabled state", () => {
    const server = {
      id: "srv-1",
      name: "Demo Server",
      protocol: "stdio" as const,
      enabled: false,
      toolCount: 0,
    };

    useMcpStore.getState().setServers([server]);
    expect(useMcpStore.getState().servers).toHaveLength(1);

    useMcpStore.getState().toggleServerEnabled(server.id);
    expect(useMcpStore.getState().servers[0]?.enabled).toBe(true);

    useMcpStore.getState().removeServer(server.id);
    expect(useMcpStore.getState().servers).toHaveLength(0);
  });

  it("tracks projects and selection state", () => {
    const project = {
      id: "proj-1",
      path: "/demo",
      name: "Demo",
      allowedServers: [],
      createdAt: Date.now(),
      lastSeen: Date.now(),
    };

    useMcpStore.getState().addProject(project);
    expect(useMcpStore.getState().projects).toHaveLength(1);

    useMcpStore.getState().selectProject(project.id);
    expect(useMcpStore.getState().selectedProjectId).toBe(project.id);

    useMcpStore.getState().reset();
    expect(useMcpStore.getState().projects).toHaveLength(0);
    expect(useMcpStore.getState().selectedProjectId).toBeNull();
  });

  it("persists only whitelisted fields", () => {
    useMcpStore.getState().setTheme("dark");
    useMcpStore.getState().selectServer("server-42");

    const snapshotRaw = localStorage.getItem("mcp-center-storage") ?? "{}";
    const snapshot = JSON.parse(snapshotRaw);
    const state = snapshot.state ?? {};

    expect(state.ui.theme).toBe("dark");
    expect(state.selectedServerId).toBe("server-42");
    expect(state).not.toHaveProperty("servers");
    expect(state).not.toHaveProperty("projects");
  });

  it("updates server and project details", () => {
    const initialServer = {
      id: "srv-1",
      name: "Alpha",
      protocol: "stdio" as const,
      enabled: true,
      toolCount: 1,
    };

    const untouchedServer = {
      id: "srv-2",
      name: "Beta",
      protocol: "http" as const,
      enabled: false,
      toolCount: 0,
    };

    useMcpStore.getState().setServers([initialServer, untouchedServer]);
    useMcpStore.getState().updateServer("srv-1", { name: "Alpha Prime" });

    expect(useMcpStore.getState().servers[0]?.name).toBe("Alpha Prime");
    expect(useMcpStore.getState().servers[1]).toMatchObject(untouchedServer);

    const project = {
      id: "proj-1",
      path: "/workspace/demo",
      name: "Demo",
      allowedServers: ["srv-1"],
      createdAt: Date.now(),
      lastSeen: Date.now(),
    };

    const untouchedProject = {
      id: "proj-2",
      path: "/workspace/other",
      name: "Other",
      allowedServers: [],
      createdAt: Date.now(),
      lastSeen: Date.now(),
    };

    useMcpStore.getState().setProjects([project, untouchedProject]);
    useMcpStore.getState().updateProject("proj-1", { name: "Demo Updated" });

    expect(useMcpStore.getState().projects[0]?.name).toBe("Demo Updated");
    expect(useMcpStore.getState().projects[1]).toMatchObject(untouchedProject);

    useMcpStore.getState().removeProject("proj-1");
    expect(useMcpStore.getState().projects).toHaveLength(1);
    expect(useMcpStore.getState().projects[0]).toMatchObject(untouchedProject);
  });

  it("toggles ui state and error flags", () => {
    useMcpStore.getState().setLanguage("ja");
    expect(useMcpStore.getState().ui.language).toBe("ja");

    const previousCollapsed = useMcpStore.getState().ui.sidebarCollapsed;
    useMcpStore.getState().toggleSidebar();
    expect(useMcpStore.getState().ui.sidebarCollapsed).toBe(!previousCollapsed);

    useMcpStore.getState().setLoading(true);
    expect(useMcpStore.getState().isLoading).toBe(true);

    useMcpStore.getState().setError("boom");
    expect(useMcpStore.getState().error).toBe("boom");
  });

  it("exposes selectors for enabled entities and clears selection on removal", () => {
    const server = {
      id: "srv-9",
      name: "Selector Server",
      protocol: "stdio" as const,
      enabled: true,
      toolCount: 3,
    };

    useMcpStore.getState().addServer(server);
    useMcpStore.getState().selectServer(server.id);

    expect(selectEnabledServers(useMcpStore.getState())).toHaveLength(1);
    expect(selectSelectedServer(useMcpStore.getState())).toMatchObject({
      id: "srv-9",
    });

    useMcpStore.getState().removeServer(server.id);
    expect(selectEnabledServers(useMcpStore.getState())).toHaveLength(0);
    expect(selectSelectedServer(useMcpStore.getState())).toBeNull();

    const project = {
      id: "proj-9",
      path: "/selector",
      name: "Selector",
      allowedServers: [],
      createdAt: Date.now(),
      lastSeen: Date.now(),
    };

    useMcpStore.getState().addProject(project);
    useMcpStore.getState().selectProject(project.id);

    expect(selectProjects(useMcpStore.getState())).toHaveLength(1);
    expect(selectSelectedProject(useMcpStore.getState())).toMatchObject({
      id: "proj-9",
    });

    useMcpStore.getState().removeProject(project.id);
    expect(selectProjects(useMcpStore.getState())).toHaveLength(0);
    expect(selectSelectedProject(useMcpStore.getState())).toBeNull();

    expect(selectTheme(useMcpStore.getState())).toBe("system");
    useMcpStore.getState().setTheme("dark");
    expect(selectTheme(useMcpStore.getState())).toBe("dark");

    expect(selectLanguage(useMcpStore.getState())).toBe("zh-CN");
    useMcpStore.getState().setLanguage("en");
    expect(selectLanguage(useMcpStore.getState())).toBe("en");
  });

  it("notifies subscribers via selectors", async () => {
    const themes: string[] = [];
    const enabledSnapshots: string[][] = [];

    const unsubscribeTheme = useMcpStore.subscribe((state) => {
      themes.push(state.ui.theme);
    });

    const unsubscribeEnabled = useMcpStore.subscribe((state) => {
      enabledSnapshots.push(
        state.servers
          .filter((item) => item.enabled)
          .map((item) => item.id),
      );
    });

    useMcpStore.getState().addServer({
      id: "srv-subscriber",
      name: "Sub Server",
      protocol: "stdio",
      enabled: false,
      toolCount: 0,
    });

    useMcpStore.getState().setTheme("dark");
    useMcpStore.getState().toggleServerEnabled("srv-subscriber");

    expect(themes.at(-1)).toBe("dark");
    expect(enabledSnapshots.at(-1)).toEqual(["srv-subscriber"]);

    unsubscribeTheme();
    unsubscribeEnabled();
  });

  it("returns null for selectors when nothing is selected", () => {
    const state = useMcpStore.getState();
    expect(selectSelectedServer(state)).toBeNull();
    expect(selectSelectedProject(state)).toBeNull();
  });

  it("returns null when selected entities are missing from collections", () => {
    useMcpStore.getState().setServers([
      {
        id: "srv-existing",
        name: "Existing",
        protocol: "stdio",
        enabled: true,
        toolCount: 0,
      },
    ]);

    useMcpStore.getState().selectServer("srv-missing");
    expect(selectSelectedServer(useMcpStore.getState())).toBeNull();

    useMcpStore.getState().setProjects([
      {
        id: "proj-existing",
        path: "/tmp",
        name: "Existing Project",
        allowedServers: [],
        createdAt: Date.now(),
        lastSeen: Date.now(),
      },
    ]);

    useMcpStore.getState().selectProject("proj-missing");
    expect(selectSelectedProject(useMcpStore.getState())).toBeNull();
  });

  it("wires hook selectors to zustand store values", () => {
    const results: Record<string, unknown> = {};

    useMcpStore.getState().reset();
    useMcpStore.getState().setServers([
      {
        id: "srv-hook",
        name: "Hook Server",
        protocol: "stdio",
        enabled: true,
        toolCount: 2,
      },
      {
        id: "srv-disabled",
        name: "Disabled",
        protocol: "http",
        enabled: false,
        toolCount: 0,
      },
    ]);
    useMcpStore.getState().selectServer("srv-hook");
    useMcpStore.getState().setProjects([
      {
        id: "proj-hook",
        path: "/hook",
        name: "Hook Project",
        allowedServers: ["srv-hook"],
        createdAt: Date.now(),
        lastSeen: Date.now(),
      },
    ]);
    useMcpStore.getState().selectProject("proj-hook");
    useMcpStore.getState().setTheme("dark");
    useMcpStore.getState().setLanguage("ja");

    const Probe = () => {
      results.servers = useServers();
      results.enabled = useEnabledServers();
      results.selectedServer = useSelectedServer();
      results.projects = useProjects();
      results.selectedProject = useSelectedProject();
      results.ui = useUiState();
      results.theme = useTheme();
      results.language = useLanguage();
      return null;
    };

    render(<Probe />);

    expect(results.servers).toEqual([
      expect.objectContaining({ id: "srv-hook" }),
      expect.objectContaining({ id: "srv-disabled" }),
    ]);
    expect(results.enabled).toEqual([
      expect.objectContaining({ id: "srv-hook" }),
    ]);
    expect(results.selectedServer).toMatchObject({ id: "srv-hook" });
    expect(results.projects).toEqual([
      expect.objectContaining({ id: "proj-hook" }),
    ]);
    expect(results.selectedProject).toMatchObject({ id: "proj-hook" });
    expect(results.ui).toMatchObject({ theme: "dark", language: "ja" });
    expect(results.theme).toBe("dark");
    expect(results.language).toBe("ja");
  });
});
