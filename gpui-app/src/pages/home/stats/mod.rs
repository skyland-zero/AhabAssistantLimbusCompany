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

mod backend;
mod details;
mod format;
mod history;
mod mirror;
mod overview;
mod run;

#[allow(unused_imports)]
pub(super) use backend::backend_status_card;
#[allow(unused_imports)]
pub(super) use details::{
    daily_details_body, daily_details_overlay, daily_table_row, daily_value,
    mirror_details_overlay, mirror_history_body,
};
#[allow(unused_imports)]
pub(super) use format::{_current_run_for_tests, format_duration, live_elapsed_secs};
#[allow(unused_imports)]
pub(super) use history::{
    combined_history_card, period_item, period_summary_section, recent_mirror_metric,
    recent_mirror_section, top_mirror_timings,
};
#[allow(unused_imports)]
pub(super) use mirror::{
    mirror_context_line, mirror_history_row, mirror_history_timing, mirror_route_name,
    mirror_system_label, mirror_team_name, mirror_team_sinners,
};
#[allow(unused_imports)]
pub(super) use overview::{RuntimeCardView, runtime_card, runtime_card_view};
#[allow(unused_imports)]
pub(super) use run::{
    current_run_card, current_task_is_mirror, display_run_state, mirror_floor_label,
    mirror_progress_ratio, run_metric,
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

    #[test]
    fn display_run_state_prefers_execution_snapshot() {
        let mut snapshot = StatsSnapshot::default();
        // execution.status 是权威快照：点了停止后这里先变 Stopping，
        // stats 事件滞后为 Running 时也不能再显示运行中。
        snapshot.stats.currentRun.runId = Some("run-1".to_string());
        snapshot.stats.currentRun.state = ExecutionState::Running;
        snapshot.stats.currentRun.currentTaskId = Some(FixedTaskId::Mirror);
        snapshot.execution.runId = Some("run-1".to_string());
        snapshot.execution.state = ExecutionState::Stopping;
        snapshot.execution.currentTaskId = Some(FixedTaskId::Mirror);

        let (state, task) = display_run_state(&snapshot);
        assert_eq!(state, ExecutionState::Stopping);
        assert_eq!(task, Some(FixedTaskId::Mirror));
    }

    #[test]
    fn display_run_state_falls_back_while_execution_snapshot_lags() {
        let mut snapshot = StatsSnapshot::default();
        snapshot.stats.currentRun.runId = Some("run-1".to_string());
        snapshot.stats.currentRun.state = ExecutionState::Running;
        snapshot.stats.currentRun.currentTaskId = Some(FixedTaskId::DailyTask);

        let (state, task) = display_run_state(&snapshot);
        assert_eq!(state, ExecutionState::Running);
        assert_eq!(task, Some(FixedTaskId::DailyTask));

        let idle = StatsSnapshot::default();
        let (state, task) = display_run_state(&idle);
        assert_eq!(state, ExecutionState::Idle);
        assert_eq!(task, None);
    }

    #[test]
    fn mirror_floor_label_hides_missing_floors() {
        assert_eq!(mirror_floor_label(None, Language::ZhCn), None);
        assert_eq!(
            mirror_floor_label(
                Some(&MirrorFloorPayload {
                    floor: 0,
                    ..Default::default()
                }),
                Language::ZhCn
            ),
            None
        );
        assert_eq!(
            mirror_floor_label(
                Some(&MirrorFloorPayload {
                    floor: 3,
                    floorTotal: 5,
                    ..Default::default()
                }),
                Language::ZhCn
            ),
            Some("第3层/共5层".to_string())
        );
        assert_eq!(
            mirror_floor_label(
                Some(&MirrorFloorPayload {
                    floor: 3,
                    ..Default::default()
                }),
                Language::EnUs
            ),
            Some("Floor 3".to_string())
        );
    }

    #[test]
    fn top_mirror_timings_returns_three_longest_phases() {
        let record = MirrorCompletionStats {
            completedAt: String::new(),
            runId: None,
            totalSeconds: 0.0,
            battleSeconds: 100.0,
            eventSeconds: 30.0,
            shopSeconds: 5.0,
            findRoadSeconds: 60.0,
            themePackSeconds: 0.0,
            rewardCardSeconds: 0.0,
            egoGiftSeconds: 0.0,
            settlementSeconds: 200.0,
            otherSeconds: 1.0,
            eventCount: 0,
            failed: None,
            failureReason: None,
            team: None,
            hardMode: false,
            mode: String::new(),
            floorCount: 0,
            routeId: String::new(),
            routeName: String::new(),
            routeNameEn: String::new(),
        };

        let top = top_mirror_timings(&record, Language::ZhCn);
        assert_eq!(top.len(), 3);
        assert_eq!(top[0].1, 200.0);
        assert_eq!(top[1].1, 100.0);
        assert_eq!(top[2].1, 60.0);
    }
}
