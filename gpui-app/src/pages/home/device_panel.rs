use super::*;

pub(super) fn connection_card(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    let language = app.state.settings.language;
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
                (text("连接失败", "Failed").get(language), BadgeTone::Danger)
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
                Animation::new(Duration::from_millis(700))
                    .repeat()
                    .with_max_fps(12.0),
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
        .child(super::panel::panel_heading(
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

    super::panel::panel_card(
        div()
            .flex()
            .flex_col()
            .child(connection_header)
            .child(connection_body)
            .children(error_banner),
    )
}
