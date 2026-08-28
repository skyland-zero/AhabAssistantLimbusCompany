//! Settings page backed by the shared settings and sidecar IPC state.
//!
//! The card stack is centered at the `max-w-2xl` width, keeps each setting in
//! its own card, and exposes appearance, language, hotkey, simulator, system,
//! update, and version states without introducing a second settings model.

mod cards;

use std::process::Command;

use gpui::{
    Context, Div, FontWeight, KeyDownEvent, deferred, div, prelude::*, px, rgb as gpui_rgb,
};

use crate::{
    app::{AhabApp, BACKGROUND, SURFACE, TEXT, TEXT_MUTED},
    components::style::{ACCENT_PRESETS, ColorScheme, GREEN, current_render_palette},
    components::{
        ButtonVariant, TextInput, action_button, button, card, is_activation_key, palette_rgb,
        render_rgb as rgb, scroll_area_with_id, select_option, select_popup, select_trigger,
        svg_icon, switch,
    },
    i18n::paired as text,
    model::{Language, ThemeMode, UpdateSource},
    state::{HotkeyTarget, SettingsSelect, SystemBool, SystemU16},
};

const REPO_URL: &str = "https://github.com/KIYI671/AhabAssistantLimbusCompany";
const ICON_EXTERNAL_LINK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><path d="M15 3h6v6"/><path d="M10 14 21 3"/><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/></svg>"#;
const ICON_SEARCH_CHECK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/><path d="m8 11 2 2 4-4"/></svg>"#;

pub fn render(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    app.ensure_settings_input(cx);

    let language = app.state.settings.language;
    let theme = app.state.settings.themeMode;
    let accent = app.state.settings.accentId.clone();
    let hotkey = app.settings_page.hotkey.clone();
    let system = app.settings_page.system.clone();
    let cdk_input = app.settings_inputs.cdk.clone();
    let feedback = app.settings_page.feedback.clone();

    let mut stack = div()
        .w_full()
        .max_w(px(672.))
        .mx_auto()
        .flex()
        .flex_col()
        .gap_3()
        .pb_8();
    stack = stack
        .child(cards::appearance_card(app, cx, theme, language, &accent))
        .child(cards::hotkey_card(app, cx, &hotkey, language))
        .child(cards::simulator_card(app, cx, &system, language))
        .child(cards::system_card(app, cx, &system, language))
        .child(cards::experimental_card(app, cx, &system, language))
        .child(cards::update_card(app, cx, &system, cdk_input, language))
        .child(cards::about_card(app, cx, language))
        .child(
            div()
                .flex()
                .justify_center()
                .pt_1()
                .text_size(px(11.))
                .text_color(rgb(TEXT_MUTED))
                .child(format!(
                    "{} · Ahab Assistant Limbus Company v{}",
                    language_code(language),
                    env!("CARGO_PKG_VERSION")
                )),
        );

    if let Some(feedback) = feedback {
        stack = stack.child(
            div()
                .text_size(px(12.))
                .text_color(rgb(GREEN))
                .child(localized_feedback(&feedback, language)),
        );
    }

    div()
        .size_full()
        .flex()
        .flex_col()
        .bg(rgb(BACKGROUND))
        .child(
            scroll_area_with_id("settings-scroll", div().w_full().p_4().child(stack))
                .size_full()
                .min_h_0(),
        )
}

fn settings_card(title: &'static str, body: Div) -> Div {
    card(
        div()
            .p_0()
            .flex()
            .flex_col()
            .child(
                div()
                    .px_3p5()
                    .py_2p5()
                    .text_size(px(14.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(TEXT))
                    .child(title),
            )
            .child(body),
    )
    .px_0()
    .py(px(24.))
    .gap(px(44.))
}

fn setting_row(label: &'static str, detail: &'static str, control: impl IntoElement) -> Div {
    let mut copy = div().flex().flex_col().gap_1().min_w_0().flex_1().child(
        div()
            .text_size(px(13.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(TEXT))
            .child(label),
    );
    if !detail.is_empty() {
        copy = copy.child(
            div()
                .text_size(px(12.))
                .text_color(rgb(TEXT_MUTED))
                .child(detail),
        );
    }
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .child(copy)
        .child(control)
}

fn setting_line(label: &'static str, control: impl IntoElement) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .child(
            div()
                .min_w_0()
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(TEXT))
                .child(label),
        )
        .child(control)
}

