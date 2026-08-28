#![allow(dead_code)]

//! Design tokens shared by the native GPUI controls.
//!
//! The values in this module are the sRGB values obtained from the CSS
//! `oklch` declarations in `ui/src/index.css`.  Keeping the conversion at the
//! boundary means pages never need to approximate an `oklch` color themselves.
//! [`Palette`] is a value object: callers derive a new value from persisted
//! settings instead of mutating a second theme state.

use std::cell::Cell;

/// A color token encoded as RGBA (`0xRRGGBBAA`), which maps directly to
/// [`gpui::rgba`].  Alpha is retained for the transparent CSS border and
/// selection colors even though the compatibility constants below are RGB.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ColorToken(pub u32);

impl ColorToken {
    pub const fn rgb(hex: u32) -> Self {
        Self((hex & 0x00ff_ffff) << 8 | 0xff)
    }

    pub const fn rgba(hex: u32) -> Self {
        Self(hex)
    }

    pub const fn rgba_hex(self) -> u32 {
        self.0
    }

    pub const fn rgb_hex(self) -> u32 {
        (self.0 >> 8) & 0x00ff_ffff
    }

    pub const fn alpha(self) -> u8 {
        self.0 as u8
    }

    pub const fn is_transparent(self) -> bool {
        self.alpha() == 0
    }
}

/// Light and dark token sets from `index.css`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ColorScheme {
    Light,
    Dark,
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self::Light
    }
}

/// The five accent identifiers accepted by the persisted UI settings.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum AccentId {
    #[default]
    Crimson,
    Blue,
    Amber,
    Emerald,
    Violet,
}

