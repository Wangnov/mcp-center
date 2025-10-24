import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Circle } from "lucide-react";
import { getHealth, type HealthResponse } from "@/lib/api";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

type HealthStatus = "healthy" | "warning" | "error";

interface StatusConfig {
  color: string;
  iconColor: string;
  animation: string;
  label: string;
}

const STATUS_CONFIG: Record<HealthStatus, StatusConfig> = {
  healthy: {
    color: "bg-green-500",
    iconColor: "text-green-500",
    animation: "animate-health-breathing",
    label: "正常",
  },
  warning: {
    color: "bg-yellow-500",
    iconColor: "text-yellow-500",
    animation: "animate-health-warning",
    label: "警告",
  },
  error: {
    color: "bg-red-500",
    iconColor: "text-red-500",
    animation: "animate-health-error",
    label: "离线",
  },
};

export function HealthStatusBadge() {
  const { t } = useTranslation();

  const { data, isError, isPending, isFetching, error } =
    useQuery<HealthResponse>({
      queryKey: ["server-health"],
      queryFn: async () => {
        const result = await getHealth();
        if (!result) {
          throw new Error(t("health_check_failed", { defaultValue: "Health check failed" }));
        }
        return result;
      },
      // 每 5 秒轮询一次
      refetchInterval: 5000,
      // 页面不可见时停止轮询
      refetchIntervalInBackground: false,
      // 失败重试 3 次
      retry: 3,
      // 数据立即过期，确保及时更新
      staleTime: 0,
    });

  // 确定健康状态
  const status: HealthStatus = isError
    ? "error"
    : data?.status === "ok"
      ? "healthy"
      : "warning";

  const config = STATUS_CONFIG[status];

  if (isPending) {
    return (
      <Tooltip>
        <TooltipTrigger asChild>
          <div className="grid h-10 w-10 place-items-center rounded-lg">
            <span className="inline-block size-3 rounded-full bg-muted animate-pulse" />
          </div>
        </TooltipTrigger>
        <TooltipContent side="right">
          <p>{t("checking", { defaultValue: "检查中..." })}</p>
        </TooltipContent>
      </Tooltip>
    );
  }

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          className="grid h-10 w-10 place-items-center rounded-lg hover:bg-accent/50 transition-colors cursor-help"
          aria-label={`${t("server_status")}: ${config.label}`}
        >
          <span
            className={`
              inline-block size-3 rounded-full
              ${config.color}
              ${config.animation}
              motion-reduce:animate-none
              ${isFetching ? "opacity-70" : ""}
            `}
          />
        </button>
      </TooltipTrigger>

      <TooltipContent side="right" className="max-w-xs">
        <div className="space-y-2">
          <div className="flex items-center gap-2">
            <Circle className={`size-4 fill-current ${config.iconColor}`} />
            <p className="font-semibold">
              {t("server_status", { defaultValue: "服务器状态" })}: {config.label}
            </p>
          </div>

          {data && (
            <div className="text-xs text-muted-foreground">
              <p>
                {t("last_check", { defaultValue: "最后检查" })}:{" "}
                {new Date().toLocaleTimeString()}
              </p>
            </div>
          )}

          {isError && error && (
            <div className="text-xs text-destructive">
              <p>
                {t("error", { defaultValue: "错误" })}: {error.message}
              </p>
            </div>
          )}

          <div className="pt-2 border-t space-y-1 text-xs text-muted-foreground">
            <div className="flex items-center gap-1.5">
              <Circle className="size-3 fill-current text-green-500" />
              <span>{t("health_all_ok", { defaultValue: "所有服务正常" })}</span>
            </div>
            <div className="flex items-center gap-1.5">
              <Circle className="size-3 fill-current text-yellow-500" />
              <span>{t("health_degraded", { defaultValue: "部分服务降级" })}</span>
            </div>
            <div className="flex items-center gap-1.5">
              <Circle className="size-3 fill-current text-red-500" />
              <span>{t("health_unavailable", { defaultValue: "服务不可用" })}</span>
            </div>
          </div>
        </div>
      </TooltipContent>
    </Tooltip>
  );
}
