// The app's runtime theme: a skin's token table with an accent laid over it.
//
// `nsh.json` bootstraps Bettbox (`theme.json`); the user's skin and accent are
// applied with `setTheme()`, which replaces the theme wholesale — so the object
// built here carries the radius scale too. `skins.ts` is the table (38 tokens
// per skin: 34 colour + 4 decoration, plus what each accent changes), kept by
// hand since the GPUI palette it was exported from left the tree.
import { setTheme } from "nsh";
import type { Theme } from "nsh";
import {
  ACCENT_OVERRIDES,
  SKIN_DEFAULT_ACCENT,
  SKIN_RADII,
  SKIN_TOKENS,
} from "./skins";
import type { AccentName, SkinName } from "./skins";
import { seedTokens } from "./seed";

export { SKIN_DEFAULT_ACCENT };
export type { AccentName, SkinName };

/** The decoration language a skin paints on top of the flat surfaces. Renderers
 *  branch on this, never on the skin id, so a skin can change its colours
 *  without every component learning a new predicate. */
export type Decor = "plain" | "glass" | "archive" | "limbus" | "mist" | "hairline" | "gradient";

/** Which decoration each skin uses (the GPUI `SkinId::decor` mapping, plus
 *  `nsh`'s structure-only `hairline` and `bettbox`'s theme-colour gradient). */
export const DECOR_OF: Record<SkinName, Decor> = {
  default: "plain",
  glass: "glass",
  archive: "archive",
  limbus: "limbus",
  mist: "mist",
  nsh: "hairline",
  bettbox: "gradient",
};

/** The skins in the order the appearance page lists them: the flat ones first,
 *  then the decorated families. */
export const SKINS: SkinName[] = ["default", "nsh", "bettbox", "glass", "archive", "limbus", "mist"];

/** The accents in the order the appearance page lists them. */
export const ACCENTS: AccentName[] = [
  "crimson",
  "rose",
  "blue",
  "amber",
  "emerald",
  "violet",
  "limbus-brass",
];

/** Each skin's name, as the appearance page shows it. */
export const SKIN_LABELS: Record<SkinName, string> = {
  default: "现代",
  glass: "玻璃",
  archive: "档案",
  limbus: "边狱",
  mist: "血雾",
  nsh: "NSH",
  bettbox: "Bettbox",
};

/** Each skin's one-line description, as the appearance page shows it. */
export const SKIN_BLURBS: Record<SkinName, string> = {
  default: "扁平平铺 · 圆角 · 轻描边",
  glass: "分层半透 · 大圆角 · 真投影",
  archive: "纸墨发丝线 · 靠留白分层",
  limbus: "冷黑暗红 · 黄铜角框 · 直角",
  mist: "近黑血雾 · 暗角颗粒 · 锐角",
  nsh: "中性灰 · 发丝描边 · 零装饰",
  bettbox: "主题色渐变 · 白卡片 · 大圆角",
};

/** Each accent's name, as the appearance page shows it. */
export const ACCENT_LABELS: Record<AccentName, string> = {
  crimson: "赤红",
  rose: "玫红",
  blue: "深蓝",
  amber: "琥珀",
  emerald: "翠绿",
  violet: "紫罗兰",
  "limbus-brass": "边狱黄铜",
};

/** The window's own shadow: it is the app's to draw, since a transparent
 *  window has none of the system's. Not a skin's token — every skin floats the
 *  same way — and Bettbox's boot theme carries the same values in
 *  `theme.json`. */
const WINDOW_SHADOW: Record<"light" | "dark", string> = {
  light: "rgba(0, 0, 0, 0.30)",
  dark: "rgba(0, 0, 0, 0.55)",
};

/** The data-series colours of charts, in the design guide's `chart-1` …
 *  `chart-5` roles. Not a skin's tokens either: a series means the same colour
 *  in every skin, so the reader can carry a legend from one to another; the
 *  two appearances only differ in how bright each hue is. */
const CHART_TOKENS: Record<"light" | "dark", Record<string, string>> = {
  light: {
    "chart-1": "#c8354f",
    "chart-2": "#2f6fbf",
    "chart-3": "#1f8a5b",
    "chart-4": "#b6791b",
    "chart-5": "#6f5bbd",
  },
  dark: {
    "chart-1": "#e05a72",
    "chart-2": "#6fa8e6",
    "chart-3": "#4ec98a",
    "chart-4": "#e0b04a",
    "chart-5": "#a48ce6",
  },
};

/** One skin's colour tokens for one appearance, with `accent` laid over them and
 *  a colour theme laid over both.
 *
 *  `setTheme` reads these as the running theme; the skin picker also reads them
 *  directly, because it draws a miniature of every other skin and those cannot
 *  come from the active theme's classes. */
export function tokensFor(
  skin: SkinName,
  accent: AccentName,
  appearance: "light" | "dark",
  seed?: string | null,
): Record<string, string> {
  const base = SKIN_TOKENS[skin][appearance];
  const override =
    accent === SKIN_DEFAULT_ACCENT[skin] ? undefined : ACCENT_OVERRIDES[skin][accent];
  const tokens = {
    ...base,
    "window-shadow": WINDOW_SHADOW[appearance],
    ...CHART_TOKENS[appearance],
    ...(override?.[appearance] ?? {}),
  };
  // The colour theme comes last: the user's own pick outranks both the skin's
  // palette and the accent.
  return seed ? { ...tokens, ...seedTokens(seed, appearance) } : tokens;
}

/** The theme object for one skin, accent and colour theme, in both appearances. */
export function themeFor(skin: SkinName, accent: AccentName, seed?: string | null): Theme {
  return {
    cssVars: {
      theme: { ...SKIN_RADII[skin] },
      light: tokensFor(skin, accent, "light", seed),
      dark: tokensFor(skin, accent, "dark", seed),
    },
  };
}

/** Makes `skin` + `accent` + `seed` the running theme; every token class and
 *  `token()` read follows on the next frame. */
export function applyTheme(skin: SkinName, accent: AccentName, seed?: string | null): void {
  setTheme(themeFor(skin, accent, seed));
}

export { SEED_PRESETS, isSeedHex, normaliseSeed, seedTokens } from "./seed";
