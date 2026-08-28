//! Home page surface: task configuration, execution controls, and the
//! connection/log side panel. All mutable page state lives in HomeState.

use std::{rc::Rc, time::Duration};

use gpui::{
    Animation, AnimationExt, Context, Div, FontWeight, KeyDownEvent, Render, Svg, Window, deferred,
    div, prelude::*, px, relative, svg,
};

use crate::{
    app::{ACCENT, AhabApp, BACKGROUND, BORDER, SURFACE, SURFACE_HOVER, TEXT, TEXT_MUTED},
    components::{
        BadgeTone, ButtonVariant, badge, button, card, current_render_palette, palette_rgb,
        render_rgb as rgb, render_rgba as rgba, scroll_area_with_id, select_option, select_popup,
        select_trigger, switch, switch_accent,
    },
    i18n::{self, Key as I18nKey},
    model::{
        AfterExitAction, AfterPowerAction, ConnectionStatus, ExecutionState, FixedTaskId, Language,
        LogEntryPayload, LogLevel,
    },
    state::{DailyCounter, HomeSelect, HomeState, MirrorOption, TaskOptionsTab},
};

const RIGHT_PANEL_DEFAULT_WIDTH: f32 = 280.0;
const RIGHT_PANEL_MIN_WIDTH: f32 = 280.0;
const RIGHT_PANEL_MAX_WIDTH: f32 = 800.0;
const SPLITTER_WIDTH: f32 = 4.0;
const SPLITTER_COLLAPSED_WIDTH: f32 = 16.0;
const SPLITTER_COLLAPSE_THRESHOLD: f32 = 160.0;
const MIN_LEFT_PANEL_WIDTH: f32 = 460.0;

const ICON_SLIDERS: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><line x1="4" y1="6" x2="20" y2="6"/><line x1="4" y1="12" x2="20" y2="12"/><line x1="4" y1="18" x2="20" y2="18"/><circle cx="9" cy="6" r="2"/><circle cx="15" cy="12" r="2"/><circle cx="10" cy="18" r="2"/></svg>"#;
const ICON_CALENDAR_CHECK: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="4" width="18" height="17" rx="2"/><path d="M16 2v4M8 2v4M3 10h18M8 15l2 2 5-5"/></svg>"#;
const ICON_GIFT: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="3" y="8" width="18" height="13" rx="2"/><path d="M12 8v13M3 12h18M12 8H7.5a2.5 2.5 0 1 1 2.5-2.5C12 5.5 12 8 12 8ZM12 8h4.5a2.5 2.5 0 1 0-2.5-2.5C12 5.5 12 8 12 8Z"/></svg>"#;
const ICON_ZAP: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/></svg>"#;
const ICON_COMPASS: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><polygon points="16.24 7.76 14.12 14.12 7.76 16.24 9.88 9.88 16.24 7.76"/></svg>"#;
const ICON_RADIO: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="2"/><path d="M16.24 7.76a6 6 0 0 1 0 8.49M7.76 16.24a6 6 0 0 1 0-8.49M19.07 4.93a10 10 0 0 1 0 14.14M4.93 19.07a10 10 0 0 1 0-14.14"/></svg>"#;
const ICON_CHECK_SQUARE: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 11 12 14 22 4"/><path d="M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11"/></svg>"#;
const ICON_ROTATE: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="1 4 1 10 7 10"/><path d="M3.51 15a9 9 0 1 0 .49-9.5L1 10"/></svg>"#;
const ICON_TRASH: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14H5V6m3 0V4h8v2M10 11v6M14 11v6"/></svg>"#;
const ICON_PAUSE: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="6" y="4" width="4" height="16"/><rect x="14" y="4" width="4" height="16"/></svg>"#;
const ICON_PLAY: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round"><polygon points="5 3 19 12 5 21 5 3"/></svg>"#;
const ICON_SQUARE: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="1" stroke-linejoin="round"><rect x="5" y="5" width="14" height="14" rx="2"/></svg>"#;
#[allow(dead_code)]
const ICON_SETTINGS: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.9l.1.1-2.8 2.8-.1-.1a1.7 1.7 0 0 0-1.9-.3 1.7 1.7 0 0 0-1 1.5v.1h-4v-.1a1.7 1.7 0 0 0-1-1.5 1.7 1.7 0 0 0-1.9.3l-.1.1-2.8-2.8.1-.1a1.7 1.7 0 0 0 .3-1.9 1.7 1.7 0 0 0-1.5-1H3v-4h.1a1.7 1.7 0 0 0 1.5-1 1.7 1.7 0 0 0-.3-1.9l-.1-.1L7 4.2l.1.1a1.7 1.7 0 0 0 1.9.3 1.7 1.7 0 0 0 1-1.5V3h4v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.9-.3l.1-.1 2.8 2.8-.1.1a1.7 1.7 0 0 0-.3 1.9c.3.6.9 1 1.5 1h.1v4h-.1c-.6 0-1.2.4-1.5 1Z"/></svg>"#;
const ICON_REFRESH: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M20 11a8.1 8.1 0 0 0-15.5-2M4 5v4h4M4 13a8.1 8.1 0 0 0 15.5 2M20 19v-4h-4"/></svg>"#;
const ICON_X: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M6 6l12 12M18 6 6 18"/></svg>"#;
const ICON_MINUS: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M5 12h14"/></svg>"#;
const ICON_PLUS: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M12 5v14M5 12h14"/></svg>"#;
const ICON_CHEVRON_DOWN: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m6 9 6 6 6-6"/></svg>"#;
const ICON_CHEVRON_UP: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m18 15-6-6-6 6"/></svg>"#;
const ICON_MONITOR: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="3" width="20" height="14" rx="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>"#;
const ICON_SMARTPHONE: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="5" y="2" width="14" height="20" rx="2" ry="2"/><path d="M12 18h.01"/></svg>"#;
const ICON_CHECK: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>"#;
const ICON_HISTORY: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8"/><path d="M3 3v5h5"/><path d="M12 7v5l4 2"/></svg>"#;
const ICON_LOADER: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 12a9 9 0 1 1-6.219-8.56"/></svg>"#;
const ICON_MONITOR_PLAY: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="3" width="20" height="14" rx="2"/><polygon points="10 8 15 11 10 14 10 8"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/></svg>"#;
const ICON_SCROLL_TEXT: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M8 5h12"/><path d="M8 9h12"/><path d="M8 13h8"/><path d="M4 4v13a3 3 0 0 0 3 3h13"/><path d="M4 8H2"/><path d="M4 12H2"/></svg>"#;
const ICON_ALERT_CIRCLE: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>"#;
const ICON_ALERT_TRIANGLE: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m21.73 18-8-14a2 2 0 0 0-3.46 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3Z"/><line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/></svg>"#;

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

struct SplitterDragGhost;

impl Render for SplitterDragGhost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w(px(1.0)).h(px(1.0)).bg(rgb(ACCENT))
    }
}

fn action_icon(data: &'static [u8], size: f32, color: u32) -> Svg {
    svg()
        .data(data)
        .size(px(size))
        .text_color(rgb(color))
        .flex_none()
}

fn brand_action_icon(data: &'static [u8], size: f32) -> Svg {
    svg()
        .data(data)
        .size(px(size))
        .text_color(palette_rgb(current_render_palette().brand_foreground))
        .flex_none()
}

fn task_icon_data(label: &str) -> &'static [u8] {
    match label {
        "SET" => ICON_SLIDERS,
        "CAL" => ICON_CALENDAR_CHECK,
        "GFT" => ICON_GIFT,
        "ZAP" => ICON_ZAP,
        "CMP" => ICON_COMPASS,
        "RAD" => ICON_RADIO,
        _ => ICON_SLIDERS,
    }
}

pub fn render(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    let busy = app.home.is_busy();
    let execution_state = app.home.execution.state;
    // The mock start call intentionally leaves currentTaskId optional. Use
    // the first executable selection for the presentation only so the
    // running sweep still exercises the same visual state as the React page.
    let current_task = app.home.execution.currentTaskId.or_else(|| {
        (execution_state == ExecutionState::Running)
            .then(|| first_executable_task(&app.home))
            .flatten()
    });

    let task_cards = vec![
        set_windows_card(app, cx, busy, current_task == Some(FixedTaskId::SetWindows)),
        daily_card(app, cx, busy, current_task == Some(FixedTaskId::DailyTask)),
        reward_card(app, cx, busy, current_task == Some(FixedTaskId::GetReward)),
        enkephalin_card(
            app,
            cx,
            busy,
            current_task == Some(FixedTaskId::BuyEnkephalin),
        ),
        mirror_card(app, cx, busy, current_task == Some(FixedTaskId::Mirror)),
        ahab_card(
            app,
            cx,
            busy,
            current_task == Some(FixedTaskId::ResonateWithAhab),
        ),
    ];

    let task_list = div()
        .id("home-task-scroll")
        .overflow_y_scroll()
        .flex_1()
        .min_h_0()
        .px(px(14.0))
        .py(px(10.0))
        .child(div().flex().flex_col().gap_2().pb_2().children(task_cards));

    let left_panel = div()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .flex()
        .flex_col()
        .border_r_1()
        .border_color(rgba(0))
        .child(task_list)
        .child(execution_toolbar(app, cx, busy, execution_state));

    let splitter = splitter(app, cx);
    let right = if app.home.right_panel_collapsed {
        div()
    } else {
        right_panel(app, cx)
    };

    let root = div()
        .relative()
        .w_full()
        .flex_1()
        .h_full()
        .min_w_0()
        .min_h_0()
        .flex()
        .overflow_hidden()
        .bg(rgb(BACKGROUND))
        .child(left_panel)
        .child(splitter)
        .child(right);
    root
}

pub fn render_overlay(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .child(after_completion_editor(app, cx, app.home.is_busy()))
}

#[allow(dead_code)]
fn task_header(
    state: ExecutionState,
    current_task: Option<FixedTaskId>,
    language: crate::model::Language,
) -> Div {
    let (label, tone) = match state {
        ExecutionState::Running => (
            current_task
                .map(|task| {
                    format!(
                        "{}: {}",
                        i18n::text(language, I18nKey::HomeRunning),
                        task_title(task, language)
                    )
                })
                .unwrap_or_else(|| i18n::text(language, I18nKey::HomeRunning).to_owned()),
            BadgeTone::Accent,
        ),
        ExecutionState::Paused => (
            i18n::text(language, I18nKey::HomePaused).to_owned(),
            BadgeTone::Accent,
        ),
        ExecutionState::Idle => (
            i18n::text(language, I18nKey::HomeIdle).to_owned(),
            BadgeTone::Neutral,
        ),
    };

    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(10.0))
        .px(px(14.0))
        .h(px(36.0))
        .py_0()
        .child(
            div()
                .text_size(px(14.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT))
                .child(i18n::text(language, I18nKey::HomeTitle)),
        )
        .child(badge(label, tone))
}

