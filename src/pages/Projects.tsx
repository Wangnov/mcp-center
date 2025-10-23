import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  allowProjectServers,
  allowProjectTools,
  denyProjectServers,
  denyProjectTools,
  listMcpServers,
  listProjects,
  McpServer,
  ProjectSummary,
  resetProjectToolDescription,
  setProjectToolDescription,
} from "@/lib/api";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Checkbox } from "@/components/ui/checkbox";
import { Label } from "@/components/ui/label";

interface PermissionPayload {
  projectId: string;
  servers: string[];
}

export function ProjectsPage() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const { data: servers } = useQuery<McpServer[], Error>({
    queryKey: ["servers"],
    queryFn: listMcpServers,
  });

  const {
    data: projects,
    isLoading,
    isError,
    error,
  } = useQuery<ProjectSummary[], Error>({
    queryKey: ["projects"],
    queryFn: listProjects,
  });

  const [isDialogOpen, setIsDialogOpen] = useState(false);
  const [selectedProject, setSelectedProject] = useState<ProjectSummary | null>(
    null,
  );
  const [tempPermissions, setTempPermissions] = useState<
    Record<string, boolean>
  >({});
  const [toolAllowInput, setToolAllowInput] = useState("");
  const [toolDenyInput, setToolDenyInput] = useState("");
  const [toolDescName, setToolDescName] = useState("");
  const [toolDescValue, setToolDescValue] = useState("");

  const serverNameById = useMemo(() => {
    const map = new Map<string, string>();
    (servers ?? []).forEach((server) => {
      map.set(server.id, server.name);
    });
    return map;
  }, [servers]);

  const { mutateAsync: mutateServerPermissions, isPending: isSavingServers } =
    useMutation({
      mutationFn: ({ projectId, servers }: PermissionPayload) =>
        allowProjectServers(projectId, servers),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: ["projects"] });
      },
    });

  const { mutateAsync: mutateDenyServers, isPending: isDenyingServers } =
    useMutation({
      mutationFn: ({ projectId, servers }: PermissionPayload) =>
        denyProjectServers(projectId, servers),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: ["projects"] });
      },
    });

  const { mutateAsync: mutateAllowTools, isPending: isAllowingTools } =
    useMutation({
      mutationFn: ({ target, tools }: { target: string; tools: string[] }) =>
        allowProjectTools(target, tools),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: ["projects"] });
      },
    });

  const { mutateAsync: mutateDenyTools, isPending: isDenyingTools } =
    useMutation({
      mutationFn: ({ target, tools }: { target: string; tools: string[] }) =>
        denyProjectTools(target, tools),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: ["projects"] });
      },
    });

  const { mutateAsync: mutateSetToolDesc, isPending: isSettingToolDesc } =
    useMutation({
      mutationFn: ({
        target,
        tool,
        description,
      }: {
        target: string;
        tool: string;
        description: string;
      }) => setProjectToolDescription(target, tool, description),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: ["projects"] });
      },
    });

  const { mutateAsync: mutateResetToolDesc, isPending: isResettingToolDesc } =
    useMutation({
      mutationFn: ({ target, tool }: { target: string; tool: string }) =>
        resetProjectToolDescription(target, tool),
      onSuccess: () => {
        queryClient.invalidateQueries({ queryKey: ["projects"] });
      },
    });

  const isSaving =
    isSavingServers ||
    isDenyingServers ||
    isAllowingTools ||
    isDenyingTools ||
    isSettingToolDesc ||
    isResettingToolDesc;

  const dialogServerList = useMemo(() => {
    return Object.keys(tempPermissions).sort();
  }, [tempPermissions]);

  const closeDialog = () => {
    setIsDialogOpen(false);
    setSelectedProject(null);
    setTempPermissions({});
    setToolAllowInput("");
    setToolDenyInput("");
    setToolDescName("");
    setToolDescValue("");
  };

  const handleEditClick = (project: ProjectSummary) => {
    const ids = new Set<string>(Array.from(serverNameById.keys()));
    project.allowed_server_ids.forEach((serverId) => ids.add(serverId));

    const initial: Record<string, boolean> = {};
    ids.forEach((serverId) => {
      initial[serverId] = project.allowed_server_ids.includes(serverId);
    });

    setSelectedProject(project);
    setTempPermissions(initial);
    setIsDialogOpen(true);
  };

  const handlePermissionChange = (serverId: string, checked: boolean) => {
    setTempPermissions((prev) => ({ ...prev, [serverId]: checked }));
  };

  const handleSaveChanges = async () => {
    if (!selectedProject) return;

    const nextAllowed = Object.entries(tempPermissions)
      .filter(([, allowed]) => allowed)
      .map(([server]) => server);

    const currentAllowed = new Set(selectedProject.allowed_server_ids);
    const serversToAllow = nextAllowed.filter((s) => !currentAllowed.has(s));
    const serversToDeny = selectedProject.allowed_server_ids.filter(
      (s) => !nextAllowed.includes(s),
    );

    try {
      if (serversToAllow.length > 0) {
        await mutateServerPermissions({
          projectId: selectedProject.id,
          servers: serversToAllow,
        });
      }

      if (serversToDeny.length > 0) {
        await mutateDenyServers({
          projectId: selectedProject.id,
          servers: serversToDeny,
        });
      }

      const allowTools = parseToolInput(toolAllowInput);
      const denyTools = parseToolInput(toolDenyInput);

      if (allowTools.length > 0) {
        await mutateAllowTools({
          target: selectedProject.id,
          tools: allowTools,
        });
      }
      if (denyTools.length > 0) {
        await mutateDenyTools({ target: selectedProject.id, tools: denyTools });
      }

      closeDialog();
    } catch (mutationError) {
      console.error("Failed to update project permissions", mutationError);
    }
  };

  return (
    <div className="p-8 h-full">
      <header className="flex items-center justify-between mb-8">
        <h1 className="text-3xl font-bold">{t("projects")}</h1>
      </header>

      {isLoading && <div>{t("loading_projects")}</div>}
      {isError && (
        <div>
          {t("error_loading_projects")}{" "}
          {error?.message === "AUTH_REQUIRED"
            ? t("auth_required")
            : error?.message === "API_BASE_URL_UNSET"
              ? t("api_base_missing")
              : error?.message}
        </div>
      )}

      {projects && (
        <div className="border rounded-lg bg-card">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t("project_path")}</TableHead>
                <TableHead>{t("project_allowed_servers")}</TableHead>
                <TableHead className="text-right">{t("actions")}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {projects.map((project) => (
                <TableRow key={project.id}>
                  <TableCell className="font-mono">{project.path}</TableCell>
                  <TableCell>
                    {project.allowed_server_ids.length > 0
                      ? project.allowed_server_ids
                          .map(
                            (serverId) =>
                              serverNameById.get(serverId) ?? serverId,
                          )
                          .join(", ")
                      : "-"}
                  </TableCell>
                  <TableCell className="text-right">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => handleEditClick(project)}
                    >
                      {t("edit_permissions")}
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}

      {selectedProject && (
        <Dialog
          open={isDialogOpen}
          onOpenChange={(open) => !open && closeDialog()}
        >
          <DialogContent>
            <DialogHeader>
              <DialogTitle>{t("edit_permissions")}</DialogTitle>
              <DialogDescription className="font-mono">
                {selectedProject.path}
              </DialogDescription>
            </DialogHeader>
            <div className="py-4 space-y-4">
              {dialogServerList.length === 0 && (
                <div className="text-sm text-muted-foreground">
                  {t("no_servers_available")}
                </div>
              )}
              {dialogServerList.map((serverId) => (
                <div key={serverId} className="flex items-center space-x-2">
                  <Checkbox
                    id={serverId}
                    checked={!!tempPermissions[serverId]}
                    onCheckedChange={(checked) =>
                      handlePermissionChange(serverId, !!checked)
                    }
                  />
                  <Label
                    htmlFor={serverId}
                    className="text-sm font-medium leading-none"
                  >
                    {serverNameById.get(serverId) ?? serverId}
                  </Label>
                </div>
              ))}
            </div>
            <div className="border-t pt-4 space-y-4">
              <div className="space-y-2">
                <Label htmlFor="tool-allow" className="text-sm font-semibold">
                  {t("project_tool_allow_label")}
                </Label>
                <textarea
                  id="tool-allow"
                  className="w-full min-h-20 rounded border border-border bg-background p-2 text-sm"
                  placeholder={t("tool_input_hint") ?? "Server::tool_name"}
                  value={toolAllowInput}
                  onChange={(event) => setToolAllowInput(event.target.value)}
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="tool-deny" className="text-sm font-semibold">
                  {t("project_tool_deny_label")}
                </Label>
                <textarea
                  id="tool-deny"
                  className="w-full min-h-20 rounded border border-border bg-background p-2 text-sm"
                  placeholder={t("tool_input_hint") ?? "Server::tool_name"}
                  value={toolDenyInput}
                  onChange={(event) => setToolDenyInput(event.target.value)}
                />
              </div>
            </div>
            <div className="border-t pt-4 space-y-3">
              <Label className="text-sm font-semibold">
                {t("project_tool_description_label")}
              </Label>
              <input
                type="text"
                className="w-full rounded border border-border bg-background p-2 text-sm"
                placeholder={
                  t("project_tool_description_tool_placeholder") ??
                  "Tool name (e.g. resolve-library-id)"
                }
                value={toolDescName}
                onChange={(event) => setToolDescName(event.target.value)}
              />
              <textarea
                className="w-full min-h-20 rounded border border-border bg-background p-2 text-sm"
                placeholder={
                  t("project_tool_description_text_placeholder") ??
                  "Custom description"
                }
                value={toolDescValue}
                onChange={(event) => setToolDescValue(event.target.value)}
              />
              <div className="flex items-center justify-end gap-2">
                <Button
                  variant="outline"
                  onClick={async () => {
                    if (!selectedProject || !toolDescName.trim()) return;
                    try {
                      await mutateResetToolDesc({
                        target: selectedProject.id,
                        tool: toolDescName.trim(),
                      });
                      setToolDescValue("");
                    } catch (mutationError) {
                      console.error(
                        "Failed to reset tool description",
                        mutationError,
                      );
                    }
                  }}
                  disabled={isResettingToolDesc || !toolDescName.trim()}
                >
                  {t("reset_description")}
                </Button>
                <Button
                  onClick={async () => {
                    if (!selectedProject || !toolDescName.trim()) return;
                    try {
                      await mutateSetToolDesc({
                        target: selectedProject.id,
                        tool: toolDescName.trim(),
                        description: toolDescValue.trim(),
                      });
                    } catch (mutationError) {
                      console.error(
                        "Failed to set tool description",
                        mutationError,
                      );
                    }
                  }}
                  disabled={isSettingToolDesc || !toolDescName.trim()}
                >
                  {t("set_description")}
                </Button>
              </div>
            </div>
            <DialogFooter>
              <Button variant="outline" onClick={closeDialog}>
                {t("cancel")}
              </Button>
              <Button onClick={handleSaveChanges} disabled={isSaving}>
                {isSaving ? t("loading") : t("save_changes")}
              </Button>
            </DialogFooter>
          </DialogContent>
        </Dialog>
      )}
    </div>
  );
}

function parseToolInput(raw: string): string[] {
  return raw
    .split(/[\n,]+/)
    .map((value) => value.trim())
    .filter(Boolean);
}