fn segmented_group() -> Div {
    div()
        .flex()
        .items_center()
        .gap_1()
        .rounded_lg()
        .bg(rgb(SURFACE))
        .p_1()
}

fn separator() -> Div {
    div()
        .h(px(1.))
        .w_full()
        .bg(palette_rgb(current_render_palette().border))
}

fn setting_switch(
    cx: &mut Context<AhabApp>,
    field: SystemBool,
    value: bool,
    id: &'static str,
) -> gpui::Stateful<Div> {
    let mut control = switch(value).id(id);
    control = control.on_click(cx.listener(move |view, _, _, cx| {
        view.settings_page.set_system_bool(field, !value);
        cx.notify();
    }));
    control.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
        if is_activation_key(event) {
            window.prevent_default();
            view.settings_page.set_system_bool(field, !value);
            cx.notify();
        }
    }))
}

fn select_system_u16(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    field: SystemU16,
    value: u16,
    label: impl Into<String>,
    options: Vec<(u16, String)>,
    id: &'static str,
) -> Div {
    let select = SettingsSelect::SimulatorType;
    let open = app.settings_page.is_select_open(select);
    let palette = current_render_palette();
    let values: Vec<u16> = options.iter().map(|(candidate, _)| *candidate).collect();
    let next = cycle_value(&values, value, 1);
    let mut trigger = select_trigger(label, open, &palette)
        .id(id)
        .on_click(cx.listener(move |view, _, _, cx| {
            if open {
                view.settings_page.close_select();
            } else {
                view.settings_page.toggle_select(select);
            }
            cx.stop_propagation();
            cx.notify();
        }));
    let values_for_key = values.clone();
    trigger = trigger.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
        let key = event.keystroke.key.to_ascii_lowercase();
        match key.as_str() {
            "left" | "arrowleft" => {
                window.prevent_default();
                view.settings_page
                    .set_system_u16(field, cycle_value(&values_for_key, value, -1));
                cx.notify();
            }
            "right" | "arrowright" => {
                window.prevent_default();
                view.settings_page.set_system_u16(field, next);
                cx.notify();
            }
            "home" => {
                window.prevent_default();
                if let Some(first) = values_for_key.first() {
                    view.settings_page.set_system_u16(field, *first);
                }
                cx.notify();
            }
            "end" => {
                window.prevent_default();
                if let Some(last) = values_for_key.last() {
                    view.settings_page.set_system_u16(field, *last);
                }
                cx.notify();
            }
            "enter" | "space" | "arrowdown" => {
                window.prevent_default();
                view.settings_page.toggle_select(select);
                cx.notify();
            }
            "escape" => {
                window.prevent_default();
                view.settings_page.close_select();
                cx.notify();
            }
            _ => {}
        }
    }));

    let mut option_list = div().flex().flex_col().gap_1();
    for (candidate, option_label) in options {
        let selected = candidate == value;
        let mut option = select_option(option_label, selected, &palette)
            .id(format!("{id}-option-{candidate}"))
            .on_click(cx.listener(move |view, _, _, cx| {
                view.settings_page.set_system_u16(field, candidate);
                cx.stop_propagation();
                cx.notify();
            }));
        option = option.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
            if is_activation_key(event) {
                window.prevent_default();
                view.settings_page.set_system_u16(field, candidate);
                cx.notify();
            }
        }));
        option_list = option_list.child(option);
    }

    let mut root = div().relative().w(px(180.)).child(trigger);
    if open {
        // Simulator Selects also sit inside settings-scroll; keep the menu on
        // the floating layer instead of letting later rows paint over it.
        let popup = select_popup(option_list, &palette).on_mouse_down_out(cx.listener(
            move |view, _, _, cx| {
                view.settings_page.close_select();
                cx.notify();
            },
        ));
        root = root.child(deferred(popup).priority(10));
    }
    root
}

