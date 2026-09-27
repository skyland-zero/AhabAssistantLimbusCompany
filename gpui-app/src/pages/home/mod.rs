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
mod views;

pub(crate) use views::HomeViewRefs;

use cards::*;
use controls::*;
use shared::*;

use std::{rc::Rc, time::Duration};

use gpui::{
    Animation, AnimationExt, Context, Div, FontWeight, KeyDownEvent, Render, Window, deferred, div,
    prelude::*, px, relative,
};

use crate::{
    app::{
        ACCENT, AhabApp, BACKGROUND, BORDER, BackendPhase, SURFACE, SURFACE_HOVER, TEXT, TEXT_MUTED,
    },
    components::{
        BadgeTone, ButtonVariant, ShapeExt, Switch, Tab, apply_card_shadow, badge, button, card,
        current_render_palette, is_activation_key, palette_rgb, render_rgb as rgb,
        render_rgba as rgba, scroll_area, scroll_area_with_id, segmented_tab_bar,
        select_keyboard_index, select_option, select_options_state, select_popup, select_trigger,
        settings_grid, shape_rounded, switch, switch_accent,
    },
    i18n::{self, Key as I18nKey, paired as text},
    model::{
        AfterExitAction, AfterPowerAction, ConnectionStatus, ExecutionState, FixedTaskId, Language,
        LogEntryPayload, LogLevel,
    },
    state::{DailyCounter, HomeSelect, HomeState, MirrorOption, TaskOptionsTab},
};

pub fn render(app: &mut AhabApp, _window: &mut Window, cx: &mut Context<AhabApp>) -> Div {
    app.ensure_home_views(cx);
    let busy = app.home.is_busy();
    let execution_state = app.home.execution.state;
    // Keep a compatibility fallback for older sidecars that may omit
    // currentTaskId while running. New sidecars and the mock report it
    // explicitly and update it before each top-level task starts.
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

    let task_list = scroll_area_with_id(
        "home-task-scroll",
        div().flex().flex_col().gap_2().pb_2().children(task_cards),
    )
    .flex_1()
    .min_h_0()
    .pl(px(10.0))
    .pr(px(4.0))
    .py(px(10.0));

    let left_panel = div()
        .flex_1()
        .min_w_0()
        .min_h_0()
        .flex()
        .flex_col()
        .border_r_1()
        .border_color(rgba(0))
        .child(
            app.home_views
                .as_ref()
                .expect("Home child views are initialized before rendering")
                .stats_view(),
        )
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
        // No background here: the root window Div owns `palette.background`
        // and the skin's artwork layer sits between the two. Painting it again
        // would hide the artwork.
        .child(left_panel)
        .child(splitter)
        .child(right)
}

pub fn render_overlay(app: &mut AhabApp, _window: &mut Window, cx: &mut Context<AhabApp>) -> Div {
    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .child(stats::daily_details_overlay(app, cx))
        .child(stats::mirror_details_overlay(app, cx))
        .child(execution::after_completion_editor(
            app,
            cx,
            app.home.is_busy(),
        ))
}

#[cfg(test)]
mod tests {
    use crate::components::IconName;
    use crate::model::Language;

    use super::{
        RIGHT_PANEL_DEFAULT_WIDTH, RIGHT_PANEL_MAX_WIDTH, RIGHT_PANEL_MIN_WIDTH,
        SPLITTER_COLLAPSED_WIDTH, SPLITTER_WIDTH,
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

    /// The task-card icons used to be embedded here as SVG bytes, so this test
    /// checked the markup. They now name entries in the `gpui-kit-assets`
    /// catalog, so the invariant worth guarding is that every name still
    /// resolves to a file the bundle ships - a typo would otherwise render an
    /// empty square at runtime instead of failing the build.
    #[test]
    fn task_icons_resolve_to_bundled_assets() {
        let icons = [
            IconName::SlidersHorizontal,
            IconName::CalendarCheck,
            IconName::Gift,
            IconName::Zap,
            IconName::Compass,
            IconName::Radio,
            IconName::SquareCheck,
            IconName::ChevronDown,
            IconName::ChevronUp,
            IconName::Monitor,
            IconName::Smartphone,
            IconName::Check,
            IconName::RotateCcwClock,
            IconName::Loader,
            IconName::MonitorPlay,
            IconName::ScrollText,
            IconName::CircleAlert,
            IconName::TriangleAlert,
            IconName::RefreshCw,
            IconName::RotateCw,
            IconName::Pause,
            IconName::Play,
            IconName::Square,
            IconName::Settings,
            IconName::Trash,
            IconName::X,
        ];
        for icon in icons {
            let path = icon.path();
            assert!(
                path.starts_with("icons/") && path.ends_with(".svg"),
                "{path} must point into the bundled icon directory"
            );
        }
        // The label lookup must not silently fall back to a different glyph.
        assert_eq!(super::shared::task_icon_name("GFT"), IconName::Gift);
        assert_eq!(
            super::shared::task_icon_name("unknown"),
            IconName::SlidersHorizontal
        );
    }
}