fn options_tabs(
    task: FixedTaskId,
    selected: TaskOptionsTab,
    language: Language,
    cx: &mut Context<AhabApp>,
) -> Div {
    let palette = current_render_palette();
    let mut tabs = div()
        .flex()
        .items_center()
        .gap_1()
        .p(px(2.))
        .rounded_md()
        .bg(palette_rgb(palette.muted));

    for (tab, label) in [
        (TaskOptionsTab::General, text("常规设置", "General")),
        (TaskOptionsTab::Advanced, text("高级设置", "Advanced")),
    ] {
        let active = selected == tab;
        let mut control = div()
            .id(format!("home-options-{task:?}-{tab:?}"))
            .flex()
            .items_center()
            .justify_center()
            .h(px(26.))
            .px_3()
            .rounded_md()
            .tab_index(0)
            .cursor_pointer()
            .text_size(px(12.))
            .font_weight(if active {
                FontWeight::MEDIUM
            } else {
                FontWeight::NORMAL
            })
            .text_color(palette_rgb(if active {
                palette.foreground
            } else {
                palette.muted_foreground
            }))
            .bg(palette_rgb(if active {
                palette.card
            } else {
                palette.muted
            }))
            .focus_visible(|style| style.border_1().border_color(palette_rgb(current_render_palette().ring)))
            .child(label.get(language));

        if !active {
            control = control.hover(|style| {
                style
                    .text_color(palette_rgb(current_render_palette().foreground))
                    .bg(palette_rgb(current_render_palette().accent_surface))
            });
        }

        control = control.on_click(cx.listener(move |view, _, _, cx| {
            view.home.set_options_tab(task, tab);
            cx.stop_propagation();
            cx.notify();
        }));
        control =
            control.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    view.home.set_options_tab(task, tab);
                    cx.notify();
                }
            }));
        tabs = tabs.child(control);
    }
    tabs
}

fn set_windows_card(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    busy: bool,
    executing: bool,
) -> Div {
    let config = &app.home.tasks.set_windows;
    let size = config.set_win_size;
    let use_post_message = config.use_post_message;
    let language = app.state.settings.language;
    let expanded = app.home.is_expanded(FixedTaskId::SetWindows);
    let body = set_windows_details(app, cx, busy);
    task_card(
        cx,
        FixedTaskId::SetWindows,
        text("窗口设置", "Window Settings").get(language),
        "SET",
        true,
        expanded,
        executing,
        vec![
            preview_tag(
                text("分辨率", "Resolution").get(language),
                format!("{size}P"),
                false,
            ),
            preview_tag(
                text("异步输入", "Async Input").get(language),
                if use_post_message {
                    text("开", "On").get(language)
                } else {
                    text("关", "Off").get(language)
                },
                use_post_message,
            ),
        ],
        Some(body),
        None,
    )
}

fn set_windows_details(app: &mut AhabApp, cx: &mut Context<AhabApp>, busy: bool) -> Div {
    let config = &app.home.tasks.set_windows;
    let language = app.state.settings.language;
    let tab = app.home.options_tab(FixedTaskId::SetWindows);
    let position = home_select(
        app,
        cx,
        HomeSelect::WindowPosition,
        config.set_win_position.clone(),
        vec![
            (
                "0".to_owned(),
                text("屏幕居中", "Center").get(language).to_owned(),
            ),
            (
                "1".to_owned(),
                text("靠左对齐", "Align Left").get(language).to_owned(),
            ),
            (
                "2".to_owned(),
                text("靠右对齐", "Align Right").get(language).to_owned(),
            ),
            (
                "3".to_owned(),
                text("保持原位 (不移动)", "Keep Current (Do Not Move)")
                    .get(language)
                    .to_owned(),
            ),
        ],
        "window-position",
        144.,
        busy,
        Rc::new(|home, value| home.set_window_position(value)),
    );
    let size_button = home_select(
        app,
        cx,
        HomeSelect::WindowResolution,
        config.set_win_size.to_string(),
        vec![
            ("720".to_owned(), "1280 × 720".to_owned()),
            ("1080".to_owned(), "1920 × 1080".to_owned()),
            ("1440".to_owned(), "2560 × 1440".to_owned()),
        ],
        "window-size",
        144.,
        busy,
        Rc::new(|home, value| {
            if let Ok(value) = value.parse::<u16>() {
                home.set_window_size(value);
            }
        }),
    );
    let restore_switch = task_option_switch(
        "",
        config.set_reduce_miscontact,
        "reduce-miscontact",
        busy,
        cx,
        |home| {
            home.update_tasks(|tasks| {
                tasks.set_windows.set_reduce_miscontact = !tasks.set_windows.set_reduce_miscontact
            })
        },
    );
    let screenshot_button = home_select(
        app,
        cx,
        HomeSelect::ScreenshotInterval,
        format!("{:.1}", config.screenshot_interval),
        vec![
            (
                "0.2".to_owned(),
                format!("0.2s ({})", text("极速", "Fast").get(language)),
            ),
            (
                "0.5".to_owned(),
                format!("0.5s ({})", text("默认", "Default").get(language)),
            ),
            (
                "1.0".to_owned(),
                format!("1.0s ({})", text("较慢", "Slow").get(language)),
            ),
        ],
        "screenshot-interval",
        144.,
        busy,
        Rc::new(|home, value| {
            if let Ok(value) = value.parse::<f32>() {
                home.set_screenshot_interval(value);
            }
        }),
    );
    let mouse_button = home_select(
        app,
        cx,
        HomeSelect::MouseInterval,
        format!("{:.1}", config.mouse_action_interval),
        vec![
            ("0.1".to_owned(), "0.1s".to_owned()),
            (
                "0.3".to_owned(),
                format!("0.3s ({})", text("默认", "Default").get(language)),
            ),
            ("0.5".to_owned(), "0.5s".to_owned()),
        ],
        "mouse-interval",
        144.,
        busy,
        Rc::new(|home, value| {
            if let Ok(value) = value.parse::<f32>() {
                home.set_mouse_action_interval(value);
            }
        }),
    );
    let post_message = task_option_switch(
        "",
        config.use_post_message,
        "post-message",
        busy,
        cx,
        |home| {
            home.update_tasks(|tasks| {
                tasks.set_windows.use_post_message = !tasks.set_windows.use_post_message
            })
        },
    );

    let content = match tab {
        TaskOptionsTab::General => div()
            .flex()
            .flex_col()
            .gap_2()
            .child(control_row(
                text("窗口分辨率", "Window Resolution").get(language),
                size_button,
            ))
            .child(control_row(
                text("窗口位置", "Window Position").get(language),
                position,
            ))
            .child(control_row(
                text("结束后恢复窗口", "Restore Window on Finish").get(language),
                restore_switch,
            )),
        TaskOptionsTab::Advanced => div()
            .flex()
            .flex_col()
            .gap_2()
            .child(control_row(
                text("截图间隔", "Screenshot Interval").get(language),
                screenshot_button,
            ))
            .child(control_row(
                text("鼠标操作间隔", "Mouse Action Interval").get(language),
                mouse_button,
            ))
            .child(control_row(
                text("异步 PostMessage 输入", "Async PostMessage Input").get(language),
                post_message,
            )),
    };
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(options_tabs(FixedTaskId::SetWindows, tab, language, cx))
        .child(content)
}

fn daily_card(app: &mut AhabApp, cx: &mut Context<AhabApp>, busy: bool, executing: bool) -> Div {
    let enabled = app.home.tasks.enabledTasks.daily_task;
    let expanded = app.home.is_expanded(FixedTaskId::DailyTask);
    let language = app.state.settings.language;
    let config = &app.home.tasks.daily_task;
    let previews = vec![
        preview_tag(
            text("经验本", "EXP").get(language),
            format!("×{}", config.set_EXP_count),
            false,
        ),
        preview_tag(
            text("纽本", "Thread").get(language),
            format!("×{}", config.set_thread_count),
            false,
        ),
        preview_tag(
            text("连战", "Chain").get(language),
            if config.use_continuous_combat {
                format!("×{}", config.use_continuous_combat_select)
            } else {
                text("关", "Off").get(language).to_owned()
            },
            config.use_continuous_combat,
        ),
    ];
    let body = daily_details(app, cx, busy);
    task_card_with_toggle(
        cx,
        busy,
        FixedTaskId::DailyTask,
        text("日常任务", "Daily Tasks").get(language),
        "CAL",
        enabled,
        expanded,
        executing,
        previews,
        Some(body),
    )
}

fn daily_details(app: &mut AhabApp, cx: &mut Context<AhabApp>, busy: bool) -> Div {
    let config = &app.home.tasks.daily_task;
    let language = app.state.settings.language;
    let tab = app.home.options_tab(FixedTaskId::DailyTask);
    let exp_count = daily_counter(
        config.set_EXP_count,
        0,
        99,
        DailyCounter::Exp,
        "daily-exp-count",
        busy,
        cx,
    );
    let thread_count = daily_counter(
        config.set_thread_count,
        0,
        99,
        DailyCounter::Thread,
        "daily-thread-count",
        busy,
        cx,
    );
    let continuous_count = daily_counter(
        config.use_continuous_combat_select,
        1,
        10,
        DailyCounter::Continuous,
        "daily-continuous-count",
        busy,
        cx,
    );
    let daily_team = daily_team_select(
        app,
        text("默认日常编队", "Default Daily Team").get(language),
        11,
        config,
        busy,
        cx,
    );
    let targeted_exp = task_option_switch(
        text("经验本针对性配队", "Targeted EXP Lineups").get(language),
        config.targeted_teaming_EXP,
        "daily-targeted-exp",
        busy,
        cx,
        |home| {
            home.update_tasks(|tasks| {
                tasks.daily_task.targeted_teaming_EXP = !tasks.daily_task.targeted_teaming_EXP
            })
        },
    );
    let targeted_thread = task_option_switch(
        text("纽本针对性配队", "Targeted Thread Lineups").get(language),
        config.targeted_teaming_thread,
        "daily-targeted-thread",
        busy,
        cx,
        |home| {
            home.update_tasks(|tasks| {
                tasks.daily_task.targeted_teaming_thread = !tasks.daily_task.targeted_teaming_thread
            })
        },
    );

    let mut continuous = div().flex().items_center().gap_2();
    if config.use_continuous_combat {
        continuous = continuous.child(continuous_count);
    }
    continuous = continuous.child(task_option_switch(
        "",
        config.use_continuous_combat,
        "daily-continuous-enabled",
        busy,
        cx,
        |home: &mut HomeState| {
            home.update_tasks(|tasks| {
                tasks.daily_task.use_continuous_combat = !tasks.daily_task.use_continuous_combat
            })
        },
    ));

    let general = div()
        .flex()
        .flex_col()
        .gap_1()
        .child(control_row(
            text("经验本次数（0~99）", "EXP Dungeon Runs (0-99)").get(language),
            exp_count,
        ))
        .child(control_row(
            text("纽本次数（0~99）", "Thread Dungeon Runs (0-99)").get(language),
            thread_count,
        ))
        .child(control_row(
            text("默认日常编队", "Default Daily Team").get(language),
            daily_team,
        ))
        .child(control_row(
            text("连续作战", "Continuous Combat").get(language),
            continuous,
        ));

    let mut advanced = div().flex().flex_col().gap_2();
    advanced = advanced
        .child(control_row(
            text("经验本按属性配队", "Targeted EXP Lineups").get(language),
            targeted_exp,
        ))
        .child(control_row(
            text("纽本按星期配队", "Targeted Thread Lineups").get(language),
            targeted_thread,
        ));
    if config.targeted_teaming_EXP || config.targeted_teaming_thread {
        advanced = advanced.child(
            div().text_size(px(11.)).text_color(rgb(TEXT_MUTED)).child(
                text(
                    "选择对应日期使用的队伍（点击按钮循环队伍）",
                    "Choose the team for each day (click to cycle teams)",
                )
                .get(language),
            ),
        );
        let mut selectors = div().flex().flex_wrap().gap_1();
        if config.targeted_teaming_EXP {
            for (label, index) in [
                (text("经验斩击", "Mon/Tue (Slash)"), 0_u8),
                (text("经验突刺", "Wed/Thu (Pierce)"), 1),
                (text("经验打击", "Fri/Sat (Blunt)"), 2),
                (text("经验全属性", "Sun (All)"), 3),
            ] {
                selectors = selectors.child(daily_team_select(
                    app,
                    label.get(language),
                    index,
                    config,
                    busy,
                    cx,
                ));
            }
        }
        if config.targeted_teaming_thread {
            for (label, index) in [
                (text("周一", "Mon (Lust)"), 4_u8),
                (text("周二", "Tue (Sloth)"), 5),
                (text("周三", "Wed (Gluttony)"), 6),
                (text("周四", "Thu (Gloom)"), 7),
                (text("周五", "Fri (Pride)"), 8),
                (text("周六", "Sat (Envy)"), 9),
                (text("周日", "Sun (Wrath)"), 10),
            ] {
                selectors = selectors.child(daily_team_select(
                    app,
                    label.get(language),
                    index,
                    config,
                    busy,
                    cx,
                ));
            }
        }
        advanced = advanced.child(selectors);
    }

    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(options_tabs(FixedTaskId::DailyTask, tab, language, cx))
        .child(match tab {
            TaskOptionsTab::General => general,
            TaskOptionsTab::Advanced => advanced,
        })
}

