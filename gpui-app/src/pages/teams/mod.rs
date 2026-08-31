//! Teams list and the five-section native team editor (P5).
//!
//! The React page used a modal with a large form. GPUI has no browser dialog
//! dependency here, so the same interaction is rendered as an application
//! overlay. All changes are kept in `TeamsState` until Save is pressed.

mod editors;
mod list;
mod overlay;
mod shared;

pub(super) use list::render;
pub(super) use overlay::render_overlay;
use shared::*;

use std::rc::Rc;

use gpui::{Context, Div, ImageSource, KeyDownEvent, deferred, div, img, prelude::*, px, relative};

use crate::{
    app::{AhabApp, TEXT, TEXT_MUTED},
    assets::{self, Asset, SinnerAsset, StatusEffectAsset},
    components::style::{ColorToken, current_render_palette},
    components::{
        BadgeTone, ButtonVariant, badge, button, card, empty_state,
        is_activation_key as team_activation_key, page_root, page_toolbar, palette_rgb,
        render_rgb as rgb, render_rgba as rgba, scroll_area_with_id, select_option, select_popup,
        select_trigger, settings_grid, svg_icon_bytes, switch, tab_surface_with_palette,
    },
    i18n::paired as text,
    model::{Language, TeamDetail, TeamMirrorConfig, TeamPurpose},
    state::{
        MirrorBool, MirrorU8, SYSTEM_NAMES, TeamEditorTab, TeamFilter, TeamSelect, TeamsState,
    },
};

fn delete_confirmation(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    language: Language,
) -> impl IntoElement {
    let Some(team) = app.teams.delete_target.as_ref() else {
        return div().into_any_element();
    };
    let name = team.name.clone();
    div()
        .id("delete-confirmation")
        .w(px(400.))
        .max_w_full()
        .p_4()
        .rounded_lg()
        .border_1()
        .border_color(palette_rgb(current_render_palette().danger))
        .bg(palette_rgb(current_render_palette().danger_light))
        .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .text_size(px(15.))
                        .text_color(rgb(TEXT))
                        .child(text("确认删除队伍？", "Delete this team?").get(language)),
                )
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(TEXT_MUTED))
                        .child(name),
                )
                .child(
                    div()
                        .flex()
                        .justify_end()
                        .gap_2()
                        .child({
                            let mut cancel =
                                button(text("取消", "Cancel").get(language), ButtonVariant::Ghost)
                                    .id("delete-cancel")
                                    .h(px(32.))
                                    .px_3()
                                    .py_0()
                                    .on_click(cx.listener(|view, _, _, cx| {
                                        view.teams.cancel_delete();
                                        cx.notify();
                                    }));
                            cancel = cancel.on_key_down(cx.listener(
                                |view, event: &KeyDownEvent, window, cx| {
                                    if team_activation_key(event) {
                                        window.prevent_default();
                                        view.teams.cancel_delete();
                                        cx.notify();
                                    }
                                },
                            ));
                            cancel
                        })
                        .child({
                            let mut confirm = button(
                                text("删除", "Delete").get(language),
                                ButtonVariant::Destructive,
                            )
                            .id("delete-confirm")
                            .h(px(32.))
                            .px_3()
                            .py_0()
                            .on_click(cx.listener(|view, _, _, cx| {
                                view.confirm_delete(cx);
                                cx.notify();
                            }));
                            confirm = confirm.on_key_down(cx.listener(
                                |view, event: &KeyDownEvent, window, cx| {
                                    if team_activation_key(event) {
                                        window.prevent_default();
                                        view.confirm_delete(cx);
                                        cx.notify();
                                    }
                                },
                            ));
                            confirm
                        }),
                ),
        )
        .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
        .into_any_element()
}

fn mirror_switch(
    _app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    field: MirrorBool,
    value: bool,
    id: impl Into<String>,
) -> gpui::Stateful<Div> {
    let mut control = switch(value).id(id.into());
    control = control.on_click(cx.listener(move |view, _, _, cx| {
        view.teams.set_mirror_bool(field, !value);
        cx.notify();
    }));
    control.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
        if team_activation_key(event) {
            window.prevent_default();
            view.teams.set_mirror_bool(field, !value);
            cx.notify();
        }
    }))
}

type TeamSelectChange = Rc<dyn Fn(&mut TeamsState, String)>;

