//! Live skin previews for the appearance card.
//!
//! Split out of `appearance.rs`: rendering a miniature of the application
//! under one skin is a self-contained concern, and keeping it separate stops
//! the settings card from growing a second renderer.

use super::*;
use crate::components::style::{AccentId, ColorToken, Decor, FONT_MD, FONT_SM, ShapeExt, SkinId};
use crate::theme::Palette;
use gpui::{AnyView, App, Render, Window};

struct AccentTooltip {
    label: String,
    palette: Palette,
}

impl Render for AccentTooltip {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .skin_rounded(&self.palette, false)
            .border_1()
            .border_color(palette_rgb(self.palette.input))
            .bg(palette_rgb(self.palette.popover))
            .text_color(palette_rgb(self.palette.popover_foreground))
            .text_size(px(FONT_SM))
            .child(self.label.clone())
    }
}

pub(super) fn accent_tooltip(
    label: String,
    palette: Palette,
) -> impl Fn(&mut Window, &mut App) -> AnyView {
    move |_window, cx| {
        cx.new(|_| AccentTooltip {
            label: label.clone(),
            palette,
        })
        .into()
    }
}

/// A code-drawn miniature of the application under one skin.
///
/// Rendering the preview from the real [`Palette`] instead of a screenshot
/// means it always reflects the current accent and colour scheme, and it costs
/// no asset bytes. The wireframe is deliberately abstract (bar, two panels,
/// one control) so it reads as a preview rather than as a stale screenshot.
fn skin_preview(palette: Palette) -> Div {
    let line = |width: f32, height: f32, color: ColorToken, opacity: f32| {
        div()
            .w(px(width))
            .h(px(height))
            .skin_radius_sm(&palette)
            .bg(palette_rgb(color))
            .opacity(opacity)
    };

    // Header strip mirroring `card_header`: limbus paints its band, mist marks
    // the title with a left accent bar, archive draws a short rule, and the
    // flat/glass skins only carry the hairline. Snapshotting the real header
    // language is what makes the preview readable as "this is that skin".
    let mut header_strip = match palette.decor {
        Decor::LimbusFrame => div().w_full().h(px(6.)).bg(gpui_rgb(if palette.is_dark() {
            0x3e0d0d
        } else {
            0xe6dcc8
        })),
        Decor::Glass => div().w_full().h(px(6.)).bg(palette_rgb(palette.hilite)),
        Decor::Mist => div()
            .flex()
            .w_full()
            .h(px(6.))
            .child(div().w(px(3.)).h_full().bg(palette_rgb(palette.decor_line)))
            .child(div().flex_1().h_full().bg(palette_rgb(palette.border))),
        Decor::Archive => div().flex().items_center().w_full().h(px(6.)).child(
            div()
                .w(px(22.))
                .h(px(3.))
                .bg(palette_rgb(palette.decor_line)),
        ),
        Decor::Plain => div().w_full().h(px(6.)).bg(palette_rgb(palette.border)),
    };
    if palette.decor == Decor::LimbusFrame {
        header_strip = header_strip
            .border_b_1()
            .border_color(palette_rgb(palette.decor_line));
    }

    let mut left_panel = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(px(3.))
        .p(px(4.))
        .bg(palette_rgb(palette.card));
    left_panel = left_panel.skin_rounded(&palette, false);
    if !palette.border.is_transparent() {
        left_panel = left_panel
            .border_1()
            .border_color(palette_rgb(palette.border));
    }
    left_panel = left_panel
        .child(header_strip)
        .child(line(0., 3., palette.muted_foreground, 0.55))
        .child(line(0., 3., palette.muted_foreground, 0.35));

    let mut right_panel = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .justify_between()
        .gap(px(3.))
        .p(px(4.))
        .bg(palette_rgb(palette.secondary));
    right_panel = right_panel.skin_rounded(&palette, false);
    right_panel = right_panel
        .child(line(0., 3., palette.secondary_foreground, 0.5))
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(3.))
                .child(
                    div()
                        .w(px(18.))
                        .h(px(10.))
                        .rounded_full()
                        .bg(palette_rgb(palette.primary)),
                )
                .child(line(0., 3., palette.muted_foreground, 0.4)),
        );

    let preview = div()
        .w_full()
        .h(px(78.))
        .flex()
        .flex_col()
        .gap(px(4.))
        .p(px(5.))
        .bg(palette_rgb(palette.background))
        .child(
            // Mini title bar with the skin's rule, a brand chip and nav ticks.
            div()
                .flex()
                .items_center()
                .gap(px(3.))
                .h(px(12.))
                .px(px(4.))
                .bg(palette_rgb(palette.card))
                .border_b_1()
                .border_color(palette_rgb(palette.decor_line))
                .child(
                    div()
                        .w(px(10.))
                        .h(px(6.))
                        .skin_radius_sm(&palette)
                        .bg(palette_rgb(palette.brand)),
                )
                .child(line(0., 3., palette.muted_foreground, 0.5))
                .child(line(0., 3., palette.muted_foreground, 0.5)),
        )
        .child(
            div()
                .flex()
                .flex_1()
                .gap(px(4.))
                .child(left_panel)
                .child(right_panel),
        );

    let mut frame = div()
        .w_full()
        .p(px(3.))
        .bg(palette_rgb(palette.card))
        .child(preview);
    frame = frame.skin_rounded(&palette, true);
    frame = frame.border_1().border_color(palette_rgb(palette.border));
    frame.child(
        div()
            .w_full()
            .flex()
            .items_center()
            .justify_center()
            .gap(px(3.))
            .pt(px(3.))
            .child(
                div()
                    .w(px(14.))
                    .h(px(4.))
                    .skin_radius_sm(&palette)
                    .bg(palette_rgb(palette.brand)),
            )
            .child(
                div()
                    .w(px(26.))
                    .h(px(4.))
                    .skin_radius_sm(&palette)
                    .bg(palette_rgb(palette.brand_hover)),
            ),
    )
}

