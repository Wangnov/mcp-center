import type { FallbackProps } from "react-error-boundary";
import { useTranslation } from "react-i18next";
import { AlertTriangle } from "lucide-react";
import { Button } from "@/components/ui/button";

export function ErrorFallback({ error, resetErrorBoundary }: FallbackProps) {
  const { t } = useTranslation();

  const title = t("app_error_title", {
    defaultValue: "应用运行出现异常",
  });
  const description = t("app_error_description", {
    defaultValue: "我们已记录错误，你可以重试或刷新页面。",
  });
  const retryLabel = t("app_error_retry", { defaultValue: "重试" });
  const refreshLabel = t("app_error_refresh", { defaultValue: "刷新页面" });

  return (
    <div className="bg-background text-foreground min-h-screen flex items-center justify-center px-6">
      <div className="w-full max-w-lg space-y-6 text-center">
        <div className="flex flex-col items-center gap-4">
          <span className="grid h-16 w-16 place-items-center rounded-full bg-destructive/10 text-destructive">
            <AlertTriangle className="h-8 w-8" aria-hidden />
          </span>
          <div className="space-y-2">
            <h1 className="text-2xl font-semibold tracking-tight">{title}</h1>
            <p className="text-muted-foreground text-sm leading-relaxed">
              {description}
            </p>
          </div>
        </div>
        {error?.message ? (
          <pre className="overflow-x-auto rounded-lg border border-border bg-muted/60 p-4 text-left text-sm text-muted-foreground">
            {error.message}
          </pre>
        ) : null}
        <div className="flex flex-col gap-3 sm:flex-row sm:justify-center">
          <Button onClick={resetErrorBoundary}>{retryLabel}</Button>
          <Button variant="outline" onClick={() => window.location.reload()}>
            {refreshLabel}
          </Button>
        </div>
      </div>
    </div>
  );
}