fn daily_team_select(
    app: &AhabApp,
    label: &'static str,
    index: u8,
    config: &crate::model::DailyTaskConfig,
    busy: bool,
    cx: &mut Context<AhabApp>,
) -> Div {
    let value = match index {
        0 => config.EXP_day_1_2,
        1 => config.EXP_day_3_4,
        2 => config.EXP_day_5_6,
        3 => config.EXP_day_7,
        4 => config.thread_day_1,
        5 => config.thread_day_2,
        6 => config.thread_day_3,
        7 => config.thread_day_4,
        8 => config.thread_day_5,
        9 => config.thread_day_6,
        10 => config.thread_day_7,
        _ => config.daily_teams,
    };
    let language = app.state.settings.language;
    let mut options = app
        .teams
        .teams
        .iter()
        .take(99)
        .enumerate()
        .map(|(team_index, team)| ((team_index + 1).to_string(), team.name.clone()))
        .collect::<Vec<_>>();
    if options.is_empty() {
        options.push((
            "1".to_owned(),
            text("编队 1", "Team 1").get(language).to_owned(),
        ));
    }
    let on_change = Rc::new(move |home: &mut HomeState, value: String| {
        if let Ok(value) = value.parse::<u8>() {
            home.set_daily_team(index, value);
        }
    });
    let select = home_select(
        app,
        cx,
        HomeSelect::DailyTeam(index),
        value.to_string(),
        options,
        format!("daily-team-{index}"),
        if index == 11 { 144. } else { 120. },
        busy,
        on_change,
    );
    if index == 11 {
        select
    } else {
        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_1()
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .truncate()
                    .text_size(px(11.))
                    .text_color(rgb(TEXT_MUTED))
                    .child(label),
            )
            .child(select)
    }
}

fn reward_card(app: &mut AhabApp, cx: &mut Context<AhabApp>, busy: bool, executing: bool) -> Div {
    let enabled = app.home.tasks.enabledTasks.get_reward;
    let language = app.state.settings.language;
    let mode = app.home.tasks.get_reward.set_get_prize;
    let expanded = app.home.is_expanded(FixedTaskId::GetReward);
    let mode_select = home_select(
        app,
        cx,
        HomeSelect::RewardMode,
        mode.to_string(),
        vec![
            (
                "0".to_owned(),
                text(
                    "全部领取 (狂气 + 通行证 + 邮件)",
                    "Claim All (Lunacy + Pass + Mail)",
                )
                .get(language)
                .to_owned(),
            ),
            (
                "1".to_owned(),
                text("狂气与通行证奖励", "Lunacy & Pass Rewards")
                    .get(language)
                    .to_owned(),
            ),
            (
                "2".to_owned(),
                text("仅领取邮件奖励", "Mail Rewards Only")
                    .get(language)
                    .to_owned(),
            ),
        ],
        "reward-mode",
        176.,
        busy,
        Rc::new(|home, value| {
            if let Ok(value) = value.parse::<u8>() {
                home.set_reward_mode(value);
            }
        }),
    );
    let body = div().flex().flex_col().gap_2().child(control_row(
        text("领取模式", "Claim Mode").get(language),
        mode_select,
    ));
    task_card_with_toggle(
        cx,
        busy,
        FixedTaskId::GetReward,
        text("领取奖励", "Claim Rewards").get(language),
        "GFT",
        enabled,
        expanded,
        executing,
        vec![preview_tag(
            text("模式", "Mode").get(language),
            reward_mode_label(mode, language),
            false,
        )],
        Some(body),
    )
}

fn enkephalin_card(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    busy: bool,
    executing: bool,
) -> Div {
    let enabled = app.home.tasks.enabledTasks.buy_enkephalin;
    let language = app.state.settings.language;
    let expanded = app.home.is_expanded(FixedTaskId::BuyEnkephalin);
    let config = app.home.tasks.buy_enkephalin.clone();
    let body = enkephalin_details(app, cx, busy);
    task_card_with_toggle(
        cx,
        busy,
        FixedTaskId::BuyEnkephalin,
        text("狂气换体", "Refill Enkephalin").get(language),
        "ZAP",
        enabled,
        expanded,
        executing,
        vec![
            preview_tag(
                text("换体", "Refills").get(language),
                config.set_lunacy_to_enkephalin.to_string(),
                false,
            ),
            preview_tag(
                text("葛朗台", "Grandet").get(language),
                if config.Dr_Grandet_mode {
                    text("开", "On").get(language)
                } else {
                    text("关", "Off").get(language)
                },
                config.Dr_Grandet_mode,
            ),
        ],
        Some(body),
    )
}

fn enkephalin_details(app: &mut AhabApp, cx: &mut Context<AhabApp>, busy: bool) -> Div {
    let config = &app.home.tasks.buy_enkephalin;
    let language = app.state.settings.language;
    let tab = app.home.options_tab(FixedTaskId::BuyEnkephalin);
    let number = task_counter(
        config.set_lunacy_to_enkephalin,
        0,
        10,
        "enkephalin-count",
        busy,
        cx,
        Rc::new(|home, delta| home.adjust_enkephalin_count(delta)),
    );
    let general = div().flex().flex_col().gap_2().child(control_row(
        text("换体次数（0~10）", "Refill Times (0-10)").get(language),
        number,
    ));

    let detail = detail_switch(
        config.Dr_Grandet_mode,
        FixedTaskId::BuyEnkephalin,
        cx,
        busy,
        "grandet-mode",
    );
    let skip = task_option_switch(
        "",
        config.skip_enkephalin,
        "skip-enkephalin",
        busy,
        cx,
        |home| {
            home.update_tasks(|tasks| {
                tasks.buy_enkephalin.skip_enkephalin = !tasks.buy_enkephalin.skip_enkephalin
            })
        },
    );
    let advanced = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(control_row(
            text("葛朗台模式", "Dr. Grandet Mode").get(language),
            detail,
        ))
        .child(
            div()
                .pt_2()
                .border_t_1()
                .border_color(rgb(BORDER))
                .child(control_row(
                    text("跳过模块合成", "Skip Module Crafting").get(language),
                    skip,
                ))
                .child(
                    div().text_size(px(11.0)).text_color(rgb(TEXT_MUTED)).child(
                        text(
                            "除狂气换体外，不自动将多余体力合成为脑啡肽模块。",
                            "Do not convert surplus enkephalin into modules.",
                        )
                        .get(language),
                    ),
                ),
        );
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(options_tabs(
            FixedTaskId::BuyEnkephalin,
            tab,
            app.state.settings.language,
            cx,
        ))
        .child(match tab {
            TaskOptionsTab::General => general,
            TaskOptionsTab::Advanced => advanced,
        })
}

fn mirror_card(app: &mut AhabApp, cx: &mut Context<AhabApp>, busy: bool, executing: bool) -> Div {
    let enabled = app.home.tasks.enabledTasks.mirror;
    let language = app.state.settings.language;
    let expanded = app.home.is_expanded(FixedTaskId::Mirror);
    let config = &app.home.tasks.mirror;
    let number = if config.infinite_dungeons {
        badge(
            text("∞ 无限", "∞ Infinite").get(language),
            BadgeTone::Neutral,
        )
    } else {
        task_counter(
            config.set_mirror_count,
            1,
            99,
            "mirror-count",
            busy,
            cx,
            Rc::new(|home, delta| home.adjust_mirror_count(delta)),
        )
    };
    let infinite = task_option_switch(
        text("无限模式", "Infinite Mode").get(language),
        config.infinite_dungeons,
        "mirror-infinite",
        busy,
        cx,
        |home| {
            home.toggle_mirror_option(MirrorOption::Infinite);
        },
    );
    let hard = detail_switch(
        config.hard_mirror,
        FixedTaskId::Mirror,
        cx,
        busy,
        "hard-mirror",
    );
    let options = [
        (
            text("不使用每周加成", "Do Not Use Weekly Bonuses").get(language),
            config.no_weekly_bonuses,
            "mirror-no-weekly",
            MirrorOption::NoWeeklyBonuses,
        ),
        (
            text("只打三层", "Exit at Floor 3").get(language),
            config.floor_3_exit,
            "mirror-floor-three",
            MirrorOption::FloorThreeExit,
        ),
        (
            text("保存困牢奖励", "Save Hard Rewards").get(language),
            config.save_rewards,
            "mirror-save-rewards",
            MirrorOption::SaveRewards,
        ),
        (
            text("困牢单次加成", "Single Bonus per Run").get(language),
            config.hard_mirror_single_bonuses,
            "mirror-single-bonus",
            MirrorOption::HardSingleBonuses,
        ),
        (
            text("第 5 层选活动包", "Pick Event Pack on F5").get(language),
            config.select_event_pack,
            "mirror-select-event",
            MirrorOption::SelectEventPack,
        ),
        (
            text("第 5 层跳过活动包", "Skip Event Pack on F5").get(language),
            config.skip_event_pack,
            "mirror-skip-event",
            MirrorOption::SkipEventPack,
        ),
        (
            text("再次领取奖励", "Re-claim Rewards").get(language),
            config.re_claim_rewards,
            "mirror-reclaim",
            MirrorOption::ReclaimRewards,
        ),
        (
            text("不跳过白棉花", "Do Not Skip Gossypium").get(language),
            config.not_skip_whitegossypium,
            "mirror-cotton",
            MirrorOption::NotSkipCotton,
        ),
        (
            text("战斗直到全灭", "Fight to the Last Sinner").get(language),
            config.fight_to_last_man,
            "mirror-last-man",
            MirrorOption::FightToLast,
        ),
        (
            text("键盘寻路", "Keyboard Pathfinding").get(language),
            config.mirror_keyboard_navigation,
            "mirror-keyboard",
            MirrorOption::KeyboardNavigation,
        ),
        (
            text("简易键盘寻路", "Simple Keyboard Pathfinding").get(language),
            config.mirror_keyboard_simple_pathfinding,
            "mirror-simple-keyboard",
            MirrorOption::SimplePathfinding,
        ),
    ];
    let mut options_view = div().flex().flex_col().gap_1();
    for (label, value, id, field) in options {
        let toggle = task_option_switch(label, value, id, busy, cx, move |home| {
            home.toggle_mirror_option(field);
        });
        options_view = options_view.child(control_row(label, toggle));
    }
    let progress = app.home.mirror_progress.as_ref().map(|progress| {
        div()
            .flex()
            .items_center()
            .justify_between()
            .px_2()
            .py_1()
            .rounded_md()
            .bg(rgba((ACCENT << 8) | 0x22))
            .child(div().text_size(px(11.0)).text_color(rgb(ACCENT)).child(
                if progress.isInfinite {
                    format!(
                        "{} {} / ∞",
                        text("镜牢进度", "Mirror Progress").get(language),
                        progress.current
                    )
                } else {
                    format!(
                        "{} {} / {}",
                        text("镜牢进度", "Mirror Progress").get(language),
                        progress.current,
                        progress.total
                    )
                },
            ))
            .child(badge(
                if progress.isHard {
                    text("困难", "Hard").get(language)
                } else {
                    text("普通", "Normal").get(language)
                },
                BadgeTone::Accent,
            ))
    });
    let tab = app.home.options_tab(FixedTaskId::Mirror);
    let general = div()
        .flex()
        .flex_col()
        .gap_2()
        .children(progress)
        .child(control_row(
            text("运行次数", "Run Count").get(language),
            number,
        ))
        .child(control_row(
            text("无限模式", "Infinite Mode").get(language),
            infinite,
        ))
        .child(control_row(
            text("困难镜牢", "Hard Mirror Dungeon").get(language),
            hard,
        ));
    let advanced = div()
        .flex()
        .flex_wrap()
        .gap_2()
        .pt_2()
        .border_t_1()
        .border_color(rgb(BORDER))
        .child(options_view);
    let body = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(options_tabs(
            FixedTaskId::Mirror,
            tab,
            app.state.settings.language,
            cx,
        ))
        .child(match tab {
            TaskOptionsTab::General => general,
            TaskOptionsTab::Advanced => advanced,
        });
    task_card_with_toggle(
        cx,
        busy,
        FixedTaskId::Mirror,
        text("坐牢设置 (镜牢)", "Mirror Dungeon").get(language),
        "CMP",
        enabled,
        expanded,
        executing,
        vec![
            preview_tag(
                text("坐牢", "Runs").get(language),
                if config.infinite_dungeons {
                    "∞".to_owned()
                } else {
                    format!("{}", config.set_mirror_count)
                },
                config.infinite_dungeons,
            ),
            preview_tag(
                text("难度", "Difficulty").get(language),
                if config.hard_mirror {
                    text("困难", "Hard").get(language)
                } else {
                    text("普通", "Normal").get(language)
                },
                config.hard_mirror,
            ),
        ],
        Some(body),
    )
}