/// Grid of clickable skin previews. Replaces the previous pair of text
/// buttons, which gave no hint of what "边狱" or "档案" would look like.
pub(super) fn skin_picker(
    language: Language,
    accent: &str,
    scheme: ColorScheme,
    current: &str,
    cx: &mut Context<AhabApp>,
) -> Div {
    let mut grid = div().flex().flex_wrap().gap_3().w_full();
    for skin in SkinId::ALL {
        let id = skin.as_str();
        let selected = current == id;
        let palette = Palette::for_skin(scheme, AccentId::parse(accent), skin);
        let label = match language {
            Language::ZhCn => skin.name_zh(),
            Language::EnUs => skin.name_en(),
        };
        let blurb = match language {
            Language::ZhCn => skin.blurb_zh(),
            Language::EnUs => skin.blurb_en(),
        };
        let message = format!(
            "{}: {label}",
            text("界面皮肤", "Interface Skin").get(language)
        );
        let key_message = message.clone();

        let mut card = div()
            .id(format!("settings-skin-{id}"))
            .flex()
            .flex_col()
            .gap_2()
            .flex_1()
            .min_w(px(176.))
            .p_2()
            .bg(palette_rgb(palette.background))
            .tab_index(0)
            .cursor_pointer()
            .aria_label(format!("{label} / {}", skin.name_en()))
            .tooltip(accent_tooltip(format!("{label} · {blurb}"), palette))
            .on_click(cx.listener(move |view, _, _, cx| {
                view.set_skin(id);
                view.show_toast(crate::shell::ToastKind::Info, message.clone(), cx);
                cx.notify();
            }))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    view.set_skin(id);
                    view.show_toast(crate::shell::ToastKind::Info, key_message.clone(), cx);
                    cx.notify();
                }
            }))
            .child(skin_preview(palette))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_size(px(FONT_MD))
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_color(palette_rgb(palette.foreground))
                                    .child(label),
                            )
                            .child(if selected {
                                div()
                                    .text_size(px(FONT_SM))
                                    .text_color(palette_rgb(palette.brand))
                                    .child(text("已启用", "Active").get(language))
                            } else {
                                div()
                            }),
                    )
                    .child(
                        div()
                            .text_size(px(FONT_SM))
                            .text_color(palette_rgb(palette.muted_foreground))
                            .child(blurb),
                    ),
            );

        card = card.skin_rounded(&palette, true);
        card = if selected {
            card.border_2().border_color(palette_rgb(palette.ring))
        } else {
            card.border_1().border_color(palette_rgb(palette.border))
        };
        let hover_bg = palette_rgb(palette.secondary);
        card = card.hover(move |style| style.bg(hover_bg));
        grid = grid.child(card);
    }
    grid
}
