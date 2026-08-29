#![allow(dead_code)]

//! Small, page-agnostic GPUI controls used by the shell and pages.
//!
//! The short functions (`button`, `card`, and friends) are compatibility
//! constructors for the existing pages. New pages should pass a derived
//! [`Palette`] to the `*_with_palette` constructors and use [`ControlState`]
//! for disabled/loading/focus-visible rendering.

pub mod action_button;
pub mod base;
pub mod controls;
pub mod icon;
pub mod keyboard;
pub mod layout;
pub mod overlays;
pub mod smooth_scroll;
pub mod style;
pub mod text_input;

pub use action_button::action_button;
pub use base::*;
pub use controls::*;
pub use icon::{svg_icon, svg_icon_bytes};
pub use keyboard::is_activation_key;
pub use layout::settings_grid;
pub use overlays::*;
pub use style::{current_render_palette, palette_rgb, render_rgb, render_rgba};
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
    fn compatibility_palette_is_not_the_old_dark_only_palette() {
        assert_eq!(BACKGROUND, Palette::default().background.rgb_hex());
        assert_eq!(SURFACE, Palette::default().card.rgb_hex());
        assert_eq!(BORDER, Palette::default().input.rgb_hex());
        assert_ne!(BACKGROUND, 0x0f141c);
        assert_eq!(ACCENT, Palette::default().brand.rgb_hex());
        assert_eq!(GREEN, Palette::default().success.rgb_hex());
        assert_eq!(DANGER, Palette::default().danger.rgb_hex());
        assert_eq!(SURFACE_HOVER, Palette::default().secondary.rgb_hex());
        assert_eq!(TEXT, Palette::default().foreground.rgb_hex());
        assert_eq!(TEXT_MUTED, Palette::default().muted_foreground.rgb_hex());
    }
}
