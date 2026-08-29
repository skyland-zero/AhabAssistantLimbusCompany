use super::*;

use crate::model::{CurrentRunStats, DailyStatEntry, StatCounts};

const STATS_CARD_HEIGHT: f32 = 156.0;

pub(super) fn overview(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    div()
        .flex_none()
        .flex()
        .items_stretch()
        .gap_2()
        .ml(px(10.0))
        .mr(px(4.0))
        .mt(px(10.0))
        .child(current_run_card(app).flex_grow(2.0).flex_shrink(1.0))
        .child(period_card(app, cx).flex_grow(1.0).flex_shrink(1.0))
        .child(backend_status_card(app, cx).flex_grow(1.0).flex_shrink(1.0))
}

fn backend_status_card(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    let language = app.state.settings.language;
    let status = &app.backend_status;
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
                .map(|retry| format!("重试中 {retry}/3"))
                .unwrap_or_else(|| text("启动中", "Starting").get(language).to_owned()),
            text("正在连接 Python 后端", "Connecting to Python backend")
                .get(language)
                .to_owned(),
            BadgeTone::Accent,
        ),
        BackendPhase::RetryWaiting => (
            status
                .retry_no
                .map(|retry| format!("自动重试 {retry}/3"))
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
                .map(|retry| format!("恢复中 {retry}/3"))
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
                        .text_size(px(12.0))
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
                .text_size(px(11.0))
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
        .text_size(px(11.0))
        .child(action_icon(ICON_REFRESH, 12., ACCENT));
        retry = retry.on_click(cx.listener(|view, _, _, cx| {
            view.retry_backend(cx);
            cx.stop_propagation();
            cx.notify();
        }));
        content = content.child(retry);
    }

    card(content)
        .h(px(STATS_CARD_HEIGHT))
        .min_w_0()
        .overflow_hidden()
}

pub(super) fn daily_details_overlay(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
) -> gpui::AnyElement {
    if !app.home.stats_details_open {
        return div().into_any_element();
    }

    let language = app.state.settings.language;
    let palette = current_render_palette();
    let selected_date = app.home.stats_selected_date.clone();
    let selected_entry = app
        .home
        .daily_stats
        .as_ref()
        .and_then(|data| {
            selected_date
                .as_deref()
                .and_then(|date| data.days.iter().find(|day| day.date == date))
                .or_else(|| data.days.first())
        })
        .cloned();

    let mut close = button("", ButtonVariant::Icon)
        .id("stats-daily-close")
        .w(px(30.0))
        .h(px(30.0))
        .p_0()
        .child(action_icon(ICON_X, 15., TEXT_MUTED));
    close = close.on_click(cx.listener(|view, _, _, cx| {
        view.close_stats_details(cx);
        cx.stop_propagation();
    }));
    close = close.on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        if is_activation_key(event) {
            window.prevent_default();
            view.close_stats_details(cx);
        }
    }));

    let header = div()
        .flex()
        .items_center()
        .justify_between()
        .gap_3()
        .px(px(18.0))
        .py(px(12.0))
        .border_b_1()
        .border_color(rgb(BORDER))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .child(action_icon(ICON_CALENDAR_CHECK, 17., ACCENT))
                .child(
                    div()
                        .text_size(px(16.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(TEXT))
                        .child(text("每日刷本明细", "Daily Run Details").get(language)),
                ),
        )
        .child(close);

    let body = if app.home.stats_details_loading {
        div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(12.0))
            .text_color(rgb(TEXT_MUTED))
            .child(text("正在加载每日统计…", "Loading daily statistics…").get(language))
    } else if let Some(error) = app.home.stats_details_error.clone() {
        div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(12.0))
            .text_color(palette_rgb(palette.danger))
            .child(error)
    } else if let Some(data) = app.home.daily_stats.clone() {
        daily_details_body(app, cx, &data, selected_entry.as_ref())
    } else {
        div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(12.0))
            .text_color(rgb(TEXT_MUTED))
            .child(text("暂无每日统计", "No daily statistics yet").get(language))
    };

    let dialog = div()
        .id("stats-daily-dialog")
        .w(px(640.0))
        .h(px(520.0))
        .max_w_full()
        .max_h(relative(0.94))
        .min_h_0()
        .overflow_hidden()
        .flex()
        .flex_col()
        .rounded_lg()
        .border_1()
        .border_color(rgb(BORDER))
        .bg(rgb(SURFACE))
        .on_click(cx.listener(|_, _, _, cx| cx.stop_propagation()))
        .child(header)
        .child(body);

    let mut surface = div()
        .id("stats-daily-overlay")
        .relative()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .p_4()
        .bg(rgba(0x00000080))
        .on_click(cx.listener(|view, _, _, cx| {
            view.close_stats_details(cx);
        }));
    surface = surface.capture_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        if event.keystroke.key.eq_ignore_ascii_case("escape") {
            window.prevent_default();
            cx.stop_propagation();
            view.close_stats_details(cx);
        }
    }));

    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .bottom_0()
        .child(surface.child(dialog))
        .into_any_element()
}