impl AccentId {
    pub const ALL: [Self; 5] = [
        Self::Crimson,
        Self::Blue,
        Self::Amber,
        Self::Emerald,
        Self::Violet,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Crimson => "crimson",
            Self::Blue => "blue",
            Self::Amber => "amber",
            Self::Emerald => "emerald",
            Self::Violet => "violet",
        }
    }

    pub const fn name_zh(self) -> &'static str {
        match self {
            Self::Crimson => "赤红",
            Self::Blue => "深蓝",
            Self::Amber => "琥珀",
            Self::Emerald => "翠绿",
            Self::Violet => "紫罗兰",
        }
    }

    pub const fn name_en(self) -> &'static str {
        match self {
            Self::Crimson => "Crimson",
            Self::Blue => "Blue",
            Self::Amber => "Amber",
            Self::Emerald => "Emerald",
            Self::Violet => "Violet",
        }
    }

    /// Unknown values intentionally fall back to crimson, matching the web
    /// theme's `ACCENT_PRESETS[0]` fallback and keeping old settings readable.
    pub fn parse(value: &str) -> Self {
        match value {
            "blue" => Self::Blue,
            "amber" => Self::Amber,
            "emerald" => Self::Emerald,
            "violet" => Self::Violet,
            "crimson" => Self::Crimson,
            _ => Self::Crimson,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AccentTokens {
    pub brand: ColorToken,
    pub brand_hover: ColorToken,
    pub brand_light: ColorToken,
    pub brand_foreground: ColorToken,
}

impl AccentTokens {
    pub const fn for_scheme(accent: AccentId, scheme: ColorScheme) -> Self {
        match (accent, scheme) {
            (AccentId::Crimson, ColorScheme::Light) => Self::new(0xc8354f, 0xa92b42, 0xfbe9ec),
            (AccentId::Crimson, ColorScheme::Dark) => Self::new(0xe05a72, 0xc8354f, 0x3a1e24),
            (AccentId::Blue, ColorScheme::Light) => Self::new(0x2563eb, 0x1d4ed8, 0xdbeafe),
            (AccentId::Blue, ColorScheme::Dark) => Self::new(0x60a5fa, 0x3b82f6, 0x1e293b),
            (AccentId::Amber, ColorScheme::Light) => Self::new(0xd97706, 0xb45309, 0xfef3c7),
            (AccentId::Amber, ColorScheme::Dark) => Self::new(0xfbbf24, 0xf59e0b, 0x3b2f14),
            (AccentId::Emerald, ColorScheme::Light) => Self::new(0x059669, 0x047857, 0xd1fae5),
            (AccentId::Emerald, ColorScheme::Dark) => Self::new(0x34d399, 0x10b981, 0x14332a),
            (AccentId::Violet, ColorScheme::Light) => Self::new(0x7c3aed, 0x6d28d9, 0xede9fe),
            (AccentId::Violet, ColorScheme::Dark) => Self::new(0xa78bfa, 0x8b5cf6, 0x2b2247),
        }
    }

    const fn new(brand: u32, brand_hover: u32, brand_light: u32) -> Self {
        Self {
            brand: ColorToken::rgb(brand),
            brand_hover: ColorToken::rgb(brand_hover),
            brand_light: ColorToken::rgb(brand_light),
            // This is the --color-brand-foreground fallback from index.css.
            brand_foreground: ColorToken::rgb(0xfafafa),
        }
    }
}

/// Metadata matching the five `ACCENT_PRESETS` entries in `ui/src/themes`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AccentPreset {
    pub id: &'static str,
    pub name: &'static str,
    pub name_en: &'static str,
    pub light: AccentTokens,
    pub dark: AccentTokens,
}

/// Accent metadata retained as a static table for settings controls.
pub const ACCENT_PRESETS: [AccentPreset; 5] = [
    AccentPreset {
        id: "crimson",
        name: "赤红",
        name_en: "Crimson",
        light: AccentTokens::for_scheme(AccentId::Crimson, ColorScheme::Light),
        dark: AccentTokens::for_scheme(AccentId::Crimson, ColorScheme::Dark),
    },
    AccentPreset {
        id: "blue",
        name: "深蓝",
        name_en: "Blue",
        light: AccentTokens::for_scheme(AccentId::Blue, ColorScheme::Light),
        dark: AccentTokens::for_scheme(AccentId::Blue, ColorScheme::Dark),
    },
    AccentPreset {
        id: "amber",
        name: "琥珀",
        name_en: "Amber",
        light: AccentTokens::for_scheme(AccentId::Amber, ColorScheme::Light),
        dark: AccentTokens::for_scheme(AccentId::Amber, ColorScheme::Dark),
    },
    AccentPreset {
        id: "emerald",
        name: "翠绿",
        name_en: "Emerald",
        light: AccentTokens::for_scheme(AccentId::Emerald, ColorScheme::Light),
        dark: AccentTokens::for_scheme(AccentId::Emerald, ColorScheme::Dark),
    },
    AccentPreset {
        id: "violet",
        name: "紫罗兰",
        name_en: "Violet",
        light: AccentTokens::for_scheme(AccentId::Violet, ColorScheme::Light),
        dark: AccentTokens::for_scheme(AccentId::Violet, ColorScheme::Dark),
    },
];

/// All visual tokens needed by pages and reusable controls.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Palette {
    pub scheme: ColorScheme,
    pub accent: AccentId,
    pub background: ColorToken,
    pub foreground: ColorToken,
    pub card: ColorToken,
    pub card_foreground: ColorToken,
    pub popover: ColorToken,
    pub popover_foreground: ColorToken,
    pub primary: ColorToken,
    pub primary_foreground: ColorToken,
    pub secondary: ColorToken,
    pub secondary_foreground: ColorToken,
    pub muted: ColorToken,
    pub muted_foreground: ColorToken,
    pub accent_surface: ColorToken,
    pub accent_foreground: ColorToken,
    pub destructive: ColorToken,
    pub border: ColorToken,
    pub input: ColorToken,
    pub ring: ColorToken,
    pub success: ColorToken,
    pub success_light: ColorToken,
    pub warning: ColorToken,
    pub warning_light: ColorToken,
    pub danger: ColorToken,
    /// A derived surface for danger badges; `index.css` defines the danger
    /// foreground but no danger-light token.
    pub danger_light: ColorToken,
    /// Selection overlay used by the entity-backed text input seam.
    pub selection: ColorToken,
    pub brand: ColorToken,
    pub brand_hover: ColorToken,
    pub brand_light: ColorToken,
    pub brand_foreground: ColorToken,
}

