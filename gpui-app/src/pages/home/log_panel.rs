use super::*;

pub(super) fn logs_card(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    let scroll_handle = app.home_log_scroll.clone();
    let language = app.state.settings.language;
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
                .child(super::panel::panel_heading(
                    ICON_SCROLL_TEXT,
                    text("运行日志", "Execution Logs").get(language),
                ))
                .child(badge(
                    visible_logs_count(app).to_string(),
                    BadgeTone::Neutral,
                )),
        )
        .child(div().flex().items_center().gap_1().child(clear_logs));
    super::panel::panel_card(
        div()
            .flex()
            .flex_col()
            .min_h_0()
            .h_full()
            .child(logs_header)
            .child(
                scroll_area_with_handle(
                    app,
                    "home-log-scroll",
                    div().children(log_rows),
                    scroll_handle,
                )
                .flex_1()
                .min_h_0()
                .px_3()
                .py_2(),
            ),
    )
    .flex_1()
    .min_h_0()
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

fn visible_logs_count(app: &AhabApp) -> usize {
    app.home.logs.len().min(300)
}
