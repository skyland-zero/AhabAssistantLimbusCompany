import type { ComponentType, LazyExoticComponent } from "react";
import { lazy, Suspense, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { TabBar } from "@/components/layout/TabBar";
import { TitleBar } from "@/components/layout/TitleBar";
import { Toaster } from "@/components/ui/sonner";
import type { PageId } from "@/stores/appStore";
import { useAppStore } from "@/stores/appStore";
import { applyTheme, watchSystemMode } from "@/themes";

// 页面按需加载，避免启动时同时解析所有页面及其依赖（尤其是帮助页的 Markdown 渲染器）。
const PAGES: Record<PageId, LazyExoticComponent<ComponentType>> = {
  home: lazy(() => import("@/pages/HomePage").then(({ HomePage }) => ({ default: HomePage }))),
  teams: lazy(() => import("@/pages/TeamsPage").then(({ TeamsPage }) => ({ default: TeamsPage }))),
  themes: lazy(() =>
    import("@/pages/ThemePacksPage").then(({ ThemePacksPage }) => ({ default: ThemePacksPage })),
  ),
  toolbox: lazy(() =>
    import("@/pages/ToolboxPage").then(({ ToolboxPage }) => ({ default: ToolboxPage })),
  ),
  resources: lazy(() =>
    import("@/pages/ResourcesPage").then(({ ResourcesPage }) => ({ default: ResourcesPage })),
  ),
  help: lazy(() => import("@/pages/HelpPage").then(({ HelpPage }) => ({ default: HelpPage }))),
  settings: lazy(() =>
    import("@/pages/SettingsPage").then(({ SettingsPage }) => ({ default: SettingsPage })),
  ),
};

export default function App() {
  const currentPage = useAppStore((s) => s.currentPage);
  const themeMode = useAppStore((s) => s.themeMode);
  const accentId = useAppStore((s) => s.accentId);
  const language = useAppStore((s) => s.language);
  const { i18n } = useTranslation();

  // 持久化的语言同步到 i18next
  useEffect(() => {
    if (i18n.language !== language) void i18n.changeLanguage(language);
  }, [language, i18n]);

  // 应用主题；跟随系统模式下监听系统变化
  useEffect(() => {
    applyTheme(themeMode, accentId);
    if (themeMode !== "system") return;
    return watchSystemMode(() => applyTheme("system", accentId));
  }, [themeMode, accentId]);

  return (
    <>
      <Toaster position="top-center" offset={80} />
      <div className="flex h-screen min-h-0 flex-col bg-background text-foreground">
        <TitleBar />
        <TabBar />
        <main
          key={currentPage}
          className="min-h-0 flex-1 animate-in overflow-hidden bg-background fade-in duration-200 slide-in-from-bottom-1"
        >
          <PageView key={currentPage} page={currentPage} />
        </main>
      </div>
    </>
  );
}

function PageView({ page }: { page: PageId }) {
  const Page = PAGES[page];
  return (
    <Suspense
      fallback={
        <div className="flex h-full items-center justify-center bg-background">
          <div className="size-5 animate-spin rounded-full border-2 border-muted border-t-brand" />
        </div>
      }
    >
      <Page />
    </Suspense>
  );
}