fn ahab_card(app: &mut AhabApp, cx: &mut Context<AhabApp>, busy: bool, executing: bool) -> Div {
    let enabled = app.home.tasks.enabledTasks.resonate_with_Ahab;
    let language = app.state.settings.language;
    task_card_with_toggle(
        cx,
        busy,
        FixedTaskId::ResonateWithAhab,
        text("亚哈共鸣", "Ahab Resonance").get(language),
        "RAD",
        enabled,
        false,
        executing,
        vec![preview_tag(
            text("语录", "Quote").get(language),
            if enabled {
                text("开启", "Enabled").get(language)
            } else {
                text("关闭", "Disabled").get(language)
            },
            enabled,
        )],
        None,
    )
}

fn daily_counter(
    current: u8,
    min: u8,
    max: u8,
    field: DailyCounter,
    id: &'static str,
    busy: bool,
    cx: &mut Context<AhabApp>,
) -> Div {
    task_counter(
        current,
        min,
        max,
        id,
        busy,
        cx,
        Rc::new(move |home, delta| home.adjust_daily_counter(field, delta)),
    )
}

fn task_counter(
    current: u8,
    min: u8,
    max: u8,
    id: &'static str,
    busy: bool,
    cx: &mut Context<AhabApp>,
    action: Rc<dyn Fn(&mut HomeState, i8)>,
) -> Div {
    let mut decrement = button("", ButtonVariant::Outline)
        .id(format!("{id}-decrement"))
        .w(px(26.))
        .h(px(26.))
        .p_0()
        .child(action_icon(ICON_MINUS, 13., TEXT));
    if !busy && current > min {
        let click_action = action.clone();
        let key_action = action.clone();
        decrement = decrement
            .on_click(cx.listener(move |view, _, _, cx| {
                click_action(&mut view.home, -1);
                cx.stop_propagation();
                cx.notify();
            }))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    key_action(&mut view.home, -1);
                    cx.notify();
                }
            }));
    } else {
        decrement = decrement.opacity(0.45).cursor_not_allowed();
    }

    let mut increment = button("", ButtonVariant::Outline)
        .id(format!("{id}-increment"))
        .w(px(26.))
        .h(px(26.))
        .p_0()
        .child(action_icon(ICON_PLUS, 13., TEXT));
    if !busy && current < max {
        let click_action = action.clone();
        let key_action = action.clone();
        increment = increment
            .on_click(cx.listener(move |view, _, _, cx| {
                click_action(&mut view.home, 1);
                cx.stop_propagation();
                cx.notify();
            }))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    key_action(&mut view.home, 1);
                    cx.notify();
                }
            }));
    } else {
        increment = increment.opacity(0.45).cursor_not_allowed();
    }

    div()
        .flex()
        .items_center()
        .gap_1()
        .child(decrement)
        .child(
            div()
                .w(px(30.))
                .text_center()
                .font_family("Consolas")
                .text_size(px(12.))
                .text_color(rgb(TEXT))
                .child(current.to_string()),
        )
        .child(increment)
}

fn task_option_switch<F>(
    _label: &'static str,
    value: bool,
    id: &'static str,
    busy: bool,
    cx: &mut Context<AhabApp>,
    action: F,
) -> gpui::Stateful<Div>
where
    F: Fn(&mut HomeState) + 'static,
{
    let mut control = switch(value).id(id);
    if !busy {
        let action = Rc::new(action);
        let click_action = action.clone();
        control = control.on_click(cx.listener(move |view, _, _, cx| {
            click_action(&mut view.home);
            cx.stop_propagation();
            cx.notify();
        }));
        control =
            control.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if matches!(
                    event.keystroke.key.to_ascii_lowercase().as_str(),
                    "enter" | "space"
                ) {
                    window.prevent_default();
                    action(&mut view.home);
                    cx.notify();
                }
            }));
    } else {
        control = control.opacity(0.45).cursor_not_allowed();
    }
    control
}

fn home_select(
    app: &AhabApp,
    cx: &mut Context<AhabApp>,
    select: HomeSelect,
    current: impl Into<String>,
    options: Vec<(String, String)>,
    id: impl Into<String>,
    width: f32,
    disabled: bool,
    on_change: Rc<dyn Fn(&mut HomeState, String)>,
) -> Div {
    let current = current.into();
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
    let open = app.home.is_select_open(select);
    let palette = crate::components::style::current_render_palette();
    let id = id.into();
    let mut trigger = select_trigger(selected_label, open, &palette)
        .id(id.clone())
        .w(px(width));

    if disabled {
        trigger = trigger.opacity(0.5).cursor_not_allowed();
    } else {
        trigger = trigger.on_click(cx.listener(move |view, _, _, cx| {
            if open {
                view.home.close_select();
            } else {
                view.home.toggle_select(select);
            }
            cx.stop_propagation();
            cx.notify();
        }));
        let key_change = on_change.clone();
        let key_values = values.clone();
        let key_current = current.clone();
        trigger =
            trigger.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                let key = event.keystroke.key.to_ascii_lowercase();
                if key == "escape" {
                    window.prevent_default();
                    view.home.close_select();
                    cx.notify();
                    return;
                }
                if matches!(key.as_str(), "enter" | "space") {
                    window.prevent_default();
                    view.home.toggle_select(select);
                    cx.notify();
                    return;
                }

                let current_index = key_values
                    .iter()
                    .position(|candidate| candidate == &key_current)
                    .unwrap_or(0);
                let next_index = match key.as_str() {
                    "left" | "arrowleft" | "up" | "arrowup" => Some(
                        (current_index as isize - 1).rem_euclid(key_values.len().max(1) as isize)
                            as usize,
                    ),
                    "right" | "arrowright" => Some((current_index + 1) % key_values.len().max(1)),
                    "down" | "arrowdown" if open => {
                        Some((current_index + 1) % key_values.len().max(1))
                    }
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
                        view.home.toggle_select(select);
                    } else {
                        key_change(&mut view.home, value.clone());
                        if !open {
                            view.home.close_select();
                        }
                    }
                    cx.notify();
                }
            }));
    }

    let mut option_list = div().flex().flex_col().gap_1();
    for (value, option_label) in options {
        let selected = value == current;
        let option_id = format!("{id}-option-{value}");
        let click_change = on_change.clone();
        let click_value = value.clone();
        let mut option = select_option(option_label, selected, &palette)
            .id(option_id)
            .on_click(cx.listener(move |view, _, _, cx| {
                click_change(&mut view.home, click_value.clone());
                view.home.close_select();
                cx.stop_propagation();
                cx.notify();
            }));
        let key_change = on_change.clone();
        let key_value = value;
        option = option.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
            if is_activation_key(event) {
                window.prevent_default();
                key_change(&mut view.home, key_value.clone());
                view.home.close_select();
                cx.notify();
            } else if event.keystroke.key.eq_ignore_ascii_case("escape") {
                window.prevent_default();
                view.home.close_select();
                cx.notify();
            }
        }));
        option_list = option_list.child(option);
    }

    let mut root = div().relative().w(px(width)).child(trigger);
    if open {
        // Home Selects live inside the scrolling task list. Defer the popup so
        // later control rows cannot paint over it, matching the floating menu
        // behavior of the React SelectContent component.
        let popup = select_popup(option_list, &palette)
            .shadow_sm()
            .on_mouse_down_out(cx.listener(move |view, _, _, cx| {
                view.home.close_select();
                cx.notify();
            }));
        root = root.child(deferred(popup).priority(10));
    }
    root
}

fn is_activation_key(event: &KeyDownEvent) -> bool {
    matches!(
        event.keystroke.key.to_ascii_lowercase().as_str(),
        "enter" | "space"
    )
}

fn preview_tag(label: impl Into<String>, value: impl Into<String>, highlight: bool) -> Div {
    let label = label.into();
    let tone = if highlight {
        BadgeTone::Accent
    } else {
        BadgeTone::Neutral
    };
    badge(format!("{label} {}", value.into()), tone)
}

fn task_card_with_toggle(
    cx: &mut Context<AhabApp>,
    busy: bool,
    task: FixedTaskId,
    title: impl Into<String>,
    icon: &'static str,
    enabled: bool,
    expanded: bool,
    executing: bool,
    preview_tags: Vec<Div>,
    body: Option<Div>,
) -> Div {
    let mut toggle = switch_accent(enabled).id(task_id(task));
    if !busy {
        toggle = toggle.on_click(cx.listener(move |view, _, _, cx| {
            view.home.toggle_task(task);
            cx.stop_propagation();
            cx.notify();
        }));
        toggle = toggle.on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
            if matches!(
                event.keystroke.key.to_ascii_lowercase().as_str(),
                "enter" | "space"
            ) {
                window.prevent_default();
                view.home.toggle_task(task);
                cx.notify();
            }
        }));
    } else {
        toggle = toggle.opacity(0.45).cursor_not_allowed();
    }
    task_card(
        cx,
        task,
        title,
        icon,
        enabled,
        expanded,
        executing,
        preview_tags,
        body,
        Some(toggle),
    )
}