fn current_run_card(app: &AhabApp) -> Div {
    let language = app.state.settings.language;
    let current = &app.home.stats.currentRun;
    let (targets, infinite) = if current.runId.is_some() {
        (current.targets.clone(), current.isMirrorInfinite)
    } else {
        (
            StatCounts {
                exp: if app.home.tasks.enabledTasks.daily_task {
                    u32::from(app.home.tasks.daily_task.set_EXP_count)
                } else {
                    0
                },
                thread: if app.home.tasks.enabledTasks.daily_task {
                    u32::from(app.home.tasks.daily_task.set_thread_count)
                } else {
                    0
                },
                mirror: if app.home.tasks.enabledTasks.mirror
                    && !app.home.tasks.mirror.infinite_dungeons
                {
                    u32::from(app.home.tasks.mirror.set_mirror_count)
                } else {
                    0
                },
            },
            app.home.tasks.enabledTasks.mirror && app.home.tasks.mirror.infinite_dungeons,
        )
    };
    let completed = if current.runId.is_some() {
        current.completed.clone()
    } else {
        StatCounts::default()
    };
    let state = if current.runId.is_some() {
        current.state
    } else {
        app.home.execution.state
    };
    let current_task = current.currentTaskId.or(app.home.execution.currentTaskId);
    let state_text = match state {
        ExecutionState::Running => text("运行中", "Running").get(language),
        ExecutionState::Paused => text("已暂停", "Paused").get(language),
        ExecutionState::Stopping => text("停止中", "Stopping").get(language),
        ExecutionState::Idle => text("待机", "Idle").get(language),
    };
    let state_tone = match state {
        ExecutionState::Running => BadgeTone::Success,
        ExecutionState::Paused => BadgeTone::Warning,
        ExecutionState::Stopping => BadgeTone::Warning,
        ExecutionState::Idle => BadgeTone::Neutral,
    };
    let task_text = current_task
        .map(|task| task_title(task, language).to_owned())
        .unwrap_or_else(|| {
            text("等待开始", "Waiting to start")
                .get(language)
                .to_owned()
        });

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
                .child(action_icon(ICON_PLAY, 14., ACCENT))
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child(text("本次运行", "Current Run").get(language)),
                ),
        )
        .child(badge(state_text, state_tone));

    let task_line = div()
        .h(px(22.0))
        .flex_none()
        .flex()
        .items_center()
        .gap_2()
        .text_size(px(11.0))
        .text_color(rgb(TEXT_MUTED))
        .child(text("当前任务", "Task").get(language))
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_color(rgb(TEXT))
                .child(task_text),
        );

    let metrics = div()
        .flex_1()
        .min_h_0()
        .flex()
        .items_stretch()
        .gap_2()
        .child(run_metric(
            text("经验本", "EXP").get(language),
            completed.exp,
            targets.exp,
            false,
        ))
        .child(run_metric(
            text("纽本", "Thread").get(language),
            completed.thread,
            targets.thread,
            false,
        ))
        .child(run_metric(
            text("镜牢", "Mirror").get(language),
            completed.mirror,
            targets.mirror,
            infinite,
        ));

    card(
        div()
            .h_full()
            .flex()
            .flex_col()
            .gap_1()
            .child(header)
            .child(task_line)
            .child(metrics),
    )
    .h(px(STATS_CARD_HEIGHT))
    .min_w_0()
    .overflow_hidden()
}