fn cycle_value(options: &[u16], current: u16, direction: i8) -> u16 {
    if options.is_empty() {
        return current;
    }
    let index = options.iter().position(|candidate| *candidate == current);
    let index = index.unwrap_or(0) as isize;
    let next = (index + isize::from(direction)).rem_euclid(options.len() as isize);
    options[next as usize]
}

fn localized_feedback(feedback: &str, language: Language) -> String {
    if matches!(language, Language::ZhCn) {
        return feedback.to_owned();
    }
    if let Some(version) = feedback.strip_prefix("当前已是最新版本：") {
        return format!("You are on the latest version ({version})");
    }
    if let Some(version) = feedback.strip_prefix("发现新版本：") {
        return format!("Update available: {version}");
    }
    match feedback {
        "设置已保存" => "Settings saved".to_owned(),
        "已请求打开 GitHub 仓库" => "GitHub repository opened".to_owned(),
        "未知" => "Unknown".to_owned(),
        _ => feedback.to_owned(),
    }
}

fn language_code(language: Language) -> &'static str {
    match language {
        Language::ZhCn => "zh-CN",
        Language::EnUs => "en-US",
    }
}

fn combo_from_keystroke(stroke: &gpui::Keystroke) -> Option<String> {
    let key = stroke.key.trim();
    if key.is_empty()
        || matches!(
            key.to_ascii_lowercase().as_str(),
            "control" | "shift" | "alt" | "super" | "meta" | "win"
        )
    {
        return None;
    }
    let mut parts: Vec<String> = Vec::new();
    if stroke.modifiers.control {
        parts.push("Ctrl".to_owned());
    }
    if stroke.modifiers.platform {
        parts.push("Super".to_owned());
    }
    if stroke.modifiers.alt {
        parts.push("Alt".to_owned());
    }
    if stroke.modifiers.shift {
        parts.push("Shift".to_owned());
    }
    parts.push(key.to_ascii_uppercase());
    Some(parts.join("+"))
}

fn open_repo() {
    #[cfg(windows)]
    {
        let _ = Command::new("cmd")
            .args(["/C", "start", "", REPO_URL])
            .spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = Command::new("open").arg(REPO_URL).spawn();
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let _ = Command::new("xdg-open").arg(REPO_URL).spawn();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{Keystroke, Modifiers};

    #[test]
    fn modifier_only_hotkeys_are_rejected() {
        assert!(
            combo_from_keystroke(&Keystroke {
                key: "control".into(),
                ..Default::default()
            })
            .is_none()
        );
    }

    #[test]
    fn hotkey_display_contains_all_pressed_modifiers() {
        let combo = combo_from_keystroke(&Keystroke {
            key: "f10".into(),
            modifiers: Modifiers {
                control: true,
                shift: true,
                ..Default::default()
            },
            ..Default::default()
        });
        assert_eq!(combo.as_deref(), Some("Ctrl+Shift+F10"));
    }

    #[test]
    fn discrete_settings_cycle_with_keyboard_boundaries() {
        let options = [16384, 5555, 62001];
        assert_eq!(cycle_value(&options, 16384, -1), 62001);
        assert_eq!(cycle_value(&options, 62001, 1), 16384);
        assert_eq!(cycle_value(&options, 9999, 1), 5555);
    }

    #[test]
    fn settings_feedback_is_localized_for_update_states() {
        assert_eq!(
            localized_feedback("当前已是最新版本：v1.0.0", Language::EnUs),
            "You are on the latest version (v1.0.0)"
        );
        assert_eq!(
            localized_feedback("发现新版本：v1.1.0", Language::EnUs),
            "Update available: v1.1.0"
        );
    }
}
