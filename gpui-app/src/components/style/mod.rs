#![allow(dead_code)]

//! Design tokens shared by the native GPUI controls.
//!
//! The values in this module are the sRGB values obtained from the design
//! token definitions. Keeping the conversion at the boundary means pages
//! never need to approximate an `oklch` color themselves.
//! [`Palette`] is a value object: callers derive a new value from persisted
//! settings instead of mutating a second theme state.

mod palette;
mod runtime;
mod tokens;

pub use palette::Palette;
#[allow(unused_imports)]
pub use runtime::{
    ACCENT, BACKGROUND, BORDER, DANGER, FONT_LG, FONT_MD, FONT_SM, FONT_XL, FONT_XS, GREEN,
    RADIUS_LG, RADIUS_MD, RADIUS_SM, RADIUS_XL, SPACE_1, SPACE_2, SPACE_3, SPACE_4, SPACE_5,
    SPACE_6, SURFACE, SURFACE_HOVER, TEXT, TEXT_MUTED, current_render_palette, palette_rgb,
    render_rgb, render_rgba, set_current_render_palette,
};
#[allow(unused_imports)]
pub use tokens::{ACCENT_PRESETS, AccentId, AccentPreset, AccentTokens, ColorScheme, ColorToken};

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
