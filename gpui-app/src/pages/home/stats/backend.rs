use super::*;

pub(crate) fn backend_status_card(snapshot: &StatsSnapshot, root: &WeakEntity<AhabApp>) -> Div {
    let language = snapshot.language;
    let status = &snapshot.backend_status;
    let (label, detail, tone) = match status.phase {
        BackendPhase::WaitingForFirstFrame => (
            text("等待首帧", "Waiting for first frame")
                .get(language)
                .to_owned(),
            text("GPUI 就绪后启动", "Starts after GPUI is ready")
                .get(language)
                .to_owned(),
            BadgeTone::Accent,
        ),
        BackendPhase::Starting => (
            status
                .retry_no
                .map(|retry| match language {
                    crate::model::Language::ZhCn => format!("重试中 {retry}/3"),
                    crate::model::Language::EnUs => format!("Retrying {retry}/3"),
                })
                .unwrap_or_else(|| text("启动中", "Starting").get(language).to_owned()),
            text("正在连接 Python 后端", "Connecting to Python backend")
                .get(language)
                .to_owned(),
            BadgeTone::Accent,
        ),
        BackendPhase::RetryWaiting => (
            status
                .retry_no
                .map(|retry| match language {
                    crate::model::Language::ZhCn => format!("自动重试 {retry}/3"),
                    crate::model::Language::EnUs => format!("Auto-retry {retry}/3"),
                })
                .unwrap_or_else(|| text("等待重试", "Retry pending").get(language).to_owned()),
            text("等待下一次启动尝试", "Waiting for the next attempt")
                .get(language)
                .to_owned(),
            BadgeTone::Warning,
        ),
        BackendPhase::Ready => (
            text("已就绪", "Ready").get(language).to_owned(),
            text("连接和协议已确认", "Connection and protocol confirmed")
                .get(language)
                .to_owned(),
            BadgeTone::Success,
        ),
        BackendPhase::Failed => (
            text("启动失败", "Start failed").get(language).to_owned(),
            text("自动重试已耗尽", "Automatic retries exhausted")
                .get(language)
                .to_owned(),
            BadgeTone::Danger,
        ),
        BackendPhase::Disconnected => (
            text("已断开", "Disconnected").get(language).to_owned(),
            text("自动恢复失败", "Automatic recovery failed")
                .get(language)
                .to_owned(),
            BadgeTone::Warning,
        ),
        BackendPhase::Restarting => (
            status
                .retry_no
                .map(|retry| match language {
                    crate::model::Language::ZhCn => format!("恢复中 {retry}/3"),
                    crate::model::Language::EnUs => format!("Recovering {retry}/3"),
                })
                .unwrap_or_else(|| text("恢复中", "Recovering").get(language).to_owned()),
            text("正在恢复 Python 后端", "Recovering the Python backend")
                .get(language)
                .to_owned(),
            BadgeTone::Accent,
        ),
        BackendPhase::Mock => (
            text("Mock 模式", "Mock mode").get(language).to_owned(),
            text("未启动 Python 后端", "Python backend is not started")
                .get(language)
                .to_owned(),
            BadgeTone::Neutral,
        ),
    };

    let header = div()
        .h(px(28.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(action_icon(ICON_RADIO, 14., ACCENT))
                .child(
                    div()
                        .text_size(px(12.))
                        .text_color(rgb(TEXT_MUTED))
                        .child(text("Python 后端", "Python Backend").get(language)),
                ),
        )
        .child(badge(label, tone));

    let mut content = div()
        .h_full()
        .flex()
        .flex_col()
        .gap_2()
        .child(header)
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_size(px(11.))
                .text_color(rgb(TEXT_MUTED))
                .child(detail),
        );

    if status.can_manual_retry() {
        let mut retry = button(
            text("重试启动", "Retry start").get(language),
            ButtonVariant::Ghost,
        )
        .id("backend-retry")
        .h(px(26.0))
        .px_2()
        .py_0()
        .gap_1()
        .text_size(px(11.))
        .child(action_icon(ICON_REFRESH, 12., ACCENT));
        let root = root.clone();
        retry = retry.on_click(move |_, _, cx| {
            if let Some(root) = root.upgrade() {
                root.update(cx, |view, cx| {
                    view.retry_backend(cx);
                    cx.stop_propagation();
                    cx.notify();
                });
            }
        });
        content = content.child(retry);
    }

    card(content)
        .p(px(10.0))
        .h(px(STATS_CARD_HEIGHT))
        .min_w_0()
        .overflow_hidden()
}
