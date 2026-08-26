/**
 * 主题系统：明暗模式 + 强调色预设（参考 MXU）
 * accent 颜色以 CSS 变量注入（--brand / --brand-hover / --brand-light），
 * index.css 的 @theme inline 将其映射为 Tailwind 的 brand 色板。
 */

export type ThemeMode = "light" | "dark" | "system";

export interface AccentPreset {
  id: string;
  name: string;
  /** oklch 或 hex */
  light: { brand: string; brandHover: string; brandLight: string };
  dark: { brand: string; brandHover: string; brandLight: string };
}

export const ACCENT_PRESETS: AccentPreset[] = [
  {
    id: "crimson",
    name: "赤红",
    light: { brand: "#c8354f", brandHover: "#a92b42", brandLight: "#fbe9ec" },
    dark: { brand: "#e05a72", brandHover: "#c8354f", brandLight: "#3a1e24" },
  },
  {
    id: "blue",
    name: "深蓝",
    light: { brand: "#2563eb", brandHover: "#1d4ed8", brandLight: "#dbeafe" },
    dark: { brand: "#60a5fa", brandHover: "#3b82f6", brandLight: "#1e293b" },
  },
  {
    id: "amber",
    name: "琥珀",
    light: { brand: "#d97706", brandHover: "#b45309", brandLight: "#fef3c7" },
    dark: { brand: "#fbbf24", brandHover: "#f59e0b", brandLight: "#3b2f14" },
  },
  {
    id: "emerald",
    name: "翠绿",
    light: { brand: "#059669", brandHover: "#047857", brandLight: "#d1fae5" },
    dark: { brand: "#34d399", brandHover: "#10b981", brandLight: "#14332a" },
  },
  {
    id: "violet",
    name: "紫罗兰",
    light: { brand: "#7c3aed", brandHover: "#6d28d9", brandLight: "#ede9fe" },
    dark: { brand: "#a78bfa", brandHover: "#8b5cf6", brandLight: "#2b2247" },
  },
];

export const DEFAULT_ACCENT = "crimson";

function getAccent(id: string): AccentPreset {
  return ACCENT_PRESETS.find((a) => a.id === id) ?? ACCENT_PRESETS[0];
}

export function resolveSystemMode(): "light" | "dark" {
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

/** 应用主题到 documentElement */
export function applyTheme(mode: ThemeMode, accentId: string): void {
  const resolved = mode === "system" ? resolveSystemMode() : mode;
  const root = document.documentElement;
  root.classList.toggle("dark", resolved === "dark");

  const colors = getAccent(accentId)[resolved];
  root.style.setProperty("--brand", colors.brand);
  root.style.setProperty("--brand-hover", colors.brandHover);
  root.style.setProperty("--brand-light", colors.brandLight);
}

/** 监听系统主题变化，mode=system 时实时跟随 */
export function watchSystemMode(cb: () => void): () => void {
  const mq = window.matchMedia("(prefers-color-scheme: dark)");
  mq.addEventListener("change", cb);
  return () => mq.removeEventListener("change", cb);
}
