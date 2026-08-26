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

  /** 主控台右侧面板宽度与收起状态（对齐 MXU rightPanelWidth） */
  rightPanelWidth: number;
  rightPanelCollapsed: boolean;

  setCurrentPage: (page: PageId) => void;
  toggleSidebar: () => void;
  setRightPanelWidth: (width: number) => void;
  setRightPanelCollapsed: (collapsed: boolean) => void;
  setThemeMode: (mode: ThemeMode) => void;
  setAccent: (accentId: string) => void;
  setLanguage: (lang: "zh-CN" | "en-US") => void;
}

export const useAppStore = create<AppState>()(
  persist(
    (set) => ({
      currentPage: "home",
      sidebarCollapsed: false,
      rightPanelWidth: 280,
      rightPanelCollapsed: false,
      themeMode: "system",
      accentId: DEFAULT_ACCENT,
      language: "zh-CN",

      setCurrentPage: (currentPage) => set({ currentPage }),
      toggleSidebar: () => set((s) => ({ sidebarCollapsed: !s.sidebarCollapsed })),
      setRightPanelWidth: (rightPanelWidth) => set({ rightPanelWidth }),
      setRightPanelCollapsed: (rightPanelCollapsed) => set({ rightPanelCollapsed }),
      setThemeMode: (themeMode) => set({ themeMode }),
      setAccent: (accentId) => set({ accentId }),
      setLanguage: (language) => set({ language }),
    }),
    {
      name: "ahab-ui-settings",
      partialize: (s) => ({
        sidebarCollapsed: s.sidebarCollapsed,
        rightPanelWidth: s.rightPanelWidth,
        rightPanelCollapsed: s.rightPanelCollapsed,
        themeMode: s.themeMode,
        accentId: s.accentId,
        language: s.language,
      }),
    },
  ),
);
