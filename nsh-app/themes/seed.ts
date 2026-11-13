// The colour themes: a seed colour becomes the app's brand colour.
//
// The reference client generates a whole Material 3 tonal palette from a seed
// with HCT. We need only the seven tokens a brand colour touches, and they must
// be readable as text, as a fill and as a page wash, so this file converts the
// seed to OKLCH, clamps its lightness into the range that works on our
// surfaces, and emits those tokens. The neutrals stay with the skin: a theme
// changes the hue, not the surface hierarchy.
//
// Everything here is pure: the theme table calls it on every skin/theme change,
// and the tests pin the contrast it has to keep.

/** One preset of the appearance page. The list is the reference client's. */
export type SeedPreset = { id: string; label: string; hex: string };

export const SEED_PRESETS: SeedPreset[] = [
  { id: "slate", label: "墨蓝", hex: "#1E293B" },
  { id: "blue", label: "蓝", hex: "#1976D2" },
  { id: "teal", label: "青绿", hex: "#00897B" },
  { id: "pink", label: "粉", hex: "#B80049" },
  { id: "purple", label: "紫", hex: "#7B1FA2" },
  { id: "amber", label: "琥珀", hex: "#D97706" },
  { id: "indigo", label: "靛", hex: "#4F46E5" },
  { id: "cyan", label: "湖蓝", hex: "#008FB6" },
  { id: "green", label: "绿", hex: "#388E3C" },
  { id: "olive", label: "橄榄", hex: "#827717" },
  { id: "brown", label: "棕", hex: "#795548" },
];

const HEX = /^#[0-9a-f]{6}$/i;

/** Whether `value` is a colour this file can derive a theme from. */
export function isSeedHex(value: string): boolean {
  return HEX.test(value.trim());
}

/** Normalises `#a1b2c3` to `#A1B2C3`; an invalid value is not a theme. */
export function normaliseSeed(value: string): string | null {
  const text = value.trim();
  return isSeedHex(text) ? text.toUpperCase() : null;
}

// ---------------------------------------------------------------- OKLCH ----
type Rgb = [number, number, number];
type Oklch = { l: number; c: number; h: number };

function channel(value: number): number {
  const v = value / 255;
  return v <= 0.04045 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
}

function gamma(value: number): number {
  const v = value <= 0.0031308 ? value * 12.92 : 1.055 * value ** (1 / 2.4) - 0.055;
  return Math.max(0, Math.min(255, Math.round(v * 255)));
}

/** `#RRGGBB` → sRGB channels in 0..255. */
function rgbOf(hex: string): Rgb {
  const h = hex.replace("#", "");
  return [parseInt(h.slice(0, 2), 16), parseInt(h.slice(2, 4), 16), parseInt(h.slice(4, 6), 16)];
}

function oklchOf(hex: string): Oklch {
  const [r0, g0, b0] = rgbOf(hex).map(channel);
  const l = 0.4122214708 * r0 + 0.5363325363 * g0 + 0.0514459929 * b0;
  const m = 0.2119034982 * r0 + 0.6806995451 * g0 + 0.1073969566 * b0;
  const s = 0.0883024619 * r0 + 0.2817188376 * g0 + 0.6299787005 * b0;
  const l_ = Math.cbrt(l);
  const m_ = Math.cbrt(m);
  const s_ = Math.cbrt(s);
  const L = 0.2104542553 * l_ + 0.793617785 * m_ - 0.0040720468 * s_;
  const a = 1.9779984951 * l_ - 2.428592205 * m_ + 0.4505937099 * s_;
  const b = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.808675766 * s_;
  const h = (Math.atan2(b, a) * 180) / Math.PI;
  return { l: L, c: Math.hypot(a, b), h: h < 0 ? h + 360 : h };
}

