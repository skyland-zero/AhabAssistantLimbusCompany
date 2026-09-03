mod config;
mod execution;
mod selection;
mod stats;

use std::{
    collections::{HashMap, HashSet, VecDeque},
    time::Instant,
};

use serde_json::json;

use crate::{
    ipc::{EventEnvelope, MockClient, RpcGateway},
    model::{
        AfterExitAction, AfterPowerAction, ConnectionStatus, DailyStatsPayload, DeviceInfo,
        DeviceStatusPayload, ExecutionState, ExecutionStatsPayload, ExecutionStatusPayload,
        FixedTaskId, LogEntryPayload, LogLevel, MirrorFloorPayload, MirrorProgressPayload,
        PreviewStatus,
        PreviewStatusPayload, ScreenshotFrame, TasksConfig,
    },
};

/// State owned by the Home page. Keeping the RPC client and page configuration
/// here leaves render code responsible only for describing the current state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MirrorOption {
    Infinite,
    NoWeeklyBonuses,
    FloorThreeExit,
    SaveRewards,
    HardSingleBonuses,
    SelectEventPack,
    SkipEventPack,
    ReclaimRewards,
    NotSkipCotton,
    FightToLast,
    KeyboardNavigation,
    SimplePathfinding,
    PreferHatredAndDespair,
    MinimizeNonBossCombat,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TaskOptionsTab {
    #[default]
    General,
    Advanced,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HomeSelect {
    WindowResolution,
    WindowPosition,
    ScreenshotInterval,
    MouseInterval,
    DailyTeam(u8),
    RewardMode,
    AfterPowerAction,
    Device,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DailyCounter {
    Exp,
    Thread,
    Continuous,
}

pub struct HomeState {
    pub rpc: RpcGateway,
    pub tasks: TasksConfig,
    pub execution: ExecutionStatusPayload,
    pub stats: ExecutionStatsPayload,
    pub daily_stats: Option<DailyStatsPayload>,
    pub stats_details_open: bool,
    pub stats_details_loading: bool,
    pub stats_details_error: Option<String>,
    pub stats_selected_date: Option<String>,
    pub mirror_details_open: bool,
    pub logs: VecDeque<LogEntryPayload>,
    pub log_revision: u64,
    pub devices: Vec<DeviceInfo>,
    pub selected_device: Option<String>,
    pub device_status: ConnectionStatus,
    pub is_scanning_devices: bool,
    pub device_error: Option<String>,
    pub right_panel_width: f32,
    pub right_panel_collapsed: bool,
    pub expanded_tasks: HashSet<FixedTaskId>,
    pub task_options_tabs: HashMap<FixedTaskId, TaskOptionsTab>,
    pub open_select: Option<HomeSelect>,
    pub mirror_progress: Option<MirrorProgressPayload>,
    pub mirror_floor: Option<MirrorFloorPayload>,
    pub latest_screenshot: Option<ScreenshotFrame>,
    pub screenshot_revision: u64,
    pub preview_status: PreviewStatus,
    pub preview_error: Option<String>,
    pub after_completion_open: bool,
    pub after_completion_draft: Option<crate::model::AfterCompletionConfig>,
    pub last_event_sequence: u64,
    pub(crate) stopping_since: Option<Instant>,
    pub(crate) state_before_stopping: Option<ExecutionState>,
}

impl Default for HomeState {
    fn default() -> Self {
        Self::with_layout(280, false)
    }
}

impl HomeState {
    pub fn with_layout(right_panel_width: u32, right_panel_collapsed: bool) -> Self {
        Self::with_client(
            MockClient::default(),
            right_panel_width,
            right_panel_collapsed,
        )
    }

    pub fn with_client(
        client: impl Into<crate::ipc::BackendClient>,
        right_panel_width: u32,
        right_panel_collapsed: bool,
    ) -> Self {
        let mut rpc = RpcGateway::new(client);
        let (tasks, devices) = if rpc.is_sidecar() {
            // Runtime hydration is scheduled from AhabApp::start_backend_hydration
            // so sidecar startup and first paint never wait on the network.
            (TasksConfig::default(), Vec::new())
        } else {
            (
                rpc.request(crate::ipc::contract::method::TASKS_GET_CONFIG, None)
                    .unwrap_or_default(),
                rpc.request(crate::ipc::contract::method::DEVICE_LIST, None)
                    .unwrap_or_default(),
            )
        };

        Self {
            rpc,
            tasks,
            execution: ExecutionStatusPayload::default(),
            stats: ExecutionStatsPayload::default(),
            daily_stats: None,
            stats_details_open: false,
            stats_details_loading: false,
            stats_details_error: None,
            stats_selected_date: None,
            mirror_details_open: false,
            logs: VecDeque::new(),
            log_revision: 0,
            devices,
            selected_device: None,
            device_status: ConnectionStatus::Disconnected,
            is_scanning_devices: false,
            device_error: None,
            right_panel_width: right_panel_width as f32,
            right_panel_collapsed,
            expanded_tasks: HashSet::new(),
            task_options_tabs: HashMap::new(),
            open_select: None,
            mirror_progress: None,
            mirror_floor: None,
            latest_screenshot: None,
            screenshot_revision: 0,
            preview_status: PreviewStatus::Stopped,
            preview_error: None,
            after_completion_open: false,
            after_completion_draft: None,
            last_event_sequence: 0,
            stopping_since: None,
            state_before_stopping: None,
        }
    }

    pub fn is_busy(&self) -> bool {
        !matches!(self.execution.state, ExecutionState::Idle)
    }
}

fn adjust_u8(value: u8, delta: i8, min: u8, max: u8) -> u8 {
    if delta < 0 {
        value.saturating_sub((-delta) as u8).max(min)
    } else {
        value.saturating_add(delta as u8).min(max)
    }
}

fn cycle_inclusive(value: u8, min: u8, max: u8) -> u8 {
    if value < min || value >= max {
        min
    } else {
        value + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_execution_follows_idle_running_paused_running_idle() {
        let mut home = HomeState::default();
        home.start();
        assert_eq!(home.execution.state, ExecutionState::Running);
        home.pause_or_resume();
        assert_eq!(home.execution.state, ExecutionState::Paused);
        home.pause_or_resume();
        assert_eq!(home.execution.state, ExecutionState::Running);
        home.stop();
        assert_eq!(home.execution.state, ExecutionState::Idle);
    }

    #[test]
    fn empty_start_logs_warning_and_logs_are_bounded() {
        let mut home = HomeState::default();
        home.set_all_tasks(false);
        home.start();
        assert!(home.logs.back().unwrap().message.contains("没有选择任务"));
        for _ in 0..400 {
            home.log("bounded");
        }
        assert_eq!(home.logs.len(), 300);
    }

    #[test]
    fn task_selection_uses_contract_defaults() {
        let home = HomeState::default();
        assert_eq!(home.selected_task_count(), 3);
        assert_eq!(home.tasks.set_windows.set_win_size, 1080);
    }

    #[test]
    fn executable_task_selection_excludes_ahab_and_window_settings() {
        let mut home = HomeState::default();
        home.set_all_tasks(false);
        home.tasks.enabledTasks.resonate_with_Ahab = true;
        assert_eq!(home.selected_task_count(), 0);
        home.set_all_tasks(true);
        assert_eq!(home.selected_task_count(), 4);
        assert!(home.tasks.enabledTasks.resonate_with_Ahab);
    }

    #[test]
    fn bounded_number_settings_cycle_at_their_contract_boundaries() {
        let mut home = HomeState::default();

        home.tasks.get_reward.set_get_prize = 2;
        home.cycle_number(FixedTaskId::GetReward);
        assert_eq!(home.tasks.get_reward.set_get_prize, 0);
        home.tasks.get_reward.set_get_prize = 0;
        home.toggle_detail(FixedTaskId::GetReward);
        assert_eq!(home.tasks.get_reward.set_get_prize, 1);

        home.tasks.buy_enkephalin.set_lunacy_to_enkephalin = 10;
        home.cycle_number(FixedTaskId::BuyEnkephalin);
        assert_eq!(home.tasks.buy_enkephalin.set_lunacy_to_enkephalin, 0);

        home.tasks.mirror.set_mirror_count = 99;
        home.cycle_number(FixedTaskId::Mirror);
        assert_eq!(home.tasks.mirror.set_mirror_count, 1);
        home.tasks.mirror.set_mirror_count = 1;
        home.cycle_number(FixedTaskId::Mirror);
        assert_eq!(home.tasks.mirror.set_mirror_count, 2);

        home.tasks.daily_task.set_EXP_count = 0;
        home.adjust_daily_counter(DailyCounter::Exp, -1);
        assert_eq!(home.tasks.daily_task.set_EXP_count, 0);
        home.adjust_daily_counter(DailyCounter::Exp, 99);
        assert_eq!(home.tasks.daily_task.set_EXP_count, 99);
        home.tasks.daily_task.use_continuous_combat_select = 10;
        home.adjust_daily_counter(DailyCounter::Continuous, 1);
        assert_eq!(home.tasks.daily_task.use_continuous_combat_select, 10);
    }

    #[test]
    fn ahab_only_selection_cannot_start_execution() {
        let mut home = HomeState::default();
        home.set_all_tasks(false);
        home.tasks.enabledTasks.resonate_with_Ahab = true;
        home.start();
        assert_eq!(home.execution.state, ExecutionState::Idle);
        assert!(home.logs.back().unwrap().message.contains("没有选择任务"));
    }

    #[test]
    fn home_and_mock_start_disconnected_until_connect() {
        let mut home = HomeState::default();
        assert_eq!(home.device_status, ConnectionStatus::Disconnected);
        assert_eq!(home.rpc.take_events().len(), 0);
        home.select_device("pc:limbus".into());
        assert_eq!(home.device_status, ConnectionStatus::Connected);
    }

    #[test]
    fn expanded_task_settings_and_after_completion_are_persisted_through_config_rpc() {
        let mut home = HomeState::default();
        home.toggle_expanded(FixedTaskId::Mirror);
        assert!(home.is_expanded(FixedTaskId::Mirror));
        home.toggle_mirror_option(MirrorOption::Infinite);
        assert!(home.tasks.mirror.infinite_dungeons);
        home.toggle_after_exit_action(crate::model::AfterExitAction::ExitEmulator);
        home.set_after_power_action(crate::model::AfterPowerAction::Lock);
        home.set_keep_after_completion(false);
        assert!(
            home.tasks
                .afterCompletion
                .actions
                .contains(&crate::model::AfterExitAction::ExitEmulator)
        );
        assert_eq!(
            home.tasks.afterCompletion.powerAction,
            crate::model::AfterPowerAction::Lock
        );
        assert!(!home.tasks.afterCompletion.keepAfterCompletion);
    }

    #[test]
    fn home_selects_are_exclusive_and_close_when_a_dialog_opens() {
        let mut home = HomeState::default();
        home.toggle_select(HomeSelect::RewardMode);
        assert!(home.is_select_open(HomeSelect::RewardMode));
        home.toggle_select(HomeSelect::Device);
        assert!(!home.is_select_open(HomeSelect::RewardMode));
        assert!(home.is_select_open(HomeSelect::Device));
        home.toggle_select(HomeSelect::WindowPosition);
        assert!(!home.is_select_open(HomeSelect::Device));
        assert!(home.is_select_open(HomeSelect::WindowPosition));
        home.set_after_completion_open(true);
        assert!(home.after_completion_open);
        assert!(home.after_completion_draft.is_some());
        assert!(home.open_select.is_none());
    }

    #[test]
    fn device_error_tracking_and_dismissal() {
        let mut home = HomeState::default();
        assert!(home.device_error.is_none());
        home.device_error = Some("未找到设备".into());
        assert_eq!(home.device_error.as_deref(), Some("未找到设备"));
        home.dismiss_device_error();
        assert!(home.device_error.is_none());

        home.device_error = Some("连接超时".into());
        home.select_device("mumu:0".into());
        assert!(home.device_error.is_none());
        assert_eq!(home.device_status, ConnectionStatus::Connected);
    }

    #[test]
    fn device_kind_detection() {
        let pc = crate::model::DeviceInfo {
            id: "pc:limbus".into(),
            name: "Limbus Company".into(),
            detail: None,
        };
        assert_eq!(pc.kind(), crate::model::DeviceKind::PcWindow);

        let mumu = crate::model::DeviceInfo {
            id: "mumu:0".into(),
            name: "MuMu 模拟器".into(),
            detail: None,
        };
        assert_eq!(mumu.kind(), crate::model::DeviceKind::MumuEmulator);

        let adb = crate::model::DeviceInfo {
            id: "adb:127.0.0.1:5555".into(),
            name: "雷电模拟器".into(),
            detail: None,
        };
        assert_eq!(adb.kind(), crate::model::DeviceKind::AdbGeneric);
    }

    #[test]
    fn home_select_values_and_counters_stay_within_contract_bounds() {
        let mut home = HomeState::default();
        home.set_window_size(1440);
        home.set_window_size(999);
        assert_eq!(home.tasks.set_windows.set_win_size, 1440);
        home.set_window_position("2");
        assert_eq!(home.tasks.set_windows.set_win_position, "right_top");
        home.set_screenshot_interval(1.0);
        home.set_mouse_action_interval(0.1);
        assert_eq!(home.tasks.set_windows.screenshot_interval, 1.0);
        assert_eq!(home.tasks.set_windows.mouse_action_interval, 0.1);

        home.tasks.buy_enkephalin.set_lunacy_to_enkephalin = 10;
        home.adjust_enkephalin_count(1);
        assert_eq!(home.tasks.buy_enkephalin.set_lunacy_to_enkephalin, 10);
        home.adjust_enkephalin_count(-20);
        assert_eq!(home.tasks.buy_enkephalin.set_lunacy_to_enkephalin, 0);
        home.tasks.mirror.set_mirror_count = 1;
        home.adjust_mirror_count(-1);
        assert_eq!(home.tasks.mirror.set_mirror_count, 1);
        home.adjust_mirror_count(127);
        assert_eq!(home.tasks.mirror.set_mirror_count, 99);
    }

    #[test]
    fn after_completion_draft_applies_only_when_confirmed() {
        let mut home = HomeState::default();
        let original = home.tasks.afterCompletion.clone();
        home.set_after_completion_open(true);
        home.toggle_after_completion_draft(crate::model::AfterExitAction::ExitEmulator);
        home.set_after_completion_draft_power(crate::model::AfterPowerAction::Lock);
        assert_eq!(home.tasks.afterCompletion, original);
        home.apply_after_completion(false);
        assert!(!home.after_completion_open);
        assert!(!home.tasks.afterCompletion.keepAfterCompletion);
        assert_eq!(
            home.tasks.afterCompletion.powerAction,
            crate::model::AfterPowerAction::Lock
        );
        assert!(
            home.tasks
                .afterCompletion
                .actions
                .contains(&crate::model::AfterExitAction::ExitEmulator)
        );
    }
}
