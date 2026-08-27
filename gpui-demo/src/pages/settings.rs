//! Settings page backed by the shared settings and Mock IPC state.
//!
//! The card stack mirrors `ui/src/pages/SettingsPage.tsx`: it is centered at
//! the `max-w-2xl` width, keeps each setting in its own card, and exposes the
//! same appearance, language, hotkey, simulator, system, update, and version
//! states without introducing a second settings model.

use std::process::Command;

use gpui::{
    Context, Div, FontWeight, KeyDownEvent, Svg, div, prelude::*, px, rgb as gpui_rgb, svg,
};

use crate::{
    app::{AhabApp, BACKGROUND, BORDER, SURFACE, TEXT, TEXT_MUTED},
    components::style::{ACCENT_PRESETS, ColorScheme, GREEN, current_render_palette},
    components::{
        ButtonVariant, TextInput, button, card, render_rgb as rgb, scroll_area_with_id,
        select_option, select_popup, select_trigger, switch,
    },
    model::{Language, ThemeMode, UpdateSource},
    state::{HotkeyTarget, SettingsSelect, SystemBool, SystemU16},
};

const REPO_URL: &str = "https://github.com/KIYI671/AhabAssistantLimbusCompany";
const ICON_EXTERNAL_LINK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><path d="M15 3h6v6"/><path d="M10 14 21 3"/><path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6"/></svg>"#;
const ICON_SEARCH_CHECK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" stroke-width="2"><circle cx="11" cy="11" r="8"/><path d="m21 21-4.3-4.3"/><path d="m8 11 2 2 4-4"/></svg>"#;

#[derive(Clone, Copy)]
struct Localized {
    zh: &'static str,
    en: &'static str,
}

impl Localized {
    fn get(self, language: Language) -> &'static str {
        match language {
            Language::ZhCn => self.zh,
            Language::EnUs => self.en,
        }
    }
}

const fn text(zh: &'static str, en: &'static str) -> Localized {
    Localized { zh, en }
}

pub fn render(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    app.ensure_settings_input(cx);

    let language = app.state.settings.language;
    let theme = app.state.settings.themeMode;
    let accent = app.state.settings.accentId.clone();
    let hotkey = app.settings_page.hotkey.clone();
    let system = app.settings_page.system.clone();
    let cdk_input = app.settings_cdk_input.clone();
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
        .child(appearance_card(app, cx, theme, language, &accent))
        .child(hotkey_card(app, cx, &hotkey, language))
        .child(simulator_card(app, cx, &system, language))
        .child(system_card(app, cx, &system, language))
        .child(experimental_card(app, cx, &system, language))
        .child(update_card(app, cx, &system, cdk_input, language))
        .child(about_card(app, cx, language))
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
                .child(feedback),
        );
    }

    let mut root = div()
        .size_full()
        .flex()
        .flex_col()
        .bg(rgb(BACKGROUND))
        .child(
            scroll_area_with_id("settings-scroll", div().w_full().p_4().child(stack))
                .size_full()
                .min_h_0(),
        );
    root = root.on_any_mouse_down(cx.listener(|view, _, _, cx| {
        if view.settings_page.open_select.is_some() {
            view.settings_page.close_select();
            cx.notify();
        }
    }));
    root
}

