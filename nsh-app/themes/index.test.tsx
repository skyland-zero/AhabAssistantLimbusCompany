// The skin x accent table is generated, so these tests check the composition
// rules rather than the values: every pair produces a complete theme, the
// accent lands on the brand tokens, and the decoration tokens are per skin.
import { expect, test } from "nsh/test";
import {
  ACCENTS,
  DECOR_OF,
  SKINS,
  SKIN_DEFAULT_ACCENT,
  SKIN_LABELS,
  ACCENT_LABELS,
  themeFor,
} from "./index";
import { SKIN_RADII, SKIN_TOKENS } from "./skins";
import { SEED_PRESETS, isSeedHex, normaliseSeed, seedTokens } from "./seed";

test("every skin and accent composes a complete theme", () => {
  for (const skin of SKINS) {
    const radii = SKIN_RADII[skin];
    const base = SKIN_TOKENS[skin];
    for (const accent of ACCENTS) {
      const theme = themeFor(skin, accent);
      expect(Object.keys(theme.cssVars.theme ?? {}).sort()).toEqual(Object.keys(radii).sort());
      for (const appearance of ["light", "dark"] as const) {
        const tokens = theme.cssVars[appearance] ?? {};
        for (const token of Object.keys(base[appearance])) {
          expect(tokens[token]).toBeDefined();
        }
      }
    }
  }
});

test("the accent lands on the brand tokens and changes them", () => {
  const crimson = themeFor("default", "crimson").cssVars.light ?? {};
  const blue = themeFor("default", "blue").cssVars.light ?? {};
  expect(crimson["brand"]).not.toBe(blue["brand"]);
  // A skin's own accent leaves its base table untouched.
  const defaultSkin = SKIN_TOKENS.default.light;
  expect(crimson["brand"]).toBe(defaultSkin["brand"]);
});

test("each skin keeps its own accent and decoration", () => {
  expect(SKIN_DEFAULT_ACCENT.limbus).toBe("limbus-brass");
  for (const skin of SKINS) {
    expect(DECOR_OF[skin]).toBeDefined();
    expect(SKIN_LABELS[skin].length).toBeGreaterThan(0);
  }
  for (const accent of ACCENTS) {
    expect(ACCENT_LABELS[accent].length).toBeGreaterThan(0);
  }
});

test("the limbus band is opaque and the flat skin has none", () => {
  const limbus = themeFor("limbus", "limbus-brass").cssVars.dark ?? {};
  const plain = themeFor("default", "crimson").cssVars.dark ?? {};
  expect(limbus["band-top"]).toBe("#3e0d0d");
  expect(plain["band-top"]).toBe("rgba(0, 0, 0, 0.000)");
});

test("the mist wash and grain are the mist skin's alone", () => {
  const mist = themeFor("mist", "crimson").cssVars.dark ?? {};
  const glass = themeFor("glass", "crimson").cssVars.dark ?? {};
  expect(mist["grain"]).not.toBe("rgba(0, 0, 0, 0.000)");
  expect(mist["wash"]).not.toBe("rgba(0, 0, 0, 0.000)");
  // Glass paints a wash too, but from the accent's glow, and no grain.
  expect(glass["wash"]).not.toBe("rgba(0, 0, 0, 0.000)");
  expect(glass["grain"]).toBe("rgba(0, 0, 0, 0.000)");
});

test("the nsh skin is hairlines over a neutral surface", () => {
  for (const appearance of ["light", "dark"] as const) {
    const tokens = themeFor("nsh", "crimson").cssVars[appearance] ?? {};
    // Nothing to paint: the skin carries its structure with the border alone.
    for (const decoration of ["band-top", "band-bottom", "wash", "grain", "hilite", "glow"]) {
      expect(tokens[decoration]).toBe("rgba(0, 0, 0, 0.000)");
    }
    // The default skin hides its light-mode border; nsh draws it everywhere.
    expect(tokens["border"]).not.toBe("rgba(0, 0, 0, 0.000)");
    expect(tokens["input"]).not.toBe("rgba(0, 0, 0, 0.000)");
  }
  // Content, not status colour: the semantic tokens are the default skin's.
  const nsh = themeFor("nsh", "crimson").cssVars.light ?? {};
  const plain = themeFor("default", "crimson").cssVars.light ?? {};
  for (const status of ["success", "warning", "danger"]) {
    expect(nsh[status]).toBe(plain[status]);
  }
});

