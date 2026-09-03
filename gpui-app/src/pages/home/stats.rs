use super::*;

use gpui::{Context, Render, WeakEntity, Window};

use crate::{
    app::{AhabApp, BackendStatus},
    model::{ExecutionStatsPayload, ExecutionStatusPayload, TasksConfig},
};

use std::time::{SystemTime, UNIX_EPOCH};

use crate::model::{
    CurrentRunStats, DailyStatEntry, MirrorCompletionStats, MirrorFloorPayload, MirrorTeamStats,
    StatCounts,
};

const STATS_CARD_HEIGHT: f32 = 185.0;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct StatsSnapshot {
    pub(super) language: Language,
    pub(super) backend_status: BackendStatus,
    pub(super) stats: ExecutionStatsPayload,
    pub(super) tasks: TasksConfig,
    pub(super) execution: ExecutionStatusPayload,
    pub(super) mirror_floor: Option<MirrorFloorPayload>,
}

impl Default for StatsSnapshot {
    fn default() -> Self {
        Self {
            language: Language::ZhCn,
            backend_status: BackendStatus::mock(),
            stats: ExecutionStatsPayload::default(),
            tasks: TasksConfig::default(),
            execution: ExecutionStatusPayload::default(),
            mirror_floor: None,
        }
    }
}

impl StatsSnapshot {
    pub(super) fn from_app(app: &AhabApp) -> Self {
        Self {
            language: app.state.settings.language,
            backend_status: app.backend_status.clone(),
            stats: app.home.stats.clone(),
            tasks: app.home.tasks.clone(),
            execution: app.home.execution.clone(),
            mirror_floor: app.home.mirror_floor.clone(),
        }
    }
}

pub(super) struct StatsView {
    root: WeakEntity<AhabApp>,
    snapshot: StatsSnapshot,
}

impl StatsView {
    pub(super) fn new(root: WeakEntity<AhabApp>) -> Self {
        Self {
            root,
            snapshot: StatsSnapshot::default(),
        }
    }

    pub(super) fn sync_snapshot(&mut self, snapshot: StatsSnapshot) {
        self.snapshot = snapshot;
    }
}

impl Render for StatsView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        render_overview(&self.snapshot, &self.root)
    }
}

fn render_overview(snapshot: &StatsSnapshot, root: &WeakEntity<AhabApp>) -> Div {
    div()
        .flex_none()
        .flex()
        .items_stretch()
        .gap_2()
        .ml(px(10.0))
        .mr(px(4.0))
        .mt(px(10.0))
        .child(runtime_card(snapshot, root))
        .child(combined_history_card(snapshot, root))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RuntimeCardView {
    Backend,
    CurrentRun,
}

impl RuntimeCardView {
    fn animation_key(self) -> &'static str {
        match self {
            Self::Backend => "backend",
            Self::CurrentRun => "current-run",
        }
    }
}

fn runtime_card_view(phase: BackendPhase) -> RuntimeCardView {
    if phase == BackendPhase::Ready {
        RuntimeCardView::CurrentRun
    } else {
        RuntimeCardView::Backend
    }
}

fn runtime_card(snapshot: &StatsSnapshot, root: &WeakEntity<AhabApp>) -> impl IntoElement {
    let view = runtime_card_view(snapshot.backend_status.phase);
    let card = match view {
        RuntimeCardView::Backend => backend_status_card(snapshot, root),
        RuntimeCardView::CurrentRun => current_run_card(snapshot),
    };

    card.flex_grow(1.0)
        .flex_shrink(1.0)
        .flex_basis(relative(0.0))
        .id("runtime-status-card")
        .with_animation(
            format!("runtime-card-{}", view.animation_key()),
            Animation::new(Duration::from_millis(150)).with_easing(gpui::ease_out_quint()),
            |card, progress| card.opacity(progress),
        )
}