impl Palette {
    pub const fn for_scheme(scheme: ColorScheme, accent: AccentId) -> Self {
        let accent_tokens = AccentTokens::for_scheme(accent, scheme);
        match scheme {
            ColorScheme::Light => Self {
                scheme,
                accent,
                // oklch(0.963 0.003 25)
                background: ColorToken::rgb(0xf5f2f2),
                // oklch(0.145 0 0)
                foreground: ColorToken::rgb(0x0a0a0a),
                card: ColorToken::rgb(0xffffff),
                card_foreground: ColorToken::rgb(0x0a0a0a),
                popover: ColorToken::rgb(0xffffff),
                popover_foreground: ColorToken::rgb(0x0a0a0a),
                primary: ColorToken::rgb(0x171717),
                primary_foreground: ColorToken::rgb(0xfafafa),
                secondary: ColorToken::rgb(0xf5f5f5),
                secondary_foreground: ColorToken::rgb(0x171717),
                muted: ColorToken::rgb(0xf5f5f5),
                muted_foreground: ColorToken::rgb(0x737373),
                accent_surface: ColorToken::rgb(0xf5f5f5),
                accent_foreground: ColorToken::rgb(0x171717),
                destructive: ColorToken::rgb(0xe7000b),
                // --border is transparent in the browser; controls use --input.
                border: ColorToken::rgba(0x00000000),
                input: ColorToken::rgb(0xd9d9d9),
                ring: ColorToken::rgb(0xa1a1a1),
                success: ColorToken::rgb(0x18a349),
                success_light: ColorToken::rgb(0xdcfce6),
                warning: ColorToken::rgb(0xd6791d),
                warning_light: ColorToken::rgb(0xfff4cb),
                danger: ColorToken::rgb(0xe7000b),
                danger_light: ColorToken::rgb(0xfee2e2),
                selection: ColorToken::rgba(0xc8354f33),
                brand: accent_tokens.brand,
                brand_hover: accent_tokens.brand_hover,
                brand_light: accent_tokens.brand_light,
                brand_foreground: accent_tokens.brand_foreground,
            },
            ColorScheme::Dark => Self {
                scheme,
                accent,
                // oklch(0.135 0.005 25)
                background: ColorToken::rgb(0x0a0707),
                // oklch(0.985 0 0)
                foreground: ColorToken::rgb(0xfafafa),
                card: ColorToken::rgb(0x181515),
                card_foreground: ColorToken::rgb(0xfafafa),
                popover: ColorToken::rgb(0x262626),
                popover_foreground: ColorToken::rgb(0xfafafa),
                primary: ColorToken::rgb(0xe5e5e5),
                primary_foreground: ColorToken::rgb(0x171717),
                secondary: ColorToken::rgb(0x262626),
                secondary_foreground: ColorToken::rgb(0xfafafa),
                muted: ColorToken::rgb(0x262626),
                muted_foreground: ColorToken::rgb(0xa1a1a1),
                accent_surface: ColorToken::rgb(0x404040),
                accent_foreground: ColorToken::rgb(0xfafafa),
                destructive: ColorToken::rgb(0xff6467),
                border: ColorToken::rgba(0x00000000),
                // oklch(1 0 0 / 15%) composited by GPUI at paint time.
                input: ColorToken::rgba(0xffffff26),
                ring: ColorToken::rgb(0x737373),
                success: ColorToken::rgb(0x4cdb7c),
                success_light: ColorToken::rgb(0x0c341e),
                warning: ColorToken::rgb(0xeab312),
                warning_light: ColorToken::rgb(0x432a08),
                danger: ColorToken::rgb(0xff6467),
                danger_light: ColorToken::rgb(0x451a1a),
                selection: ColorToken::rgba(0xe05a7233),
                brand: accent_tokens.brand,
                brand_hover: accent_tokens.brand_hover,
                brand_light: accent_tokens.brand_light,
                brand_foreground: accent_tokens.brand_foreground,
            },
        }
    }

    pub const fn light(accent: AccentId) -> Self {
        Self::for_scheme(ColorScheme::Light, accent)
    }

    pub const fn dark(accent: AccentId) -> Self {
        Self::for_scheme(ColorScheme::Dark, accent)
    }

