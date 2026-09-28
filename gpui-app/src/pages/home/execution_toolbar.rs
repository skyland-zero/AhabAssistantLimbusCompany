use super::*;
use crate::components::IconName;

fn can_pause_or_resume(state: ExecutionState) -> bool {
    matches!(state, ExecutionState::Running | ExecutionState::Paused)
}

fn is_stop_pending(state: ExecutionState) -> bool {
    matches!(state, ExecutionState::Stopping | ExecutionState::Restoring)
}

pub(super) fn execution_toolbar(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    busy: bool,
    state: ExecutionState,
) -> Div {
    let language = app.state.settings.language;
    let palette = current_render_palette();

    let mut select_all = button("select-all", "", ButtonVariant::Outline)
        .h(px(32.0))
        .px(px(10.0))
        .gap(px(4.0))
        .text_size(px(12.))
        .child(action_icon(IconName::SquareCheck, 14., TEXT))
        .child(text("全选", "Select All").get(language));
    let mut clear_all = button("clear-all", "", ButtonVariant::Outline)
        .h(px(32.0))
        .px(px(10.0))
        .gap(px(4.0))
        .text_size(px(12.))
        .child(action_icon(IconName::RotateCw, 14., TEXT_MUTED))
        .child(
            div()
                .text_color(rgb(TEXT_MUTED))
                .child(text("清空", "Clear All").get(language)),
        );
    if !busy {
        select_all = select_all
            .on_click(cx.listener(|view, _, _, cx| {
                view.home.set_all_tasks(true);
                cx.stop_propagation();
                cx.notify();
            }))
            .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    view.home.set_all_tasks(true);
                    cx.stop_propagation();
                    cx.notify();
                }
            }));
        clear_all = clear_all
            .on_click(cx.listener(|view, _, _, cx| {
                view.home.set_all_tasks(false);
                cx.stop_propagation();
                cx.notify();
            }))
            .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    view.home.set_all_tasks(false);
                    cx.stop_propagation();
                    cx.notify();
                }
            }));
    }

    let mut after_button = button("after-completion-open", "", ButtonVariant::Ghost)
        .h(px(32.0))
        .flex_none()
        .max_w(px(280.0))
        .min_w_0()
        .px(px(10.0))
        .gap(px(6.0))
        .text_size(px(12.))
        .child(action_icon(IconName::SlidersHorizontal, 14., ACCENT))
        .child(
            div()
                .min_w_0()
                .truncate()
                .child(super::execution::after_completion_summary(
                    &app.home.tasks.afterCompletion,
                    app.state.settings.language,
                )),
        );
    if !busy {
        after_button = after_button
            .on_click(cx.listener(|view, _, window, cx| {
                view.open_after_completion(window, cx);
                cx.stop_propagation();
            }))
            .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    view.open_after_completion(window, cx);
                    cx.stop_propagation();
                }
            }));
    }

    let (pause_icon, pause_label, pause_icon_color) = if state == ExecutionState::Paused {
        (
            IconName::Play,
            text("继续", "Resume").get(language),
            palette.success.rgb_hex(),
        )
    } else if is_stop_pending(state) {
        (
            IconName::Loader,
            if state == ExecutionState::Restoring {
                text("恢复中", "Restoring").get(language)
            } else {
                text("停止中", "Stopping").get(language)
            },
            palette.warning.rgb_hex(),
        )
    } else {
        (
            IconName::Pause,
            text("暂停", "Pause").get(language),
            palette.warning.rgb_hex(),
        )
    };
    let mut pause = button("pause-resume", "", ButtonVariant::Outline)
        .h(px(34.0))
        .px(px(12.0))
        .gap(px(6.0))
        .text_size(px(12.))
        .child(action_icon(pause_icon, 14., pause_icon_color))
        .child(pause_label);
    if busy && can_pause_or_resume(state) {
        pause = pause
            .on_click(cx.listener(|view, _, _, cx| {
                view.home.pause_or_resume();
                cx.stop_propagation();
                cx.notify();
            }))
            .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    view.home.pause_or_resume();
                    cx.stop_propagation();
                    cx.notify();
                }
            }));
    } else if is_stop_pending(state) || state == ExecutionState::Starting {
        pause = pause.opacity(0.65).cursor_not_allowed();
    }

    let (run_icon, run_label, run_variant) = if is_stop_pending(state) {
        (
            IconName::Loader,
            if state == ExecutionState::Restoring {
                text("正在恢复设备", "Restoring device").get(language)
            } else {
                text("正在停止", "Stopping").get(language)
            },
            ButtonVariant::Destructive,
        )
    } else if busy {
        (IconName::Square, "Stop!", ButtonVariant::Destructive)
    } else {
        (IconName::Play, "Link Start!", ButtonVariant::Default)
    };
    let run_icon_element: gpui::AnyElement = if is_stop_pending(state) {
        brand_action_icon(run_icon, 14.)
            .with_animation(
                "execution-stop-spin",
                Animation::new(Duration::from_millis(700))
                    .repeat()
                    .with_max_fps(12.0),
                |svg, progress| {
                    svg.transform(gpui::Transformation::rotate(gpui::percentage(progress)))
                },
            )
            .into_any_element()
    } else {
        brand_action_icon(run_icon, 14.).into_any_element()
    };
    let mut run = button("start-stop", "", run_variant)
        .h(px(34.0))
        .px(px(16.0))
        .gap(px(6.0))
        .text_size(px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .child(run_icon_element)
        .child(run_label);
    if !busy {
        // The keycap sits on the brand button, so derive its overlay from
        // the button foreground token; a literal 0xffffff is aliased to the
        // card color by the legacy palette map and disappears in dark mode.
        let kbd_bg = rgba((palette.brand_foreground.rgb_hex() << 8) | 0x33);
        run = run.child(
            div()
                .rounded_sm()
                .bg(kbd_bg)
                .px(px(5.0))
                .py(px(1.5))
                .font_family("monospace")
                .text_size(px(10.))
                .font_weight(FontWeight::NORMAL)
                .text_color(palette_rgb(palette.brand_foreground))
                .child("F10"),
        );
    }
    if busy && !is_stop_pending(state) {
        run = run
            .on_click(cx.listener(|view, _, _, cx| {
                view.stop_execution(cx);
                cx.stop_propagation();
                cx.notify();
            }))
            .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    view.stop_execution(cx);
                    cx.stop_propagation();
                    cx.notify();
                }
            }));
    } else if is_stop_pending(state) {
        run = run.opacity(0.65).cursor_not_allowed();
    } else {
        run = run
            .on_click(cx.listener(|view, _, _, cx| {
                view.start_execution(cx);
                cx.stop_propagation();
                cx.notify();
            }))
            .on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
                if is_activation_key(event) {
                    window.prevent_default();
                    view.start_execution(cx);
                    cx.stop_propagation();
                    cx.notify();
                }
            }));
    }

    let mut command_group = div().min_w_0().flex().items_center().gap_2();
    if busy && can_pause_or_resume(state) {
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
        .ml(px(14.0))
        .mr(px(4.0))
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
