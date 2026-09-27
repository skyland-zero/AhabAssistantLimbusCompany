//! Shell icon lookup.
//!
//! These were previously embedded Lucide SVG bytes; they now name entries in
//! the `gpui-kit-assets` catalog so the shell shares one artwork source with
//! every page. The catalog carries the current Lucide revision, so `Home`
//! resolves to `house.svg` (Lucide's rename of `home`) - the same glyph.

use gpui::Styled as _;
use gpui::px;
use gpui_component::{Icon, Sizable as _};
use gpui_component_assets::IconName;

/// Icons the window shell draws. Kept as a small enum so the title bar does not
/// import the whole catalog and every glyph it uses stays reviewable in one
/// place.
#[derive(Clone, Copy)]
pub(super) enum ShellIcon {
    Home,
    Users,
    Palette,
    Wrench,
    Package,
    Help,
    Sun,
    Moon,
    Monitor,
    Settings,
    Minus,
    Square,
    Restore,
    Close,
}

impl ShellIcon {
    const fn name(self) -> IconName {
        match self {
            Self::Home => IconName::House,
            Self::Users => IconName::Users,
            Self::Palette => IconName::Palette,
            Self::Wrench => IconName::Wrench,
            Self::Package => IconName::Package,
            Self::Help => IconName::CircleQuestionMark,
            Self::Sun => IconName::Sun,
            Self::Moon => IconName::Moon,
            Self::Monitor => IconName::Monitor,
            Self::Settings => IconName::Settings,
            Self::Minus => IconName::Minus,
            Self::Square => IconName::Square,
            Self::Restore => IconName::WindowRestore,
            Self::Close => IconName::Close,
        }
    }
}

pub(super) fn icon(kind: ShellIcon, size: f32) -> Icon {
    // No explicit colour on purpose: GPUI Kit falls back to the window text
    // colour, which is how these icons inherited `currentColor` before.
    Icon::new(kind.name()).with_size(px(size)).flex_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shell_icon_resolves_to_a_bundled_asset() {
        for kind in [
            ShellIcon::Home,
            ShellIcon::Users,
            ShellIcon::Palette,
            ShellIcon::Wrench,
            ShellIcon::Package,
            ShellIcon::Help,
            ShellIcon::Sun,
            ShellIcon::Moon,
            ShellIcon::Monitor,
            ShellIcon::Settings,
            ShellIcon::Minus,
            ShellIcon::Square,
            ShellIcon::Restore,
            ShellIcon::Close,
        ] {
            let path = kind.name().path();
            assert!(
                path.starts_with("icons/") && path.ends_with(".svg"),
                "{path} must point into the bundled icon directory"
            );
        }
    }
}
