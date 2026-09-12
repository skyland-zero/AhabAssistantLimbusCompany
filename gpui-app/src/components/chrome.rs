//! Card chrome and per-skin decoration language.
//!
//! Split out of `base.rs`: the surface primitives there answer "what is a
//! card", while this module answers "how does this skin decorate one"
//! (brackets, stencilled bands, rules and dividers). Both sides use the same
//! `Palette`, so a skin switch cannot desynchronise them.

use gpui::{FontWeight, linear_color_stop, linear_gradient};

use super::style::FONT_LG;
use super::*;

/// Shared card-header height. Every decoration language uses the same slot so
/// switching skins cannot shift the layout below the header.
pub const CARD_HEADER_HEIGHT: f32 = 44.0;

/// Accent-toned L-brackets pinned to the four corners of a framed surface.
///
/// The brackets are absolutely positioned, so they decorate the frame
/// without moving any content or changing the surface's box. They are only
/// rendered for skins whose decoration language is
/// [`Decor::LimbusFrame`](super::style::Decor::LimbusFrame), and they take
/// `decor_line` so a non-brass accent never fights a brass frame.
pub fn frame_corner_brackets(palette: &Palette) -> [Div; 4] {
    let tone = paint_color(palette.decor_line);
    let bracket = |top: bool, left: bool| {
        let mut corner = div().absolute().w(px(13.)).h(px(13.));
        corner = if top {
            corner.top(px(3.))
        } else {
            corner.bottom(px(3.))
        };
        corner = if left {
            corner.left(px(3.))
        } else {
            corner.right(px(3.))
        };
        corner = if top {
            corner.border_t_2()
        } else {
            corner.border_b_2()
        };
        corner = if left {
            corner.border_l_2()
        } else {
            corner.border_r_2()
        };
        corner.border_color(tone)
    };
    [
        bracket(true, true),
        bracket(true, false),
        bracket(false, true),
        bracket(false, false),
    ]
}

/// The dark-red (or parchment) band behind a limbus card title, plus the
/// splatter marks that used to come from a stretched bitmap.
///
/// Every mark is positioned in fixed pixels, so the band cannot distort as the
/// window resizes: the old `tagband.png` was stretched to the full card width
/// and turned its rivets and smears into ellipses.
fn limbus_band(palette: &Palette) -> Div {
    let dark = palette.is_dark();
    let (top, bottom) = if dark {
        (
            palette_rgb(crate::components::style::LIMBUS_BAND_TOP),
            palette_rgb(crate::components::style::LIMBUS_BAND_BOTTOM),
        )
    } else {
        (
            palette_rgb(crate::components::style::LIMBUS_BAND_TOP_LIGHT),
            palette_rgb(crate::components::style::LIMBUS_BAND_BOTTOM_LIGHT),
        )
    };
    // Black smears read as grime over the dark-red band, but over parchment
    // they read as opaque redaction bars. The light scheme uses translucent
    // oxblood ink so the marks stay a texture instead of a solid block.
    let smear = if dark {
        gpui::rgba(0x00000059)
    } else {
        gpui::rgba(0x4a101029)
    };
    let mut band = div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .h(px(CARD_HEADER_HEIGHT))
        .bg(linear_gradient(
            180.0,
            linear_color_stop(top, 0.0),
            linear_color_stop(bottom, 1.0),
        ));
    // Alternating left/right anchored smears keep both edges decorated at any
    // card width instead of relying on a centre-out layout.
    for (left, top, width, height) in [
        (Some(56.0), 9.0, 34.0, 8.0),
        (Some(132.0), 27.0, 16.0, 5.0),
        (None, 13.0, 28.0, 7.0),
        (None, 30.0, 44.0, 6.0),
    ] {
        let mut mark = div()
            .absolute()
            .top(px(top))
            .w(px(width))
            .h(px(height))
            .rounded_full()
            .bg(smear);
        mark = match left {
            Some(left) => mark.left(px(left)),
            None => mark.right(px(width + if width > 30.0 { 88.0 } else { 24.0 })),
        };
        band = band.child(mark);
    }
    band
}

