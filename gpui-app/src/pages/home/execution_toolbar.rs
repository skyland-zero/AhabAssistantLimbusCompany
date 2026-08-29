use super::*;

pub(super) fn execution_toolbar(
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
    } else if state == ExecutionState::Stopping {
        (
            ICON_LOADER,
            text("停止中", "Stopping").get(language),
            palette.warning.rgb_hex(),
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
    if busy && state != ExecutionState::Stopping {
        pause = pause.on_click(cx.listener(|view, _, _, cx| {
            view.home.pause_or_resume();
            cx.stop_propagation();
            cx.notify();
        }));
    } else if state == ExecutionState::Stopping {
        pause = pause.opacity(0.65).cursor_not_allowed();
    }

    let (run_icon, run_label, run_variant) = if state == ExecutionState::Stopping {
        (ICON_LOADER, "Stopping...", ButtonVariant::Destructive)
    } else if busy {
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
    if busy && state != ExecutionState::Stopping {
        run = run.on_click(cx.listener(|view, _, _, cx| {
            view.home.stop();
            cx.stop_propagation();
            cx.notify();
        }));
    } else if state == ExecutionState::Stopping {
        run = run.opacity(0.65).cursor_not_allowed();
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