struct TeamSelectConfig {
    select: TeamSelect,
    current: String,
    options: Vec<(String, String)>,
    id: String,
    width: f32,
    on_change: TeamSelectChange,
}

fn team_select(app: &AhabApp, cx: &mut Context<AhabApp>, config: TeamSelectConfig) -> Div {
    let TeamSelectConfig {
        select,
        current,
        options,
        id,
        width,
        on_change,
    } = config;
    let selected_index = options
        .iter()
        .position(|(value, _)| value == &current)
        .unwrap_or(0);
    let selected_label = options
        .get(selected_index)
        .map(|(_, label)| label.clone())
        .unwrap_or_else(|| current.clone());
    let values = options
        .iter()
        .map(|(value, _)| value.clone())
        .collect::<Vec<_>>();
    let has_current = options.iter().any(|(value, _)| value == &current);
    let open = app.teams.is_select_open(select);
    let palette = current_render_palette();
    let mut trigger = select_trigger(selected_label, open, &palette)
        .id(id.clone())
        .w(px(width));
    if has_current {
        trigger = trigger
            .bg(palette_rgb(palette.brand_light))
            .text_color(palette_rgb(palette.brand))
            .font_weight(gpui::FontWeight::MEDIUM);
        if !open {
            trigger = trigger.border_color(palette_rgb(palette.brand));
        }
    }
    trigger = trigger.on_click(cx.listener(move |view, _, _, cx| {
        if open {
            view.teams.close_select();
        } else {
            view.teams.toggle_select(select);
        }
        cx.stop_propagation();
        cx.notify();
    }));
    let key_change = on_change.clone();
    let key_values = values.clone();
    let key_current = current.clone();
    trigger = trigger.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
        let key = event.keystroke.key.to_ascii_lowercase();
        if key == "escape" {
            window.prevent_default();
            view.teams.close_select();
            cx.notify();
            return;
        }
        if matches!(key.as_str(), "enter" | "space") {
            window.prevent_default();
            view.teams.toggle_select(select);
            cx.notify();
            return;
        }

        let current_index = key_values
            .iter()
            .position(|candidate| candidate == &key_current)
            .unwrap_or(0);
        let next_index = match key.as_str() {
            "left" | "arrowleft" | "up" | "arrowup" => Some(
                (current_index as isize - 1).rem_euclid(key_values.len().max(1) as isize) as usize,
            ),
            "right" | "arrowright" => Some((current_index + 1) % key_values.len().max(1)),
            "down" | "arrowdown" if open => Some((current_index + 1) % key_values.len().max(1)),
            "down" | "arrowdown" => Some(current_index),
            "home" => Some(0),
            "end" => key_values.len().checked_sub(1),
            _ => None,
        };
        if let Some(next_index) = next_index
            && let Some(value) = key_values.get(next_index)
        {
            window.prevent_default();
            if matches!(key.as_str(), "down" | "arrowdown" | "up" | "arrowup") && !open {
                view.teams.toggle_select(select);
            } else {
                key_change(&mut view.teams, value.clone());
                if !open {
                    view.teams.close_select();
                }
            }
            cx.notify();
        }
    }));

    let mut option_list = div().flex().flex_col().gap_1();
    for (value, option_label) in options {
        let selected = value == current;
        let option_id = format!("{id}-option-{value}");
        let click_change = on_change.clone();
        let click_value = value.clone();
        let mut option = select_option(option_label, selected, &palette)
            .id(option_id)
            .on_click(cx.listener(move |view, _, _, cx| {
                click_change(&mut view.teams, click_value.clone());
                view.teams.close_select();
                cx.stop_propagation();
                cx.notify();
            }));
        if selected {
            option = option
                .border_1()
                .border_color(palette_rgb(palette.brand))
                .font_weight(gpui::FontWeight::MEDIUM);
        }
        let key_change = on_change.clone();
        let key_value = value;
        option = option.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
            if team_activation_key(event) {
                window.prevent_default();
                key_change(&mut view.teams, key_value.clone());
                view.teams.close_select();
                cx.notify();
            } else if event.keystroke.key.eq_ignore_ascii_case("escape") {
                window.prevent_default();
                view.teams.close_select();
                cx.notify();
            }
        }));
        option_list = option_list.child(option);
    }

    let mut root = div().relative().w(px(width)).child(trigger);
    if open {
        // The editor content lives inside a scrolling element. Defer the menu
        // paint so the popup is composited above the following form sections
        // instead of being covered by them (and by the scroll area's paint
        // order), matching the floating SelectContent used by the React UI.
        let popup = select_popup(option_list, &palette)
            .shadow_sm()
            .on_mouse_down_out(cx.listener(move |view, _, _, cx| {
                view.teams.close_select();
                cx.notify();
            }));
        root = root.child(deferred(popup).priority(10));
    }
    root
}

