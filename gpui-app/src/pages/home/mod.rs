//! Home page surface: task configuration, execution controls, and the
//! connection/log side panel. All mutable page state lives in HomeState.

mod cards;
mod completion;
mod completion_editor;
mod controls;
mod device_panel;
mod execution;
mod execution_toolbar;
mod log_panel;
mod panel;
mod shared;
mod stats;
mod task_details;
mod tasks;

use cards::*;
use controls::*;
use shared::*;

use std::{rc::Rc, time::Duration};

use gpui::{
    Animation, AnimationExt, Context, Div, FontWeight, KeyDownEvent, Render, Svg, Window, deferred,
    div, prelude::*, px, relative, svg,
};

use crate::{
    app::{ACCENT, AhabApp, BACKGROUND, BORDER, SURFACE, SURFACE_HOVER, TEXT, TEXT_MUTED},
    components::{
        BadgeTone, ButtonVariant, badge, button, card, current_render_palette, is_activation_key,
        palette_rgb, render_rgb as rgb, render_rgba as rgba, scroll_area_with_id, select_option,
        select_popup, select_trigger, switch, switch_accent,
    },
    i18n::{self, Key as I18nKey, paired as text},
    model::{
        AfterExitAction, AfterPowerAction, ConnectionStatus, ExecutionState, FixedTaskId, Language,
        LogEntryPayload, LogLevel,
    },
    state::{DailyCounter, HomeSelect, HomeState, MirrorOption, TaskOptionsTab},
};

pub fn render(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    let busy = app.home.is_busy();
    let execution_state = app.home.execution.state;
    // The mock start call intentionally leaves currentTaskId optional. Use
    // the first executable selection for the presentation only so the
    // running sweep still exercises the same visual state as the React page.
    let current_task = app.home.execution.currentTaskId.or_else(|| {
        (execution_state == ExecutionState::Running)
            .then(|| panel::first_executable_task(&app.home))
            .flatten()
    });

    let task_cards = vec![
        tasks::set_windows_card(app, cx, busy, current_task == Some(FixedTaskId::SetWindows)),
        tasks::daily_card(app, cx, busy, current_task == Some(FixedTaskId::DailyTask)),
        tasks::reward_card(app, cx, busy, current_task == Some(FixedTaskId::GetReward)),
        tasks::enkephalin_card(
            app,
            cx,
            busy,
            current_task == Some(FixedTaskId::BuyEnkephalin),
        ),
        tasks::mirror_card(app, cx, busy, current_task == Some(FixedTaskId::Mirror)),
        tasks::ahab_card(
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
        .pl(px(10.0))
        .pr(px(4.0))
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
        .child(stats::overview(app, cx))
        .child(task_list)
        .child(execution::execution_toolbar(app, cx, busy, execution_state));

    let splitter = panel::splitter(app, cx);
    let right = if app.home.right_panel_collapsed {
        div()
    } else {
        panel::right_panel(app, cx)
    };

    div()
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
        .child(right)
}

pub fn render_overlay(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .child(stats::daily_details_overlay(app, cx))
        .child(execution::after_completion_editor(
            app,
            cx,
            app.home.is_busy(),
        ))
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
        RIGHT_PANEL_MIN_WIDTH, SPLITTER_COLLAPSED_WIDTH, SPLITTER_WIDTH,
        execution::after_completion_summary,
        panel::{bounded_right_panel_width, reward_mode_label},
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
            "什么也不干 (本次)"
        );
        assert_eq!(
            after_completion_summary(&config, Language::EnUs),
            "Do nothing (This run)"
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
            assert!(!source.contains(r#"\"#));
        }
    }
}
