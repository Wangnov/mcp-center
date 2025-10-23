import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { getAppVersion } from "@/lib/api";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Globe } from "lucide-react";

const languages: Record<string, string> = {
  en: "English",
  "zh-CN": "简体中文",
  "zh-TW": "繁體中文",
  ja: "日本語",
};

export function SettingsPage() {
  const { t, i18n } = useTranslation();

  const { data: appVersion, isLoading } = useQuery<string, Error>({
    queryKey: ["appVersion"],
    queryFn: getAppVersion,
  });

  const currentLanguage = languages[i18n.language] || i18n.language;

  return (
    <div className="p-8 h-full">
      <header className="flex items-center justify-between mb-8">
        <h1 className="text-3xl font-bold">{t("settings")}</h1>
      </header>
      <div className="space-y-6">
        <div className="border rounded-lg p-6 bg-card">
          <h2 className="text-lg font-semibold mb-4">{t("language_label")}</h2>
          <div className="flex items-center space-x-4">
            <p className="text-muted-foreground">{t("language_desc")}</p>
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="outline">
                  <Globe className="mr-2 h-4 w-4" />
                  {currentLanguage}
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                {Object.entries(languages).map(([code, name]) => (
                  <DropdownMenuItem
                    key={code}
                    onSelect={() => i18n.changeLanguage(code)}
                  >
                    {name}
                  </DropdownMenuItem>
                ))}
              </DropdownMenuContent>
            </DropdownMenu>
          </div>
        </div>
        <div className="border rounded-lg p-6 bg-card">
          <h2 className="text-lg font-semibold mb-4">{t("about_label")}</h2>
          <div className="text-sm text-muted-foreground">
            <p>MCP Center</p>
            <p>
              {t("version_label")}:{" "}
              {isLoading ? t("loading") : appVersion || t("unknown_version")}
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
