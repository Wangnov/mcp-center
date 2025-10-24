import { useState } from "react";
import { useTranslation } from "react-i18next";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { toast } from "sonner";
import {
  useKeyboardShortcuts,
  getModifierSymbol,
} from "@/hooks/use-keyboard-shortcuts";
import {
  listMcpServers,
  McpServer,
  toggleMcpEnabled,
  getMcpServerTools,
  deleteMcpServer,
  ToolInfo,
} from "@/lib/api";
import { AddServerDialog } from "@/components/AddServerDialog";
import { ServerDetailDrawer } from "@/components/servers/ServerDetailDrawer";
import { ToolDetailDialog } from "@/components/servers/ToolDetailDialog";
import { Switch } from "@/components/ui/switch";
import { Badge } from "@/components/ui/badge";
import type { BadgeVariant } from "@/components/ui/badge-variants";
import { Button } from "@/components/ui/button";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  ChevronDown,
  ChevronRight,
  Edit,
  Trash2,
  Eye,
  Server as ServerIcon,
  Search,
  Loader2,
  MoreHorizontal,
} from "lucide-react";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";

export function ServersPage() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [pendingServer, setPendingServer] = useState<string | null>(null);
  const [expandedRows, setExpandedRows] = useState<Set<string>>(new Set());
  const [searchTerm, setSearchTerm] = useState("");
  const [protocolFilter, setProtocolFilter] = useState<string | null>(null);
  const [statusFilter, setStatusFilter] = useState<boolean | null>(null);
  const [addDialogOpen, setAddDialogOpen] = useState(false);

  // Drawer state
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [selectedServer, setSelectedServer] = useState<McpServer | null>(null);

  // Delete dialog state
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [serverToDelete, setServerToDelete] = useState<McpServer | null>(null);

  // 键盘快捷键
  useKeyboardShortcuts([
    {
      key: "n",
      ctrl: true,
      callback: () => setAddDialogOpen(true),
    },
    {
      key: "/",
      callback: (e) => {
        e.preventDefault();
        document.querySelector<HTMLInputElement>('input[type="text"]')?.focus();
      },
    },
    {
      key: "Escape",
      callback: () => {
        setDrawerOpen(false);
        setDeleteDialogOpen(false);
        setAddDialogOpen(false);
      },
    },
  ]);

  const {
    data: servers,
    isLoading,
    isError,
    error,
  } = useQuery<McpServer[], Error>({
    queryKey: ["servers"],
    queryFn: listMcpServers,
  });

  const toggleMutation = useMutation({
    mutationFn: ({ id, enabled }: { id: string; enabled: boolean }) =>
      toggleMcpEnabled(id, enabled),
    onMutate: ({ id }) => {
      setPendingServer(id);
    },
    onSuccess: (result) => {
      if (result?.warning) {
        toast.warning(t("mcp_toggle_warning", { reason: result.warning }));
      } else {
        toast.success(t("mcp_toggle_success") || "服务器状态已更新");
      }
    },
    onError: (mutationError) => {
      console.error("Failed to toggle MCP server", mutationError);
      toast.error(t("mcp_toggle_error") || "切换服务器状态失败", {
        description: mutationError.message,
      });
    },
    onSettled: () => {
      setPendingServer(null);
      queryClient.invalidateQueries({ queryKey: ["servers"] });
    },
  });

  const deleteMutation = useMutation({
    mutationFn: (serverId: string) => deleteMcpServer(serverId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["servers"] });
      setDeleteDialogOpen(false);
      setServerToDelete(null);
      toast.success(t("mcp_delete_success") || "服务器已删除");
    },
    onError: (mutationError) => {
      console.error("Failed to delete MCP server", mutationError);
      toast.error(t("mcp_delete_error") || "删除服务器失败", {
        description: mutationError.message,
      });
    },
  });

  // 切换行展开状态
  const toggleRow = (serverId: string) => {
    setExpandedRows((prev) => {
      const next = new Set(prev);
      if (next.has(serverId)) {
        next.delete(serverId);
      } else {
        next.add(serverId);
      }
      return next;
    });
  };

  // 过滤服务器
  const filteredServers = servers?.filter((server) => {
    const matchesSearch = server.name
      .toLowerCase()
      .includes(searchTerm.toLowerCase());
    const matchesProtocol =
      protocolFilter === null || server.protocol === protocolFilter;
    const matchesStatus =
      statusFilter === null || server.enabled === statusFilter;
    return matchesSearch && matchesProtocol && matchesStatus;
  });

  // 协议徽章颜色
  const getProtocolBadgeVariant = (protocol: string): BadgeVariant => {
    switch (protocol) {
      case "stdio":
        return "default";
      case "sse":
        return "secondary";
      case "http":
        return "outline";
      default:
        return "default";
    }
  };

  // 格式化时间
  const formatTime = (timestamp?: number | null) => {
    if (timestamp == null) return "-";
    const date = new Date(timestamp * 1000);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    const minutes = Math.floor(diff / 60000);
    const hours = Math.floor(diff / 3600000);
    const days = Math.floor(diff / 86400000);

    if (minutes < 1) return t("just_now") || "刚刚";
    if (minutes < 60) return `${minutes}${t("minutes_ago") || "分钟前"}`;
    if (hours < 24) return `${hours}${t("hours_ago") || "小时前"}`;
    if (days < 7) return `${days}${t("days_ago") || "天前"}`;
    return date.toLocaleDateString();
  };

  // 查看服务器详情
  const handleViewServer = (server: McpServer) => {
    setSelectedServer(server);
    setDrawerOpen(true);
  };

  // 编辑服务器（TODO: 实现编辑对话框）
  const handleEditServer = (server: McpServer) => {
    console.log("Edit server:", server);
    // TODO: 打开编辑对话框
  };

  // 删除服务器
  const handleDeleteServer = (server: McpServer) => {
    setServerToDelete(server);
    setDeleteDialogOpen(true);
  };

  const confirmDelete = () => {
    if (serverToDelete) {
      deleteMutation.mutate(serverToDelete.id);
    }
  };

  return (
    <div className="flex flex-col h-full p-8">
      {/* 页面头部 */}
      <header className="flex items-center justify-between mb-6">
        <div>
          <h1 className="text-3xl font-bold">{t("mcp_servers")}</h1>
          <p className="text-sm text-muted-foreground mt-1">
            {t("manage_mcp_servers") || "管理您的 MCP 服务器连接"}
          </p>
        </div>
        <AddServerDialog open={addDialogOpen} onOpenChange={setAddDialogOpen}>
          <Button>
            {t("add_server")}
            <kbd className="ml-2 pointer-events-none inline-flex h-5 select-none items-center gap-1 rounded border bg-muted px-1.5 font-mono text-[10px] font-medium text-muted-foreground opacity-100">
              <span className="text-xs">{getModifierSymbol()}</span>N
            </kbd>
          </Button>
        </AddServerDialog>
      </header>

      {/* 搜索和过滤器 */}
      <div className="flex flex-col gap-4 mb-6">
        <div className="flex items-center gap-4">
          {/* 搜索框 */}
          <div className="flex-1 relative">
            <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
            <input
              type="text"
              placeholder={t("search_servers") || "搜索服务器..."}
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="w-full rounded-md border border-input bg-background pl-10 pr-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2"
            />
          </div>

          {/* 协议过滤 */}
          <div className="flex gap-2">
            <Button
              variant={protocolFilter === null ? "default" : "outline"}
              size="sm"
              onClick={() => setProtocolFilter(null)}
            >
              {t("all") || "全部"}
            </Button>
            <Button
              variant={protocolFilter === "stdio" ? "default" : "outline"}
              size="sm"
              onClick={() => setProtocolFilter("stdio")}
            >
              stdio
            </Button>
            <Button
              variant={protocolFilter === "sse" ? "default" : "outline"}
              size="sm"
              onClick={() => setProtocolFilter("sse")}
            >
              sse
            </Button>
            <Button
              variant={protocolFilter === "http" ? "default" : "outline"}
              size="sm"
              onClick={() => setProtocolFilter("http")}
            >
              http
            </Button>
          </div>

          {/* 状态过滤 */}
          <div className="flex gap-2">
            <Button
              variant={statusFilter === true ? "default" : "outline"}
              size="sm"
              onClick={() =>
                setStatusFilter((prev) => (prev === true ? null : true))
              }
            >
              {t("enabled") || "已启用"}
            </Button>
            <Button
              variant={statusFilter === false ? "default" : "outline"}
              size="sm"
              onClick={() =>
                setStatusFilter((prev) => (prev === false ? null : false))
              }
            >
              {t("disabled") || "已禁用"}
            </Button>
          </div>
        </div>

        {/* 统计信息 */}
        {servers && (
          <div className="flex items-center gap-6 text-sm text-muted-foreground">
            <span>
              {t("total_servers") || "总计"} {servers.length}
            </span>
            <span>
              {t("enabled") || "已启用"}{" "}
              {servers.filter((s) => s.enabled).length}
            </span>
            <span>
              {t("total_tools") || "总工具"}{" "}
              {servers.reduce((sum, s) => sum + (s.toolCount || 0), 0)}
            </span>
          </div>
        )}
      </div>

      {/* 加载和错误状态 */}
      {isLoading && (
        <div className="flex items-center justify-center py-12">
          <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
        </div>
      )}
      {isError && (
        <div className="text-center py-8 text-destructive">
          {t("error_loading_servers") || "加载服务器失败"}:{" "}
          {error.message === "AUTH_REQUIRED"
            ? t("auth_required")
            : error.message === "API_BASE_URL_UNSET"
              ? t("api_base_missing")
              : error.message}
        </div>
      )}

      {/* 增强表格 */}
      {filteredServers && filteredServers.length > 0 && (
        <div className="border rounded-lg bg-card overflow-hidden">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead className="w-12"></TableHead>
                <TableHead>{t("name") || "名称"}</TableHead>
                <TableHead>{t("protocol") || "协议"}</TableHead>
                <TableHead className="text-right">
                  {t("tool_count") || "工具数"}
                </TableHead>
                <TableHead>{t("created_at") || "创建时间"}</TableHead>
                <TableHead>{t("last_seen") || "最后使用"}</TableHead>
                <TableHead className="w-32">{t("status") || "状态"}</TableHead>
                <TableHead className="text-right w-40">
                  {t("actions") || "操作"}
                </TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {filteredServers.map((server) => {
                const isExpanded = expandedRows.has(server.id);
                return (
                  <ExpandableServerRow
                    key={server.id}
                    server={server}
                    isExpanded={isExpanded}
                    onToggle={() => toggleRow(server.id)}
                    onView={() => handleViewServer(server)}
                    onEdit={() => handleEditServer(server)}
                    onDelete={() => handleDeleteServer(server)}
                    onToggleEnabled={(enabled) =>
                      toggleMutation.mutate({ id: server.id, enabled })
                    }
                    isPending={
                      toggleMutation.isPending && pendingServer === server.id
                    }
                    getProtocolBadgeVariant={getProtocolBadgeVariant}
                    formatTime={formatTime}
                  />
                );
              })}
            </TableBody>
          </Table>
        </div>
      )}

      {/* 空状态 */}
      {filteredServers && filteredServers.length === 0 && (
        <div className="text-center py-12 text-muted-foreground">
          <ServerIcon className="h-12 w-12 mx-auto mb-4 opacity-50" />
          <p>{t("no_servers_found") || "未找到服务器"}</p>
          {(searchTerm || protocolFilter || statusFilter !== null) && (
            <Button
              variant="link"
              onClick={() => {
                setSearchTerm("");
                setProtocolFilter(null);
                setStatusFilter(null);
              }}
              className="mt-2"
            >
              {t("clear_filters") || "清除过滤器"}
            </Button>
          )}
        </div>
      )}

      {/* 详情抽屉 */}
      <ServerDetailDrawer
        server={selectedServer}
        open={drawerOpen}
        onOpenChange={setDrawerOpen}
        onEdit={handleEditServer}
        onDelete={handleDeleteServer}
      />

      {/* 删除确认对话框 */}
      <AlertDialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              {t("confirm_delete") || "确认删除"}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {t("delete_server_confirm") || "确定要删除服务器"}{" "}
              <strong>{serverToDelete?.name}</strong>？
              {t("action_cannot_undone") || "此操作无法撤销。"}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t("cancel") || "取消"}</AlertDialogCancel>
            <AlertDialogAction
              onClick={confirmDelete}
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              disabled={deleteMutation.isPending}
            >
              {deleteMutation.isPending ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  {t("deleting") || "删除中..."}
                </>
              ) : (
                t("delete") || "删除"
              )}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}

