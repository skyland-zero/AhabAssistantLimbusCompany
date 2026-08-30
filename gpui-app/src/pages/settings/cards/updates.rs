use super::*;

use gpui::deferred;

use crate::{
    components::{select_option, select_popup, select_trigger},
    state::SettingsSelect,
};

pub fn update_card(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    system: &crate::model::SystemSettingsConfig,
    cdk_input: Option<gpui::Entity<TextInput>>,
    language: Language,
) -> Div {
    let source_value = system.update_source;
    let source_label = match source_value {
        UpdateSource::GitHub => "GitHub",
        UpdateSource::MirrorChyan => "Mirror 酱",
    };
    let source_open = app
        .settings_page
        .is_select_open(SettingsSelect::UpdateSource);
    let palette = current_render_palette();
    let mut source_trigger = select_trigger(source_label, source_open, &palette)
        .id("settings-update-source")
        .on_click(cx.listener(move |view, _, _, cx| {
            if source_open {
                view.settings_page.close_select();
            } else {
                view.settings_page
                    .toggle_select(SettingsSelect::UpdateSource);
            }
            cx.stop_propagation();
            cx.notify();
        }));
    source_trigger =
        source_trigger.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
            let key = event.keystroke.key.to_ascii_lowercase();
            match key.as_str() {
                "left" | "arrowleft" | "right" | "arrowright" => {
                    window.prevent_default();
                    view.settings_page.set_update_source(match source_value {
                        UpdateSource::GitHub => UpdateSource::MirrorChyan,
                        UpdateSource::MirrorChyan => UpdateSource::GitHub,
                    });
                    cx.notify();
                }
                "enter" | "space" | "arrowdown" => {
                    window.prevent_default();
                    view.settings_page
                        .toggle_select(SettingsSelect::UpdateSource);
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
    let mut source_options = div().flex().flex_col().gap_1();
    for (candidate, label) in [
        (UpdateSource::GitHub, "GitHub"),
        (UpdateSource::MirrorChyan, "Mirror 酱"),
    ] {
        let selected = candidate == system.update_source;
        let mut option = select_option(label, selected, &palette)
            .id(format!("settings-update-source-option-{candidate:?}"))
            .on_click(cx.listener(move |view, _, _, cx| {
                view.settings_page.set_update_source(candidate);
                cx.stop_propagation();
                cx.notify();
            }));
        option = option.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
            if is_activation_key(event) {
                window.prevent_default();
                view.settings_page.set_update_source(candidate);
                cx.notify();
            }
        }));
        source_options = source_options.child(option);
    }
    let mut source = div().relative().w(px(180.)).child(source_trigger);
    if source_open {
        // This card is inside settings-scroll. Defer the popup so the
        // following settings rows cannot cover the floating menu.
        let popup = select_popup(source_options, &palette).on_mouse_down_out(cx.listener(
            move |view, _, _, cx| {
                view.settings_page.close_select();
                cx.notify();
            },
        ));
        source = source.child(deferred(popup).priority(10));
    }

    let mut check = action_button(
        text("检查更新", "Check for Updates").get(language),
        ButtonVariant::Outline,
        Some(svg_icon(ICON_SEARCH_CHECK, 14., TEXT)),
        28.,
    )
    .id("settings-check-update");
    check = check
        .on_click(cx.listener(|view, _, _, cx| {
            view.settings_page.check_update();
            cx.notify();
        }))
        .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
            if is_activation_key(event) {
                window.prevent_default();
                view.settings_page.check_update();
                cx.notify();
            }
        }));

    let mut save_cdk = button(
        text("保存 CDK", "Save CDK").get(language),
        ButtonVariant::Default,
    )
    .id("settings-save-cdk")
    .px_3()
    .py_1()
    .text_size(px(12.));
    save_cdk = save_cdk
        .on_click(cx.listener(|view, _, _, cx| view.save_settings_cdk(cx)))
        .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
            if is_activation_key(event) {
                window.prevent_default();
                view.save_settings_cdk(cx);
            }
        }));

    let mut body = div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .px_4()
        .pb_4()
        .child(settings_list(vec![
            setting_row(
                text("参与预览版渠道", "Pre-release Channel").get(language),
                text(
                    "接收测试版与预发布版更新推送",
                    "Receive beta and preview update notifications",
                )
                .get(language),
                setting_switch(
                    cx,
                    SystemBool::Prerelease,
                    system.update_prerelease_enable,
                    "settings-prerelease",
                ),
            ),
            setting_row(
                text("更新源选择", "Update Mirror").get(language),
                text(
                    "选择检查与下载更新使用的镜像服务",
                    "Select mirror service for downloads",
                )
                .get(language),
                source,
            ),
        ]));

    if system.update_source == UpdateSource::MirrorChyan {
        body = body.child(setting_row(
            text("Mirror 酱 CDK", "Mirror-Chyan CDK").get(language),
            text("可选", "Optional").get(language),
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    cdk_input
                        .map(|input| div().w(px(240.)).child(input))
                        .unwrap_or_else(|| {
                            div().child(text("初始化中", "Initializing…").get(language))
                        }),
                )
                .child(save_cdk),
        ));
    }

    body = body.child(div().flex().items_start().pt_1().child(check));
    settings_card(
        text("更新与源配置", "Updates & Sources").get(language),
        body,
    )
}

pub fn about_card(_app: &mut AhabApp, cx: &mut Context<AhabApp>, language: Language) -> Div {
    let mut repo = action_button(
        "GitHub",
        ButtonVariant::Ghost,
        Some(svg_icon(ICON_EXTERNAL_LINK, 12., TEXT_MUTED)),
        28.,
    )
    .id("settings-open-repo");
    repo = repo
        .on_click(cx.listener(|view, _, _, cx| {
            open_repo();
            view.settings_page.feedback = Some("已请求打开 GitHub 仓库".to_owned());
            cx.notify();
        }))
        .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
            if is_activation_key(event) {
                window.prevent_default();
                open_repo();
                view.settings_page.feedback = Some("已请求打开 GitHub 仓库".to_owned());
                cx.notify();
            }
        }));

    let body = div()
        .flex()
        .flex_col()
        .gap(px(12.))
        .px_4()
        .pb_4()
        .child(settings_list(vec![
            setting_line(
                text("版本", "Version").get(language),
                div()
                    .font_family("Consolas")
                    .text_size(px(12.))
                    .text_color(rgb(TEXT_MUTED))
                    .child(format!("v{}", env!("CARGO_PKG_VERSION"))),
            ),
            setting_line(text("开源地址", "Repository").get(language), repo),
        ]));
    settings_card(text("关于", "About").get(language), body)
}