fn editor_section_title(title: impl Into<String>) -> Div {
    let palette = current_render_palette();
    div()
        .text_size(px(12.))
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(palette_rgb(palette.muted_foreground))
        .child(title.into())
}

fn field_block(title: impl Into<String>, body: impl IntoElement) -> Div {
    editor_card(
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(editor_section_title(title))
            .child(body),
    )
}

fn editor_card(child: impl IntoElement) -> Div {
    card(child).w_full().p_3().overflow_hidden()
}

/// Dense settings stay responsive, but each option gets its own surface so
/// the two columns remain visually distinct instead of reading as one long
/// list of switches.
fn editor_option_grid(children: impl IntoIterator<Item = Div>) -> Div {
    let palette = current_render_palette();
    let hover = palette_rgb(palette.accent_surface);
    let cells = children.into_iter().map(|child| {
        div()
            .min_w_0()
            .p_2()
            .rounded_md()
            .bg(palette_rgb(palette.secondary))
            .hover(move |style| style.bg(hover))
            .child(child)
    });
    settings_grid(cells, 260.)
}

fn editor_choice_button(label: impl Into<String>, selected: bool) -> Div {
    let palette = current_render_palette();
    let mut control = button(label, ButtonVariant::Outline)
        .border_color(palette_rgb(if selected {
            palette.brand
        } else {
            palette.input
        }))
        .bg(palette_rgb(if selected {
            palette.brand_light
        } else {
            palette.card
        }))
        .text_color(palette_rgb(if selected {
            palette.brand
        } else {
            palette.foreground
        }));
    if selected {
        control = control.font_weight(gpui::FontWeight::MEDIUM);
    }
    control
}

fn labeled_field(title: impl Into<String>, input: impl IntoElement) -> Div {
    div()
        .flex()
        .flex_col()
        .gap_1()
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(TEXT_MUTED))
                .child(title.into()),
        )
        .child(input)
}

fn control_row(label: impl Into<String>, control: impl IntoElement) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .py_1()
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(px(13.))
                .text_color(rgb(TEXT))
                .child(label.into()),
        )
        .child(control)
}

fn system_choice(index: usize, selected: bool, destructive: bool, language: Language) -> Div {
    let index = index.min(SYSTEM_NAMES.len() - 1);
    let palette = current_render_palette();
    let (border, background, foreground) = if destructive && selected {
        (palette.danger, palette.danger_light, palette.danger)
    } else if selected {
        (palette.brand, palette.brand_light, palette.brand)
    } else {
        (palette.border, palette.card, palette.muted_foreground)
    };
    let mut control = div()
        .flex()
        .items_center()
        .gap_2()
        .h(px(34.))
        .w_full()
        .min_w_0()
        .px_2()
        .rounded_md()
        .border_1()
        .border_color(palette_rgb(border))
        .bg(palette_rgb(background))
        .tab_index(0)
        .cursor_pointer()
        .focus_visible(|style| style.border_color(palette_rgb(current_render_palette().ring)))
        .text_size(px(11.))
        .text_color(palette_rgb(foreground))
        .child(
            img(status_effect_path(SYSTEM_NAMES[index]))
                .w(px(18.))
                .h(px(18.)),
        )
        .child(
            div()
                .min_w_0()
                .truncate()
                .child(system_label(index, language)),
        );
    if selected {
        control = control.font_weight(gpui::FontWeight::MEDIUM);
    }
    control
}

fn scheme_badge(scheme: &str, language: Language) -> Div {
    let scheme = normalized_scheme(scheme);
    div()
        .flex()
        .items_center()
        .gap_1()
        .h(px(20.))
        .px_2()
        .rounded_md()
        .border_1()
        .border_color(palette_rgb(current_render_palette().input))
        .text_size(px(11.))
        .text_color(palette_rgb(current_render_palette().muted_foreground))
        .child(img(status_effect_path(scheme)).w(px(14.)).h(px(14.)))
        .child(scheme_label(scheme, language))
}
