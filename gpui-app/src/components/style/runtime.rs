use std::cell::Cell;

use super::{AccentId, ColorToken, Palette};

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
