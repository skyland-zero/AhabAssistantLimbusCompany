use std::collections::VecDeque;

use gpui::{
    Context, MouseButton, MouseDownEvent, Render, ScrollHandle, ScrollWheelEvent, WeakEntity,
    Window, point,
};

use crate::{app::AhabApp, model::Language};

use super::*;

pub(super) struct LogPanelView {
    root: WeakEntity<AhabApp>,
    logs: VecDeque<LogEntryPayload>,
    language: Language,
    revision: u64,
    scroll_handle: ScrollHandle,
}

impl LogPanelView {
    pub(super) fn new(root: WeakEntity<AhabApp>) -> Self {
        Self {
            root,
            logs: VecDeque::new(),
            language: Language::ZhCn,
            revision: u64::MAX,
            scroll_handle: ScrollHandle::new(),
        }
    }

    pub(super) fn sync_snapshot(
        &mut self,
        logs: VecDeque<LogEntryPayload>,
        revision: u64,
        language: Language,
    ) {
        self.language = language;
        if self.revision == revision {
            return;
        }
        self.logs = logs;
        self.revision = revision;
        self.scroll_handle.scroll_to_bottom();
    }
}

pub(super) fn logs_card(app: &AhabApp) -> Div {
    let logs = app.home_views.as_ref().map(|views| views.logs_view());
    div()
        .flex_1()
        .min_h_0()
        .h_full()
        .when_some(logs, |this, logs| this.child(logs))
}

impl Render for LogPanelView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let language = self.language;
        let log_rows: Vec<_> = self
            .logs
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

        let root = self.root.clone();
        let mut clear_logs = button("", ButtonVariant::Ghost)
            .id("clear-logs")
            .h(px(24.0))
            .px(px(8.0))
            .gap(px(4.0))
            .text_size(px(12.0))
            .text_color(rgb(TEXT_MUTED))
            .child(action_icon(ICON_TRASH, 14., TEXT_MUTED))
            .child(text("清空", "Clear").get(language));
        clear_logs = clear_logs.on_click(move |_, _, cx| {
            if let Some(root) = root.upgrade() {
                root.update(cx, |view, cx| {
                    view.home.clear_logs();
                    cx.stop_propagation();
                    cx.notify();
                });
            }
        });

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
                    .child(badge(self.logs.len().to_string(), BadgeTone::Neutral)),
            )
            .child(div().flex().items_center().gap_1().child(clear_logs));

        let log_content = div()
            .relative()
            .flex_1()
            .min_h_0()
            .w_full()
            .child(
                scroll_area(div().children(log_rows))
                    .track_scroll(&self.scroll_handle)
                    .size_full()
                    .min_h_0()
                    .px_3()
                    .py_2()
                    .on_scroll_wheel(cx.listener(|_view, _: &ScrollWheelEvent, _window, cx| {
                        cx.notify();
                    })),
            )
            .child(render_log_scrollbar(&self.scroll_handle, cx));

        super::panel::panel_card(
            div()
                .flex()
                .flex_col()
                .min_h_0()
                .h_full()
                .child(logs_header)
                .child(log_content),
        )
        .w_full()
        .h_full()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LogScrollDragGhost;

impl Render for LogScrollDragGhost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().w(px(1.0)).h(px(1.0)).bg(rgba(0))
    }
}

fn render_log_scrollbar(
    scroll_handle: &ScrollHandle,
    cx: &mut Context<LogPanelView>,
) -> impl IntoElement {
    let offset_y = scroll_handle.offset().y.as_f32().abs();
    let max_offset_y = scroll_handle.max_offset().y.as_f32().abs();
    let viewport_h = scroll_handle.bounds().size.height.as_f32();

    if max_offset_y <= 2.0 || viewport_h <= 10.0 {
        return div().into_any_element();
    }

    let total_content_h = viewport_h + max_offset_y;
    let thumb_ratio = (viewport_h / total_content_h).clamp(0.08, 0.92);
    let scroll_progress = (offset_y / max_offset_y).clamp(0.0, 1.0);
    let thumb_top_ratio = scroll_progress * (1.0 - thumb_ratio);

    let mut thumb = div()
        .id("log-scrollbar-thumb")
        .absolute()
        .top(relative(thumb_top_ratio))
        .h(relative(thumb_ratio))
        .w_full()
        .rounded_full()
        .bg(rgba((TEXT_MUTED << 8) | 0x60))
        .hover(|style| style.bg(rgba((ACCENT << 8) | 0xaa)))
        .cursor_pointer();

    thumb = thumb
        .on_drag(LogScrollDragGhost, |_, _, _, cx| {
            cx.new(|_| LogScrollDragGhost)
        })
        .on_drag_move(cx.listener(
            |view, event: &gpui::DragMoveEvent<LogScrollDragGhost>, _, cx| {
                let track_height = event.bounds.size.height.as_f32().max(1.0);
                let position_y = (event.event.position.y - event.bounds.top()).as_f32();
                let scroll_ratio = (position_y / track_height).clamp(0.0, 1.0);
                let max_offset_y = view.scroll_handle.max_offset().y.as_f32().abs();
                view.scroll_handle
                    .set_offset(point(px(0.0), px(-scroll_ratio * max_offset_y)));
                cx.notify();
            },
        ));

    let mut track = div()
        .id("log-scrollbar-track")
        .absolute()
        .top(px(2.0))
        .bottom(px(2.0))
        .right(px(2.0))
        .w(px(5.0))
        .rounded_full()
        .bg(rgba((SURFACE_HOVER << 8) | 0x30))
        .hover(|style| style.bg(rgba((SURFACE_HOVER << 8) | 0x60)))
        .child(thumb);

    track = track.on_mouse_down(
        MouseButton::Left,
        cx.listener(|view, event: &MouseDownEvent, _, cx| {
            let max_offset_y = view.scroll_handle.max_offset().y.as_f32().abs();
            if max_offset_y > 0.0 {
                let track_height = view.scroll_handle.bounds().size.height.as_f32().max(1.0);
                let click_y = (event.position.y - view.scroll_handle.bounds().top()).as_f32();
                let scroll_ratio = (click_y / track_height).clamp(0.0, 1.0);
                view.scroll_handle
                    .set_offset(point(px(0.0), px(-scroll_ratio * max_offset_y)));
                cx.notify();
            }
        }),
    );

    track.into_any_element()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_log_time_handles_millis_and_seconds() {
        assert_eq!(format_log_time(0), "00:00:00");
        assert_eq!(format_log_time(3661), "01:01:01");
        assert_eq!(format_log_time(172_803_661_000), "01:01:01");
    }
}