test("the bettbox skin is a theme-coloured wash under borderless cards", () => {
  for (const appearance of ["light", "dark"] as const) {
    const tokens = themeFor("bettbox", "rose").cssVars[appearance] ?? {};
    // The page gradient is drawn from `wash`; without it the skin is a plain
    // white card on a flat page.
    expect(tokens["wash"]).toMatch(/^rgba\(/);
    expect(tokens["wash"]).not.toBe("rgba(0, 0, 0, 0.000)");
    // No border: the cards separate by surface and one shadow.
    expect(tokens["border"]).toBe("rgba(0, 0, 0, 0.000)");
    // The theme colour drives actions, not just decoration.
    expect(tokens["primary"]).toBe(tokens["brand"]);
  }
});

test("a theme colour carries every accent, including rose", () => {
  for (const skin of SKINS) {
    for (const accent of ACCENTS) {
      const tokens = themeFor(skin, accent).cssVars.light ?? {};
      expect(tokens["brand"]).toBeDefined();
      expect(tokens["ring"]).toBeDefined();
    }
  }
});

/** WCAG contrast of two `#rrggbb` colours. */
function contrast(a: string, b: string): number {
  const lum = (hex: string) => {
    const h = hex.replace("#", "");
    const [r, g, bl] = [0, 2, 4].map((i) => {
      const v = parseInt(h.slice(i, i + 2), 16) / 255;
      return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
    });
    return 0.2126 * r + 0.7152 * g + 0.0722 * bl;
  };
  const [hi, lo] = [lum(a), lum(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}

test("every preset theme reads on its own brand colour", () => {
  for (const preset of SEED_PRESETS) {
    const light = seedTokens(preset.hex, "light");
    const dark = seedTokens(preset.hex, "dark");
    expect(contrast(light["brand"], light["brand-foreground"])).toBeGreaterThan(4.5);
    expect(contrast(dark["brand"], dark["brand-foreground"])).toBeGreaterThan(4.5);
    // The same seed is the same theme: the table is a pure function.
    expect(seedTokens(preset.hex, "light")).toEqual(light);
    // A wash and a container, not just a brand colour.
    expect(light["wash"]).toMatch(/^rgba\(/);
    expect(light["brand-light"]).not.toBe(light["brand"]);
  }
});

test("a colour theme outranks the skin's accent", () => {
  const plain = themeFor("default", "crimson").cssVars.light ?? {};
  const themed = themeFor("default", "crimson", "#B80049").cssVars.light ?? {};
  expect(themed["brand"]).not.toBe(plain["brand"]);
  expect(themed["wash"]).toMatch(/^rgba\(/);
  // The surfaces are the skin's: only the brand family moves.
  expect(themed["background"]).toBe(plain["background"]);
  expect(themed["card"]).toBe(plain["card"]);
});

test("a custom colour is only accepted as #rrggbb", () => {
  expect(isSeedHex("#B80049")).toBe(true);
  expect(isSeedHex("#b80049")).toBe(true);
  expect(isSeedHex("b80049")).toBe(false);
  expect(isSeedHex("#B8004")).toBe(false);
  expect(normaliseSeed(" #b80049 ")).toBe("#B80049");
  expect(normaliseSeed("pink")).toBeNull();
});

test("each skin's radii step from small parts up to containers", () => {
  for (const skin of SKINS) {
    const scale = ["radius-sm", "radius", "radius-lg", "radius-xl"].map((key) =>
      parseFloat(SKIN_RADII[skin][key]),
    );
    expect(scale).toEqual([...scale].sort((a, b) => a - b));
  }
});
