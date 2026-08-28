use super::*;

use gpui::{Context, Div, KeyDownEvent, Render, Window, div, px};

use crate::components::{BadgeTone, badge, card, slider};

struct ThemeWeightDragGhost;

impl Render for ThemeWeightDragGhost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w(px(1.)).h(px(1.))
    }
}

pub(super) fn pack_row(
    _app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    pack: ThemePack,
    language: Language,
) -> Div {
    let enabled = pack.enabled;
    let id = pack.id.clone();
    let mut toggle = switch(enabled).id(format!("theme-enabled-{id}"));
    let id_for_click = id.clone();
    toggle = toggle.on_click(cx.listener(move |view, _, _, cx| {
        view.theme_packs.toggle_enabled(&id_for_click);
        cx.notify();
    }));
    let id_for_key = id.clone();
    toggle = toggle.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
        if is_activation_key(event) {
            window.prevent_default();
            view.theme_packs.toggle_enabled(&id_for_key);
            cx.notify();
        }
    }));

    let id_for_slider = pack.id.clone();
    let mut weight_slider = slider(pack.weight as f32, 0., 10.)
        .id(format!("theme-weight-slider-{}", pack.id))
        .flex_1()
        .min_w(px(80.))
        .min_h(px(16.));
    if enabled {
        let id_for_key = id_for_slider.clone();
        let id_for_drag = id_for_slider.clone();
        weight_slider = weight_slider
            .on_click(cx.listener(move |view, _, _, cx| {
                view.theme_packs.cycle_weight(&id_for_slider);
                cx.notify();
            }))
            .on_drag(ThemeWeightDragGhost, |_, _, _, cx| {
                cx.new(|_| ThemeWeightDragGhost)
            })
            .on_drag_move(cx.listener(
                move |view, event: &gpui::DragMoveEvent<ThemeWeightDragGhost>, _, cx| {
                    let width = event.bounds.size.width.as_f32();
                    let position = (event.event.position.x - event.bounds.left()).as_f32();
                    let weight = slider_weight_from_position(position, width);
                    view.theme_packs.set_weight(&id_for_drag, weight);
                    cx.notify();
                },
            ))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                let delta = match event.keystroke.key.to_ascii_lowercase().as_str() {
                    "left" | "arrowleft" => Some(-1),
                    "right" | "arrowright" => Some(1),
                    "home" => Some(-10),
                    "end" => Some(10),
                    _ => None,
                };
                if let Some(delta) = delta {
                    window.prevent_default();
                    view.theme_packs.adjust_weight(&id_for_key, delta);
                    cx.notify();
                }
            }));
    } else {
        weight_slider = weight_slider.opacity(0.5).cursor_not_allowed();
    }

    let card_content = div()
        .flex()
        .items_center()
        .gap_4()
        .px_5()
        .py_3()
        .child(toggle)
        .child(
            div().w(px(176.)).min_w_0().flex_none().child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .min_w_0()
                            .text_size(px(14.))
                            .text_color(rgb(TEXT))
                            .child(pack.name),
                    )
                    .child(badge(pack.tier, BadgeTone::Neutral)),
            ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .min_w_0()
                .flex_1()
                .child(
                    div()
                        .flex_none()
                        .text_size(px(12.))
                        .text_color(rgb(TEXT_MUTED))
                        .child(text("权重", "Weight").get(language)),
                )
                .child(weight_slider)
                .child(
                    div()
                        .w(px(24.))
                        .flex_none()
                        .font_family("Consolas")
                        .text_size(px(12.))
                        .text_color(rgb(TEXT))
                        .child(pack.weight.to_string()),
                ),
        );

    let mut row = card(card_content).p_0();
    if !enabled {
        row = row.opacity(0.6);
    }
    row
}

pub(super) fn slider_weight_from_position(position: f32, width: f32) -> u8 {
    if !position.is_finite() || !width.is_finite() || width <= 0. {
        return 0;
    }
    ((position / width).clamp(0., 1.) * 10.).round() as u8
}
