use super::tokens::{AccentId, AccentTokens, ColorScheme, ColorToken};

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
