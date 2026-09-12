use std::cell::Cell;

use gpui::{BoxShadow, Hsla, Pixels, Styled, hsla, px};

use super::{ColorToken, Palette, ShadowLevel};

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

/// Apply an explicit corner radius. Pages should prefer [`skin_rounded`] so
/// the radius follows the active skin; this exists for the few surfaces that
/// must use a size outside the semantic sm/md/lg/xl set.
pub fn shape_rounded(control: gpui::Div, radius: u32) -> gpui::Div {
    control.rounded(px(radius as f32))
}

/// Corner radius that follows the active skin's [`super::Shape`]. Call sites
/// keep their existing shape choice (`large` = card-level) so a skin switch
/// cannot move layout, only corners.
pub fn skin_rounded(control: gpui::Div, large: bool) -> gpui::Div {
    shape_rounded(control, current_render_palette().shape.radius(large))
}

/// Builder-friendly form of [`shape_rounded`] / [`skin_rounded`].
///
/// The controls were written as `.rounded_md()` / `.rounded_lg()` chains, so a
/// one-token replacement keeps the diff honest while making every control
/// follow the skin's geometry.
pub trait ShapeExt: Styled + Sized {
    /// Apply the skin's control-level (`large == false`) or card-level radius.
    fn skin_rounded(self, palette: &Palette, large: bool) -> Self;
    /// Apply the skin's smallest semantic radius (badges, list rows).
    fn skin_radius_sm(self, palette: &Palette) -> Self;
}

impl<T: Styled + Sized> ShapeExt for T {
    fn skin_rounded(self, palette: &Palette, large: bool) -> Self {
        self.rounded(px(palette.shape.radius(large) as f32))
    }

    fn skin_radius_sm(self, palette: &Palette) -> Self {
        self.rounded(px(palette.shape.radius_sm as f32))
    }
}

/// Apply the skin's card elevation.
///
/// `glow` is an optional accent-tinted halo used by the translucent skins; it
/// is ignored by every skin whose palette leaves `glow` transparent.
pub fn apply_card_shadow(control: gpui::Div, level: ShadowLevel, glow: Option<Hsla>) -> gpui::Div {
    let shadows = card_shadows(level, glow);
    match shadows {
        Some(shadows) => control.shadow(shadows),
        None => control.shadow_none(),
    }
}

fn card_shadows(level: ShadowLevel, glow: Option<Hsla>) -> Option<Vec<BoxShadow>> {
    let mut shadows = match level {
        ShadowLevel::None => return None,
        ShadowLevel::Subtle => vec![
            BoxShadow::new(px(0.), px(1.), hsla(0., 0., 0., 0.10)).blur_radius(px(3.)),
            BoxShadow::new(px(0.), px(1.), hsla(0., 0., 0., 0.10))
                .blur_radius(px(2.))
                .spread_radius(px(-1.)),
        ],
        ShadowLevel::Raised => vec![
            BoxShadow::new(px(0.), px(4.), hsla(0., 0., 0., 0.22))
                .blur_radius(px(12.))
                .spread_radius(px(-2.)),
            BoxShadow::new(px(0.), px(1.), hsla(0., 0., 0., 0.18))
                .blur_radius(px(3.))
                .spread_radius(px(-1.)),
        ],
        ShadowLevel::Floating => vec![
            BoxShadow::new(px(0.), px(10.), hsla(0., 0., 0., 0.30))
                .blur_radius(px(24.))
                .spread_radius(px(-8.)),
            BoxShadow::new(px(0.), px(2.), hsla(0., 0., 0., 0.22))
                .blur_radius(px(6.))
                .spread_radius(px(-2.)),
        ],
    };
    if let Some(glow) = glow {
        // Draw the halo first so it sits behind the neutral layers.
        shadows.insert(
            0,
            BoxShadow::new(px(0.), px(0.), glow)
                .blur_radius(px(20.))
                .spread_radius(px(-8.)),
        );
    }
    Some(shadows)
}

