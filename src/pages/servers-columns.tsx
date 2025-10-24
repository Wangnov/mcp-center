import { ColumnDef } from "@tanstack/react-table";
import {
  ArrowUpDown,
  ChevronDown,
  ChevronRight,
  Edit,
  Eye,
  MoreHorizontal,
  Trash2,
} from "lucide-react";
import { McpServer } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Switch } from "@/components/ui/switch";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

export interface ServerColumnCallbacks {
  onToggle: (serverId: string) => void;
  onToggleEnabled: (server: McpServer, enabled: boolean) => void;
  onView: (server: McpServer) => void;
  onEdit: (server: McpServer) => void;
  onDelete: (server: McpServer) => void;
  isExpanded: (serverId: string) => boolean;
  isPending: (serverId: string) => boolean;
  formatTime: (timestamp?: number | null) => string;
  getProtocolBadgeVariant: (
    protocol: string,
  ) => "default" | "secondary" | "outline";
  t: (key: string) => string;
}

export function createColumns(
  callbacks: ServerColumnCallbacks,
): ColumnDef<McpServer>[] {
  const {
    onToggle,
    onToggleEnabled,
    onView,
    onEdit,
    onDelete,
    isExpanded,
    isPending,
    formatTime,
    getProtocolBadgeVariant,
    t,
  } = callbacks;

  return [
    {
      id: "expander",
      header: () => null,
      cell: ({ row }) => {
        const expanded = isExpanded(row.original.id);
        return (
          <Tooltip>
            <TooltipTrigger asChild>
              <button
                onClick={() => onToggle(row.original.id)}
                className="p-1 hover:bg-accent rounded-sm transition-colors"
              >
                {expanded ? (
                  <ChevronDown className="h-4 w-4" />
                ) : (
                  <ChevronRight className="h-4 w-4" />
                )}
              </button>
            </TooltipTrigger>
            <TooltipContent>
              <p>
                {expanded
                  ? t("collapse") || "收起详情"
                  : t("expand") || "展开详情"}
              </p>
            </TooltipContent>
          </Tooltip>
        );
      },
      size: 50,
    },
    {
      accessorKey: "name",
      header: ({ column }) => {
        return (
          <Button
            variant="ghost"
            onClick={() => column.toggleSorting(column.getIsSorted() === "asc")}
            className="h-8 px-2"
          >
            {t("name") || "名称"}
            <ArrowUpDown className="ml-2 h-4 w-4" />
          </Button>
        );
      },
      cell: ({ row }) => {
        return <div className="font-medium">{row.original.name}</div>;
      },
    },
    {
      accessorKey: "protocol",
      header: ({ column }) => {
        return (
          <Button
            variant="ghost"
            onClick={() => column.toggleSorting(column.getIsSorted() === "asc")}
            className="h-8 px-2"
          >
            {t("protocol") || "协议"}
            <ArrowUpDown className="ml-2 h-4 w-4" />
          </Button>
        );
      },
      cell: ({ row }) => {
        return (
          <Badge variant={getProtocolBadgeVariant(row.original.protocol)}>
            {row.original.protocol}
          </Badge>
        );
      },
    },
    {
      accessorKey: "toolCount",
      header: () => (
        <div className="text-right">{t("tool_count") || "工具数"}</div>
      ),
      cell: ({ row }) => {
        return (
          <div className="text-right tabular-nums">
            {row.original.toolCount}
          </div>
        );
      },
    },
    {
      accessorKey: "lastSeen",
      header: ({ column }) => {
        return (
          <Button
            variant="ghost"
            onClick={() => column.toggleSorting(column.getIsSorted() === "asc")}
            className="h-8 px-2"
          >
            {t("last_seen") || "最后使用"}
            <ArrowUpDown className="ml-2 h-4 w-4" />
          </Button>
        );
      },
      cell: ({ row }) => {
        return (
          <div className="text-muted-foreground text-sm">
            {formatTime(row.original.lastSeen)}
          </div>
        );
      },
    },
    {
      accessorKey: "enabled",
      header: () => <div>{t("status") || "状态"}</div>,
      cell: ({ row }) => {
        const server = row.original;
        return (
          <div className="flex items-center gap-2">
            <Switch
              checked={server.enabled}
              onCheckedChange={(enabled) => onToggleEnabled(server, enabled)}
              disabled={isPending(server.id)}
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
        );
      },
    },
    {
      id: "actions",
      header: () => <div className="text-right">{t("actions") || "操作"}</div>,
      cell: ({ row }) => {
        const server = row.original;
        return (
          <div className="text-right">
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" className="h-8 w-8 p-0">
                  <span className="sr-only">{t("actions") || "操作"}</span>
                  <MoreHorizontal className="h-4 w-4" />
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuLabel>{t("actions") || "操作"}</DropdownMenuLabel>
                <DropdownMenuItem onClick={() => onView(server)}>
                  <Eye className="mr-2 h-4 w-4" />
                  {t("view_details") || "查看详情"}
                </DropdownMenuItem>
                <DropdownMenuItem onClick={() => onEdit(server)}>
                  <Edit className="mr-2 h-4 w-4" />
                  {t("edit_server") || "编辑服务器"}
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem
                  onClick={() => onDelete(server)}
                  className="text-destructive focus:text-destructive"
                >
                  <Trash2 className="mr-2 h-4 w-4" />
                  {t("delete_server") || "删除服务器"}
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        );
      },
    },
  ];
}
