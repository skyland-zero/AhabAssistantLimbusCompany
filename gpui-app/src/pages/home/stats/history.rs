use super::*;

pub(crate) fn combined_history_card(snapshot: &StatsSnapshot, root: &WeakEntity<AhabApp>) -> Div {
    let language = snapshot.language;
    let period_section = period_summary_section(snapshot, root, language);
    let divider = div()
        .h(px(1.0))
        .w_full()
        .bg(rgba((BORDER << 8) | 0x60))
        .my(px(1.5));
    let mirror_section = recent_mirror_section(snapshot, language, root);

    card(
        div()
            .flex()
            .flex_col()
            .gap(px(2.0))
            .child(period_section)
            .child(divider)
            .child(mirror_section),
    )
    .p(px(10.0))
    .h(px(STATS_CARD_HEIGHT))
    .flex_grow(1.0)
    .flex_shrink(1.0)
    .flex_basis(relative(0.0))
    .min_w_0()
    .overflow_hidden()
}

pub(crate) fn period_summary_section(
    snapshot: &StatsSnapshot,
    root: &WeakEntity<AhabApp>,
    language: Language,
) -> Div {
    let today = &snapshot.stats.today;
    let week = &snapshot.stats.week;

    let mut details = button(text("明细", "Details").get(language), ButtonVariant::Ghost)
        .id("stats-daily-open")
        .h(px(20.0))
        .px(px(6.0))
        .py_0()
        .gap_1()
        .text_size(px(10.))
        .child(action_icon(ICON_CALENDAR_CHECK, 11., ACCENT));
    let root_for_details = root.clone();
    details = details.on_click(move |_, _, cx| {
        if let Some(root) = root_for_details.upgrade() {
            root.update(cx, |view, cx| {
                view.open_stats_details(cx);
                cx.stop_propagation();
            });
        }
    });

    let header = div()
        .h(px(20.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .child(
            div()
                .flex()
                .items_center()
                .gap_1p5()
                .child(action_icon(ICON_HISTORY, 13., ACCENT))
                .child(div().text_size(px(11.)).text_color(rgb(TEXT_MUTED)).child(
                    text("周期统计 (今日/本周)", "Period Stats (Today/Week)").get(language),
                )),
        )
        .child(details);

    let items = div()
        .flex()
        .items_center()
        .gap_1p5()
        .mt(px(2.0))
        .child(period_item(
            text("经验本", "EXP").get(language),
            today.exp,
            week.exp,
        ))
        .child(period_item(
            text("纽本", "Thread").get(language),
            today.thread,
            week.thread,
        ))
        .child(period_item(
            format!(
                "{}({})",
                text("镜牢", "Mirror").get(language),
                text("累计", "total").get(language)
            ),
            today.mirror,
            week.mirror,
        ));

    div()
        .flex_none()
        .flex()
        .flex_col()
        .child(header)
        .child(items)
}

pub(crate) fn period_item(label: impl Into<String>, today: u32, week: u32) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .justify_between()
        .gap(px(1.0))
        .px_2()
        .py(px(3.0))
        .rounded_md()
        .bg(rgba((SURFACE_HOVER << 8) | 0x35))
        .child(
            div()
                .text_size(px(9.5))
                .text_color(rgb(TEXT_MUTED))
                .truncate()
                .child(label.into()),
        )
        .child(
            div()
                .flex()
                .items_baseline()
                .gap(px(2.0))
                .child(
                    div()
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT))
                        .child(today.to_string()),
                )
                .child(
                    div()
                        .text_size(px(9.5))
                        .text_color(rgb(TEXT_MUTED))
                        .child(format!("/ {week}")),
                ),
        )
}