/// Compatibility constants for pages that still import the fixed names from
/// `app.rs`.
///
/// These are **tags**, not colors: each value is a reserved sentinel inside
/// the 24-bit space [`render_rgb`] inspects. Before this they were real RGB
/// literals that `render_rgb` had to guess the meaning of by looking the hex
/// up in a table, which silently mis-resolved as soon as a skin reused one of
/// those literals for a different role. A tag cannot collide with a real color
/// because [`TAG_BASE`] is reserved.
///
/// Prefer [`Palette`] fields or the `*_with_palette` constructors in new code.
const TAG_BASE: u32 = 0x00ff_0000;

pub const BACKGROUND: u32 = TAG_BASE | 1;
pub const SURFACE: u32 = TAG_BASE | 2;
pub const SURFACE_HOVER: u32 = TAG_BASE | 3;
pub const BORDER: u32 = TAG_BASE | 4;
pub const TEXT: u32 = TAG_BASE | 5;
pub const TEXT_MUTED: u32 = TAG_BASE | 6;
pub const ACCENT: u32 = TAG_BASE | 7;
pub const GREEN: u32 = TAG_BASE | 8;
pub const DANGER: u32 = TAG_BASE | 9;

/// The reserved tag range, after the 24-bit mask `render_rgb` applies.
const TAG_FIRST: u32 = 0x00ff_0001;
const TAG_LAST: u32 = 0x00ff_0009;

fn token_for_tag(tag: u32, palette: Palette) -> Option<ColorToken> {
    Some(match tag {
        BACKGROUND => palette.background,
        SURFACE => palette.card,
        SURFACE_HOVER => palette.secondary,
        BORDER => palette.input,
        TEXT => palette.foreground,
        TEXT_MUTED => palette.muted_foreground,
        ACCENT => palette.brand,
        GREEN => palette.success,
        DANGER => palette.danger,
        _ => return None,
    })
}

