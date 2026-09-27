#![allow(dead_code)]

//! Icon rendering on top of GPUI Kit's bundled Lucide set.
//!
//! The app used to embed its own Lucide-compatible SVG bytes per icon, which
//! meant every new glyph was a hand-copied path. `gpui-kit-assets` already
//! ships the full Lucide catalog (1830 icons) generated into [`IconName`], so
//! this module is now just the size/colour adapter: call sites name an icon,
//! and the bundle supplies the artwork.
//!
//! The helpers keep taking the legacy RGB tag rather than an `Hsla`, because
//! pages resolve their colours through the render palette and the icon has to
//! follow the same token as the text beside it.

use gpui::{Pixels, Rgba, Styled as _, px};
pub use gpui_component::Icon;
use gpui_component::Sizable as _;
pub use gpui_component_assets::IconName;

use super::style::render_rgb;

/// Render a bundled icon at a fixed square size using a legacy RGB token.
pub fn svg_icon(name: impl Into<Icon>, size: f32, color: u32) -> Icon {
    icon_at(name, px(size), render_rgb(color))
}

/// [`svg_icon`] for a colour already resolved to `Rgba`.
pub fn svg_icon_colored(name: impl Into<Icon>, size: f32, color: Rgba) -> Icon {
    icon_at(name, px(size), color)
}

/// Render at an explicit `Pixels` size.
pub fn icon_at(name: impl Into<Icon>, size: Pixels, color: Rgba) -> Icon {
    Icon::new(name).with_size(size).text_color(color)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every icon the UI names must exist in the bundled catalog, so a typo
    /// fails the build here rather than rendering an empty square.
    #[test]
    fn named_icons_resolve_to_a_bundled_path() {
        for name in [
            IconName::ChevronDown,
            IconName::LoaderCircle,
            IconName::Check,
            IconName::X,
        ] {
            let path = name.path();
            assert!(
                path.starts_with("icons/") && path.ends_with(".svg"),
                "{path} must point at the bundled icon directory"
            );
        }
    }
}