fn task_card(
    cx: &mut Context<AhabApp>,
    task: FixedTaskId,
    title: impl Into<String>,
    icon: &'static str,
    enabled: bool,
    expanded: bool,
    executing: bool,
    preview_tags: Vec<Div>,
    body: Option<Div>,
    toggle: Option<gpui::Stateful<Div>>,
) -> Div {
    let has_options = body.is_some();
    let mut header = div()
        .id(format!("task-header-{}", task_id(task)))
        .flex()
        .items_center()
        .gap_2()
        .px_3()
        .h(px(36.0))
        .py_0();
    if has_options {
        header = header
            .cursor_pointer()
            .tab_index(0)
            .hover(|style| style.bg(rgba((SURFACE_HOVER << 8) | 0x35)))
            .on_click(cx.listener(move |view, _, _, cx| {
                view.home.toggle_expanded(task);
                cx.notify();
            }))
            .on_key_down(cx.listener(move |view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    view.home.toggle_expanded(task);
                    cx.notify();
                }
            }));
    }

    if let Some(toggle) = toggle {
        header = header.child(div().flex_none().child(toggle));
    }
    header = header.child(
        div()
            .flex()
            .items_center()
            .gap_2()
            .flex_1()
            .min_w_0()
            .child(task_icon(icon, executing))
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_size(px(14.0))
                    .text_color(rgb(if executing {
                        ACCENT
                    } else if !enabled {
                        TEXT_MUTED
                    } else {
                        TEXT
                    }))
                    .child(title.into()),
            ),
    );
    if !expanded {
        header = header.child(
            div()
                .flex()
                .items_center()
                .justify_end()
                .gap_1()
                .min_w_0()
                .flex_1()
                .overflow_hidden()
                .children(preview_tags),
        );
    }
    if has_options {
        header = header.child(
            div()
                .w(px(16.0))
                .flex_none()
                .text_center()
                .text_size(px(16.0))
                .text_color(rgb(TEXT_MUTED))
                .child(action_icon(
                    if expanded {
                        ICON_CHEVRON_UP
                    } else {
                        ICON_CHEVRON_DOWN
                    },
                    14.,
                    TEXT_MUTED,
                )),
        );
    }

    let mut root = div()
        .relative()
        .min_w_0()
        .overflow_hidden()
        .rounded_lg()
        .border_1()
        .border_color(if executing { rgb(ACCENT) } else { rgba(0) })
        .bg(rgb(SURFACE));
    root = root.child(header);
    if expanded && let Some(body) = body {
        let body = div()
            .relative()
            .bg(rgba((SURFACE_HOVER << 8) | 0x59))
            .px_3()
            .py(px(10.0))
            .child(body)
            .with_animation(
                format!("task-details-{}", task_id(task)),
                Animation::new(Duration::from_millis(150))
                    .with_easing(gpui::ease_out_quint()),
                |body, progress| body.opacity(progress).top(px(-4.0 * (1.0 - progress))),
            );
        root = root.child(body);
    }
    if executing {
        root = root.child(running_sweep());
    }
    root
}

fn task_icon(label: &'static str, executing: bool) -> Div {
    let color = if executing { BACKGROUND } else { TEXT_MUTED };
    if label == "SET" {
        return div()
            .flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .w(px(20.0))
                    .h(px(20.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded_md()
                    .bg(rgb(if executing { ACCENT } else { SURFACE_HOVER }))
                    .text_color(rgb(color))
                    .font_family("monospace")
                    .text_size(px(9.0))
                    .child(label),
            )
            .child(action_icon(ICON_SLIDERS, 16., color));
    }

    div()
        .w(px(16.0))
        .h(px(16.0))
        .flex()
        .items_center()
        .justify_center()
        .text_color(rgb(color))
        .child(action_icon(task_icon_data(label), 16., color))
}

fn running_sweep() -> Div {
    let palette = current_render_palette();
    let is_dark = matches!(palette.scheme, crate::components::style::ColorScheme::Dark);
    let track_color = rgba((palette.brand.rgb_hex() << 8) | if is_dark { 0x38 } else { 0x28 });
    let core_color = palette_rgb(palette.brand);

    div()
        .absolute()
        .top(px(6.0))
        .bottom(px(6.0))
        .left(px(4.0))
        .w(px(3.0))
        .rounded_full()
        .overflow_hidden()
        .bg(track_color)
        .child(
            div()
                .absolute()
                .left_0()
                .right_0()
                .h(relative(0.45))
                .rounded_full()
                .bg(core_color)
                .with_animation(
                    "home-task-sweep",
                    Animation::new(Duration::from_millis(1300)).repeat(),
                    |element, progress| element.top(relative(progress * 1.45 - 0.45)),
                ),
        )
}

fn detail_switch(
    checked: bool,
    task: FixedTaskId,
    cx: &mut Context<AhabApp>,
    busy: bool,
    id: &'static str,
) -> gpui::Stateful<Div> {
    task_option_switch("", checked, id, busy, cx, move |home| {
        home.toggle_detail(task);
    })
}

fn control_row(label: impl Into<String>, control: impl IntoElement) -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .py_1()
        .child(
            div()
                .flex_1()
                .min_w_0()
                .text_size(px(13.))
                .text_color(rgb(TEXT))
                .child(label.into()),
        )
        .child(div().flex_none().child(control))
}

fn execution_toolbar(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    busy: bool,
    state: ExecutionState,
) -> Div {
    let language = app.state.settings.language;
    let palette = current_render_palette();
    let is_dark = matches!(palette.scheme, crate::components::style::ColorScheme::Dark);

    let mut select_all = button("", ButtonVariant::Outline)
        .id("select-all")
        .h(px(32.0))
        .px(px(10.0))
        .gap(px(4.0))
        .text_size(px(12.0))
        .child(action_icon(ICON_CHECK_SQUARE, 14., TEXT))
        .child(text("全选", "Select All").get(language));
    let mut clear_all = button("", ButtonVariant::Outline)
        .id("clear-all")
        .h(px(32.0))
        .px(px(10.0))
        .gap(px(4.0))
        .text_size(px(12.0))
        .child(action_icon(ICON_ROTATE, 14., TEXT_MUTED))
        .child(
            div()
                .text_color(rgb(TEXT_MUTED))
                .child(text("清空", "Clear All").get(language)),
        );
    if !busy {
        select_all = select_all.on_click(cx.listener(|view, _, _, cx| {
            view.home.set_all_tasks(true);
            cx.stop_propagation();
            cx.notify();
        }));
        clear_all = clear_all.on_click(cx.listener(|view, _, _, cx| {
            view.home.set_all_tasks(false);
            cx.stop_propagation();
            cx.notify();
        }));
    }

    let mut after_button = button("", ButtonVariant::Ghost)
        .id("after-completion-open")
        .h(px(32.0))
        .flex_none()
        .max_w(px(280.0))
        .min_w_0()
        .px(px(10.0))
        .gap(px(6.0))
        .text_size(px(12.0))
        .child(action_icon(ICON_SLIDERS, 14., ACCENT))
        .child(div().min_w_0().truncate().child(after_completion_summary(
            &app.home.tasks.afterCompletion,
            app.state.settings.language,
        )));
    if !busy {
        after_button = after_button.on_click(cx.listener(|view, _, _, cx| {
            view.home.set_after_completion_open(true);
            cx.stop_propagation();
            cx.notify();
        }));
    }

    let (pause_icon, pause_label, pause_icon_color) = if state == ExecutionState::Paused {
        (
            ICON_PLAY,
            text("继续", "Resume").get(language),
            palette.success.rgb_hex(),
        )
    } else {
        (
            ICON_PAUSE,
            text("暂停", "Pause").get(language),
            palette.warning.rgb_hex(),
        )
    };
    let mut pause = button("", ButtonVariant::Outline)
        .id("pause-resume")
        .h(px(34.0))
        .px(px(12.0))
        .gap(px(6.0))
        .text_size(px(12.0))
        .child(action_icon(pause_icon, 14., pause_icon_color))
        .child(pause_label);
    if busy {
        pause = pause.on_click(cx.listener(|view, _, _, cx| {
            view.home.pause_or_resume();
            cx.stop_propagation();
            cx.notify();
        }));
    }

    let (run_icon, run_label, run_variant) = if busy {
        (ICON_SQUARE, "Stop!", ButtonVariant::Destructive)
    } else {
        (ICON_PLAY, "Link Start!", ButtonVariant::Default)
    };
    let mut run = button("", run_variant)
        .id("start-stop")
        .h(px(34.0))
        .px(px(16.0))
        .gap(px(6.0))
        .text_size(px(12.0))
        .font_weight(FontWeight::SEMIBOLD)
        .child(brand_action_icon(run_icon, 14.))
        .child(run_label);
    if !busy {
        let kbd_bg = if is_dark {
            rgba(0xffffff33)
        } else {
            rgba(0x00000028)
        };
        run = run.child(
            div()
                .rounded_sm()
                .bg(kbd_bg)
                .px(px(5.0))
                .py(px(1.5))
                .font_family("monospace")
                .text_size(px(10.0))
                .font_weight(FontWeight::NORMAL)
                .text_color(palette_rgb(palette.brand_foreground))
                .child("F10"),
        );
    }
    if busy {
        run = run.on_click(cx.listener(|view, _, _, cx| {
            view.home.stop();
            cx.stop_propagation();
            cx.notify();
        }));
    } else {
        run = run.on_click(cx.listener(|view, _, _, cx| {
            let language = view.state.settings.language;
            if view.home.selected_task_count() == 0 {
                view.show_toast(
                    crate::shell::ToastKind::Warning,
                    text(
                        "请至少勾选一个要执行的任务",
                        "Select at least one task to run",
                    )
                    .get(language),
                    cx,
                );
            } else if view.home.device_status != ConnectionStatus::Connected {
                if view.home.devices.is_empty() {
                    view.home.open_select = Some(HomeSelect::Device);
                    view.home.device_error = Some(
                        text(
                            "未检测到可用的游戏窗口或模拟器，请先启动游戏并连接",
                            "No game window or emulator detected, please launch and connect first",
                        )
                        .get(language)
                        .to_owned(),
                    );
                    view.show_toast(
                        crate::shell::ToastKind::Warning,
                        text(
                            "未连接设备，请先选择游戏窗口或模拟器",
                            "Device not connected, please select game window first",
                        )
                        .get(language),
                        cx,
                    );
                } else {
                    let last_id_opt = view
                        .state
                        .settings
                        .lastDeviceId
                        .clone()
                        .filter(|id| view.home.devices.iter().any(|d| &d.id == id));
                    if let Some(last_id) = last_id_opt {
                        view.show_toast(
                            crate::shell::ToastKind::Info,
                            text(
                                "正在自动连接上次使用的设备...",
                                "Auto-connecting to last used device...",
                            )
                            .get(language),
                            cx,
                        );
                        view.select_device(last_id, cx);
                    } else {
                        view.home.open_select = Some(HomeSelect::Device);
                        view.show_toast(
                            crate::shell::ToastKind::Warning,
                            text(
                                "请先选择并连接游戏窗口或模拟器",
                                "Please select and connect a device first",
                            )
                            .get(language),
                            cx,
                        );
                    }
                }
            } else {
                view.home.start();
            }
            cx.stop_propagation();
            cx.notify();
        }));
    }

    let mut command_group = div().min_w_0().flex().flex_wrap().items_center().gap_2();
    if busy {
        command_group = command_group.child(pause);
    }
    command_group = command_group.child(run);

    div()
        .flex_none()
        .min_w_0()
        .flex()
        .flex_wrap()
        .items_center()
        .justify_between()
        .gap_3()
        .mx(px(14.0))
        .mb(px(10.0))
        .mt(px(4.0))
        .rounded_lg()
        .border_1()
        .border_color(rgba(0))
        .bg(palette_rgb(palette.card))
        .px(px(12.0))
        .py(px(8.0))
        .child(
            div()
                .min_w_0()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_1()
                .child(select_all)
                .child(clear_all)
                .child(
                    div()
                        .mx(px(4.0))
                        .w(px(1.0))
                        .h(px(16.0))
                        .bg(palette_rgb(palette.input)),
                )
                .child(after_button),
        )
        .child(command_group)
}