fn period_card(app: &mut AhabApp, cx: &mut Context<AhabApp>) -> Div {
    let language = app.state.settings.language;
    let today = app.home.stats.today.clone();
    let week = app.home.stats.week.clone();
    let mut details = button(
        text("查看明细", "View Details").get(language),
        ButtonVariant::Ghost,
    )
    .id("stats-daily-open")
    .h(px(26.0))
    .px_2()
    .py_0()
    .gap_1()
    .text_size(px(11.0))
    .child(action_icon(ICON_CALENDAR_CHECK, 13., ACCENT));
    details = details.on_click(cx.listener(|view, _, _, cx| {
        view.open_stats_details(cx);
        cx.stop_propagation();
    }));

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
                .child(action_icon(ICON_HISTORY, 14., ACCENT))
                .child(
                    div()
                        .text_size(px(12.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child(text("今日 / 本周", "Today / Week").get(language)),
                ),
        )
        .child(details);

    let column_header = div()
        .h(px(18.0))
        .flex_none()
        .flex()
        .items_center()
        .text_size(px(10.0))
        .text_color(rgb(TEXT_MUTED))
        .child(div().flex_1().child(" "))
        .child(period_value(text("今日", "Today").get(language)))
        .child(period_value(text("本周", "Week").get(language)));

    let rows = [
        (text("经验本", "EXP").get(language), today.exp, week.exp),
        (
            text("纽本", "Thread").get(language),
            today.thread,
            week.thread,
        ),
        (
            text("镜牢", "Mirror").get(language),
            today.mirror,
            week.mirror,
        ),
    ];
    let mut body = div().flex().flex_col().gap_1().flex_1().min_h_0();
    for (label, today, week) in rows {
        body = body.child(
            div()
                .h(px(22.0))
                .flex_none()
                .flex()
                .items_center()
                .text_size(px(12.0))
                .text_color(rgb(TEXT))
                .child(div().flex_1().child(label))
                .child(period_value(today.to_string()))
                .child(period_value(week.to_string())),
        );
    }

    card(
        div()
            .h_full()
            .flex()
            .flex_col()
            .gap_1()
            .child(header)
            .child(column_header)
            .child(body),
    )
    .h(px(STATS_CARD_HEIGHT))
    .min_w_0()
    .overflow_hidden()
}

fn run_metric(label: &'static str, completed: u32, target: u32, infinite: bool) -> Div {
    let ratio = if infinite || target == 0 {
        0.0
    } else {
        (completed as f32 / target as f32).clamp(0.0, 1.0)
    };
    let value = if infinite {
        format!("{completed} / ∞")
    } else {
        format!("{completed} / {target}")
    };
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .justify_between()
        .gap_1()
        .px_2()
        .py_1()
        .rounded_md()
        .bg(rgba((SURFACE_HOVER << 8) | 0x45))
        .child(
            div()
                .text_size(px(10.0))
                .text_color(rgb(TEXT_MUTED))
                .truncate()
                .child(label),
        )
        .child(
            div()
                .text_size(px(17.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT))
                .child(value),
        )
        .child(
            div()
                .h(px(3.0))
                .w_full()
                .rounded_full()
                .bg(rgba((TEXT_MUTED << 8) | 0x24))
                .child(
                    div()
                        .h_full()
                        .w(relative(ratio))
                        .rounded_full()
                        .bg(rgb(ACCENT)),
                ),
        )
}

fn period_value(value: impl Into<String>) -> Div {
    div()
        .w(px(54.0))
        .flex_none()
        .text_center()
        .child(value.into())
}

fn daily_details_body(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
    data: &crate::model::DailyStatsPayload,
    selected: Option<&DailyStatEntry>,
) -> Div {
    let language = app.state.settings.language;
    let selected = selected.cloned().unwrap_or_default();
    let summary = div()
        .flex_none()
        .mx(px(18.0))
        .mt(px(14.0))
        .p_3()
        .rounded_md()
        .bg(rgba((ACCENT << 8) | 0x18))
        .child(
            div()
                .text_size(px(12.0))
                .text_color(rgb(TEXT_MUTED))
                .child(format!(
                    "{}  ·  {}",
                    selected.date,
                    text("当日完成", "Completed").get(language)
                )),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_3()
                .mt_1()
                .text_size(px(13.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(TEXT))
                .child(format!(
                    "{} {}",
                    text("经验本", "EXP").get(language),
                    selected.exp
                ))
                .child(format!(
                    "{} {}",
                    text("纽本", "Thread").get(language),
                    selected.thread
                ))
                .child(format!(
                    "{} {}",
                    text("镜牢", "Mirror").get(language),
                    selected.mirror
                )),
        );

    let table_header = daily_table_row(
        text("日期", "Date").get(language),
        text("经验本", "EXP").get(language),
        text("纽本", "Thread").get(language),
        text("镜牢", "Mirror").get(language),
        text("合计", "Total").get(language),
        true,
    );
    let mut table = div().flex().flex_col().gap_1().px(px(18.0)).pb(px(14.0));
    for day in &data.days {
        let date = day.date.clone();
        let active = selected.date == day.date;
        let mut row = daily_table_row(
            &day.date,
            day.exp.to_string(),
            day.thread.to_string(),
            day.mirror.to_string(),
            day.total.to_string(),
            false,
        )
        .id(format!("stats-day-{}", day.date));
        if active {
            row = row.bg(rgba((ACCENT << 8) | 0x28));
        } else {
            row = row.hover(|style| style.bg(rgba((SURFACE_HOVER << 8) | 0x45)));
        }
        row = row.on_click(cx.listener(move |view, _, _, cx| {
            view.home.select_stats_date(date.clone());
            cx.stop_propagation();
            cx.notify();
        }));
        table = table.child(row);
    }
    let table_body: gpui::AnyElement = if data.days.is_empty() {
        div()
            .flex()
            .items_center()
            .justify_center()
            .h(px(100.0))
            .text_size(px(12.0))
            .text_color(rgb(TEXT_MUTED))
            .child(text("暂无每日数据", "No daily data").get(language))
            .into_any_element()
    } else {
        scroll_area_with_id(app, "stats-daily-scroll", table)
            .flex_1()
            .min_h_0()
            .into_any_element()
    };

    div()
        .flex()
        .flex_col()
        .min_h_0()
        .flex_1()
        .child(summary)
        .child(table_header)
        .child(table_body)
}

fn daily_table_row(
    date: &str,
    exp: impl Into<String>,
    thread: impl Into<String>,
    mirror: impl Into<String>,
    total: impl Into<String>,
    header: bool,
) -> Div {
    div()
        .h(px(28.0))
        .flex_none()
        .items_center()
        .flex()
        .rounded_sm()
        .px_2()
        .text_size(px(if header { 10.0 } else { 11.0 }))
        .text_color(rgb(if header { TEXT_MUTED } else { TEXT }))
        .child(div().w(px(112.0)).flex_none().child(date.to_owned()))
        .child(daily_value(exp))
        .child(daily_value(thread))
        .child(daily_value(mirror))
        .child(daily_value(total))
}

fn daily_value(value: impl Into<String>) -> Div {
    div().flex_1().text_center().child(value.into())
}

#[allow(dead_code)]
fn _current_run_for_tests(current: &CurrentRunStats) -> &CurrentRunStats {
    current
}
