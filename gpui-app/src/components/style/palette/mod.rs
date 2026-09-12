mod skins;

use super::tokens::{AccentId, ColorScheme, ColorToken, Decor, Shape, SkinId};

/// Re-encode a token's RGB with a new alpha channel. Used for translucent
/// surfaces, selection overlays and accent glows that must follow the accent.
pub const fn tint(token: ColorToken, alpha: u8) -> ColorToken {
    ColorToken::rgba((token.rgb_hex() << 8) | alpha as u32)
}

/// Top of the dark-red card-header band the limbus skin paints behind its
/// stencilled titles. Exposed so the renderer and the contrast regression test
/// cannot drift apart.
pub const LIMBUS_BAND_TOP: ColorToken = ColorToken::rgb(0x3e0d0d);
/// Bottom of the same band; the gradient is deliberately subtle.
pub const LIMBUS_BAND_BOTTOM: ColorToken = ColorToken::rgb(0x240a0a);
/// Parchment card-header band used by the limbus light scheme.
pub const LIMBUS_BAND_TOP_LIGHT: ColorToken = ColorToken::rgb(0xe6dcc8);
pub const LIMBUS_BAND_BOTTOM_LIGHT: ColorToken = ColorToken::rgb(0xd8c9ab);

/// All visual tokens needed by pages and reusable controls.
///
/// `Palette` stays a `Copy` value object so a skin/scheme/accent change is a
/// pure re-derivation in `AhabApp::render`; nothing mutates a second theme
/// store. `shape` and `decor` are part of the palette on purpose: a skin must
/// not be able to change its colours without also owning its geometry and
/// decoration language.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Palette {
    pub scheme: ColorScheme,
    pub accent: AccentId,
    pub skin: SkinId,
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
    /// Foreground paired with [`Palette::danger`] for destructive buttons and
    /// danger badges. Picked per skin for at least 4.5:1 contrast: using
    /// `brand_foreground` here produced dark-on-dark-red text under the
    /// limbus skin.
    pub danger_foreground: ColorToken,
    /// Selection overlay used by the entity-backed text input seam.
    pub selection: ColorToken,
    /// Modal / overlay scrim.
    pub scrim: ColorToken,
    /// Frame and rule tone. Framed skins paint corner brackets, card-header
    /// rules and the title-bar separator with this instead of a hardcoded
    /// gold, so any accent stays visually coherent.
    pub decor_line: ColorToken,
    /// One-pixel top highlight for translucent surfaces. Transparent in every
    /// skin that does not draw a bevel.
    pub hilite: ColorToken,
    /// Accent-tinted glow used as a coloured drop shadow. Transparent in
    /// every skin that does not float its surfaces.
    pub glow: ColorToken,
    pub brand: ColorToken,
    pub brand_hover: ColorToken,
    pub brand_light: ColorToken,
    pub brand_foreground: ColorToken,
    pub shape: Shape,
    pub decor: Decor,
}

impl Palette {
    pub const fn for_scheme(scheme: ColorScheme, accent: AccentId) -> Self {
        Self::for_skin(scheme, accent, SkinId::Default)
    }

    /// Derive the full token set for one skin.
    ///
    /// Every skin is scheme-aware: a skin may make light and dark look very
    /// different, but it may never ignore the scheme, because the appearance
    /// page exposes the scheme control for all skins.
    pub const fn for_skin(scheme: ColorScheme, accent: AccentId, skin: SkinId) -> Self {
        skins::tokens(scheme, accent, skin)
    }

    pub const fn light(accent: AccentId) -> Self {
        Self::for_scheme(ColorScheme::Light, accent)
    }

    pub const fn dark(accent: AccentId) -> Self {
        Self::for_scheme(ColorScheme::Dark, accent)
    }

    pub const fn limbus_dark(accent: AccentId) -> Self {
        Self::for_skin(ColorScheme::Dark, accent, SkinId::Limbus)
    }

    pub const fn limbus_light(accent: AccentId) -> Self {
        Self::for_skin(ColorScheme::Light, accent, SkinId::Limbus)
    }

    /// The skin paints gold L corner brackets around card-level surfaces.
    pub const fn uses_frame_decor(self) -> bool {
        matches!(self.decor, Decor::LimbusFrame)
    }

    /// The skin separates sections with a rule image instead of a plain line.
    pub const fn uses_rule_decor(self) -> bool {
        matches!(self.decor, Decor::Archive | Decor::LimbusFrame)
    }

    pub const fn is_dark(self) -> bool {
        matches!(self.scheme, ColorScheme::Dark)
    }

    pub fn from_strings(scheme: ColorScheme, accent: &str) -> Self {
        Self::for_scheme(scheme, AccentId::parse(accent))
    }

    pub fn from_strings_with_skin(scheme: ColorScheme, accent: &str, skin: SkinId) -> Self {
        Self::for_skin(scheme, AccentId::parse(accent), skin)
    }
}

impl Default for Palette {
    /// The web app's default `:root` is light with the crimson accent.
    fn default() -> Self {
        Self::light(AccentId::Crimson)
    }
}