fn after_completion_editor(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    busy: bool,
) -> gpui::AnyElement {
    if !app.home.after_completion_open {
        return div().into_any_element();
    }
    let config = app
        .home
        .after_completion_draft
        .clone()
        .unwrap_or_else(|| app.home.tasks.afterCompletion.clone());
    let language = app.state.settings.language;
    let mut exits = div().flex().flex_col().gap_2();
    for action in [
        AfterExitAction::ExitGame,
        AfterExitAction::ExitEmulator,
        AfterExitAction::ExitAalc,
    ] {
        let control = task_option_switch(
            "",
            config.actions.contains(&action),
            match action {
                AfterExitAction::ExitGame => "after-exit-game",
                AfterExitAction::ExitEmulator => "after-exit-emulator",
                AfterExitAction::ExitAalc => "after-exit-aalc",
            },
            busy,
            cx,
            move |home| home.toggle_after_completion_draft(action),
        );
        exits = exits.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap_3()
                .py_0()
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(TEXT))
                        .child(after_exit_label(action, language)),
                )
                .child(control),
        );
    }

    let power = home_select(
        app,
        cx,
        HomeSelect::AfterPowerAction,
        after_power_key(config.powerAction),
        vec![
            (
                "none".to_owned(),
                after_power_label(AfterPowerAction::None, language).to_owned(),
            ),
            (
                "sleep".to_owned(),
                after_power_label(AfterPowerAction::Sleep, language).to_owned(),
            ),
            (
                "hibernate".to_owned(),
                after_power_label(AfterPowerAction::Hibernate, language).to_owned(),
            ),
            (
                "lock".to_owned(),
                after_power_label(AfterPowerAction::Lock, language).to_owned(),
            ),
            (
                "shutdown".to_owned(),
                after_power_label(AfterPowerAction::Shutdown, language).to_owned(),
            ),
        ],
        "after-power-action",
        464.,
        busy,
        Rc::new(|home, value| {
            if let Some(action) = parse_after_power_action(&value) {
                home.set_after_completion_draft_power(action);
            }
        }),
    );

    let mut close = button("", ButtonVariant::Ghost)
        .id("after-completion-close")
        .w(px(32.))
        .h(px(32.))
        .px_0()
        .child(action_icon(ICON_X, 16., TEXT_MUTED));
    close = close.on_click(cx.listener(|view, _, _, cx| {
        view.home.set_after_completion_open(false);
        cx.notify();
    }));
    close = close.on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        if is_activation_key(event) {
            window.prevent_default();
            view.home.set_after_completion_open(false);
            cx.notify();
        }
    }));

    let mut apply_once = button(
        text("仅本次生效", "Apply Once").get(language),
        ButtonVariant::Outline,
    )
    .id("after-completion-apply-once");
    let mut save_default = button(
        text("保存为默认", "Save as Default").get(language),
        ButtonVariant::Default,
    )
    .id("after-completion-save-default");
    if !busy {
        apply_once = apply_once.on_click(cx.listener(|view, _, _, cx| {
            view.home.apply_after_completion(false);
            cx.notify();
        }));
        apply_once =
            apply_once.on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    view.home.apply_after_completion(false);
                    cx.notify();
                }
            }));
        save_default = save_default.on_click(cx.listener(|view, _, _, cx| {
            view.home.apply_after_completion(true);
            cx.notify();
        }));
        save_default =
            save_default.on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    view.home.apply_after_completion(true);
                    cx.notify();
                }
            }));
    } else {
        apply_once = apply_once.opacity(0.45).cursor_not_allowed();
        save_default = save_default.opacity(0.45).cursor_not_allowed();
    }

    let exit_group = div()
        .flex()
        .flex_col()
        .gap_2()
        .rounded_lg()
        .border_1()
        .border_color(palette_rgb(current_render_palette().input))
        .bg(palette_rgb(current_render_palette().card))
        .p_3()
        .child(exits);
    let exits_section = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(TEXT_MUTED))
                .child(text("退出动作（可多选）", "Exit Actions (Multi-select)").get(language)),
        )
        .child(exit_group);
    let power_section = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_size(px(11.))
                .text_color(rgb(TEXT_MUTED))
                .child(text("最终电源动作", "Power Action").get(language)),
        )
        .child(power);
    let dialog = card(
        div()
            .flex()
            .flex_col()
            .gap(px(15.))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(px(16.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(rgb(TEXT))
                            .child(text("结束后操作", "After Completion Actions").get(language)),
                    )
                    .child(close),
            )
            .child(exits_section)
            .child(power_section)
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap_2()
                    .pt_1()
                    .child(apply_once)
                    .child(save_default),
            ),
    )
    .p_6()
    .w(px(512.0))
    .max_w_full()
    .id("after-completion-dialog")
    .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()));

    let mut overlay = div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .flex()
        .items_center()
        .justify_center()
        .p_4()
        .bg(rgba(0x00000080))
        .id("after-completion-overlay")
        .child(dialog)
        .on_click(cx.listener(|view, _, _, cx| {
            view.home.set_after_completion_open(false);
            cx.notify();
        }));
    overlay = overlay.capture_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        if event.keystroke.key.eq_ignore_ascii_case("escape") {
            window.prevent_default();
            cx.stop_propagation();
            view.home.set_after_completion_open(false);
            cx.notify();
        }
    }));
    overlay.into_any_element()
}

fn after_completion_summary(
    config: &crate::model::AfterCompletionConfig,
    language: Language,
) -> String {
    let power = match config.powerAction {
        crate::model::AfterPowerAction::None => None,
        action => Some(after_power_label(action, language)),
    };
    let mode = if config.keepAfterCompletion {
        text("默认", "Default").get(language)
    } else {
        text("本次", "This run").get(language)
    };

    let summary = if config.actions.is_empty() && power.is_none() {
        text("什么也不干", "Do nothing").get(language).to_owned()
    } else if config.actions.is_empty() {
        power.unwrap_or_default().to_owned()
    } else {
        let exits = config
            .actions
            .iter()
            .map(|action| after_exit_short_label(*action, language))
            .collect::<Vec<_>>()
            .join(if matches!(language, Language::ZhCn) {
                "与"
            } else {
                ", "
            });
        let prefix = text("退出", "Exit ").get(language);
        match power {
            Some(power) => format!(
                "{prefix}{exits}{}{}",
                text("后", " then ").get(language),
                power
            ),
            None => format!("{prefix}{exits}"),
        }
    };

    format!("{summary} ({mode})")
}

fn after_exit_label(action: AfterExitAction, language: Language) -> &'static str {
    match action {
        AfterExitAction::ExitGame => text("退出游戏", "Exit Game").get(language),
        AfterExitAction::ExitEmulator => text("退出模拟器", "Exit Emulator").get(language),
        AfterExitAction::ExitAalc => text("退出 AALC", "Exit AALC").get(language),
    }
}

fn after_exit_short_label(action: AfterExitAction, language: Language) -> &'static str {
    match action {
        AfterExitAction::ExitGame => text("游戏", "Game").get(language),
        AfterExitAction::ExitEmulator => text("模拟器", "Emulator").get(language),
        AfterExitAction::ExitAalc => text("AALC", "AALC").get(language),
    }
}

fn after_power_label(action: AfterPowerAction, language: Language) -> &'static str {
    match action {
        AfterPowerAction::None => text("无动作", "Do Nothing").get(language),
        AfterPowerAction::Sleep => text("睡眠", "Sleep").get(language),
        AfterPowerAction::Hibernate => text("休眠", "Hibernate").get(language),
        AfterPowerAction::Lock => text("锁屏", "Lock Screen").get(language),
        AfterPowerAction::Shutdown => text("关机", "Shut Down").get(language),
    }
}

fn after_power_key(action: AfterPowerAction) -> &'static str {
    match action {
        AfterPowerAction::None => "none",
        AfterPowerAction::Sleep => "sleep",
        AfterPowerAction::Hibernate => "hibernate",
        AfterPowerAction::Lock => "lock",
        AfterPowerAction::Shutdown => "shutdown",
    }
}

fn parse_after_power_action(value: &str) -> Option<AfterPowerAction> {
    Some(match value {
        "none" => AfterPowerAction::None,
        "sleep" => AfterPowerAction::Sleep,
        "hibernate" => AfterPowerAction::Hibernate,
        "lock" => AfterPowerAction::Lock,
        "shutdown" => AfterPowerAction::Shutdown,
        _ => return None,
    })
}

