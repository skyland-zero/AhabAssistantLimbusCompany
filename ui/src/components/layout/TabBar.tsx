import {
  CircleHelp,
  House,
  type LucideIcon,
  Moon,
  Package,
  Palette,
  Settings,
  Sun,
  Users,
  Wrench,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { type PageId, useAppStore } from "@/stores/appStore";

interface NavTab {
  id: PageId;
  labelKey: string;
  icon: LucideIcon;
}

const NAV_TABS: NavTab[] = [
  { id: "home", labelKey: "nav.home", icon: House },
  { id: "teams", labelKey: "nav.teams", icon: Users },
  { id: "themes", labelKey: "nav.themes", icon: Palette },
  { id: "toolbox", labelKey: "nav.toolbox", icon: Wrench },
  { id: "resources", labelKey: "nav.resources", icon: Package },
  { id: "help", labelKey: "nav.help", icon: CircleHelp },
];

export function TabBar() {
  const { t } = useTranslation();
  const currentPage = useAppStore((s) => s.currentPage);
  const setCurrentPage = useAppStore((s) => s.setCurrentPage);
  const themeMode = useAppStore((s) => s.themeMode);
  const setThemeMode = useAppStore((s) => s.setThemeMode);

  const toggleTheme = () => {
    if (themeMode === "light") setThemeMode("dark");
    else if (themeMode === "dark") setThemeMode("system");
    else setThemeMode("light");
  };

  return (
    <div className="flex h-10 shrink-0 items-center justify-between border-b border-border bg-card/60 px-3 backdrop-blur-sm">
      {/* 左侧主要页面标签 */}
      <nav className="flex items-center gap-1">
        {NAV_TABS.map((tab) => {
          const Icon = tab.icon;
          const isActive = currentPage === tab.id;
          return (
            <button
              key={tab.id}
              type="button"
              onClick={() => setCurrentPage(tab.id)}
              className={cn(
                "flex items-center gap-1.5 rounded-md px-3 py-1.5 text-xs font-medium transition-all duration-150",
                isActive
                  ? "bg-accent/15 text-brand font-semibold shadow-xs"
                  : "text-muted-foreground hover:bg-muted/60 hover:text-foreground",
              )}
            >
              <Icon className={cn("size-3.5", isActive && "text-brand")} />
              <span>{t(tab.labelKey)}</span>
            </button>
          );
        })}
      </nav>

      {/* 右侧快捷工具栏：主题切换 + 设置入口 */}
      <div className="flex items-center gap-1">
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="size-7 text-muted-foreground hover:text-foreground"
          onClick={toggleTheme}
          title={t("settings.themeMode")}
        >
          {themeMode === "dark" ? <Moon className="size-3.5" /> : <Sun className="size-3.5" />}
        </Button>

        <Button
          type="button"
          variant={currentPage === "settings" ? "secondary" : "ghost"}
          size="icon"
          className={cn(
            "size-7 text-muted-foreground hover:text-foreground",
            currentPage === "settings" && "text-brand font-semibold",
          )}
          onClick={() => setCurrentPage("settings")}
          title={t("nav.settings")}
        >
          <Settings className="size-3.5" />
        </Button>
      </div>
    </div>
  );
}
