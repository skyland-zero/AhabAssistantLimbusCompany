import {
  House,
  type LucideIcon,
  Package,
  Palette,
  PanelLeftClose,
  PanelLeftOpen,
  Settings,
  Users,
  Wrench,
} from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";
import { type PageId, useAppStore } from "@/stores/appStore";

interface NavItem {
  id: PageId;
  labelKey: string;
  icon: LucideIcon;
}

const NAV_ITEMS: NavItem[] = [
  { id: "home", labelKey: "nav.home", icon: House },
  { id: "teams", labelKey: "nav.teams", icon: Users },
  { id: "themes", labelKey: "nav.themes", icon: Palette },
  { id: "toolbox", labelKey: "nav.toolbox", icon: Wrench },
  { id: "resources", labelKey: "nav.resources", icon: Package },
];

export function SideNav() {
  const { t } = useTranslation();
  const currentPage = useAppStore((s) => s.currentPage);
  const setCurrentPage = useAppStore((s) => s.setCurrentPage);
  const collapsed = useAppStore((s) => s.sidebarCollapsed);
  const toggleSidebar = useAppStore((s) => s.toggleSidebar);

  return (
    <TooltipProvider delayDuration={200}>
      <nav
        className={cn(
          "flex shrink-0 flex-col border-r border-border bg-card transition-[width] duration-150",
          collapsed ? "w-16" : "w-44",
        )}
      >
        <div className="flex flex-1 flex-col gap-1 p-2">
          {NAV_ITEMS.map((item) => (
            <NavButton
              key={item.id}
              item={item}
              active={currentPage === item.id}
              collapsed={collapsed}
              onSelect={() => setCurrentPage(item.id)}
            />
          ))}
        </div>

        {/* 底部：设置 + 折叠开关 */}
        <div className="flex flex-col gap-1 p-2">
          <NavButton
            item={{ id: "settings", labelKey: "nav.settings", icon: Settings }}
            active={currentPage === "settings"}
            collapsed={collapsed}
            onSelect={() => setCurrentPage("settings")}
          />
          <Button
            variant="ghost"
            size="sm"
            className="justify-start gap-3 text-muted-foreground"
            onClick={toggleSidebar}
          >
            {collapsed ? (
              <PanelLeftOpen className="size-4 shrink-0" aria-label={t("nav.expand")} />
            ) : (
              <>
                <PanelLeftClose className="size-4 shrink-0" />
                <span>{t("nav.collapse")}</span>
              </>
            )}
          </Button>
        </div>
      </nav>
    </TooltipProvider>
  );
}

function NavButton({
  item,
  active,
  collapsed,
  onSelect,
}: {
  item: NavItem;
  active: boolean;
  collapsed: boolean;
  onSelect: () => void;
}) {
  const { t } = useTranslation();
  const Icon = item.icon;

  const button = (
    <Button
      variant="ghost"
      size="sm"
      onClick={onSelect}
      aria-current={active ? "page" : undefined}
      className={cn(
        "h-9 justify-start gap-3",
        active
          ? "bg-brand-light text-brand hover:bg-brand-light hover:text-brand"
          : "text-muted-foreground hover:text-foreground",
        collapsed && "justify-center px-0",
      )}
    >
      <Icon className="size-4 shrink-0" />
      {!collapsed && <span>{t(item.labelKey)}</span>}
    </Button>
  );

  if (!collapsed) return button;
  return (
    <Tooltip>
      <TooltipTrigger asChild>{button}</TooltipTrigger>
      <TooltipContent side="right">{t(item.labelKey)}</TooltipContent>
    </Tooltip>
  );
}