// 可展开的服务器行组件
interface ExpandableServerRowProps {
  server: McpServer;
  isExpanded: boolean;
  onToggle: () => void;
  onView: () => void;
  onEdit: () => void;
  onDelete: () => void;
  onToggleEnabled: (enabled: boolean) => void;
  isPending: boolean;
  getProtocolBadgeVariant: (protocol: string) => BadgeVariant;
  formatTime: (timestamp?: number | null) => string;
}

function ExpandableServerRow({
  server,
  isExpanded,
  onToggle,
  onView,
  onEdit,
  onDelete,
  onToggleEnabled,
  isPending,
  getProtocolBadgeVariant,
  formatTime,
}: ExpandableServerRowProps) {
  const { t } = useTranslation();

  // 工具详情对话框状态
  const [selectedTool, setSelectedTool] = useState<ToolInfo | null>(null);
  const [toolDialogOpen, setToolDialogOpen] = useState(false);

  // 获取展开行的工具列表
  const { data: tools, isLoading: isLoadingTools } = useQuery<
    ToolInfo[],
    Error
  >({
    queryKey: ["server-tools", server.id],
    queryFn: () => getMcpServerTools(server.id),
    enabled: isExpanded,
  });

  return (
    <>
      {/* 主行 */}
      <TableRow className="hover:bg-muted/50">
        {/* 展开按钮 */}
        <TableCell>
          <Tooltip>
            <TooltipTrigger asChild>
              <button
                onClick={onToggle}
                className="p-1 hover:bg-accent rounded-sm transition-colors"
              >
                {isExpanded ? (
                  <ChevronDown className="h-4 w-4" />
                ) : (
                  <ChevronRight className="h-4 w-4" />
                )}
              </button>
            </TooltipTrigger>
            <TooltipContent>
              <p>
                {isExpanded
                  ? t("collapse") || "收起详情"
                  : t("expand") || "展开详情"}
              </p>
            </TooltipContent>
          </Tooltip>
        </TableCell>

        {/* 名称 */}
        <TableCell className="font-medium">
          <div className="flex items-center gap-2">
            <ServerIcon className="h-4 w-4 text-muted-foreground" />
            <span>{server.name}</span>
          </div>
        </TableCell>

        {/* 协议 */}
        <TableCell>
          <Badge variant={getProtocolBadgeVariant(server.protocol)}>
            {server.protocol}
          </Badge>
        </TableCell>

        {/* 工具数 */}
        <TableCell className="text-right tabular-nums">
          {server.toolCount || 0}
        </TableCell>

        {/* 创建时间 */}
        <TableCell className="text-muted-foreground text-sm">
          {formatTime(server.createdAt)}
        </TableCell>

        {/* 最后使用 */}
        <TableCell className="text-muted-foreground text-sm">
          {formatTime(server.lastSeen)}
        </TableCell>

        {/* 状态 Badge - 呼吸效果 */}
        <TableCell>
          <div className="flex items-center gap-2">
            <Switch
              checked={server.enabled}
              onCheckedChange={onToggleEnabled}
              disabled={isPending}
            />
            {server.enabled ? (
              <Badge className="bg-green-500/10 text-green-600 border-green-500/20 animate-pulse">
                ● {t("enabled") || "已启用"}
              </Badge>
            ) : (
              <Badge variant="secondary" className="text-muted-foreground">
                {t("disabled") || "已禁用"}
              </Badge>
            )}
          </div>
        </TableCell>

        {/* 操作按钮 */}
        <TableCell className="text-right">
          <DropdownMenu>
            <DropdownMenuTrigger asChild>
              <Button variant="ghost" className="h-8 w-8 p-0">
                <span className="sr-only">{t("actions") || "操作"}</span>
                <MoreHorizontal className="h-4 w-4" />
              </Button>
            </DropdownMenuTrigger>
            <DropdownMenuContent align="end">
              <DropdownMenuLabel>{t("actions") || "操作"}</DropdownMenuLabel>
              <DropdownMenuItem onClick={onView}>
                <Eye className="mr-2 h-4 w-4" />
                {t("view_details") || "查看详情"}
              </DropdownMenuItem>
              <DropdownMenuItem onClick={onEdit}>
                <Edit className="mr-2 h-4 w-4" />
                {t("edit_server") || "编辑服务器"}
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                onClick={onDelete}
                className="text-destructive focus:text-destructive"
              >
                <Trash2 className="mr-2 h-4 w-4" />
                {t("delete_server") || "删除服务器"}
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </TableCell>
      </TableRow>

      {/* 展开的详细信息行 */}
      {isExpanded && (
        <TableRow>
          <TableCell colSpan={7} className="bg-muted/30 p-6">
            <div className="space-y-4">
              {/* 基本信息 */}
              <div>
                <h4 className="text-sm font-semibold mb-2">
                  {t("basic_info") || "基本信息"}
                </h4>
                <dl className="grid grid-cols-2 gap-x-4 gap-y-2 text-sm">
                  <div>
                    <dt className="text-muted-foreground">
                      {t("server_id") || "服务器 ID"}
                    </dt>
                    <dd className="font-mono">{server.id}</dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">
                      {t("created_at") || "创建时间"}
                    </dt>
                    <dd>{formatTime(server.createdAt)}</dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">
                      {t("tool_count") || "工具数量"}
                    </dt>
                    <dd>{server.toolCount || 0}</dd>
                  </div>
                  <div>
                    <dt className="text-muted-foreground">
                      {t("last_seen") || "最后使用"}
                    </dt>
                    <dd>{formatTime(server.lastSeen)}</dd>
                  </div>
                </dl>
              </div>

              {/* 启动配置（stdio）*/}
              {server.protocol === "stdio" && server.command && (
                <div>
                  <h4 className="text-sm font-semibold mb-2">
                    {t("startup_config") || "启动配置"}
                  </h4>
                  <div className="space-y-2 text-sm">
                    <div>
                      <span className="text-muted-foreground">
                        {t("command") || "命令"}:
                      </span>
                      <code className="ml-2 px-2 py-1 bg-muted rounded text-xs font-mono">
                        {server.command}
                      </code>
                    </div>
                    {server.args && server.args.length > 0 && (
                      <div>
                        <span className="text-muted-foreground">
                          {t("arguments") || "参数"}:
                        </span>
                        <code className="ml-2 px-2 py-1 bg-muted rounded text-xs font-mono">
                          {server.args.join(" ")}
                        </code>
                      </div>
                    )}
                  </div>
                </div>
              )}

              {/* 远程配置（sse/http）*/}
              {(server.protocol === "sse" || server.protocol === "http") &&
                server.url && (
                  <div>
                    <h4 className="text-sm font-semibold mb-2">
                      {t("remote_config") || "远程配置"}
                    </h4>
                    <div className="text-sm">
                      <span className="text-muted-foreground">
                        {t("endpoint") || "端点"}:
                      </span>
                      <code className="ml-2 px-2 py-1 bg-muted rounded text-xs font-mono">
                        {server.url}
                      </code>
                    </div>
                  </div>
                )}

              {/* 环境变量 */}
              {server.env && Object.keys(server.env).length > 0 && (
                <div>
                  <h4 className="text-sm font-semibold mb-2">
                    {t("environment_variables") || "环境变量"} (
                    {Object.keys(server.env).length})
                  </h4>
                  <div className="space-y-1">
                    {Object.entries(server.env).map(([key, value]) => (
                      <div
                        key={key}
                        className="text-sm flex items-center gap-2"
                      >
                        <code className="px-2 py-1 bg-muted rounded text-xs font-mono">
                          {key}
                        </code>
                        <span className="text-muted-foreground">=</span>
                        <code className="px-2 py-1 bg-muted rounded text-xs font-mono">
                          {value.length > 20
                            ? value.substring(0, 20) + "***"
                            : value}
                        </code>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* HTTP Headers */}
              {server.headers && Object.keys(server.headers).length > 0 && (
                <div>
                  <h4 className="text-sm font-semibold mb-2">
                    HTTP Headers ({Object.keys(server.headers).length})
                  </h4>
                  <div className="space-y-1">
                    {Object.entries(server.headers).map(([key, value]) => (
                      <div
                        key={key}
                        className="text-sm flex items-center gap-2"
                      >
                        <code className="px-2 py-1 bg-muted rounded text-xs font-mono">
                          {key}
                        </code>
                        <span className="text-muted-foreground">:</span>
                        <code className="px-2 py-1 bg-muted rounded text-xs font-mono">
                          {value}
                        </code>
                      </div>
                    ))}
                  </div>
                </div>
              )}

              {/* 工具列表 */}
              <div>
                <h4 className="text-sm font-semibold mb-2">
                  {t("tools") || "工具列表"} ({tools?.length || 0})
                </h4>
                {isLoadingTools ? (
                  <div className="flex items-center justify-center py-4">
                    <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                  </div>
                ) : tools && tools.length > 0 ? (
                  <div className="flex flex-wrap gap-2">
                    {tools.map((tool) => (
                      <Tooltip key={tool.name}>
                        <TooltipTrigger asChild>
                          <Badge
                            variant="outline"
                            className="cursor-pointer hover:bg-accent/50 transition-colors px-3 py-1"
                            onClick={() => {
                              setSelectedTool(tool);
                              setToolDialogOpen(true);
                            }}
                          >
                            {tool.name}
                          </Badge>
                        </TooltipTrigger>
                        <TooltipContent>
                          <p>
                            {t("click_to_view_details") || "点击查看工具详情"}
                          </p>
                        </TooltipContent>
                      </Tooltip>
                    ))}
                  </div>
                ) : (
                  <p className="text-sm text-muted-foreground py-2">
                    {t("no_tools_available") || "暂无工具"}
                  </p>
                )}
              </div>
            </div>
          </TableCell>
        </TableRow>
      )}

      {/* 工具详情对话框 */}
      <ToolDetailDialog
        tool={selectedTool}
        open={toolDialogOpen}
        onOpenChange={setToolDialogOpen}
      />
    </>
  );
}