fn appearance_card(
    _app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    theme: ThemeMode,
    language: Language,
    accent: &str,
) -> Div {
    let mut modes = segmented_group();
    for (candidate, label) in [
        (ThemeMode::Light, text("浅色", "Light")),
        (ThemeMode::Dark, text("深色", "Dark")),
        (ThemeMode::System, text("跟随系统", "System")),
    ] {
        let mut control = button(
            label.get(language),
            if theme == candidate {
                ButtonVariant::Secondary
            } else {
                ButtonVariant::Ghost
            },
        )
        .id(format!("settings-theme-{candidate:?}"))
        .px_3()
        .py_1()
        .text_size(px(12.));
        let message = label.get(language).to_owned();
        control = control.on_click(cx.listener(move |view, _, _, cx| {
            view.set_theme_mode(candidate);
            view.show_toast(crate::shell::ToastKind::Info, message.clone(), cx);
            cx.notify();
        }));
        modes = modes.child(control);
    }

    let dark_accent = matches!(current_render_palette().scheme, ColorScheme::Dark);
    let mut accents = div().flex().items_center().gap_2();
    for preset in ACCENT_PRESETS {
        let selected = accent == preset.id;
        let color = if dark_accent {
            preset.dark.brand.rgb_hex()
        } else {
            preset.light.brand.rgb_hex()
        };
        let mut control = div()
            .id(format!("settings-accent-{}", preset.id))
            .w(px(24.))
            .h(px(24.))
            .rounded_full()
            .cursor_pointer()
            .bg(gpui_rgb(color));
        if selected {
            control = control.border_2().border_color(rgb(TEXT)).opacity(1.);
        } else {
            control = control.opacity(0.7);
        }
        let id = preset.id;
        control = control.on_click(cx.listener(move |view, _, _, cx| {
            view.set_accent(id);
            view.show_toast(crate::shell::ToastKind::Info, format!("Accent: {id}"), cx);
            cx.notify();
        }));
        accents = accents.child(control);
    }

    let mut languages = segmented_group();
    for (candidate, label) in [(Language::ZhCn, "简体中文"), (Language::EnUs, "English")] {
        let mut control = button(
            label,
            if language == candidate {
                ButtonVariant::Secondary
            } else {
                ButtonVariant::Ghost
            },
        )
        .id(format!("settings-language-{candidate:?}"))
        .px_3()
        .py_1()
        .text_size(px(12.));
        control = control.on_click(cx.listener(move |view, _, _, cx| {
            view.set_language(candidate);
            view.show_toast(crate::shell::ToastKind::Info, label, cx);
            cx.notify();
        }));
        languages = languages.child(control);
    }

    let body = div()
        .flex()
        .flex_col()
        .gap(px(16.))
        .px_3p5()
        .pb_3p5()
        .child(setting_line(
            text("主题模式", "Theme Mode").get(language),
            modes,
        ))
        .child(separator())
        .child(setting_line(
            text("强调色", "Accent Color").get(language),
            accents,
        ))
        .child(separator())
        .child(setting_line(
            text("语言 / Language", "Language / 语言").get(language),
            languages,
        ));
    settings_card(text("外观", "Appearance").get(language), body)
}

