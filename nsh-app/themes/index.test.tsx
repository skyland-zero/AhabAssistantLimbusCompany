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

test("each skin's radii step from small parts up to containers", () => {
  for (const skin of SKINS) {
    const scale = ["radius-sm", "radius", "radius-lg", "radius-xl"].map((key) =>
      parseFloat(SKIN_RADII[skin][key]),
    );
    expect(scale).toEqual([...scale].sort((a, b) => a - b));
  }
});
