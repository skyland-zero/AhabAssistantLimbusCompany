use std::collections::{HashSet, VecDeque};

use serde_json::json;

use crate::{
    ipc::{EventEnvelope, MockClient},
    model::{
        AfterExitAction, AfterPowerAction, ConnectionStatus, DeviceInfo, DeviceStatusPayload,
        ExecutionState, ExecutionStatusPayload, FixedTaskId, MirrorProgressPayload,
        ScreenshotFrame, TasksConfig,
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
}

pub struct HomeState {
    pub client: MockClient,
    pub tasks: TasksConfig,
    pub execution: ExecutionStatusPayload,
    pub logs: VecDeque<String>,
    pub devices: Vec<DeviceInfo>,
    pub selected_device: Option<String>,
    pub device_status: ConnectionStatus,
    pub right_panel_width: f32,
    pub right_panel_collapsed: bool,
    pub expanded_tasks: HashSet<FixedTaskId>,
    pub mirror_progress: Option<MirrorProgressPayload>,
    pub latest_screenshot: Option<ScreenshotFrame>,
    pub after_completion_open: bool,
}

impl Default for HomeState {
    fn default() -> Self {
        Self::with_layout(280, false)
    }
}

impl HomeState {
    pub fn with_layout(right_panel_width: u32, right_panel_collapsed: bool) -> Self {
        let mut client = MockClient::default();
        let tasks = client
            .call(crate::ipc::contract::method::TASKS_GET_CONFIG, None)
            .result
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default();
        let devices = client
            .call(crate::ipc::contract::method::DEVICE_LIST, None)
            .result
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default();

        Self {
            client,
            tasks,
            execution: ExecutionStatusPayload::default(),
            logs: VecDeque::from([
                "[12:00:01] GPUI 原生 Home 已启动".to_owned(),
                "[12:00:02] 等待本地自动化服务".to_owned(),
            ]),
            devices,
            selected_device: None,
            device_status: ConnectionStatus::Disconnected,
            right_panel_width: right_panel_width as f32,
            right_panel_collapsed,
            expanded_tasks: HashSet::new(),
            mirror_progress: None,
            latest_screenshot: None,
            after_completion_open: false,
        }
    }

    pub fn is_busy(&self) -> bool {
        !matches!(self.execution.state, ExecutionState::Idle)
    }

    pub fn selected_task_count(&self) -> usize {
        let enabled = &self.tasks.enabledTasks;
        [
            enabled.daily_task,
            enabled.get_reward,
            enabled.buy_enkephalin,
            enabled.mirror,
        ]
        .into_iter()
        .filter(|selected| *selected)
        .count()
    }

    pub fn is_expanded(&self, task: FixedTaskId) -> bool {
        self.expanded_tasks.contains(&task)
    }

    pub fn toggle_expanded(&mut self, task: FixedTaskId) {
        if !self.expanded_tasks.insert(task) {
            self.expanded_tasks.remove(&task);
        }
    }

    /// Apply one complete task-config mutation and send the canonical config
    /// through IPC. This is the seam used by all Home controls, so no control
    /// can accidentally update the UI without updating the backend.
    pub fn update_tasks(&mut self, update: impl FnOnce(&mut TasksConfig)) {
        if self.is_busy() {
            return;
        }
        update(&mut self.tasks);
        self.save_tasks();
    }

    pub fn set_after_completion_open(&mut self, open: bool) {
        self.after_completion_open = open;
    }

    pub fn toggle_mirror_option(&mut self, field: MirrorOption) {
        self.update_tasks(|tasks| match field {
            MirrorOption::Infinite => {
                tasks.mirror.infinite_dungeons = !tasks.mirror.infinite_dungeons
            }
            MirrorOption::NoWeeklyBonuses => {
                tasks.mirror.no_weekly_bonuses = !tasks.mirror.no_weekly_bonuses
            }
            MirrorOption::FloorThreeExit => tasks.mirror.floor_3_exit = !tasks.mirror.floor_3_exit,
            MirrorOption::SaveRewards => tasks.mirror.save_rewards = !tasks.mirror.save_rewards,
            MirrorOption::HardSingleBonuses => {
                tasks.mirror.hard_mirror_single_bonuses = !tasks.mirror.hard_mirror_single_bonuses
            }
            MirrorOption::SelectEventPack => {
                tasks.mirror.select_event_pack = !tasks.mirror.select_event_pack
            }
            MirrorOption::SkipEventPack => {
                tasks.mirror.skip_event_pack = !tasks.mirror.skip_event_pack
            }
            MirrorOption::ReclaimRewards => {
                tasks.mirror.re_claim_rewards = !tasks.mirror.re_claim_rewards
            }
            MirrorOption::NotSkipCotton => {
                tasks.mirror.not_skip_whitegossypium = !tasks.mirror.not_skip_whitegossypium
            }
            MirrorOption::FightToLast => {
                tasks.mirror.fight_to_last_man = !tasks.mirror.fight_to_last_man
            }
            MirrorOption::KeyboardNavigation => {
                tasks.mirror.mirror_keyboard_navigation = !tasks.mirror.mirror_keyboard_navigation
            }
            MirrorOption::SimplePathfinding => {
                tasks.mirror.mirror_keyboard_simple_pathfinding =
                    !tasks.mirror.mirror_keyboard_simple_pathfinding
            }
        });
    }

    pub fn toggle_after_exit_action(&mut self, action: AfterExitAction) {
        self.update_tasks(|tasks| {
            if let Some(index) = tasks
                .afterCompletion
                .actions
                .iter()
                .position(|item| *item == action)
            {
                tasks.afterCompletion.actions.remove(index);
            } else {
                tasks.afterCompletion.actions.push(action);
            }
        });
    }

    pub fn set_after_power_action(&mut self, action: AfterPowerAction) {
        self.update_tasks(|tasks| tasks.afterCompletion.powerAction = action);
    }

    pub fn set_keep_after_completion(&mut self, keep: bool) {
        self.update_tasks(|tasks| tasks.afterCompletion.keepAfterCompletion = keep);
    }

    pub fn set_all_tasks(&mut self, selected: bool) {
        if self.is_busy() {
            return;
        }
        let enabled = &mut self.tasks.enabledTasks;
        enabled.daily_task = selected;
        enabled.get_reward = selected;
        enabled.buy_enkephalin = selected;
        enabled.mirror = selected;
        self.save_tasks();
        self.log(if selected {
            "已全选可执行任务"
        } else {
            "已清空可执行任务"
        });
    }

    pub fn toggle_task(&mut self, task: FixedTaskId) {
        if self.is_busy() {
            return;
        }
        let enabled = &mut self.tasks.enabledTasks;
        match task {
            FixedTaskId::DailyTask => enabled.daily_task = !enabled.daily_task,
            FixedTaskId::GetReward => enabled.get_reward = !enabled.get_reward,
            FixedTaskId::BuyEnkephalin => enabled.buy_enkephalin = !enabled.buy_enkephalin,
            FixedTaskId::Mirror => enabled.mirror = !enabled.mirror,
            FixedTaskId::ResonateWithAhab => {
                enabled.resonate_with_Ahab = !enabled.resonate_with_Ahab
            }
            FixedTaskId::SetWindows => return,
        }
        self.save_tasks();
    }

    pub fn toggle_detail(&mut self, task: FixedTaskId) {
        if self.is_busy() {
            return;
        }
        match task {
            FixedTaskId::SetWindows => {
                self.tasks.set_windows.set_reduce_miscontact =
                    !self.tasks.set_windows.set_reduce_miscontact;
            }
            FixedTaskId::DailyTask => {
                self.tasks.daily_task.use_continuous_combat =
                    !self.tasks.daily_task.use_continuous_combat;
            }
            FixedTaskId::GetReward => {
                self.tasks.get_reward.set_get_prize =
                    cycle_inclusive(self.tasks.get_reward.set_get_prize, 0, 2);
            }
            FixedTaskId::BuyEnkephalin => {
                self.tasks.buy_enkephalin.Dr_Grandet_mode =
                    !self.tasks.buy_enkephalin.Dr_Grandet_mode;
            }
            FixedTaskId::Mirror => {
                self.tasks.mirror.hard_mirror = !self.tasks.mirror.hard_mirror;
            }
            // The nested field is retained for contract compatibility but is
            // not an execution toggle; the top-level enabledTasks field is
            // the only selectable Ahab task setting.
            FixedTaskId::ResonateWithAhab => return,
        }
        self.save_tasks();
    }

    pub fn cycle_number(&mut self, task: FixedTaskId) {
        if self.is_busy() {
            return;
        }
        match task {
            FixedTaskId::SetWindows => {
                self.tasks.set_windows.set_win_size = match self.tasks.set_windows.set_win_size {
                    720 => 1080,
                    1080 => 1440,
                    _ => 720,
                };
            }
            FixedTaskId::DailyTask => {
                self.tasks.daily_task.daily_teams = self.tasks.daily_task.daily_teams % 3 + 1;
            }
            FixedTaskId::GetReward => {
                self.tasks.get_reward.set_get_prize =
                    cycle_inclusive(self.tasks.get_reward.set_get_prize, 0, 2);
            }
            FixedTaskId::BuyEnkephalin => {
                self.tasks.buy_enkephalin.set_lunacy_to_enkephalin =
                    cycle_inclusive(self.tasks.buy_enkephalin.set_lunacy_to_enkephalin, 0, 10);
            }
            FixedTaskId::Mirror => {
                self.tasks.mirror.set_mirror_count =
                    cycle_inclusive(self.tasks.mirror.set_mirror_count, 1, 99);
            }
            FixedTaskId::ResonateWithAhab => return,
        }
        self.save_tasks();
    }

    pub fn start(&mut self) {
        if self.is_busy() {
            return;
        }
        if self.selected_task_count() == 0 {
            self.log("警告：没有选择任务，无法开始");
            return;
        }
        self.send(crate::ipc::contract::method::EXECUTION_START, None);
        self.log("任务已开始");
    }

    pub fn stop(&mut self) {
        if !self.is_busy() {
            return;
        }
        self.send(crate::ipc::contract::method::EXECUTION_STOP, None);
        self.log("任务已停止");
    }

    pub fn pause_or_resume(&mut self) {
        let method = match self.execution.state {
            ExecutionState::Running => crate::ipc::contract::method::EXECUTION_PAUSE,
            ExecutionState::Paused => crate::ipc::contract::method::EXECUTION_RESUME,
            ExecutionState::Idle => return,
        };
        self.send(method, None);
        self.log(if self.execution.state == ExecutionState::Paused {
            "任务已暂停"
        } else {
            "任务已继续"
        });
    }

    pub fn clear_logs(&mut self) {
        self.logs.clear();
    }

    pub fn select_device(&mut self, id: String) {
        self.send(
            crate::ipc::contract::method::DEVICE_CONNECT,
            Some(json!({ "id": id })),
        );
    }

    pub fn disconnect_device(&mut self) {
        self.send(crate::ipc::contract::method::DEVICE_DISCONNECT, None);
    }

    fn save_tasks(&mut self) {
        let value = serde_json::to_value(&self.tasks).expect("TasksConfig is serializable");
        self.send(crate::ipc::contract::method::TASKS_SET_CONFIG, Some(value));
    }

    fn send(&mut self, method: &str, params: Option<serde_json::Value>) {
        let response = self.client.call(method, params);
        if response.is_error() {
            self.log(&format!("IPC 错误：{}", response.error.unwrap().message));
            return;
        }
        let events = self.client.take_events();
        self.apply_events(events);
    }

    fn apply_events(&mut self, events: Vec<EventEnvelope>) {
        for event in events {
            match event.event.as_str() {
                crate::ipc::contract::event::EXECUTION_STATUS => {
                    if let Ok(status) = serde_json::from_value(event.payload) {
                        self.execution = status;
                        if self.execution.state == ExecutionState::Idle {
                            self.mirror_progress = None;
                        }
                    }
                }
                crate::ipc::contract::event::EXECUTION_MIRROR_PROGRESS => {
                    if let Ok(progress) = serde_json::from_value(event.payload) {
                        self.mirror_progress = Some(progress);
                    }
                }
                crate::ipc::contract::event::SCREENSHOT_FRAME => {
                    if let Ok(frame) = serde_json::from_value(event.payload) {
                        self.latest_screenshot = Some(frame);
                    }
                }
                crate::ipc::contract::event::DEVICE_STATUS => {
                    if let Ok(status) = serde_json::from_value::<DeviceStatusPayload>(event.payload)
                    {
                        self.selected_device = status.deviceId;
                        self.device_status = status.status;
                    }
                }
                crate::ipc::contract::event::LOG_ENTRY => {
                    if let Some(message) = event.payload.get("message").and_then(|v| v.as_str()) {
                        self.log(message);
                    }
                }
                _ => {}
            }
        }
    }

    fn log(&mut self, message: &str) {
        let second = self.logs.len() + 3;
        self.logs
            .push_back(format!("[12:00:{second:02}] {message}"));
        while self.logs.len() > 300 {
            self.logs.pop_front();
        }
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
        assert!(home.logs.back().unwrap().contains("没有选择任务"));
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
    }

    #[test]
    fn ahab_only_selection_cannot_start_execution() {
        let mut home = HomeState::default();
        home.set_all_tasks(false);
        home.tasks.enabledTasks.resonate_with_Ahab = true;
        home.start();
        assert_eq!(home.execution.state, ExecutionState::Idle);
        assert!(home.logs.back().unwrap().contains("没有选择任务"));
    }

    #[test]
    fn home_and_mock_start_disconnected_until_connect() {
        let mut home = HomeState::default();
        assert_eq!(home.device_status, ConnectionStatus::Disconnected);
        assert_eq!(home.client.take_events().len(), 0);
        home.select_device("win-limbus".into());
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
}
