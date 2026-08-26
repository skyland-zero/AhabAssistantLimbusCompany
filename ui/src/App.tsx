import { useEffect } from "react";
import { useTranslation } from "react-i18next";
import { SideNav } from "@/components/layout/SideNav";
import { TitleBar } from "@/components/layout/TitleBar";
import { HomePage } from "@/pages/HomePage";
import { PlaceholderPage } from "@/pages/PlaceholderPage";
import { SettingsPage } from "@/pages/SettingsPage";
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
    <div className="flex h-screen min-h-0 flex-col">
      <TitleBar />
      <div className="flex min-h-0 flex-1">
        <SideNav />
        <main className="min-w-0 flex-1 overflow-hidden bg-background">
          <PageView page={currentPage} />
        </main>
      </div>
    </div>
  );
}

function PageView({ page }: { page: ReturnType<typeof useAppStore.getState>["currentPage"] }) {
  switch (page) {
    case "home":
      return <HomePage />;
    case "teams":
      return <PlaceholderPage pageKey="teams" />;
    case "themes":
      return <PlaceholderPage pageKey="themes" />;
    case "toolbox":
      return <PlaceholderPage pageKey="toolbox" />;
    case "resources":
      return <PlaceholderPage pageKey="resources" />;
    case "settings":
      return <SettingsPage />;
  }
}