    pub fn from_strings(scheme: ColorScheme, accent: &str) -> Self {
        Self::for_scheme(scheme, AccentId::parse(accent))
    }
}

impl Default for Palette {
    /// The web app's default `:root` is light with the crimson accent.
    fn default() -> Self {
        Self::light(AccentId::Crimson)
    }
}

thread_local! {
    /// Render-scoped palette used by legacy page helpers while they are being
    /// migrated to explicit palette parameters. It is a derived snapshot, not
    /// a second settings store: the root replaces it before every render.
    static CURRENT_RENDER_PALETTE: Cell<Palette> = Cell::new(Palette::default());
}

pub fn set_current_render_palette(palette: Palette) {
    CURRENT_RENDER_PALETTE.with(|current| current.set(palette));
}

pub fn current_render_palette() -> Palette {
    CURRENT_RENDER_PALETTE.with(Cell::get)
}

/// Paint a token without routing it through the legacy RGB compatibility
/// resolver. New page code should use this helper so semantic colors cannot
/// be confused with an older literal that happened to have the same value.
pub fn palette_rgb(token: ColorToken) -> gpui::Rgba {
    gpui::rgba(token.rgba_hex())
}

fn token_for_legacy_hex(hex: u32, palette: Palette) -> Option<ColorToken> {
    Some(match hex & 0x00ff_ffff {
        0xf5f2f2 => palette.background,
        0x0a0a0a => palette.foreground,
        0xffffff => palette.card,
        0x171717 => palette.primary,
        0xfafafa => palette.foreground,
        0xf5f5f5 => palette.secondary,
        0x737373 => palette.muted_foreground,
        0xe7000b => palette.danger,
        0xd9d9d9 => palette.input,
        0xa1a1a1 => palette.ring,
        0x18a349 => palette.success,
        0xdcfce6 => palette.success_light,
        0xd6791d => palette.warning,
        0xfff4cb => palette.warning_light,
        0xfee2e2 => palette.danger_light,
        0xc8354f | 0xe05a72 | 0x2563eb | 0x60a5fa | 0xd97706 | 0xfbbf24 | 0x059669 | 0x34d399
        | 0x7c3aed | 0xa78bfa => palette.brand,
        0xa92b42 | 0x3b82f6 | 0xb45309 | 0xf59e0b | 0x047857 | 0x10b981 | 0x6d28d9 | 0x8b5cf6 => {
            palette.brand_hover
        }
        0xfbe9ec | 0x3a1e24 | 0xdbeafe | 0x1e293b | 0xfef3c7 | 0x3b2f14 | 0xd1fae5 | 0x14332a
        | 0xede9fe | 0x2b2247 => palette.brand_light,
        0x17120a => palette.brand_foreground,
        0x202936 | 0x354152 => palette.muted,
        0x1b222d => palette.card,
        0x3d301b | 0x5d4820 => palette.brand_light,
        0x1d513b => palette.success_light,
        0x8de3b2 | 0x4dcc89 => palette.success,
        // Legacy Teams controls used an amber accent even before accent
        // presets were wired. Treat it as the current semantic brand color.
        0xd9a441 => palette.brand,
        0x232c39 | 0x273345 => palette.secondary,
        0x24151b | 0x542b34 => palette.danger_light,
        0xc45b68 => palette.danger,
        0xf0c36a => palette.warning,
        0x171e29 => palette.popover,
        0xb8c8dd => palette.foreground,
        _ => return None,
    })
}

/// Resolve a legacy RGB literal through the current render snapshot. Unknown
/// literals are passed through unchanged so SVG and one-off diagnostic colors
/// keep their original meaning.
pub fn render_rgb(hex: u32) -> gpui::Rgba {
    let palette = current_render_palette();
    token_for_legacy_hex(hex, palette)
        .map(|token| gpui::rgba(token.rgba_hex()))
        .unwrap_or_else(|| gpui::rgb(hex))
}

/// Resolve an RGBA literal while preserving the caller's alpha. This is used
/// for translucent card overlays such as `card/30` and `brand/20`.
pub fn render_rgba(value: u32) -> gpui::Rgba {
    let palette = current_render_palette();
    let alpha = value as u8;
    let rgb = (value >> 8) & 0x00ff_ffff;
    token_for_legacy_hex(rgb, palette)
        .map(|token| gpui::rgba((token.rgb_hex() << 8) | u32::from(alpha)))
        .unwrap_or_else(|| gpui::rgba(value))
}

