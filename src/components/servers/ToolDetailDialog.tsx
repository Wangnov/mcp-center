import { useTranslation } from "react-i18next";
import { ToolInfo } from "@/lib/api";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Badge } from "@/components/ui/badge";
import { Package, Server as ServerIcon } from "lucide-react";

interface ToolDetailDialogProps {
  tool: ToolInfo | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function ToolDetailDialog({
  tool,
  open,
  onOpenChange,
}: ToolDetailDialogProps) {
  const { t } = useTranslation();

  if (!tool) return null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl max-h-[80vh] overflow-y-auto">
        <DialogHeader>
          <div className="flex items-start gap-3">
            <div className="p-2 rounded-lg bg-primary/10 mt-1">
              <Package className="h-5 w-5 text-primary" />
            </div>
            <div className="flex-1 min-w-0">
              <DialogTitle className="text-xl font-mono break-all">
                {tool.name}
              </DialogTitle>
              <DialogDescription className="flex items-center gap-2 mt-1">
                <ServerIcon className="h-3 w-3" />
                <span>{tool.serverName}</span>
              </DialogDescription>
            </div>
          </div>
        </DialogHeader>

        <div className="space-y-4 mt-4">
          {/* 工具名称 */}
          <div>
            <h4 className="text-sm font-semibold mb-2 text-muted-foreground">
              {t("tool_name", { defaultValue: "工具名称" })}
            </h4>
            <code className="block px-3 py-2 bg-muted rounded text-sm font-mono break-all">
              {tool.name}
            </code>
          </div>

          {/* 所属服务器 */}
          <div>
            <h4 className="text-sm font-semibold mb-2 text-muted-foreground">
              {t("belongs_to_server", { defaultValue: "所属服务器" })}
            </h4>
            <div className="flex items-center gap-2">
              <Badge variant="secondary">{tool.serverName}</Badge>
              <code className="text-xs text-muted-foreground font-mono">
                ({tool.serverId})
              </code>
            </div>
          </div>

          {/* 工具描述 */}
          <div>
            <h4 className="text-sm font-semibold mb-2 text-muted-foreground">
              {t("tool_description", { defaultValue: "工具描述" })}
            </h4>
            {tool.description ? (
              <div className="px-3 py-2 bg-muted/50 rounded text-sm whitespace-pre-wrap break-words">
                {tool.description}
              </div>
            ) : (
              <p className="text-sm text-muted-foreground italic">
                {t("no_description", { defaultValue: "暂无描述" })}
              </p>
            )}
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
