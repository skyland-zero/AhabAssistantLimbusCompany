// The app's runtime theme: a skin's token table with an accent laid over it.
//
// `nsh.json` bootstraps the default skin; the user's skin and accent are
// applied with `setTheme()`, which replaces the theme wholesale — so the object
// built here carries the radius scale too. `skins.generated.ts` is the table
// (34 colour tokens + 4 decoration tokens per skin, plus what each accent
// changes), generated from the GPUI palette.
import { setTheme } from "nsh";
import type { Theme } from "nsh";
import {
  ACCENT_OVERRIDES,
  SKIN_DEFAULT_ACCENT,
  SKIN_RADII,
  SKIN_TOKENS,
} from "./skins.generated";
import type { AccentName, SkinName } from "./skins.generated";

export { SKIN_DEFAULT_ACCENT };
export type { AccentName, SkinName };

/** The decoration language a skin paints on top of the flat surfaces. Renderers
 *  branch on this, never on the skin id, so a skin can change its colours
 *  without every component learning a new predicate. */
export type Decor = "plain" | "glass" | "archive" | "limbus" | "mist";

/** Which decoration each skin uses (the GPUI `SkinId::decor` mapping). */
export const DECOR_OF: Record<SkinName, Decor> = {
  default: "plain",
  glass: "glass",
  archive: "archive",
  limbus: "limbus",
  mist: "mist",
};

/** The skins in the order the appearance page lists them. */
export const SKINS: SkinName[] = ["default", "glass", "archive", "limbus", "mist"];

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

/** The theme object for one skin and accent, in both appearances. */
export function themeFor(skin: SkinName, accent: AccentName): Theme {
  const base = SKIN_TOKENS[skin];
  const override = accent === SKIN_DEFAULT_ACCENT[skin] ? undefined : ACCENT_OVERRIDES[skin][accent];
  return {
    cssVars: {
      theme: { ...SKIN_RADII[skin] },
      light: { ...base.light, ...(override?.light ?? {}) },
      dark: { ...base.dark, ...(override?.dark ?? {}) },
    },
  };
}

/** Makes `skin` + `accent` the running theme; every token class and `token()`
 *  read follows on the next frame. */
export function applyTheme(skin: SkinName, accent: AccentName): void {
  setTheme(themeFor(skin, accent));
}