// Compatibility exports for pages that still import the old fixed names from
// `app.rs`. They intentionally point at the light root palette, not the old
// dark-only prototype values. New code should consume `Palette` directly.
pub const BACKGROUND: u32 = Palette::light(AccentId::Crimson).background.rgb_hex();
pub const SURFACE: u32 = Palette::light(AccentId::Crimson).card.rgb_hex();
pub const SURFACE_HOVER: u32 = Palette::light(AccentId::Crimson).secondary.rgb_hex();
pub const BORDER: u32 = Palette::light(AccentId::Crimson).input.rgb_hex();
pub const TEXT: u32 = Palette::light(AccentId::Crimson).foreground.rgb_hex();
pub const TEXT_MUTED: u32 = Palette::light(AccentId::Crimson).muted_foreground.rgb_hex();
pub const ACCENT: u32 = Palette::light(AccentId::Crimson).brand.rgb_hex();
pub const GREEN: u32 = Palette::light(AccentId::Crimson).success.rgb_hex();
pub const DANGER: u32 = Palette::light(AccentId::Crimson).danger.rgb_hex();

pub const SPACE_1: f32 = 4.0;
pub const SPACE_2: f32 = 8.0;
pub const SPACE_3: f32 = 12.0;
pub const SPACE_4: f32 = 16.0;
pub const SPACE_5: f32 = 20.0;
pub const SPACE_6: f32 = 24.0;

pub const FONT_XS: f32 = 10.0;
pub const FONT_SM: f32 = 12.0;
pub const FONT_MD: f32 = 14.0;
pub const FONT_LG: f32 = 18.0;
pub const FONT_XL: f32 = 20.0;

pub const RADIUS_SM: f32 = 4.0;
pub const RADIUS_MD: f32 = 6.0;
pub const RADIUS_LG: f32 = 8.0;
pub const RADIUS_XL: f32 = 12.0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn css_palettes_have_distinct_light_and_dark_surfaces() {
        let light = Palette::default();
        let dark = Palette::dark(AccentId::Crimson);
        assert_ne!(light.background, dark.background);
        assert_ne!(light.card, dark.card);
        assert!(light.border.is_transparent());
        assert_eq!(dark.input.alpha(), 0x26);
    }

    #[test]
    fn all_accent_ids_are_stable_and_unknown_values_fall_back() {
        for accent in AccentId::ALL {
            assert_eq!(AccentId::parse(accent.as_str()), accent);
            assert_ne!(
                Palette::light(accent).brand,
                Palette::light(accent).brand_hover
            );
        }
        assert_eq!(AccentId::parse("old-value"), AccentId::Crimson);
    }

    #[test]
    fn compatibility_constants_use_the_light_root_tokens() {
        let palette = Palette::default();
        assert_eq!(BACKGROUND, palette.background.rgb_hex());
        assert_eq!(SURFACE, palette.card.rgb_hex());
        assert_eq!(ACCENT, palette.brand.rgb_hex());
    }

    #[test]
    fn legacy_page_colors_follow_the_current_render_snapshot() {
        let palette = Palette::dark(AccentId::Violet);
        set_current_render_palette(palette);
        assert_eq!(
            render_rgb(BACKGROUND),
            gpui::rgba(palette.background.rgba_hex())
        );
        assert_eq!(render_rgb(ACCENT), gpui::rgba(palette.brand.rgba_hex()));
        assert_eq!(
            render_rgba((SURFACE << 8) | 0x33),
            gpui::rgba((palette.card.rgb_hex() << 8) | 0x33)
        );
    }

    #[test]
    fn legacy_page_accent_colors_follow_the_selected_accent() {
        let palette = Palette::dark(AccentId::Emerald);
        set_current_render_palette(palette);
        assert_eq!(render_rgb(0xd9a441), gpui::rgba(palette.brand.rgba_hex()));
        assert_eq!(render_rgb(0x8de3b2), gpui::rgba(palette.success.rgba_hex()));
    }
}