pub(crate) fn recent_mirror_section(
    snapshot: &StatsSnapshot,
    language: Language,
    root: &WeakEntity<AhabApp>,
) -> Div {
    let mut details = button(text("明细", "Details").get(language), ButtonVariant::Ghost)
        .id("stats-mirror-open")
        .h(px(20.0))
        .px(px(6.0))
        .py_0()
        .gap_1()
        .text_size(px(10.))
        .child(action_icon(ICON_SCROLL_TEXT, 11., ACCENT));
    let root_for_details = root.clone();
    details = details.on_click(move |_, _, cx| {
        if let Some(root) = root_for_details.upgrade() {
            root.update(cx, |view, cx| {
                view.open_mirror_details(cx);
                cx.stop_propagation();
            });
        }
    });

    let header_right = match snapshot.stats.lastMirror.as_ref() {
        Some(record) => {
            let is_failed = record.failed.unwrap_or(false);
            let is_timeout = record.failureReason.as_deref() == Some("settlement_timeout");
            let status_badge = if is_failed {
                badge(
                    if is_timeout {
                        text("领取超时", "Claim Timeout").get(language)
                    } else {
                        text("未完成", "Incomplete").get(language)
                    },
                    BadgeTone::Danger,
                )
            } else {
                div()
            };
            div()
                .flex()
                .items_center()
                .gap_1p5()
                .child(
                    div()
                        .text_size(px(9.5))
                        .text_color(rgb(TEXT_MUTED))
                        .child(format!(
                            "{}{}",
                            record.eventCount,
                            text("次事件", " events").get(language)
                        )),
                )
                .child(
                    div()
                        .text_size(px(11.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(ACCENT))
                        .child(format_duration(record.totalSeconds)),
                )
                .child(status_badge)
        }
        None => div(),
    };
    let header_actions = div()
        .flex()
        .items_center()
        .gap_1p5()
        .child(header_right)
        .child(details);

    let header = div()
        .h(px(20.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .child(
            div()
                .flex()
                .items_center()
                .gap_1p5()
                .child(action_icon(ICON_COMPASS, 13., ACCENT))
                .child(
                    div()
                        .text_size(px(11.))
                        .text_color(rgb(TEXT_MUTED))
                        .child(text("上次镜牢", "Last Mirror").get(language)),
                ),
        )
        .child(header_actions);

    let body = match snapshot.stats.lastMirror.as_ref() {
        Some(record) => {
            // 摘要只放耗时 Top3，完整 9 项在明细弹窗看，避免 185px 卡片被 3 行格子塞满。
            let mut top_row = div().flex().gap_1p5();
            for (label, seconds) in top_mirror_timings(record, language) {
                top_row = top_row.child(recent_mirror_metric(label, seconds));
            }
            div()
                .flex()
                .flex_col()
                .gap(px(2.5))
                .mt(px(2.0))
                .child(mirror_context_line(record, language))
                .child(top_row)
        }
        None => div()
            .h(px(40.0))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(10.))
            .text_color(rgb(TEXT_MUTED))
            .child(text("暂无完成记录", "No completed mirror").get(language)),
    };

    div()
        .flex_none()
        .flex()
        .flex_col()
        .child(header)
        .child(body)
}

/// 上次镜牢耗时 Top3（标签, 秒数），按耗时降序。卡片摘要用，完整 9 项在明细弹窗。
pub(crate) fn top_mirror_timings(
    record: &MirrorCompletionStats,
    language: Language,
) -> Vec<(&'static str, f64)> {
    let mut timings = vec![
        (text("战斗", "Battle").get(language), record.battleSeconds),
        (text("事件", "Events").get(language), record.eventSeconds),
        (text("商店", "Shop").get(language), record.shopSeconds),
        (text("寻路", "Path").get(language), record.findRoadSeconds),
        (
            text("主题包", "Theme").get(language),
            record.themePackSeconds,
        ),
        (
            text("奖励卡", "Reward").get(language),
            record.rewardCardSeconds,
        ),
        (text("饰品", "Ego").get(language), record.egoGiftSeconds),
        (
            text("结算", "Claim").get(language),
            record.settlementSeconds,
        ),
        (text("其他", "Other").get(language), record.otherSeconds),
    ];
    timings.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    timings.truncate(3);
    timings
}

pub(crate) fn recent_mirror_metric(label: &'static str, seconds: f64) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .items_center()
        .justify_between()
        .rounded_sm()
        .px(px(6.0))
        .py(px(2.0))
        .bg(rgba((SURFACE_HOVER << 8) | 0x35))
        .text_size(px(9.5))
        .child(div().text_color(rgb(TEXT_MUTED)).truncate().child(label))
        .child(
            div()
                .text_color(rgb(TEXT))
                .font_weight(FontWeight::MEDIUM)
                .child(format_duration(seconds)),
        )
}
