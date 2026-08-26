import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { ThemeMode } from "@/themes";
import { DEFAULT_ACCENT } from "@/themes";

/** 左侧主导航页面 */
export type PageId = "home" | "teams" | "themes" | "toolbox" | "resources" | "help" | "settings";
interface AppState {
  /* 布局 */
  currentPage: PageId;
  sidebarCollapsed: boolean;

  /* 外观 */
  themeMode: ThemeMode;
  accentId: string;
  language: "zh-CN" | "en-US";

  /** 主控台右侧面板（截图+日志）收起状态 */
  rightPanelCollapsed: boolean;

  setCurrentPage: (page: PageId) => void;
  toggleSidebar: () => void;
  toggleRightPanel: () => void;
  setThemeMode: (mode: ThemeMode) => void;
  setAccent: (accentId: string) => void;
  setLanguage: (lang: "zh-CN" | "en-US") => void;
}

export const useAppStore = create<AppState>()(
  persist(
    (set) => ({
      currentPage: "home",
      sidebarCollapsed: false,
      rightPanelCollapsed: false,
      themeMode: "system",
      accentId: DEFAULT_ACCENT,
      language: "zh-CN",

      setCurrentPage: (currentPage) => set({ currentPage }),
      toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
      toggleRightPanel: () => set((s) => ({ rightPanelCollapsed: !s.rightPanelCollapsed })),
      setThemeMode: (themeMode) => set({ themeMode }),
      setAccent: (accentId) => set({ accentId }),
      setLanguage: (language) => set({ language }),
    }),
    {
      name: "ahab-ui-settings",
      partialize: (s) => ({
        sidebarCollapsed: s.sidebarCollapsed,
        rightPanelCollapsed: s.rightPanelCollapsed,
        themeMode: s.themeMode,
        accentId: s.accentId,
        language: s.language,
      }),
    },
  ),
);
