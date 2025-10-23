import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { getMcpServerDetail, McpServer, type ToolInfo } from "@/lib/api";
import {
  Drawer,
  DrawerClose,
  DrawerContent,
  DrawerDescription,
  DrawerFooter,
  DrawerHeader,
  DrawerTitle,
} from "@/components/ui/drawer";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import type { BadgeVariant } from "@/components/ui/badge-variants";
import { ToolDetailDialog } from "./ToolDetailDialog";
import {
  Server as ServerIcon,
  X,
  Terminal,
  Globe,
  Key,
  Package,
  Loader2,
} from "lucide-react";

interface ServerDetailDrawerProps {
  server: McpServer | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onEdit?: (server: McpServer) => void;
  onDelete?: (server: McpServer) => void;
}

export function ServerDetailDrawer({
  server,
  open,
  onOpenChange,
  onEdit,
  onDelete,
}: ServerDetailDrawerProps) {
  const { t } = useTranslation();

  // 工具详情对话框状态
  const [selectedTool, setSelectedTool] = useState<ToolInfo | null>(null);
  const [toolDialogOpen, setToolDialogOpen] = useState(false);

  // 使用 getMcpServerDetail 获取完整的服务器详情和工具列表
  const {
    data: serverDetail,
    refetch: refetchDetail,
    isLoading: isLoadingDetail,
  } = useQuery({
    queryKey: ["server-detail", server?.id],
    queryFn: () =>
      server ? getMcpServerDetail(server.id) : Promise.resolve(null),
    enabled: !!server && open,
    staleTime: 0, // 数据立即过期，每次都重新获取
    gcTime: 0, // 不缓存数据（React Query v5 使用 gcTime 替代 cacheTime）
  });

  // 当 drawer 打开且 server 变化时，重新获取详情
  useEffect(() => {
    if (open && server) {
      refetchDetail();
    }
  }, [open, server, refetchDetail]);

  if (!server) return null;

  const serverData: McpServer = serverDetail?.server ?? server;
  const tools = serverDetail?.tools || [];

  // 兼容不同的字段命名：列表用 toolCount，详情用 tool_count
  const toolCount = serverData.tool_count ?? serverData.toolCount ?? 0;

  // 兼容时间戳字段的不同命名
  const createdAt = serverData.createdAt ?? serverData.created_at;
  const lastSeen = serverData.lastSeen ?? serverData.last_seen;

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
  const formatTimestamp = (timestamp?: number) => {
    if (!timestamp) return "-";
    return new Date(timestamp * 1000).toLocaleString();
  };

  return (
    <Drawer open={open} onOpenChange={onOpenChange}>
      <DrawerContent className="h-[90vh]">
        <div className="mx-auto w-full max-w-4xl h-full flex flex-col">
          <DrawerHeader className="border-b">
            <div className="flex items-start justify-between">
              <div className="flex items-center gap-3">
                <div className="p-2 rounded-lg bg-primary/10">
                  <ServerIcon className="h-6 w-6 text-primary" />
                </div>
                <div>
                  <DrawerTitle className="text-2xl">
                    {serverData.name}
                  </DrawerTitle>
                  <DrawerDescription className="flex items-center gap-2 mt-1">
                    <Badge
                      variant={getProtocolBadgeVariant(serverData.protocol)}
                    >
                      {serverData.protocol}
                    </Badge>
                    <span className="text-xs font-mono text-muted-foreground">
                      {serverData.id}
                    </span>
                  </DrawerDescription>
                </div>
              </div>
              <DrawerClose asChild>
                <Button variant="ghost" size="icon">
                  <X className="h-4 w-4" />
                </Button>
              </DrawerClose>
            </div>
          </DrawerHeader>

          {/* 内容区域 - 可滚动 */}
          <div className="flex-1 overflow-y-auto p-6">
            {isLoadingDetail ? (
              <div className="flex items-center justify-center py-12">
                <Loader2 className="h-8 w-8 animate-spin text-muted-foreground" />
              </div>
            ) : (
              <div className="space-y-6">
                {/* 基本信息 */}
                <section>
                  <h3 className="text-lg font-semibold mb-3 flex items-center gap-2">
                    <Package className="h-5 w-5" />
                    {t("basic_info") || "基本信息"}
                  </h3>
                  <div className="grid grid-cols-2 gap-4 p-4 rounded-lg bg-muted/30">
                    <div>
                      <dt className="text-sm text-muted-foreground mb-1">
                        {t("server_id") || "服务器 ID"}
                      </dt>
                      <dd className="text-sm font-mono">{serverData.id}</dd>
                    </div>
                    <div>
                      <dt className="text-sm text-muted-foreground mb-1">
                        {t("status") || "状态"}
                      </dt>
                      <dd>
                        {serverData.enabled ? (
                          <Badge className="bg-green-500/10 text-green-600 border-green-500/20">
                            ● {t("enabled") || "已启用"}
                          </Badge>
                        ) : (
                          <Badge variant="secondary">
                            {t("disabled") || "已禁用"}
                          </Badge>
                        )}
                      </dd>
                    </div>
                    <div>
                      <dt className="text-sm text-muted-foreground mb-1">
                        {t("tool_count") || "工具数量"}
                      </dt>
                      <dd className="text-sm font-semibold">{toolCount}</dd>
                    </div>
                    <div>
                      <dt className="text-sm text-muted-foreground mb-1">
                        {t("created_at") || "创建时间"}
                      </dt>
                      <dd className="text-sm">{formatTimestamp(createdAt)}</dd>
                    </div>
                    {lastSeen && (
                      <div>
                        <dt className="text-sm text-muted-foreground mb-1">
                          {t("last_seen") || "最后使用"}
                        </dt>
                        <dd className="text-sm">{formatTimestamp(lastSeen)}</dd>
                      </div>
                    )}
                  </div>
                </section>

                {/* 启动配置 (stdio) */}
                {serverData.protocol === "stdio" && serverData.command && (
                  <section>
                    <h3 className="text-lg font-semibold mb-3 flex items-center gap-2">
                      <Terminal className="h-5 w-5" />
                      {t("startup_config") || "启动配置"}
                    </h3>
                    <div className="space-y-3 p-4 rounded-lg bg-muted/30">
                      <div>
                        <span className="text-sm text-muted-foreground">
                          {t("command") || "命令"}:
                        </span>
                        <code className="block mt-1 px-3 py-2 bg-muted rounded text-sm font-mono">
                          {serverData.command}
                        </code>
                      </div>
                      {serverData.args && serverData.args.length > 0 && (
                        <div>
                          <span className="text-sm text-muted-foreground">
                            {t("arguments") || "参数"}:
                          </span>
                          <code className="block mt-1 px-3 py-2 bg-muted rounded text-sm font-mono">
                            {serverData.args.join(" ")}
                          </code>
                        </div>
                      )}
                    </div>
                  </section>
                )}

                {/* 远程配置 (sse/http) */}
                {(serverData.protocol === "sse" ||
                  serverData.protocol === "http") &&
                  serverData.url && (
                    <section>
                      <h3 className="text-lg font-semibold mb-3 flex items-center gap-2">
                        <Globe className="h-5 w-5" />
                        {t("remote_config") || "远程配置"}
                      </h3>
                      <div className="p-4 rounded-lg bg-muted/30">
                        <span className="text-sm text-muted-foreground">
                          {t("endpoint") || "端点"}:
                        </span>
                        <code className="block mt-1 px-3 py-2 bg-muted rounded text-sm font-mono break-all">
                          {serverData.url}
                        </code>
                      </div>
                    </section>
                  )}

                {/* 环境变量 */}
                {serverData.env && Object.keys(serverData.env).length > 0 && (
                  <section>
                    <h3 className="text-lg font-semibold mb-3 flex items-center gap-2">
                      <Key className="h-5 w-5" />
                      {t("environment_variables") || "环境变量"} (
                      {Object.keys(serverData.env).length})
                    </h3>
                    <div className="space-y-2 p-4 rounded-lg bg-muted/30">
                      {Object.entries(serverData.env).map(([key, value]) => (
                        <div key={key} className="flex items-center gap-2">
                          <code className="px-2 py-1 bg-muted rounded text-xs font-mono">
                            {key}
                          </code>
                          <span className="text-muted-foreground">=</span>
                          <code className="px-2 py-1 bg-muted rounded text-xs font-mono flex-1 truncate">
                            {value.length > 30
                              ? value.substring(0, 30) + "..."
                              : value}
                          </code>
                        </div>
                      ))}
                    </div>
                  </section>
                )}

                {/* HTTP Headers */}
                {serverData.headers &&
                  Object.keys(serverData.headers).length > 0 && (
                    <section>
                      <h3 className="text-lg font-semibold mb-3">
                        HTTP Headers ({Object.keys(serverData.headers).length})
                      </h3>
                      <div className="space-y-2 p-4 rounded-lg bg-muted/30">
                        {Object.entries(serverData.headers).map(
                          ([key, value]) => (
                            <div key={key} className="flex items-center gap-2">
                              <code className="px-2 py-1 bg-muted rounded text-xs font-mono">
                                {key}
                              </code>
                              <span className="text-muted-foreground">:</span>
                              <code className="px-2 py-1 bg-muted rounded text-xs font-mono flex-1 truncate">
                                {value}
                              </code>
                            </div>
                          ),
                        )}
                      </div>
                    </section>
                  )}

                {/* 工具列表 */}
                <section>
                  <h3 className="text-lg font-semibold mb-3">
                    {t("tools") || "工具列表"} ({tools.length})
                  </h3>
                  {tools.length > 0 ? (
                    <div className="space-y-2">
                      {tools.map((tool) => (
                        <button
                          key={tool.name}
                          onClick={() => {
                            setSelectedTool(tool);
                            setToolDialogOpen(true);
                          }}
                          className="w-full p-3 rounded-lg border bg-card hover:bg-accent/50 transition-colors text-left cursor-pointer"
                        >
                          <div className="flex items-start justify-between gap-2">
                            <div className="flex-1 min-w-0">
                              <h4 className="text-sm font-mono font-medium truncate">
                                {tool.name}
                              </h4>
                              {tool.description && (
                                <p className="text-xs text-muted-foreground mt-1 line-clamp-2">
                                  {tool.description}
                                </p>
                              )}
                            </div>
                            <Package className="h-4 w-4 text-muted-foreground flex-shrink-0" />
                          </div>
                        </button>
                      ))}
                    </div>
                  ) : (
                    <div className="p-8 text-center text-muted-foreground">
                      <Package className="h-12 w-12 mx-auto mb-2 opacity-50" />
                      <p className="text-sm">
                        {t("no_tools_available") || "暂无工具"}
                      </p>
                    </div>
                  )}
                </section>
              </div>
            )}
          </div>

          {/* 底部操作按钮 */}
          <DrawerFooter className="border-t">
            <div className="flex gap-2">
              {onEdit && (
                <Button
                  variant="default"
                  onClick={() => {
                    onEdit(server);
                    onOpenChange(false);
                  }}
                  className="flex-1"
                >
                  {t("edit_server") || "编辑服务器"}
                </Button>
              )}
              {onDelete && (
                <Button
                  variant="destructive"
                  onClick={() => {
                    onDelete(server);
                    onOpenChange(false);
                  }}
                  className="flex-1"
                >
                  {t("delete_server") || "删除服务器"}
                </Button>
              )}
              <DrawerClose asChild>
                <Button variant="outline" className="flex-1">
                  {t("close") || "关闭"}
                </Button>
              </DrawerClose>
            </div>
          </DrawerFooter>
        </div>
      </DrawerContent>

      {/* 工具详情对话框 */}
      <ToolDetailDialog
        tool={selectedTool}
        open={toolDialogOpen}
        onOpenChange={setToolDialogOpen}
      />
    </Drawer>
  );
}