fn hotkey_card(
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
        }));
    let body = div()
        .flex()
        .flex_col()
        .gap(px(14.))
        .px_3p5()
        .pb_3p5()
        .child(setting_line(
            text("启用全局热键", "Enable Global Hotkeys").get(language),
            enable,
        ))
        .child(separator())
        .child(setting_line(
            text("启动 / 停止热键", "Start / Stop Hotkey").get(language),
            hotkey_capture(
                app,
                cx,
                HotkeyTarget::StartStop,
                hotkey.startStop.clone(),
                language,
            ),
        ))
        .child(setting_line(
            text("暂停 / 继续热键", "Pause / Resume Hotkey").get(language),
            hotkey_capture(
                app,
                cx,
                HotkeyTarget::PauseResume,
                hotkey.pauseResume.clone().unwrap_or_default(),
                language,
            ),
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
    capture = capture.on_click(cx.listener(move |view, _, _, cx| {
        view.settings_page.capture(target);
        cx.notify();
    }));
    capture = capture.on_key_down(cx.listener(move |view, event: &KeyDownEvent, _, cx| {
        if view.settings_page.capturing == Some(target) {
            if let Some(combo) = combo_from_keystroke(&event.keystroke) {
                view.settings_page.finish_capture(Some(combo));
                cx.notify();
            }
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
        clear = clear.on_click(cx.listener(move |view, _, _, cx| {
            view.settings_page.set_hotkey(target, None);
            cx.notify();
        }));
    }
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(capture)
        .child(clear)
}

fn simulator_card(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    system: &crate::model::SystemSettingsConfig,
    language: Language,
) -> Div {
    let simulator = setting_switch(
        cx,
        SystemBool::Simulator,
        system.simulator,
        "settings-simulator",
    );
    let simulator_type = select_system_u16(
        app,
        cx,
        SystemU16::SimulatorType,
        u16::from(system.simulator_type),
        if system.simulator_type == 0 {
            text("MuMu 模拟器（推荐）", "MuMu Player (Recommended)").get(language)
        } else {
            text("其他模拟器", "Other Emulators").get(language)
        },
        vec![
            (
                0,
                text("MuMu 模拟器（推荐）", "MuMu Player (Recommended)")
                    .get(language)
                    .to_owned(),
            ),
            (
                10,
                text("其他模拟器", "Other Emulators")
                    .get(language)
                    .to_owned(),
            ),
        ],
        "settings-simulator-type",
    );
    let port = app
        .settings_port_input
        .clone()
        .map(|input| div().w(px(112.)).child(input))
        .unwrap_or_else(|| div().child(text("初始化中", "Initializing…").get(language)));
    let timeout = app
        .settings_timeout_input
        .clone()
        .map(|input| div().w(px(112.)).child(input))
        .unwrap_or_else(|| div().child(text("初始化中", "Initializing…").get(language)));

    let mut body = div()
        .flex()
        .flex_col()
        .gap(px(14.))
        .px_3p5()
        .pb_3p5()
        .child(setting_row(
            text("使用模拟器模式", "Use Simulator Mode").get(language),
            text(
                "启用模拟器 ADB 自动化控制连接",
                "Enable ADB automation connection to Android emulator",
            )
            .get(language),
            simulator,
        ));
    if system.simulator {
        body = body
            .child(separator())
            .child(setting_row(
                text("模拟器类型", "Simulator Type").get(language),
                text("选择当前运行的安卓模拟器", "Select active Android emulator").get(language),
                simulator_type,
            ))
            .child(setting_row(
                text("ADB 端口号", "ADB Port").get(language),
                text(
                    "模拟器连接端口（MuMu 默认 16384）",
                    "Port used for connection (MuMu default 16384)",
                )
                .get(language),
                port,
            ));
        if system.simulator_type == 0 {
            body = body.child(setting_row(
                text("启动模拟器超时（秒）", "Launch Timeout (seconds)").get(language),
                text(
                    "仅限 MuMu 模拟器拉起等待时间",
                    "Wait duration when launching MuMu Player",
                )
                .get(language),
                timeout,
            ));
        }
    }
    settings_card(text("模拟器设置", "Simulator Settings").get(language), body)
}

fn system_card(
    _app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    system: &crate::model::SystemSettingsConfig,
    language: Language,
) -> Div {
    let body = div()
        .flex()
        .flex_col()
        .gap(px(14.))
        .px_3p5()
        .pb_3p5()
        .child(setting_row(
            text("内存占用保护", "Memory Protection").get(language),
            text(
                "电脑总内存占用超过 90% 时自动清理内存防崩溃",
                "Clean memory automatically if overall RAM usage exceeds 90%",
            )
            .get(language),
            setting_switch(
                cx,
                SystemBool::MemoryProtection,
                system.memory_protection,
                "settings-memory",
            ),
        ))
        .child(separator())
        .child(setting_row(
            text("最小化到托盘", "Minimize to Tray").get(language),
            text(
                "窗口最小化时隐藏到系统托盘区",
                "Hide window to system tray when minimized",
            )
            .get(language),
            setting_switch(
                cx,
                SystemBool::MinimizeToTray,
                system.minimize_to_tray,
                "settings-tray",
            ),
        ))
        .child(separator())
        .child(setting_row(
            text("开机自动启动", "Start on Boot").get(language),
            text(
                "跟随 Windows 系统开机自启 AALC",
                "Launch AALC automatically when Windows boots",
            )
            .get(language),
            setting_switch(
                cx,
                SystemBool::Autostart,
                system.autostart,
                "settings-autostart",
            ),
        ));
    settings_card(
        text("系统与防护", "System & Protection").get(language),
        body,
    )
}

fn experimental_card(
    _app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    system: &crate::model::SystemSettingsConfig,
    language: Language,
) -> Div {
    let body = div()
        .flex()
        .flex_col()
        .gap(px(14.))
        .px_3p5()
        .pb_3p5()
        .child(setting_row(
            text("运行时阻止休眠", "Prevent System Sleep").get(language),
            text(
                "任务执行期间阻止系统与显示器进入休眠，任务结束后自动恢复",
                "Prevent sleep and display turn-off during task execution",
            )
            .get(language),
            setting_switch(
                cx,
                SystemBool::KeepScreenAwake,
                system.experimental_keep_screen_awake,
                "settings-awake",
            ),
        ))
        .child(separator())
        .child(setting_row(
            text("显示器 HDR 检测提示", "Display HDR Detection").get(language),
            text(
                "检测到游戏处于 HDR 显示器时提示潜在图像识别问题",
                "Warn about potential visual recognition issues on HDR monitors",
            )
            .get(language),
            setting_switch(
                cx,
                SystemBool::HdrWarning,
                system.experimental_hdr_warning,
                "settings-hdr",
            ),
        ));
    settings_card(
        text("实验性功能", "Experimental Features").get(language),
        body,
    )
}

fn update_card(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    system: &crate::model::SystemSettingsConfig,
    cdk_input: Option<gpui::Entity<TextInput>>,
    language: Language,
) -> Div {
    let source_label = match system.update_source {
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
            if matches!(
                event.keystroke.key.to_ascii_lowercase().as_str(),
                "enter" | "space"
            ) {
                window.prevent_default();
                view.settings_page.set_update_source(candidate);
                cx.notify();
            }
        }));
        source_options = source_options.child(option);
    }
    let mut source = div().relative().w(px(180.)).child(source_trigger);
    if source_open {
        source = source.child(select_popup(source_options, &palette));
    }

    let mut check = action_button(
        text("检查更新", "Check for Updates").get(language),
        ButtonVariant::Outline,
        icon(ICON_SEARCH_CHECK, 14., TEXT),
    )
    .id("settings-check-update");
    check = check.on_click(cx.listener(|view, _, _, cx| {
        view.settings_page.check_update();
        cx.notify();
    }));

    let mut save_cdk = button(
        text("保存 CDK", "Save CDK").get(language),
        ButtonVariant::Default,
    )
    .id("settings-save-cdk")
    .px_3()
    .py_1()
    .text_size(px(12.));
    save_cdk = save_cdk.on_click(cx.listener(|view, _, _, cx| view.save_settings_cdk(cx)));

    let mut body = div()
        .flex()
        .flex_col()
        .gap(px(14.))
        .px_3p5()
        .pb_3p5()
        .child(setting_row(
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
        ))
        .child(separator())
        .child(setting_row(
            text("更新源选择", "Update Mirror").get(language),
            text(
                "选择检查与下载更新使用的镜像服务",
                "Select mirror service for downloads",
            )
            .get(language),
            source,
        ));

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

    body = body.child(div().pt_1().child(check));
    settings_card(
        text("更新与源配置", "Updates & Sources").get(language),
        body,
    )
}

fn about_card(_app: &mut AhabApp, cx: &mut Context<AhabApp>, language: Language) -> Div {
    let mut repo = action_button(
        "GitHub",
        ButtonVariant::Ghost,
        icon(ICON_EXTERNAL_LINK, 12., TEXT_MUTED),
    )
    .id("settings-open-repo");
    repo = repo.on_click(cx.listener(|view, _, _, cx| {
        open_repo();
        view.settings_page.feedback = Some("已请求打开 GitHub 仓库".to_owned());
        cx.notify();
    }));

    let body = div()
        .flex()
        .flex_col()
        .gap(px(14.))
        .px_3p5()
        .pb_3p5()
        .child(setting_line(
            text("版本", "Version").get(language),
            div()
                .font_family("Consolas")
                .text_size(px(12.))
                .text_color(rgb(TEXT_MUTED))
                .child(format!("v{}", env!("CARGO_PKG_VERSION"))),
        ))
        .child(separator())
        .child(setting_line(
            text("开源地址", "Repository").get(language),
            repo,
        ));
    settings_card(text("关于", "About").get(language), body)
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
    .p_0()
}

fn setting_row(label: &'static str, detail: &'static str, control: impl IntoElement) -> Div {
    let mut copy = div().flex().flex_col().gap_1().min_w_0().flex_1().child(
        div()
            .text_size(px(12.))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgb(TEXT))
            .child(label),
    );
    if !detail.is_empty() {
        copy = copy.child(
            div()
                .text_size(px(11.))
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
                .text_size(px(12.))
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
    div().h(px(1.)).w_full().bg(rgb(BORDER))
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
        if matches!(
            event.keystroke.key.to_ascii_lowercase().as_str(),
            "enter" | "space"
        ) {
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
            if matches!(
                event.keystroke.key.to_ascii_lowercase().as_str(),
                "enter" | "space"
            ) {
                window.prevent_default();
                view.settings_page.set_system_u16(field, candidate);
                cx.notify();
            }
        }));
        option_list = option_list.child(option);
    }

    let mut root = div().relative().w(px(180.)).child(trigger);
    if open {
        root = root.child(select_popup(option_list, &palette));
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

fn action_button(label: &'static str, variant: ButtonVariant, icon: Svg) -> Div {
    button("", variant)
        .h(px(28.))
        .px_3()
        .py_0()
        .text_size(px(12.))
        .child(icon)
        .child(label)
}

fn icon(data: &'static str, size: f32, color: u32) -> Svg {
    svg()
        .data(data.as_bytes())
        .size(px(size))
        .text_color(rgb(color))
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
}
