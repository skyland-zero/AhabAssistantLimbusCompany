#![allow(dead_code)]

//! Small, page-agnostic GPUI controls used by the shell and pages.
//!
//! The short functions (`button`, `card`, and friends) are compatibility
//! constructors for the existing pages. New pages should pass a derived
//! [`Palette`] to the `*_with_palette` constructors and use [`ControlState`]
//! for disabled/loading/focus-visible rendering.

pub mod action_button;
pub mod base;
pub mod chrome;
pub mod controls;
pub mod icon;
pub mod keyboard;
pub mod layout;
pub mod overlays;
pub mod style;
pub mod text_input;

pub use action_button::action_button;
pub use base::*;
pub use chrome::*;
pub use controls::*;
pub use icon::{svg_icon, svg_icon_bytes};
pub use keyboard::is_activation_key;
pub use layout::{page_root, page_toolbar, settings_grid};
pub use overlays::*;
#[allow(unused_imports)]
pub use style::{
    Decor, ShadowLevel, Shape, ShapeExt, apply_card_shadow, current_render_palette, palette_hsla,
    palette_rgb, render_rgb, render_rgba, shape_rounded, skin_rounded,
};
pub use text_input::TextInput;

use gpui::{Div, Rgba, Stateful, div, prelude::*, px};

use icon::{Icon, icon};
use style::{AccentId, ColorToken, Palette};

fn paint_color(token: ColorToken) -> Rgba {
    gpui::rgba(token.rgba_hex())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::style::{
        ACCENT, AccentId, BACKGROUND, BORDER, DANGER, GREEN, SURFACE, SURFACE_HOVER, TEXT,
        TEXT_MUTED,
    };

    #[test]
    fn slider_normalization_is_bounded() {
        assert_eq!(normalize_slider(-1.0, 0.0, 10.0), 0.0);
        assert_eq!(normalize_slider(5.0, 0.0, 10.0), 0.5);
        assert_eq!(normalize_slider(11.0, 0.0, 10.0), 1.0);
        assert_eq!(normalize_slider(5.0, 1.0, 1.0), 0.0);
    }

    #[test]
    fn number_stepper_normalization_is_bounded() {
        assert_eq!(clamp_number(-2, 0, 5), 0);
        assert_eq!(clamp_number(3, 0, 5), 3);
        assert_eq!(clamp_number(9, 0, 5), 5);
        assert_eq!(clamp_number(9, 5, 0), 5);
    }

    #[test]
    fn controls_have_stable_defaults_and_accents() {
        assert_eq!(ButtonVariant::default(), ButtonVariant::Default);
        assert_eq!(BadgeTone::default(), BadgeTone::Neutral);
        assert!(ControlState::loading().is_inert());
        assert_eq!(parse_accent_id("violet"), AccentId::Violet);
    }

    #[test]
    fn compatibility_constants_are_reserved_tags_not_colors() {
        // The tagged constants must never be mistaken for a real sRGB value:
        // render_rgb is the only thing that gives them meaning.
        let palette = Palette::default();
        crate::components::style::set_current_render_palette(palette);
        assert_eq!(
            render_rgb(BACKGROUND),
            gpui::rgba(palette.background.rgba_hex())
        );
        assert_eq!(render_rgb(SURFACE), gpui::rgba(palette.card.rgba_hex()));
        assert_eq!(render_rgb(BORDER), gpui::rgba(palette.input.rgba_hex()));
        assert_eq!(render_rgb(ACCENT), gpui::rgba(palette.brand.rgba_hex()));
        assert_eq!(render_rgb(GREEN), gpui::rgba(palette.success.rgba_hex()));
        assert_eq!(render_rgb(DANGER), gpui::rgba(palette.danger.rgba_hex()));
        assert_eq!(
            render_rgb(SURFACE_HOVER),
            gpui::rgba(palette.secondary.rgba_hex())
        );
        assert_eq!(render_rgb(TEXT), gpui::rgba(palette.foreground.rgba_hex()));
        assert_eq!(
            render_rgb(TEXT_MUTED),
            gpui::rgba(palette.muted_foreground.rgba_hex())
        );
        // The old dark-only prototype background must stay unrepresentable.
        assert_ne!(render_rgb(BACKGROUND), gpui::rgb(0x0f141c));
    }
}
