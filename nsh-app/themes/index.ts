// The app's runtime theme: a skin's token table with an accent laid over it.
//
// `nsh.json` bootstraps the default skin; the user's skin and accent are
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

export { SKIN_DEFAULT_ACCENT };
export type { AccentName, SkinName };

/** The decoration language a skin paints on top of the flat surfaces. Renderers
 *  branch on this, never on the skin id, so a skin can change its colours
 *  without every component learning a new predicate. */
export type Decor = "plain" | "glass" | "archive" | "limbus" | "mist" | "hairline";

/** Which decoration each skin uses (the GPUI `SkinId::decor` mapping, plus
 *  `nsh`'s structure-only `hairline`). */
export const DECOR_OF: Record<SkinName, Decor> = {
  default: "plain",
  glass: "glass",
  archive: "archive",
  limbus: "limbus",
  mist: "mist",
  nsh: "hairline",
};

/** The skins in the order the appearance page lists them: the two flat ones
 *  first, then the decorated families. */
export const SKINS: SkinName[] = ["default", "nsh", "glass", "archive", "limbus", "mist"];

/** The accents in the order the appearance page lists them. */
export const ACCENTS: AccentName[] = [
  "crimson",
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
};

/** Each skin's one-line description, as the appearance page shows it. */
export const SKIN_BLURBS: Record<SkinName, string> = {
  default: "扁平平铺 · 圆角 · 轻描边",
  glass: "分层半透 · 大圆角 · 真投影",
  archive: "纸墨发丝线 · 靠留白分层",
  limbus: "冷黑暗红 · 黄铜角框 · 直角",
  mist: "近黑血雾 · 暗角颗粒 · 锐角",
  nsh: "中性灰 · 发丝描边 · 零装饰",
};

/** Each accent's name, as the appearance page shows it. */
export const ACCENT_LABELS: Record<AccentName, string> = {
  crimson: "赤红",
  blue: "深蓝",
  amber: "琥珀",
  emerald: "翠绿",
  violet: "紫罗兰",
  "limbus-brass": "边狱黄铜",
};

/** One skin's colour tokens for one appearance, with `accent` laid over them.
 *
 *  `setTheme` reads these as the running theme; the skin picker also reads them
 *  directly, because it draws a miniature of every other skin and those cannot
 *  come from the active theme's classes. */
export function tokensFor(
  skin: SkinName,
  accent: AccentName,
  appearance: "light" | "dark",
): Record<string, string> {
  const base = SKIN_TOKENS[skin][appearance];
  const override =
    accent === SKIN_DEFAULT_ACCENT[skin] ? undefined : ACCENT_OVERRIDES[skin][accent];
  return { ...base, ...(override?.[appearance] ?? {}) };
}

/** The theme object for one skin and accent, in both appearances. */
export function themeFor(skin: SkinName, accent: AccentName): Theme {
  return {
    cssVars: {
      theme: { ...SKIN_RADII[skin] },
      light: tokensFor(skin, accent, "light"),
      dark: tokensFor(skin, accent, "dark"),
    },
  };
}

/** Makes `skin` + `accent` the running theme; every token class and `token()`
 *  read follows on the next frame. */
export function applyTheme(skin: SkinName, accent: AccentName): void {
  setTheme(themeFor(skin, accent));
}