/// Resolve a literal written for the original web `index.css` tokens.
///
/// **Transition layer.** Existing pages still pass raw hex literals to
/// [`render_rgb`]; this table keeps them following the active palette while
/// they are migrated to explicit `Palette` fields. Do not add entries: new
/// code must read the semantic token instead, because a literal cannot
/// distinguish two roles that happen to share a value.
fn token_for_legacy_hex(hex: u32, palette: Palette) -> Option<ColorToken> {
    Some(match hex & 0x00ff_ffff {
        0xf5f2f2 => palette.background,
        0x0a0a0a => palette.foreground,
        0xffffff => palette.card,
        0x171717 => palette.primary,
        0xfafafa => palette.foreground,
        0xf5f5f5 => palette.secondary,
        0x737373 => palette.muted_foreground,
        0x6b6b6b => palette.muted_foreground,
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
        // Limbus skin surfaces: pages hardcoding the modern dark tokens
        // keep following the active palette after a skin switch.
        0x0b0a0e => palette.background,
        0xd8d0bc => palette.foreground,
        0x1b1114 => palette.card,
        0x241619 => palette.popover,
        0x2a1b1e => palette.secondary,
        0x968a70 => palette.muted_foreground,
        0x3a2023 => palette.accent_surface,
        0xe8c43c => palette.warning,
        0x462c2c => palette.border,
        0x6b4a1a => palette.input,
        0xd8a800 => palette.ring,
        0xb92828 => palette.danger,
        0x3a1414 => palette.danger_light,
        0x3a2e14 => palette.warning_light,
        0x202936 | 0x354152 => palette.muted,
        0x1b222d => palette.card,
        0x3d301b | 0x5d4820 => palette.brand_light,
        0x1d513b => palette.success_light,
        0x8de3b2 | 0x4dcc89 => palette.success,
        // Legacy Teams controls used an amber accent even before accent
        // presets were wired. Treat this as the current semantic brand color.
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
    let masked = hex & 0x00ff_ffff;
    if (TAG_FIRST..=TAG_LAST).contains(&masked) {
        return token_for_tag(masked, palette)
            .map(|token| gpui::rgba(token.rgba_hex()))
            .unwrap_or_else(|| gpui::rgb(hex));
    }
    token_for_legacy_hex(masked, palette)
        .map(|token| gpui::rgba(token.rgba_hex()))
        .unwrap_or_else(|| gpui::rgb(masked))
}

/// Resolve an RGBA literal while preserving the caller's alpha. This is used
/// for translucent card overlays such as `card/30` and `brand/20`.
pub fn render_rgba(value: u32) -> gpui::Rgba {
    let palette = current_render_palette();
    let alpha = value as u8;
    let rgb = (value >> 8) & 0x00ff_ffff;
    if (TAG_FIRST..=TAG_LAST).contains(&rgb) {
        return token_for_tag(rgb, palette)
            .map(|token| gpui::rgba((token.rgb_hex() << 8) | u32::from(alpha)))
            .unwrap_or_else(|| gpui::rgba(value));
    }
    token_for_legacy_hex(rgb, palette)
        .map(|token| gpui::rgba((token.rgb_hex() << 8) | u32::from(alpha)))
        .unwrap_or_else(|| gpui::rgba(value))
}

/// Convert a palette token into an `Hsla` for APIs that need one (shadows).
pub fn palette_hsla(token: ColorToken) -> Hsla {
    gpui::rgba(token.rgba_hex()).into()
}

pub const SPACE_1: f32 = 4.0;
pub const SPACE_2: f32 = 8.0;
pub const SPACE_3: f32 = 12.0;
pub const SPACE_4: f32 = 16.0;
pub const SPACE_5: f32 = 20.0;
pub const SPACE_6: f32 = 24.0;

/// The type scale. Six steps replace the 17 ad-hoc sizes that had accumulated
/// in page code, which is what made the visual rhythm inconsistent.
pub const FONT_2XS: f32 = 9.5;
pub const FONT_XS: f32 = 10.0;
pub const FONT_SM: f32 = 11.0;
pub const FONT_MD: f32 = 12.0;
pub const FONT_LG: f32 = 14.0;
pub const FONT_XL: f32 = 16.0;

pub const RADIUS_SM: f32 = 4.0;
pub const RADIUS_MD: f32 = 6.0;
pub const RADIUS_LG: f32 = 8.0;
pub const RADIUS_XL: f32 = 12.0;

/// Convenience re-export so page code can pull a size without importing
/// `Pixels` helpers directly.
pub fn px_value(value: f32) -> Pixels {
    px(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::style::{AccentId as Accent, ColorScheme, SkinId};

    /// The tag range must stay inside the 24-bit window `render_rgb` masks,
    /// and must not overlap any literal the transition table resolves.
    #[test]
    fn compatibility_tags_are_unique_and_reserved() {
        let tags = [
            BACKGROUND,
            SURFACE,
            SURFACE_HOVER,
            BORDER,
            TEXT,
            TEXT_MUTED,
            ACCENT,
            GREEN,
            DANGER,
        ];
        for (index, tag) in tags.iter().enumerate() {
            assert_eq!(tag & 0x00ff_ffff, *tag, "tag escapes the 24-bit mask");
            assert!(
                (TAG_FIRST..=TAG_LAST).contains(&(tag & 0x00ff_ffff)),
                "tag outside the reserved range"
            );
            for other in tags.iter().skip(index + 1) {
                assert_ne!(tag, other, "duplicate compatibility tag");
            }
        }
        assert_eq!(tags.len(), (TAG_LAST - TAG_FIRST + 1) as usize);
    }

    #[test]
    fn tagged_constants_resolve_for_every_skin() {
        for skin in SkinId::ALL {
            for scheme in [ColorScheme::Light, ColorScheme::Dark] {
                for accent in Accent::ALL {
                    let palette = Palette::for_skin(scheme, accent, skin);
                    set_current_render_palette(palette);
                    for tag in [
                        BACKGROUND,
                        SURFACE,
                        SURFACE_HOVER,
                        BORDER,
                        TEXT,
                        TEXT_MUTED,
                        ACCENT,
                        GREEN,
                        DANGER,
                    ] {
                        let resolved = render_rgb(tag);
                        assert!(
                            resolved.a > 0.0,
                            "tag {tag:#x} resolved transparent under {skin:?}/{scheme:?}/{accent:?}"
                        );
                    }
                    // Alpha must survive the tagged path.
                    assert!(
                        render_rgba((SURFACE << 8) | 0x59).a < 0.5,
                        "tag path dropped the caller's alpha"
                    );
                }
            }
        }
    }

    #[test]
    fn unknown_literals_pass_through_untouched() {
        set_current_render_palette(Palette::default());
        assert_eq!(render_rgb(0x123456), gpui::rgb(0x123456));
        assert_eq!(render_rgba(0x12345680), gpui::rgba(0x12345680));
    }
}