fn splitter(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> gpui::Stateful<Div> {
    let collapsed = app.home.right_panel_collapsed;
    let width = if collapsed {
        SPLITTER_COLLAPSED_WIDTH
    } else {
        SPLITTER_WIDTH
    };
    let mut handle = div()
        .id("home-panel-splitter")
        .w(px(width))
        .h_full()
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .cursor(gpui::CursorStyle::ResizeColumn)
        .hover(|style| style.bg(rgba((ACCENT << 8) | 0x35)))
        .child(div().w(px(2.0)).h(px(32.0)).rounded_full().bg(rgba(0)))
        .on_drag(SplitterDragGhost, |_, _, _, cx| {
            cx.new(|_| SplitterDragGhost)
        });
    handle = handle.on_drag_move(cx.listener(
        |view, event: &gpui::DragMoveEvent<SplitterDragGhost>, window, cx| {
            let viewport_width = window.viewport_size().width.as_f32();
            let requested_width = viewport_width - event.event.position.x.as_f32();
            if requested_width < SPLITTER_COLLAPSE_THRESHOLD {
                view.set_right_panel_collapsed(true);
            } else {
                let available_max = (viewport_width - MIN_LEFT_PANEL_WIDTH - SPLITTER_WIDTH)
                    .max(RIGHT_PANEL_MIN_WIDTH)
                    .min(RIGHT_PANEL_MAX_WIDTH);
                view.set_right_panel_width(
                    requested_width
                        .clamp(RIGHT_PANEL_MIN_WIDTH, available_max)
                        .round() as u32,
                );
                view.set_right_panel_collapsed(false);
            }
            cx.notify();
        },
    ));
    handle
}

fn right_panel(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    let language = app.state.settings.language;
    let width = bounded_right_panel_width(app.home.right_panel_width);
    let palette = current_render_palette();

    let connection_status = match app.home.device_status {
        ConnectionStatus::Connected => (
            text("已连接", "Connected").get(language),
            BadgeTone::Success,
        ),
        ConnectionStatus::Connecting => (
            text("连接中", "Connecting").get(language),
            BadgeTone::Accent,
        ),
        ConnectionStatus::Disconnected => {
            if app.home.device_error.is_some() {
                (
                    text("连接失败", "Failed").get(language),
                    BadgeTone::Danger,
                )
            } else {
                (
                    text("未连接", "Not connected").get(language),
                    BadgeTone::Neutral,
                )
            }
        }
    };

    let selected_id = app.home.selected_device.clone();
    let selected_device = selected_id
        .as_deref()
        .and_then(|id| app.home.devices.iter().find(|device| device.id == id));
    let is_open = app.home.is_select_open(HomeSelect::Device);
    let is_busy = app.home.is_busy();
    let is_connecting = app.home.device_status == ConnectionStatus::Connecting;
    let is_scanning = app.home.is_scanning_devices;

    let trigger_icon = match selected_device.map(|d| d.kind()) {
        Some(crate::model::DeviceKind::PcWindow) => ICON_MONITOR,
        Some(crate::model::DeviceKind::MumuEmulator) => ICON_SMARTPHONE,
        _ => ICON_RADIO,
    };

    let selected_name = selected_device
        .map(|device| device.name.clone())
        .unwrap_or_else(|| {
            text("选择游戏窗口 / 模拟器", "Select game window / emulator")
                .get(language)
                .to_owned()
        });

    let mut device_trigger = div()
        .id("device-select")
        .flex_1()
        .min_w_0()
        .h(px(30.0))
        .px(px(8.0))
        .gap(px(6.0))
        .rounded_md()
        .border_1()
        .border_color(palette_rgb(if is_open {
            palette.ring
        } else {
            palette.input
        }))
        .bg(palette_rgb(palette.card))
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(6.0))
                .min_w_0()
                .child(action_icon(trigger_icon, 14., TEXT))
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(palette_rgb(if selected_device.is_some() {
                            palette.foreground
                        } else {
                            palette.muted_foreground
                        }))
                        .truncate()
                        .child(selected_name),
                ),
        )
        .child(action_icon(
            if is_open {
                ICON_CHEVRON_UP
            } else {
                ICON_CHEVRON_DOWN
            },
            14.,
            TEXT_MUTED,
        ));

    if is_busy || is_connecting {
        device_trigger = device_trigger.opacity(0.5).cursor_not_allowed();
    } else {
        let hover = palette_rgb(palette.accent_surface);
        device_trigger = device_trigger
            .cursor_pointer()
            .hover(move |s| s.bg(hover))
            .on_click(cx.listener(|view, _, _, cx| {
                view.home.toggle_select(HomeSelect::Device);
                cx.stop_propagation();
                cx.notify();
            }));
    }

    let refresh_icon: gpui::AnyElement = if is_scanning {
        action_icon(ICON_LOADER, 14., ACCENT)
            .with_animation(
                "device-refresh-spin",
                Animation::new(Duration::from_millis(700)).repeat(),
                |svg, progress| {
                    svg.with_transformation(gpui::Transformation::rotate(gpui::percentage(
                        progress,
                    )))
                },
            )
            .into_any_element()
    } else {
        action_icon(ICON_REFRESH, 14., TEXT_MUTED).into_any_element()
    };

    let mut refresh = button("", ButtonVariant::Icon)
        .id("device-refresh")
        .w(px(28.0))
        .h(px(30.0))
        .p_0()
        .gap_0()
        .child(refresh_icon);

    if is_scanning {
        refresh = refresh
            .border_1()
            .border_color(palette_rgb(palette.ring))
            .bg(palette_rgb(palette.accent_surface))
            .cursor_not_allowed();
    } else if !is_connecting {
        refresh = refresh.on_click(cx.listener(|view, _, _, cx| {
            view.refresh_devices(cx);
            cx.stop_propagation();
            cx.notify();
        }));
    }

    let mut disconnect = button("", ButtonVariant::Icon)
        .id("disconnect-device")
        .w(px(28.0))
        .h(px(30.0))
        .p_0()
        .gap_0()
        .child(action_icon(ICON_X, 14., TEXT_MUTED));
    if app.home.device_status == ConnectionStatus::Connected {
        disconnect = disconnect.on_click(cx.listener(|view, _, _, cx| {
            view.disconnect_device(cx);
            cx.stop_propagation();
            cx.notify();
        }));
    }

    let mut device_control_wrapper = div().relative().flex_1().min_w_0().child(device_trigger);

    if is_open {
        let last_device_id = app.state.settings.lastDeviceId.clone();
        let selected_id_popup = app.home.selected_device.clone();
        let mut device_list = div().flex().flex_col().gap_1();

        if app.home.devices.is_empty() {
            let empty_view = div()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .py_3()
                .px_2()
                .gap_2()
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(palette_rgb(palette.muted_foreground))
                        .child(
                            text(
                                "未检测到游戏窗口或模拟器",
                                "No game window or emulator detected",
                            )
                            .get(language),
                        ),
                )
                .child(
                    button(
                        text("重新扫描", "Rescan").get(language),
                        ButtonVariant::Outline,
                    )
                    .id("rescan-devices")
                    .h(px(24.0))
                    .text_size(px(11.0))
                    .px_2()
                    .on_click(cx.listener(|view, _, _, cx| {
                        view.refresh_devices(cx);
                        cx.stop_propagation();
                        cx.notify();
                    })),
                );
            device_list = device_list.child(empty_view);
        } else {
            for device in &app.home.devices {
                let dev_id = device.id.clone();
                let dev_name = device.name.clone();
                let dev_detail = device.detail.clone();
                let dev_kind = device.kind();
                let is_last = last_device_id.as_deref() == Some(&dev_id);
                let is_selected = selected_id_popup.as_deref() == Some(&dev_id);

                let item_icon = match dev_kind {
                    crate::model::DeviceKind::PcWindow => ICON_MONITOR,
                    crate::model::DeviceKind::MumuEmulator => ICON_SMARTPHONE,
                    crate::model::DeviceKind::AdbGeneric => ICON_RADIO,
                };

                let click_id = dev_id.clone();
                let opt_id = format!("device-opt-{}", dev_id);
                let mut item = div()
                    .id(opt_id)
                    .flex()
                    .items_center()
                    .justify_between()
                    .w_full()
                    .min_h(px(32.0))
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .cursor_pointer()
                    .hover({
                        let hover_bg = palette_rgb(palette.accent_surface);
                        move |s| s.bg(hover_bg)
                    })
                    .on_click(cx.listener(move |view, _, _, cx| {
                        view.select_device(click_id.clone(), cx);
                        view.home.close_select();
                        cx.stop_propagation();
                        cx.notify();
                    }));

                if is_selected {
                    item = item.bg(palette_rgb(palette.accent_surface));
                }

                let left_content = div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .min_w_0()
                    .child(action_icon(
                        item_icon,
                        14.,
                        if is_selected { ACCENT } else { TEXT },
                    ))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .min_w_0()
                            .child(
                                div()
                                    .text_size(px(12.0))
                                    .font_weight(if is_selected {
                                        FontWeight::SEMIBOLD
                                    } else {
                                        FontWeight::NORMAL
                                    })
                                    .text_color(palette_rgb(if is_selected {
                                        palette.brand
                                    } else {
                                        palette.foreground
                                    }))
                                    .truncate()
                                    .child(dev_name),
                            )
                            .children(dev_detail.map(|d| {
                                div()
                                    .text_size(px(10.0))
                                    .text_color(palette_rgb(palette.muted_foreground))
                                    .truncate()
                                    .child(d)
                            })),
                    );

                let mut right_content = div().flex().items_center().gap_1();
                if is_last {
                    right_content = right_content.child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(2.0))
                            .px(px(4.0))
                            .py(px(1.0))
                            .rounded_sm()
                            .bg(palette_rgb(palette.warning_light))
                            .text_size(px(9.0))
                            .text_color(palette_rgb(palette.warning))
                            .child(action_icon(ICON_HISTORY, 10., 0xFAAD14))
                            .child(text("上次", "Last").get(language)),
                    );
                }
                if is_selected {
                    right_content = right_content.child(action_icon(ICON_CHECK, 14., ACCENT));
                }

                item = item.child(left_content).child(right_content);
                device_list = device_list.child(item);
            }
        }

        let popup = select_popup(device_list, &palette)
            .shadow_md()
            .on_mouse_down_out(cx.listener(move |view, _, _, cx| {
                view.home.close_select();
                cx.notify();
            }));
        device_control_wrapper = device_control_wrapper.child(deferred(popup).priority(10));
    }

    let connection_header = div()
        .h(px(36.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .border_b_1()
        .border_color(rgba(0))
        .px_3()
        .child(panel_heading(
            ICON_MONITOR,
            text("设备连接", "Device Connection").get(language),
        ))
        .child(badge(connection_status.0, connection_status.1));

    let connection_body = div()
        .flex()
        .items_center()
        .gap_2()
        .p_3()
        .child(device_control_wrapper)
        .child(refresh)
        .children((app.home.device_status == ConnectionStatus::Connected).then_some(disconnect));

    let error_banner = app.home.device_error.as_ref().map(|err| {
        div()
            .flex()
            .items_center()
            .justify_between()
            .mx_3()
            .mb_2()
            .px_2()
            .py_1()
            .rounded_md()
            .border_1()
            .border_color(rgba(0xFF4D4F40))
            .bg(rgba(0xFF4D4F14))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .min_w_0()
                    .child(action_icon(ICON_ALERT_CIRCLE, 13., 0xFF4D4F))
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(rgb(0xFF4D4F))
                            .truncate()
                            .child(err.clone()),
                    ),
            )
            .child(
                button("", ButtonVariant::Icon)
                    .id("dismiss-device-err")
                    .w(px(20.0))
                    .h(px(20.0))
                    .p_0()
                    .gap_0()
                    .child(action_icon(ICON_X, 12., 0xFF4D4F))
                    .on_click(cx.listener(|view, _, _, cx| {
                        view.home.dismiss_device_error();
                        cx.stop_propagation();
                        cx.notify();
                    })),
            )
    });

    let screenshot_detail = app
        .home
        .latest_screenshot
        .as_ref()
        .map(|frame| {
            format!(
                "{} · {}×{}",
                text("收到最新画面", "Latest frame").get(language),
                frame.width,
                frame.height
            )
        })
        .unwrap_or_else(|| {
            text("等待游戏窗口画面接入", "Waiting for game screen connection")
                .get(language)
                .to_owned()
        });
    let screenshot_header = div()
        .h(px(32.0))
        .flex_none()
        .flex()
        .items_center()
        .px(px(10.0))
        .child(panel_heading(
            ICON_MONITOR_PLAY,
            text("实时画面", "Live Screen").get(language),
        ));
    let screenshot_body = div()
        .w_full()
        .aspect_ratio(16.0 / 9.0)
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_1()
        .rounded_md()
        .border_1()
        .border_dashed()
        .border_color(rgb(SURFACE_HOVER))
        .bg(rgb(BACKGROUND))
        .text_size(px(11.0))
        .text_color(rgb(TEXT_MUTED))
        .child(
            div()
                .opacity(0.25)
                .child(action_icon(ICON_MONITOR_PLAY, 32., TEXT_MUTED)),
        )
        .child(screenshot_detail)
        .child(
            div()
                .text_size(px(10.0))
                .text_color(rgb(TEXT_MUTED))
                .child(text("16:9 · 1280×720", "16:9 · 1280×720").get(language)),
        );
    let screenshot_card = panel_card(
        div()
            .flex()
            .flex_col()
            .child(screenshot_header)
            .child(div().p(px(10.0)).child(screenshot_body)),
    )
    .flex_none();

    let visible_logs: Vec<LogEntryPayload> = app
        .home
        .logs
        .iter()
        .rev()
        .take(300)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let log_rows: Vec<_> = visible_logs
        .iter()
        .cloned()
        .map(|entry| {
            let level_color = match entry.level {
                LogLevel::Error => 0xe7000b,
                LogLevel::Warn => 0xd6791d,
                LogLevel::Debug | LogLevel::Info => TEXT_MUTED,
            };
            div()
                .w_full()
                .flex()
                .items_start()
                .gap_2()
                .py(px(2.0))
                .font_family("monospace")
                .text_size(px(11.0))
                .child(
                    div()
                        .w(px(62.0))
                        .flex_none()
                        .text_color(rgb(TEXT_MUTED))
                        .child(format_log_time(entry.ts)),
                )
                .child(log_marker(entry.level, level_color))
                .child(
                    div()
                        .min_w_0()
                        .text_color(rgb(level_color))
                        .child(entry.message),
                )
        })
        .collect();
    let mut clear_logs = button("", ButtonVariant::Ghost)
        .id("clear-logs")
        .h(px(24.0))
        .px(px(8.0))
        .gap(px(4.0))
        .text_size(px(12.0))
        .text_color(rgb(TEXT_MUTED))
        .child(action_icon(ICON_TRASH, 14., TEXT_MUTED))
        .child(text("清空", "Clear").get(language));
    clear_logs = clear_logs.on_click(cx.listener(|view, _, _, cx| {
        view.home.clear_logs();
        cx.stop_propagation();
        cx.notify();
    }));
    let logs_header = div()
        .h(px(32.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .px(px(10.0))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(panel_heading(
                    ICON_SCROLL_TEXT,
                    text("运行日志", "Execution Logs").get(language),
                ))
                .child(badge(
                    visible_logs_count(app).to_string(),
                    BadgeTone::Neutral,
                )),
        )
        .child(div().flex().items_center().gap_1().child(clear_logs));
    let logs_card = panel_card(
        div()
            .flex()
            .flex_col()
            .min_h_0()
            .h_full()
            .child(logs_header)
            .child(
                scroll_area_with_id("home-log-scroll", div().children(log_rows))
                    .track_scroll(&app.home_log_scroll)
                    .flex_1()
                    .min_h_0()
                    .px_3()
                    .py_2(),
            ),
    )
    .flex_1()
    .min_h_0();

    div()
        .w(px(width))
        .min_w(px(240.0))
        .h_full()
        .min_h_0()
        .flex_shrink_1()
        .flex()
        .flex_col()
        .gap_2()
        .overflow_x_hidden()
        .p(px(10.0))
        .child(
            panel_card(
                div()
                    .flex()
                    .flex_col()
                    .child(connection_header)
                    .child(connection_body)
                    .children(error_banner),
            )
            .flex_none(),
        )
        .child(screenshot_card)
        .child(logs_card)
}

