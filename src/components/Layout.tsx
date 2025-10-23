import type { ComponentType } from "react";
import { useTranslation } from "react-i18next";
import { Server, FolderKanban, Settings } from "lucide-react";
import { NavLink, Outlet } from "react-router-dom";
import { McpCenterLogo } from "@/components/icons/McpCenterLogo";
import { HealthStatusBadge } from "@/components/HealthStatusBadge";
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from "@/components/ui/tooltip";

type NavItemProps = {
  to: string;
  icon: ComponentType<{ className?: string }>;
  label: string;
};

const NavItem = ({ to, icon: Icon, label }: NavItemProps) => (
  <Tooltip>
    <TooltipTrigger asChild>
      <NavLink
        to={to}
        className={({ isActive }) =>
          [
            "grid h-10 w-10 place-items-center rounded-lg text-muted-foreground transition-colors hover:text-foreground",
            isActive ? "bg-accent text-accent-foreground" : "",
          ]
            .filter(Boolean)
            .join(" ")
        }
      >
        <Icon className="h-5 w-5" />
        <span className="sr-only">{label}</span>
      </NavLink>
    </TooltipTrigger>
    <TooltipContent side="right">{label}</TooltipContent>
  </Tooltip>
);

export function Layout() {
  const { t } = useTranslation();

  return (
    <TooltipProvider delayDuration={0}>
      <div className="bg-background text-foreground min-h-screen flex">
        <aside className="fixed inset-y-0 left-0 z-10 flex h-full w-16 flex-col border-r bg-muted/40">
          <div className="flex flex-1 flex-col items-center justify-between py-6">
            <nav className="flex flex-col items-center gap-4">
              <div className="grid h-12 w-12 place-items-center rounded-full bg-primary text-primary-foreground">
                <McpCenterLogo className="h-10 w-10" />
                <span className="sr-only">MCP Center</span>
              </div>
              <div className="flex flex-col items-center gap-3">
                <NavItem to="/mcp" icon={Server} label={t("mcp_servers")} />
                <NavItem
                  to="/project"
                  icon={FolderKanban}
                  label={t("projects")}
                />
              </div>
            </nav>
            <nav className="flex flex-col items-center gap-3">
              <HealthStatusBadge />
              <NavItem to="/settings" icon={Settings} label={t("settings")} />
            </nav>
          </div>
        </aside>
        <main className="flex-1 pl-16 flex flex-col grid-background">
          <Outlet />
        </main>
      </div>
    </TooltipProvider>
  );
}
