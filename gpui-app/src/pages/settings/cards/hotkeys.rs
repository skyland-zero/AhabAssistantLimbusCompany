use super::*;

pub fn hotkey_card(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    hotkey: &crate::model::HotkeyConfig,
    language: Language,
) -> Div {
    let enabled = hotkey.enabled;
    let enable = switch(enabled)
        .id("settings-hotkey-enabled")
        .on_click(cx.listener(move |view, _, _, cx| {
            view.settings_page.set_hotkeys_enabled(!enabled);
            cx.notify();
        }))
        .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
            if is_activation_key(event) {
                window.prevent_default();
                view.settings_page.set_hotkeys_enabled(!enabled);
                cx.notify();
            }
        }));
    let body = div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .px_3p5()
        .pb_3p5()
        .child(settings_grid(
            vec![
                setting_line(
                    text("启用全局热键", "Enable Global Hotkeys").get(language),
                    enable,
                ),
                setting_line(
                    text("启动 / 停止热键", "Start / Stop Hotkey").get(language),
                    hotkey_capture(
                        app,
                        cx,
                        HotkeyTarget::StartStop,
                        hotkey.startStop.clone(),
                        language,
                    ),
                ),
                setting_line(
                    text("暂停 / 继续热键", "Pause / Resume Hotkey").get(language),
                    hotkey_capture(
                        app,
                        cx,
                        HotkeyTarget::PauseResume,
                        hotkey.pauseResume.clone().unwrap_or_default(),
                        language,
                    ),
                ),
            ],
            296.,
        ));
    settings_card(text("全局热键", "Global Hotkeys").get(language), body)
}

fn hotkey_capture(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    target: HotkeyTarget,
    value: String,
    language: Language,
) -> Div {
    let capturing = app.settings_page.capturing == Some(target);
    let label = if capturing {
        text("按下组合键…", "Press a key combination…").get(language)
    } else if value.is_empty() {
        text("未设置", "Not set").get(language)
    } else {
        value.as_str()
    };
    let mut capture = button(label, ButtonVariant::Outline)
        .id(format!("settings-hotkey-{target:?}"))
        .min_w(px(128.))
        .px_3()
        .py_1()
        .font_family("Consolas")
        .text_size(px(12.));
    capture = capture
        .on_click(cx.listener(move |view, _, _, cx| {
            view.settings_page.capture(target);
            cx.notify();
        }))
        .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
            if view.settings_page.capturing == Some(target) {
                if let Some(combo) = combo_from_keystroke(&event.keystroke) {
                    window.prevent_default();
                    view.settings_page.finish_capture(Some(combo));
                    cx.notify();
                }
            } else if is_activation_key(event) {
                window.prevent_default();
                view.settings_page.capture(target);
                cx.notify();
            }
        }));

    let mut clear = button(text("清除", "Clear").get(language), ButtonVariant::Ghost)
        .id(format!("settings-hotkey-clear-{target:?}"))
        .px_2()
        .py_1()
        .text_size(px(12.));
    if value.is_empty() {
        clear = clear.opacity(0.45).cursor_not_allowed();
    } else {
        clear = clear
            .on_click(cx.listener(move |view, _, _, cx| {
                view.settings_page.set_hotkey(target, None);
                cx.notify();
            }))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    view.settings_page.set_hotkey(target, None);
                    cx.notify();
                }
            }));
    }
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(capture)
        .child(clear)
}