/// Frame-language header used by every card-level surface.
///
/// The label slot is identical in height for every skin, so a skin switch
/// changes the decoration without moving the body below it.
pub fn card_header(label: impl Into<String>, palette: &Palette) -> Div {
    let label = label.into();
    let title_size = px(FONT_LG);
    match palette.decor {
        Decor::LimbusFrame => div()
            .relative()
            .w_full()
            .h(px(CARD_HEADER_HEIGHT))
            .flex_none()
            .overflow_hidden()
            .child(limbus_band(palette))
            .child(
                div()
                    .absolute()
                    .top(px(0.))
                    .bottom(px(0.))
                    .left(px(14.))
                    .w(px(3.))
                    .bg(paint_color(palette.decor_line)),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .flex()
                    .items_center()
                    .px_4()
                    .text_size(title_size)
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(paint_color(palette.accent_foreground))
                    .child(label),
            )
            .child(
                div()
                    .absolute()
                    .bottom_0()
                    .left_0()
                    .right_0()
                    .h(px(1.))
                    .bg(paint_color(palette.decor_line)),
            ),
        Decor::Mist => div()
            .relative()
            .w_full()
            .h(px(CARD_HEADER_HEIGHT))
            .flex_none()
            .flex()
            .items_center()
            .px_4()
            .border_b_1()
            .border_color(paint_color(palette.border))
            .text_size(title_size)
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(paint_color(palette.foreground))
            .child(
                div()
                    .absolute()
                    .left(px(0.))
                    .top(px(0.))
                    .bottom(px(0.))
                    .w(px(3.))
                    .bg(paint_color(palette.decor_line)),
            )
            .child(label),
        Decor::Archive => div()
            .w_full()
            .h(px(CARD_HEADER_HEIGHT))
            .flex_none()
            .flex()
            .items_center()
            .gap_3()
            .px_4()
            .border_b_1()
            .border_color(paint_color(palette.border))
            .text_size(title_size)
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(paint_color(palette.foreground))
            .child(label)
            .child(
                div()
                    .flex_1()
                    .h(px(1.))
                    .bg(paint_color(palette.decor_line))
                    .opacity(0.35),
            ),
        Decor::Glass => div()
            .w_full()
            .h(px(CARD_HEADER_HEIGHT))
            .flex_none()
            .flex()
            .items_center()
            .px_4()
            .border_t_1()
            .border_color(paint_color(palette.hilite))
            .text_size(title_size)
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(paint_color(palette.foreground))
            .child(label),
        Decor::Plain => div()
            .w_full()
            .h(px(CARD_HEADER_HEIGHT))
            .flex_none()
            .flex()
            .items_center()
            .px_4()
            .border_b_1()
            .border_color(paint_color(palette.input))
            .text_size(title_size)
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(paint_color(palette.foreground))
            .child(label),
    }
}

/// A hairline rule that follows the skin's rule language.
pub fn rule(palette: &Palette) -> Div {
    let mut line = div()
        .w_full()
        .h(px(1.))
        .bg(paint_color(if palette.uses_rule_decor() {
            palette.decor_line
        } else {
            palette.border
        }));
    if palette.uses_rule_decor() {
        line = line.opacity(0.6);
    }
    line
}

/// Decorative section rule.
///
/// The ruled skins draw the shipped gold flourish, always at the source art's
/// aspect ratio and at a fixed width, so it can never stretch. Every other
/// skin draws a plain hairline in the same slot.
pub fn themed_divider(palette: &Palette) -> Div {
    if !palette.uses_rule_decor() {
        return rule(palette);
    }
    let asset = crate::assets::ThemeAsset::Divider;
    let (natural_width, natural_height) = asset.natural_size();
    // 120 px wide keeps the flourish legible without turning it into a
    // full-width band; the height follows from the natural ratio.
    let width = 120.0;
    let height = width * natural_height / natural_width;
    div().w_full().flex().justify_center().py_1().child(
        gpui::img(crate::assets::image_source(crate::assets::theme(asset)))
            .w(px(width))
            .h(px(height))
            .opacity(0.85),
    )
}