fn panel_card(child: impl IntoElement) -> Div {
    div()
        .min_w_0()
        .overflow_hidden()
        .rounded_lg()
        .border_1()
        .border_color(rgba(0))
        .bg(rgb(SURFACE))
        .child(child)
}

fn panel_heading(icon_data: &'static [u8], title: &'static str) -> Div {
    div()
        .flex()
        .items_center()
        .gap_2()
        .text_size(px(12.0))
        .text_color(rgb(TEXT_MUTED))
        .child(action_icon(icon_data, 14., TEXT_MUTED))
        .child(title)
}

fn visible_logs_count(app: &AhabApp) -> usize {
    app.home.logs.len().min(300)
}

fn log_marker(level: LogLevel, color: u32) -> gpui::AnyElement {
    match level {
        LogLevel::Error => action_icon(ICON_ALERT_CIRCLE, 12., color).into_any_element(),
        LogLevel::Warn => action_icon(ICON_ALERT_TRIANGLE, 12., color).into_any_element(),
        LogLevel::Debug | LogLevel::Info => div()
            .mt(px(5.))
            .w(px(5.))
            .h(px(5.))
            .flex_none()
            .rounded_full()
            .bg(rgb(color))
            .into_any_element(),
    }
}

fn format_log_time(timestamp: i64) -> String {
    // The wire contract uses JavaScript-compatible milliseconds. Accepting
    // second-resolution values as well keeps Mock and future sidecars easy to
    // inspect during development.
    let millis = if timestamp.unsigned_abs() < 100_000_000_000 {
        timestamp.saturating_mul(1000)
    } else {
        timestamp
    };
    let seconds = millis.div_euclid(1000).rem_euclid(24 * 60 * 60);
    let hours = seconds / 3600;
    let minutes = (seconds / 60) % 60;
    let seconds = seconds % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

fn first_executable_task(home: &HomeState) -> Option<FixedTaskId> {
    let enabled = &home.tasks.enabledTasks;
    if enabled.daily_task {
        Some(FixedTaskId::DailyTask)
    } else if enabled.get_reward {
        Some(FixedTaskId::GetReward)
    } else if enabled.buy_enkephalin {
        Some(FixedTaskId::BuyEnkephalin)
    } else if enabled.mirror {
        Some(FixedTaskId::Mirror)
    } else {
        None
    }
}

fn bounded_right_panel_width(width: f32) -> f32 {
    if !width.is_finite() {
        RIGHT_PANEL_DEFAULT_WIDTH
    } else {
        width.clamp(RIGHT_PANEL_MIN_WIDTH, RIGHT_PANEL_MAX_WIDTH)
    }
}

fn reward_mode_label(mode: u8, language: Language) -> &'static str {
    match mode {
        0 => text("全部", "All").get(language),
        1 => text("狂气/通行证", "Lunacy/Pass").get(language),
        2 => text("邮件", "Mail").get(language),
        _ => text("全部", "All").get(language),
    }
}

#[cfg(test)]
mod tests {
    use crate::model::Language;

    use super::{
        ICON_ALERT_CIRCLE, ICON_ALERT_TRIANGLE, ICON_CALENDAR_CHECK, ICON_CHECK, ICON_CHECK_SQUARE,
        ICON_CHEVRON_DOWN, ICON_CHEVRON_UP, ICON_COMPASS, ICON_GIFT, ICON_HISTORY, ICON_LOADER,
        ICON_MONITOR, ICON_MONITOR_PLAY, ICON_PAUSE, ICON_PLAY, ICON_RADIO, ICON_REFRESH,
        ICON_ROTATE, ICON_SCROLL_TEXT, ICON_SETTINGS, ICON_SLIDERS, ICON_SMARTPHONE, ICON_SQUARE,
        ICON_TRASH, ICON_X, ICON_ZAP, RIGHT_PANEL_DEFAULT_WIDTH, RIGHT_PANEL_MAX_WIDTH,
        RIGHT_PANEL_MIN_WIDTH, SPLITTER_COLLAPSED_WIDTH, SPLITTER_WIDTH, after_completion_summary,
        bounded_right_panel_width, reward_mode_label,
    };

    fn available_home_left_width(
        window_width: u32,
        right_panel_width: u32,
        splitter_width: u32,
    ) -> u32 {
        window_width.saturating_sub(right_panel_width + splitter_width)
    }

    #[test]
    fn minimum_window_keeps_home_columns_reachable() {
        assert_eq!(
            available_home_left_width(800, 280, SPLITTER_WIDTH as u32),
            516
        );
        assert!(available_home_left_width(800, 280, SPLITTER_WIDTH as u32) > 0);
        assert_eq!(
            available_home_left_width(800, 0, SPLITTER_COLLAPSED_WIDTH as u32),
            784
        );
    }

    #[test]
    fn right_panel_width_is_bounded_to_the_visual_contract() {
        assert_eq!(
            bounded_right_panel_width(f32::NAN),
            RIGHT_PANEL_DEFAULT_WIDTH
        );
        assert_eq!(bounded_right_panel_width(100.0), RIGHT_PANEL_MIN_WIDTH);
        assert_eq!(bounded_right_panel_width(280.0), RIGHT_PANEL_MIN_WIDTH);
        assert_eq!(bounded_right_panel_width(900.0), RIGHT_PANEL_MAX_WIDTH);
    }

    #[test]
    fn reward_modes_use_ui_names() {
        assert_eq!(reward_mode_label(0, Language::ZhCn), "全部");
        assert_eq!(reward_mode_label(1, Language::ZhCn), "狂气/通行证");
        assert_eq!(reward_mode_label(2, Language::ZhCn), "邮件");
    }

    #[test]
    fn after_completion_summary_matches_web_toolbar_copy() {
        let config = crate::model::AfterCompletionConfig::default();
        assert_eq!(
            after_completion_summary(&config, Language::ZhCn),
            "退出游戏 (默认)"
        );
        assert_eq!(
            after_completion_summary(&config, Language::EnUs),
            "Exit Game (Default)"
        );
    }

    #[test]
    fn inline_svg_payloads_are_valid_unescaped_markup() {
        let payloads = [
            ICON_SLIDERS,
            ICON_CALENDAR_CHECK,
            ICON_GIFT,
            ICON_ZAP,
            ICON_COMPASS,
            ICON_RADIO,
            ICON_CHECK_SQUARE,
            ICON_CHEVRON_DOWN,
            ICON_CHEVRON_UP,
            ICON_MONITOR,
            ICON_SMARTPHONE,
            ICON_CHECK,
            ICON_HISTORY,
            ICON_LOADER,
            ICON_MONITOR_PLAY,
            ICON_SCROLL_TEXT,
            ICON_ALERT_CIRCLE,
            ICON_ALERT_TRIANGLE,
            ICON_REFRESH,
            ICON_ROTATE,
            ICON_PAUSE,
            ICON_PLAY,
            ICON_SQUARE,
            ICON_SETTINGS,
            ICON_TRASH,
            ICON_X,
        ];
        for payload in payloads {
            let source = std::str::from_utf8(payload).unwrap();
            assert!(source.starts_with("<svg "));
            assert!(!source.contains(r#"\""#));
        }
    }
}

#[allow(dead_code)]
fn task_title(task: FixedTaskId, language: Language) -> &'static str {
    match task {
        FixedTaskId::SetWindows => text("窗口设置", "Window Settings").get(language),
        FixedTaskId::DailyTask => text("日常任务", "Daily Tasks").get(language),
        FixedTaskId::GetReward => text("领取奖励", "Claim Rewards").get(language),
        FixedTaskId::BuyEnkephalin => text("狂气换体", "Refill Enkephalin").get(language),
        FixedTaskId::Mirror => text("坐牢设置 (镜牢)", "Mirror Dungeon").get(language),
        FixedTaskId::ResonateWithAhab => text("亚哈共鸣", "Ahab Resonance").get(language),
    }
}

fn task_id(task: FixedTaskId) -> &'static str {
    match task {
        FixedTaskId::SetWindows => "set-windows",
        FixedTaskId::DailyTask => "daily-task",
        FixedTaskId::GetReward => "get-reward",
        FixedTaskId::BuyEnkephalin => "buy-enkephalin",
        FixedTaskId::Mirror => "mirror",
        FixedTaskId::ResonateWithAhab => "resonate-ahab",
    }
}
