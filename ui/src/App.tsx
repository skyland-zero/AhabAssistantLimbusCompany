import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { TabBar } from "@/components/layout/TabBar";
import { TitleBar } from "@/components/layout/TitleBar";
import { Toaster } from "@/components/ui/sonner";
import { HelpPage } from "@/pages/HelpPage";
import { HomePage } from "@/pages/HomePage";
import { ResourcesPage } from "@/pages/ResourcesPage";
import { SettingsPage } from "@/pages/SettingsPage";
import { TeamsPage } from "@/pages/TeamsPage";
import { ThemePacksPage } from "@/pages/ThemePacksPage";
import { ToolboxPage } from "@/pages/ToolboxPage";
import { useAppStore } from "@/stores/appStore";
import { applyTheme, watchSystemMode } from "@/themes";

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

function PageView({ page }: { page: ReturnType<typeof useAppStore.getState>["currentPage"] }) {
  switch (page) {
    case "home":
      return <HomePage />;
    case "teams":
      return <TeamsPage />;
    case "themes":
      return <ThemePacksPage />;
    case "toolbox":
      return <ToolboxPage />;
    case "resources":
      return <ResourcesPage />;
    case "help":
      return <HelpPage />;
    case "settings":
      return <SettingsPage />;
  }
}
