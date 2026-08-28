#![allow(dead_code)]

//! Small inline Lucide-compatible icons used by reusable controls.
//!
//! GPUI does not have a browser DOM in which to mount the Lucide React
//! components. These paths keep the same SVG viewBox/stroke contract while
//! allowing controls to render the icon from embedded bytes. No Unicode or
//! emoji glyph is used as an icon fallback.

use gpui::{Pixels, Rgba, Styled, Svg, px, svg};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Icon {
    ChevronDown,
    LoaderCircle,
    Check,
    X,
}

impl Icon {
    const fn data(self) -> &'static [u8] {
        match self {
            Self::ChevronDown => {
                br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>"#
            }
            Self::LoaderCircle => {
                br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 1 1-6.219-8.56"/></svg>"#
            }
            Self::Check => {
                br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m5 12 4 4L19 6"/></svg>"#
            }
            Self::X => {
                br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M18 6 6 18M6 6l12 12"/></svg>"#
            }
        }
    }
}

/// Render an embedded Lucide path at a fixed square size.
pub fn icon(kind: Icon, size: Pixels, color: Rgba) -> Svg {
    svg().data(kind.data()).w(size).h(size).text_color(color)
}

pub fn chevron_down(color: Rgba) -> Svg {
    icon(Icon::ChevronDown, px(14.), color)
}

pub fn loader_circle(color: Rgba) -> Svg {
    icon(Icon::LoaderCircle, px(14.), color)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_paths_use_the_lucide_viewbox() {
        let source = std::str::from_utf8(Icon::ChevronDown.data()).unwrap();
        assert!(source.contains("viewBox=\"0 0 24 24\""));
        assert!(source.contains("stroke=\"currentColor\""));
    }
}