function hexOf({ l: L, c: C, h }: Oklch): string {
  const rad = (h * Math.PI) / 180;
  const a = C * Math.cos(rad);
  const b = C * Math.sin(rad);
  const l_ = (L + 0.3963377774 * a + 0.2158037573 * b) ** 3;
  const m_ = (L - 0.1055613458 * a - 0.0638541728 * b) ** 3;
  const s_ = (L - 0.0894841775 * a - 1.291485548 * b) ** 3;
  const r = 4.0767416621 * l_ - 3.3077115913 * m_ + 0.2309699292 * s_;
  const g = -1.2684380046 * l_ + 2.6097574011 * m_ - 0.3413193965 * s_;
  const bb = -0.0041960863 * l_ - 0.7034186147 * m_ + 1.707614701 * s_;
  return `#${[r, g, bb].map(gamma).map((v) => v.toString(16).padStart(2, "0")).join("")}`;
}

function rgba(hex: string, alpha: number): string {
  const [r, g, b] = rgbOf(hex);
  return `rgba(${r}, ${g}, ${b}, ${alpha.toFixed(3)})`;
}

/** WCAG relative luminance of an sRGB colour. */
function luminance(hex: string): number {
  const [r, g, b] = rgbOf(hex).map(channel);
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/** WCAG contrast ratio of two colours. */
export function contrastRatio(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

const clamp = (value: number, min: number, max: number): number =>
  Math.min(max, Math.max(min, value));

/** The lightness a brand colour may take: dark enough to carry white text on a
 *  light surface, light enough to read on a dark one. */
const LIGHT_RANGE: [number, number] = [0.45, 0.57];
const DARK_RANGE: [number, number] = [0.74, 0.86];

/** The tokens a seed colour replaces, for one appearance.
 *
 *  `brand-foreground` is chosen rather than given: the light brand is dark
 *  enough for white, the dark brand is a pale tint that wants the seed's own
 *  dark tone, so text keeps its hue relation to the fill. */
export function seedTokens(seed: string, appearance: "light" | "dark"): Record<string, string> {
  const base = oklchOf(seed);
  const light = appearance === "light";
  const l = clamp(base.l, LIGHT_RANGE[0], LIGHT_RANGE[1]);
  const c = Math.min(base.c, light ? 0.17 : 0.15);
  let brandLightness = light ? l : clamp(base.l, DARK_RANGE[0], DARK_RANGE[1]);
  const container = light
    ? hexOf({ l: 0.945, c: Math.min(c * 0.35, 0.06), h: base.h })
    : hexOf({ l: 0.31, c: Math.min(c * 0.45, 0.07), h: base.h });
  // The label on the brand fill takes whichever of white and the seed's own dark
  // tone reads better; when neither clears 4.5:1 the fill steps darker (for
  // white) or lighter (for the ink) until it does. A mid-lightness seed such as
  // amber cannot satisfy both at once otherwise.
  const white = "#FFFFFF";
  const inkLightness = light ? 0.22 : 0.26;
  let brand = hexOf({ l: brandLightness, c, h: base.h });
  let foreground = white;
  for (let step = 0; step < 24; step++) {
    const ink = hexOf({ l: inkLightness, c: c * 0.5, h: base.h });
    const onWhite = contrastRatio(brand, white);
    const onInk = contrastRatio(brand, ink);
    foreground = onWhite >= onInk ? white : ink;
    if (Math.max(onWhite, onInk) >= 4.5) break;
    brandLightness = clamp(
      brandLightness + (foreground === white ? -0.015 : 0.015),
      0.3,
      0.95,
    );
    brand = hexOf({ l: brandLightness, c, h: base.h });
  }
  // The hover state is the same hue, one step along the same axis the fill took.
  const hover = hexOf({
    l: clamp(brandLightness + (light ? -0.06 : 0.05), 0.2, 0.96),
    c: light ? Math.min(c * 1.04, 0.17) : c * 0.9,
    h: base.h,
  });
  return {
    brand,
    // Switches and other "the action is on" surfaces read the palette's
    // primary, so the theme colours them too; otherwise a teal page would keep
    // a rose switch.
    primary: brand,
    "primary-foreground": foreground,
    "brand-hover": hover,
    "brand-light": container,
    "brand-foreground": foreground,
    ring: brand,
    selection: rgba(brand, light ? 0.14 : 0.18),
    wash: rgba(brand, light ? 0.1 : 0.06),
  };
}