fn backend_status_card(snapshot: &StatsSnapshot, root: &WeakEntity<AhabApp>) -> Div {
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

pub(super) fn mirror_details_overlay(
    app: &mut AhabApp,
    cx: &mut Context<AhabApp>,
) -> gpui::AnyElement {
    if !app.home.mirror_details_open {
        return div().into_any_element();
    }

    let language = app.state.settings.language;
    let records = if app.home.stats.mirrorHistory.is_empty() {
        app.home.stats.lastMirror.clone().into_iter().collect()
    } else {
        app.home.stats.mirrorHistory.clone()
    };
    let mut close = button("", ButtonVariant::Icon)
        .id("stats-mirror-close")
        .w(px(30.0))
        .h(px(30.0))
        .p_0()
        .child(action_icon(ICON_X, 15., TEXT_MUTED));
    close = close.on_click(cx.listener(|view, _, _, cx| {
        view.close_mirror_details(cx);
        cx.stop_propagation();
    }));
    close = close.on_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        if is_activation_key(event) {
            window.prevent_default();
            view.close_mirror_details(cx);
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
                .child(action_icon(ICON_SCROLL_TEXT, 17., ACCENT))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(1.0))
                        .child(
                            div()
                                .text_size(px(16.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(rgb(TEXT))
                                .child(text("镜牢明细", "Mirror Details").get(language)),
                        )
                        .child(div().text_size(px(10.5)).text_color(rgb(TEXT_MUTED)).child(
                            format!(
                                "{} {}/30",
                                text("最近", "Latest").get(language),
                                records.len()
                            ),
                        )),
                ),
        )
        .child(close);

    let body = mirror_history_body(app, &records, language);
    let dialog = div()
        .id("stats-mirror-dialog")
        .w(px(820.0))
        .h(px(650.0))
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
        .id("stats-mirror-overlay")
        .relative()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .p_4()
        .bg(rgba(0x00000080))
        .on_click(cx.listener(|view, _, _, cx| {
            view.close_mirror_details(cx);
        }));
    surface = surface.capture_key_down(cx.listener(|view, event: &KeyDownEvent, window, cx| {
        if event.keystroke.key.eq_ignore_ascii_case("escape") {
            window.prevent_default();
            cx.stop_propagation();
            view.close_mirror_details(cx);
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

fn mirror_history_body(
    app: &mut AhabApp,
    records: &[MirrorCompletionStats],
    language: Language,
) -> gpui::AnyElement {
    if records.is_empty() {
        return div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(12.0))
            .text_color(rgb(TEXT_MUTED))
            .child(text("暂无镜牢完成记录", "No completed mirror runs").get(language))
            .into_any_element();
    }

    let mut list = div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .px(px(18.0))
        .pt(px(14.0))
        .pb(px(16.0));
    for (index, record) in records.iter().enumerate() {
        list = list.child(mirror_history_row(index + 1, record, language));
    }
    scroll_area_with_id(app, "stats-mirror-scroll", list)
        .flex_1()
        .min_h_0()
        .into_any_element()
}

fn mirror_history_row(index: usize, record: &MirrorCompletionStats, language: Language) -> Div {
    let failed = record.failed.unwrap_or(false);
    let mode_label = if record.hardMode {
        text("困难", "Hard").get(language)
    } else {
        text("普通", "Normal").get(language)
    };
    let status_label = if failed {
        text("未完成", "Incomplete").get(language)
    } else {
        text("已完成", "Completed").get(language)
    };
    let status_tone = if failed {
        BadgeTone::Danger
    } else {
        BadgeTone::Success
    };
    let team = record.team.as_ref();
    let team_name = mirror_team_name(team, language);
    let team_number = team
        .map(|team| team.number)
        .filter(|number| *number > 0)
        .map(|number| format!(" #{number}"))
        .unwrap_or_default();
    let sinners = mirror_team_sinners(team, language);
    let route = mirror_route_name(record, language);
    let completed_at = record.completedAt.replace('T', " ");

    let heading = div()
        .flex()
        .items_center()
        .justify_between()
        .gap_2()
        .child(
            div()
                .min_w_0()
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_size(px(10.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child(format!("#{index}")),
                )
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_size(px(11.0))
                        .text_color(rgb(TEXT))
                        .child(completed_at),
                ),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap_1p5()
                .child(badge(
                    mode_label,
                    if record.hardMode {
                        BadgeTone::Warning
                    } else {
                        BadgeTone::Neutral
                    },
                ))
                .child(badge(status_label, status_tone))
                .child(
                    div()
                        .text_size(px(12.0))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(ACCENT))
                        .child(format_duration(record.totalSeconds)),
                ),
        );

    let metadata = div()
        .min_w_0()
        .flex()
        .items_center()
        .gap_1p5()
        .text_size(px(10.5))
        .text_color(rgb(TEXT_MUTED))
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_color(rgb(TEXT))
                .child(format!("{}{}", team_name, team_number)),
        )
        .child(
            div()
                .text_color(rgb(ACCENT))
                .child(mirror_system_label(team, language)),
        )
        .child(
            div()
                .min_w_0()
                .truncate()
                .child(format!("{} · {}", route, sinners)),
        );
    let metadata = if let Some(reason) = record.failureReason.as_deref() {
        metadata.child(
            div()
                .min_w_0()
                .truncate()
                .text_color(rgb(0xff8f8f))
                .child(format!(
                    "{}: {}",
                    text("原因", "Reason").get(language),
                    reason
                )),
        )
    } else {
        metadata
    };

    let mut timing = div().w_full().grid().grid_cols(3).gap_1();
    for (label, seconds) in [
        (
            text("战斗", "Battle").get(language).to_owned(),
            record.battleSeconds,
        ),
        (
            format!(
                "{} {}次",
                text("事件", "Events").get(language),
                record.eventCount
            ),
            record.eventSeconds,
        ),
        (
            text("商店", "Shop").get(language).to_owned(),
            record.shopSeconds,
        ),
        (
            text("寻路", "Path").get(language).to_owned(),
            record.findRoadSeconds,
        ),
        (
            text("主题包", "Theme").get(language).to_owned(),
            record.themePackSeconds,
        ),
        (
            text("奖励卡", "Reward").get(language).to_owned(),
            record.rewardCardSeconds,
        ),
        (
            text("饰品", "Ego").get(language).to_owned(),
            record.egoGiftSeconds,
        ),
        (
            text("结算", "Claim").get(language).to_owned(),
            record.settlementSeconds,
        ),
        (
            text("其他", "Other").get(language).to_owned(),
            record.otherSeconds,
        ),
    ] {
        timing = timing.child(mirror_history_timing(label, seconds));
    }

    card(
        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(heading)
            .child(metadata)
            .child(timing),
    )
    .w_full()
    .p_3()
}

fn mirror_history_timing(label: String, seconds: f64) -> Div {
    div()
        .min_w_0()
        .flex()
        .items_center()
        .justify_between()
        .gap_1()
        .rounded_sm()
        .px(px(6.0))
        .py(px(3.0))
        .bg(rgba((SURFACE_HOVER << 8) | 0x35))
        .text_size(px(9.5))
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_color(rgb(TEXT_MUTED))
                .child(label),
        )
        .child(
            div()
                .text_color(rgb(TEXT))
                .font_weight(FontWeight::MEDIUM)
                .child(format_duration(seconds)),
        )
}

fn mirror_context_line(record: &MirrorCompletionStats, language: Language) -> Div {
    let team = record.team.as_ref();
    let mode = if record.hardMode {
        text("困难", "Hard").get(language)
    } else {
        text("普通", "Normal").get(language)
    };
    div()
        .min_w_0()
        .flex()
        .items_center()
        .gap_1p5()
        .text_size(px(9.5))
        .text_color(rgb(TEXT_MUTED))
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_color(rgb(TEXT))
                .child(mirror_team_name(team, language)),
        )
        .child(
            div()
                .text_color(rgb(ACCENT))
                .child(mirror_system_label(team, language)),
        )
        .child(div().text_color(rgb(TEXT_MUTED)).child(mode))
}

fn mirror_team_name(team: Option<&MirrorTeamStats>, language: Language) -> String {
    team.and_then(|team| (!team.name.is_empty()).then_some(team.name.clone()))
        .unwrap_or_else(|| text("未知队伍", "Unknown team").get(language).to_owned())
}

fn mirror_team_sinners(team: Option<&MirrorTeamStats>, language: Language) -> String {
    let Some(team) = team else {
        return text("未记录人格", "Sinners unavailable")
            .get(language)
            .to_owned();
    };
    let names = if matches!(language, Language::ZhCn) {
        &team.sinnerNames
    } else {
        &team.sinnerNamesEn
    };
    let values = if names.is_empty() {
        &team.sinners
    } else {
        names
    };
    if values.is_empty() {
        text("未记录人格", "Sinners unavailable")
            .get(language)
            .to_owned()
    } else {
        values.join(if matches!(language, Language::ZhCn) {
            "、"
        } else {
            ", "
        })
    }
}

fn mirror_system_label(team: Option<&MirrorTeamStats>, language: Language) -> String {
    let system = team
        .map(|team| {
            if !team.system.is_empty() {
                team.system.as_str()
            } else {
                team.accessoryScheme.as_str()
            }
        })
        .unwrap_or_default();
    let label = match system {
        "burn" => text("烧伤", "Burn"),
        "bleed" => text("流血", "Bleed"),
        "tremor" => text("震颤", "Tremor"),
        "rupture" => text("破裂", "Rupture"),
        "poise" => text("呼吸", "Poise"),
        "sinking" => text("沉沦", "Sinking"),
        "charge" => text("充能", "Charge"),
        "slash" => text("斩击", "Slash"),
        "pierce" => text("突刺", "Pierce"),
        "blunt" => text("打击", "Blunt"),
        _ if system.is_empty() => text("未知体系", "Unknown system"),
        _ => return system.to_owned(),
    };
    label.get(language).to_owned()
}

fn mirror_route_name(record: &MirrorCompletionStats, language: Language) -> String {
    let route = if matches!(language, Language::ZhCn) {
        &record.routeName
    } else {
        &record.routeNameEn
    };
    if !route.is_empty() {
        route.clone()
    } else if !record.routeId.is_empty() {
        record.routeId.clone()
    } else {
        text("默认路线", "Default route").get(language).to_owned()
    }
}

fn current_run_card(snapshot: &StatsSnapshot) -> Div {
    let language = snapshot.language;
    let current = &snapshot.stats.currentRun;
    let (targets, infinite) = if current.runId.is_some() {
        (current.targets.clone(), current.isMirrorInfinite)
    } else {
        (
            StatCounts {
                exp: if snapshot.tasks.enabledTasks.daily_task {
                    u32::from(snapshot.tasks.daily_task.set_EXP_count)
                } else {
                    0
                },
                thread: if snapshot.tasks.enabledTasks.daily_task {
                    u32::from(snapshot.tasks.daily_task.set_thread_count)
                } else {
                    0
                },
                mirror: if snapshot.tasks.enabledTasks.mirror
                    && !snapshot.tasks.mirror.infinite_dungeons
                {
                    u32::from(snapshot.tasks.mirror.set_mirror_count)
                } else {
                    0
                },
            },
            snapshot.tasks.enabledTasks.mirror && snapshot.tasks.mirror.infinite_dungeons,
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
        snapshot.execution.state
    };
    let current_task = current.currentTaskId.or(snapshot.execution.currentTaskId);
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
        .h(px(24.0))
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
        .h(px(20.0))
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

    let info_row = {
        let elapsed_secs = live_elapsed_secs(current, snapshot.stats.updatedAt, state);
        let elapsed_str = format_duration(elapsed_secs);
        let started_hm = current.startedAt.map(|ms| {
            let secs_of_day = ((ms / 1000) % 86400 + 86400) % 86400;
            // UTC+8 for local display
            let local = (secs_of_day + 8 * 3600) % 86400;
            format!("{:02}:{:02}", local / 3600, (local % 3600) / 60)
        });
        let is_running = current.runId.is_some() && state == ExecutionState::Running;
        if is_running && current.startedAt.is_some() {
            let mut row = div()
                .h(px(16.0))
                .flex_none()
                .flex()
                .items_center()
                .gap_2()
                .text_size(px(10.0))
                .text_color(rgb(TEXT_MUTED));
            row = row.child(format!("已运行 {} · 开始 {}", elapsed_str, started_hm.unwrap_or_else(|| "--:--".into())));
            if current_task_is_mirror(current_task) {
                let base_label = if infinite {
                    "∞".to_string()
                } else if targets.mirror > 0 {
                    format!("{}层", targets.mirror)
                } else {
                    "5层".to_string()
                };
                // Append the live in-run floor (1-based). `base_label` is the
                // configured run count; the suffix is the current floor.
                let floor_label = match snapshot.mirror_floor.as_ref() {
                    Some(floor) if floor.floor > 0 => match language {
                        crate::model::Language::ZhCn => {
                            format!("{} · 第{}层", base_label, floor.floor)
                        }
                        _ => format!("{} · F{}", base_label, floor.floor),
                    },
                    _ => base_label,
                };
                row = row.child(
                    div()
                        .ml_auto()
                        .px(px(6.0))
                        .py(px(1.0))
                        .rounded_sm()
                        .bg(rgba((ACCENT << 8) | 0x18))
                        .text_size(px(9.0))
                        .text_color(rgb(ACCENT))
                        .child(floor_label),
                );
            }
            row
        } else {
            div().h(px(16.0)).flex_none()
        }
    };

    let mirror_block: gpui::AnyElement = if current_task_is_mirror(current_task) && current.runId.is_some() {
        let is_running = state == ExecutionState::Running;
        let display_completed = if is_running {
            if infinite {
                completed.mirror + 1
            } else if targets.mirror > 0 {
                (completed.mirror + 1).min(targets.mirror)
            } else {
                completed.mirror + 1
            }
        } else {
            completed.mirror
        };
        div()
            .flex_none()
            .flex()
            .flex_col()
            .gap(px(4.0))
            .child(
                div()
                    .h(px(3.0))
                    .w_full()
                    .rounded_full()
                    .bg(rgba((TEXT_MUTED << 8) | 0x24))
                    .child(
                        div()
                            .h_full()
                            .w(relative(mirror_progress_ratio(display_completed, targets.mirror, infinite)))
                            .rounded_full()
                            .bg(rgb(ACCENT)),
                    ),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .text_size(px(9.5))
                    .text_color(rgb(TEXT_MUTED))
                    .child({
                        let mut label = format!(
                            "镜牢进度 {}/{}{}",
                            display_completed,
                            if infinite { "∞".to_string() } else { targets.mirror.to_string() },
                            if is_running { " · 进行中" } else { "" }
                        );
                        if let Some(floor) = snapshot.mirror_floor.as_ref()
                            && floor.floor > 0
                        {
                            label.push_str(&format!(" · 第{}层", floor.floor));
                        }
                        label
                    })
                    .child(div().text_color(rgb(TEXT)).child(format_duration(
                        live_elapsed_secs(current, snapshot.stats.updatedAt, state)
                    ))),
            )
            .into_any_element()
    } else {
        // 不在镜牢：用 flex 占位把 metrics 压底，不显 “镜牢未运行” 文案
        div().flex_1().min_h(px(8.0)).into_any_element()
    };

    let metrics = div()
        .flex_none()
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
            .gap(px(4.0))
            .child(header)
            .child(task_line)
            .child(info_row)
            .child(mirror_block)
            .child(metrics),
    )
    .p(px(10.0))
    .h(px(STATS_CARD_HEIGHT))
    .min_w_0()
    .overflow_hidden()
}

fn current_task_is_mirror(task: Option<FixedTaskId>) -> bool {
    matches!(task, Some(FixedTaskId::Mirror))
}

fn mirror_progress_ratio(completed: u32, target: u32, infinite: bool) -> f32 {
    if infinite || target == 0 {
        if completed == 0 { 0.0 } else { (completed as f32 * 0.15).clamp(0.0, 1.0) }
    } else {
        (completed as f32 / target as f32).clamp(0.0, 1.0)
    }
}

fn combined_history_card(snapshot: &StatsSnapshot, root: &WeakEntity<AhabApp>) -> Div {
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

fn period_summary_section(
    snapshot: &StatsSnapshot,
    root: &WeakEntity<AhabApp>,
    language: Language,
) -> Div {
    let today = &snapshot.stats.today;
    let week = &snapshot.stats.week;

    let mut details = button(
        text("明细", "Details").get(language),
        ButtonVariant::Ghost,
    )
    .id("stats-daily-open")
    .h(px(20.0))
    .px(px(6.0))
    .py_0()
    .gap_1()
    .text_size(px(10.0))
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
                .child(
                    div()
                        .text_size(px(11.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child(text("周期统计 (今日/本周)", "Period Stats (Today/Week)").get(language)),
                ),
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
            text("镜牢", "Mirror").get(language),
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

fn period_item(label: &'static str, today: u32, week: u32) -> Div {
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
                .child(label),
        )
        .child(
            div()
                .flex()
                .items_baseline()
                .gap(px(2.0))
                .child(
                    div()
                        .text_size(px(13.0))
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

fn recent_mirror_section(
    snapshot: &StatsSnapshot,
    language: Language,
    root: &WeakEntity<AhabApp>,
) -> Div {
    let mut details = button(
        text("明细", "Details").get(language),
        ButtonVariant::Ghost,
    )
    .id("stats-mirror-open")
    .h(px(20.0))
    .px(px(6.0))
    .py_0()
    .gap_1()
    .text_size(px(10.0))
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
                        .text_size(px(11.0))
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
                        .text_size(px(11.0))
                        .text_color(rgb(TEXT_MUTED))
                        .child(text("最近镜牢", "Recent Mirror").get(language)),
                ),
        )
        .child(header_actions);

    let body = match snapshot.stats.lastMirror.as_ref() {
        Some(record) => div()
            .flex()
            .flex_col()
            .gap(px(2.5))
            .mt(px(2.0))
            .child(mirror_context_line(record, language))
            .child(
                div()
                    .flex()
                    .gap_1p5()
                    .child(recent_mirror_metric(
                        text("战斗", "Battle").get(language),
                        record.battleSeconds,
                    ))
                    .child(recent_mirror_metric(
                        text("事件", "Events").get(language),
                        record.eventSeconds,
                    ))
                    .child(recent_mirror_metric(
                        text("商店", "Shop").get(language),
                        record.shopSeconds,
                    )),
            )
            .child(
                div()
                    .flex()
                    .gap_1p5()
                    .child(recent_mirror_metric(
                        text("寻路", "Path").get(language),
                        record.findRoadSeconds,
                    ))
                    .child(recent_mirror_metric(
                        text("主题包", "Theme").get(language),
                        record.themePackSeconds,
                    ))
                    .child(recent_mirror_metric(
                        text("奖励卡", "Reward").get(language),
                        record.rewardCardSeconds,
                    )),
            )
            .child(
                div()
                    .flex()
                    .gap_1p5()
                    .child(recent_mirror_metric(
                        text("饰品", "Ego").get(language),
                        record.egoGiftSeconds,
                    ))
                    .child(recent_mirror_metric(
                        text("结算", "Claim").get(language),
                        record.settlementSeconds,
                    ))
                    .child(recent_mirror_metric(
                        text("其他", "Other").get(language),
                        record.otherSeconds,
                    )),
            ),
        None => div()
            .h(px(40.0))
            .flex()
            .items_center()
            .justify_center()
            .text_size(px(10.5))
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

fn recent_mirror_metric(label: &'static str, seconds: f64) -> Div {
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
        .child(
            div()
                .text_color(rgb(TEXT_MUTED))
                .truncate()
                .child(label),
        )
        .child(
            div()
                .text_color(rgb(TEXT))
                .font_weight(FontWeight::MEDIUM)
                .child(format_duration(seconds)),
        )
}

fn live_elapsed_secs(
    current: &CurrentRunStats,
    fallback_updated_at: i64,
    state: ExecutionState,
) -> f64 {
    let Some(started) = current.startedAt else {
        return 0.0;
    };
    let elapsed_ms = if state == ExecutionState::Running && current.runId.is_some() {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or(fallback_updated_at);
        (now_ms - started).max(0)
    } else if let Some(updated) = current.updatedAt {
        (updated - started).max(0)
    } else {
        (fallback_updated_at - started).max(0)
    };
    elapsed_ms as f64 / 1000.0
}

fn format_duration(seconds: f64) -> String {
    let total_seconds = if seconds.is_finite() {
        seconds.max(0.0).floor() as u64
    } else {
        0
    };
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{minutes:02}:{seconds:02}")
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
        .py_1p5()
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
                .text_size(px(16.0))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_card_shows_current_run_only_after_backend_is_ready() {
        assert_eq!(
            runtime_card_view(BackendPhase::Ready),
            RuntimeCardView::CurrentRun
        );

        for phase in [
            BackendPhase::WaitingForFirstFrame,
            BackendPhase::Starting,
            BackendPhase::RetryWaiting,
            BackendPhase::Failed,
            BackendPhase::Disconnected,
            BackendPhase::Restarting,
            BackendPhase::Mock,
        ] {
            assert_eq!(runtime_card_view(phase), RuntimeCardView::Backend);
        }
    }

    #[test]
    fn recent_mirror_duration_is_non_negative_and_human_readable() {
        assert_eq!(format_duration(0.0), "00:00");
        assert_eq!(format_duration(3661.9), "61:01");
        assert_eq!(format_duration(1879.2), "31:19");
        assert_eq!(format_duration(-10.0), "00:00");
        assert_eq!(format_duration(f64::NAN), "00:00");
    }

    #[test]
    fn stats_snapshot_defaults_to_zh_cn_and_empty_stats() {
        let snapshot = StatsSnapshot::default();
        assert_eq!(snapshot.language, Language::ZhCn);
        assert!(snapshot.stats.lastMirror.is_none());
        assert_eq!(snapshot.stats.today.exp, 0);
    }
}
