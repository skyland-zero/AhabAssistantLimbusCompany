//! Settings page backed by the shared settings and sidecar IPC state.
//!
//! The card stack is centered at the `max-w-2xl` width, keeps each setting in
//! its own card, and exposes appearance, language, hotkey, system,
//! update, and version states without introducing a second settings model.

mod cards;

use std::process::Command;

use gpui::{
    Context, Div, FontWeight, KeyDownEvent, ScrollWheelEvent, div, img, prelude::*, px,
    rgb as gpui_rgb,
};

use crate::{
    app::{AhabApp, SURFACE, TEXT, TEXT_MUTED},
    components::style::{ACCENT_PRESETS, ColorScheme, GREEN, current_render_palette, skin_rounded},
    components::{
        ButtonVariant, TextInput, action_button, button, card, is_activation_key, page_root,
        palette_rgb, render_rgb as rgb, scroll_area_with_handle_children, svg_icon, switch,
    },
    i18n::{Localized, paired as text},
    model::{Language, ThemeMode, UpdateSource},
    state::{HotkeyTarget, SystemBool},
};

const REPO_URL: &str = "https://github.com/KIYI671/AhabAssistantLimbusCompany";
const ICON_EXTERNAL_LINK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><path d="M15 3h6v6"/><path d="M10 14 21 3"/><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/></svg>"#;
const ICON_SEARCH_CHECK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/><path d="m8 11 2 2 4-4"/></svg>"#;
const SETTINGS_NAV_WIDTH: f32 = 200.;

const SETTINGS_SECTIONS: [Localized; 7] = [
    text("外观", "Appearance"),
    text("全局热键", "Global Hotkeys"),
    text("系统与防护", "System & Protection"),
    text("实验性功能", "Experimental Features"),
    text("更新与源配置", "Updates & Sources"),
    text("任务通知", "Task Notifications"),
    text("关于", "About"),
];

pub fn render(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    app.ensure_settings_input(cx);

    let language = app.state.settings.language;
    let theme = app.state.settings.themeMode;
    let accent = app.state.settings.accentId.clone();
    let skin = app.state.settings.skinId.clone();
    let hotkey = app.settings_page.hotkey.clone();
    let system = app.settings_page.system.clone();
    let cdk_input = app.settings_inputs.cdk.clone();
    let wxpusher_spt_input = app.settings_inputs.wxpusher_spt.clone();
    let feedback = app.settings_page.feedback.clone();

    let mut sections = vec![
        cards::appearance_card(app, cx, theme, language, &accent, &skin),
        cards::hotkey_card(app, cx, &hotkey, language),
        cards::system_card(app, cx, &system, language),
        cards::experimental_card(app, cx, &system, language),
        cards::update_card(app, cx, &system, cdk_input, language),
        cards::notification_card(app, cx, wxpusher_spt_input, language),
        cards::about_card(app, cx, language),
    ];
    sections.push(
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
        sections.push(
            div()
                .text_size(px(12.))
                .text_color(rgb(GREEN))
                .child(localized_feedback(&feedback, language)),
        );
    }

    let mut scroll = scroll_area_with_handle_children(
        app,
        "settings-scroll",
        sections,
        app.settings_scroll.clone(),
    )
    .flex()
    .flex_col()
    .gap_3()
    .flex_1()
    .min_w_0()
    .min_h_0();
    let scroll_handle = app.settings_scroll.clone();
    scroll = scroll.on_scroll_wheel(cx.listener(move |_view, _: &ScrollWheelEvent, window, cx| {
        let scroll_handle = scroll_handle.clone();
        cx.on_next_frame(window, move |view, _window, cx| {
            let active = active_settings_section(scroll_handle.top_item(), SETTINGS_SECTIONS.len());
            if view.settings_active_section != active {
                view.settings_active_section = active;
                cx.notify();
            }
        });
    }));

    page_root()
        .flex_row()
        .child(settings_navigation(app, cx, language))
        .child(scroll)
}

fn settings_navigation(app: &mut AhabApp, cx: &mut Context<AhabApp>, language: Language) -> Div {
    let mut links = div().flex().flex_col().gap_1().w_full();
    for (index, label) in SETTINGS_SECTIONS.iter().enumerate() {
        let active = app.settings_active_section == index;
        let mut link = button(
            label.get(language),
            if active {
                ButtonVariant::Secondary
            } else {
                ButtonVariant::Ghost
            },
        )
        .id(format!("settings-nav-{index}"))
        .w_full()
        .justify_start()
        .px_3()
        .py_2()
        .text_size(px(12.));
        link = link
            .on_click(cx.listener(move |view, _, _, cx| {
                view.settings_active_section = index;
                view.settings_scroll.scroll_to_top_of_item(index);
                cx.notify();
            }))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    view.settings_active_section = index;
                    view.settings_scroll.scroll_to_top_of_item(index);
                    cx.notify();
                }
            }));
        links = links.child(link);
    }

    card(
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(TEXT_MUTED))
                    .child(text("快速导航", "Quick Navigation").get(language)),
            )
            .child(links),
    )
    .w(px(SETTINGS_NAV_WIDTH))
    .h_full()
    .flex_none()
    .p_3()
}

fn active_settings_section(top_item: usize, section_count: usize) -> usize {
    top_item.min(section_count.saturating_sub(1))
}

fn settings_card(title: &'static str, body: Div) -> Div {
    // Limbus skin swaps the flat title row for a riveted dark-red metal
    // band with a gold stencil title. Same 44px slot, no layout shift.
    let header = if current_render_palette().skin.is_limbus() {
        div()
            .relative()
            .w_full()
            .h(px(44.))
            .overflow_hidden()
            .child(
                img(crate::assets::image_source(crate::assets::theme(
                    crate::assets::ThemeAsset::LimbusTagband,
                )))
                .w_full()
                .h_full(),
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
                    .text_size(px(15.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(palette_rgb(current_render_palette().accent_foreground))
                    .child(title),
            )
    } else {
        div()
            .px_4()
            .py_3()
            .border_b_1()
            .border_color(palette_rgb(current_render_palette().input))
            .text_size(px(14.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgb(TEXT))
            .child(title)
    };
    card(div().p_0().flex().flex_col().child(header).child(body)).p_0()
}

fn settings_list(children: impl IntoIterator<Item = Div>) -> Div {
    let mut list = div().w_full().flex().flex_col().gap(px(12.));
    for child in children {
        list = list.child(child);
    }
    list
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
        .child(div().flex_none().child(control))
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
                .flex_1()
                .text_size(px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgb(TEXT))
                .child(label),
        )
        .child(div().flex_none().child(control))
}

fn segmented_group() -> Div {
    skin_rounded(
        div()
            .flex()
            .items_center()
            .gap_1()
            .bg(rgb(SURFACE))
            .p_1(),
        true,
    )
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
        "正在发送测试通知" => "Sending test notification".to_owned(),
        "测试通知已发送" => "Test notification sent".to_owned(),
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

    #[test]
    fn settings_scroll_item_is_clamped_to_the_last_section() {
        assert_eq!(active_settings_section(0, SETTINGS_SECTIONS.len()), 0);
        assert_eq!(active_settings_section(4, SETTINGS_SECTIONS.len()), 4);
        assert_eq!(active_settings_section(99, SETTINGS_SECTIONS.len()), 6);
    }
}
