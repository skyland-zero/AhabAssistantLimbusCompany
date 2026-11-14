//! The console's backend state: tasks, execution, devices, logs and stats.

mod bridge;

pub use bridge::HomeStore;

use std::collections::VecDeque;

use serde_json::{Value, json};

use crate::{
    ipc::{
        BackendClient, RpcGateway,
        contract::{event, method},
    },
    model::{
        AfterCompletionConfig, AfterExitAction, AfterPowerAction, ConnectionStatus,
        DailyStatsPayload, DeviceInfo, ExecutionState, ExecutionStatsPayload,
        ExecutionStatusPayload, LogEntryPayload, MirrorFloorPayload, MirrorProgressPayload,
        PreviewStatus, PreviewStatusPayload, TasksConfig,
    },
};

const MAX_LOGS: usize = 300;

/// A decoded preview frame, kept until the next one arrives.
#[derive(Clone, Debug)]
pub struct PreviewFrame {
    pub jpeg: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub revision: u64,
}

pub struct HomeState {
    rpc: RpcGateway,
    pub tasks: TasksConfig,
    pub execution: ExecutionStatusPayload,
    pub stats: ExecutionStatsPayload,
    pub logs: VecDeque<LogEntryPayload>,
    pub devices: Vec<DeviceInfo>,
    pub selected_device: Option<String>,
    pub device_status: ConnectionStatus,
    pub preview_status: PreviewStatus,
    pub preview_frame: Option<PreviewFrame>,
    pub preview_revision: u64,
    /// The current run's mirror counts, when the backend reports them.
    pub mirror_progress: Option<MirrorProgressPayload>,
    /// The floor the current run has reached, 1-based.
    pub mirror_floor: Option<MirrorFloorPayload>,
    pub feedback: Option<String>,
    pub error: Option<String>,
    next_client_request: u64,
}

impl HomeState {
    pub fn new(client: impl Into<BackendClient>) -> Self {
        let mut state = Self {
            rpc: RpcGateway::new(client),
            tasks: TasksConfig::default(),
            execution: ExecutionStatusPayload::default(),
            stats: ExecutionStatsPayload::default(),
            logs: VecDeque::new(),
            devices: Vec::new(),
            selected_device: None,
            device_status: ConnectionStatus::Disconnected,
            preview_status: PreviewStatus::Stopped,
            preview_frame: None,
            preview_revision: 0,
            mirror_progress: None,
            mirror_floor: None,
            feedback: None,
            error: None,
            next_client_request: 0,
        };
        if let Err(error) = state.refresh() {
            state.error = Some(error);
        }
        state
    }

    pub fn from_shared() -> Self {
        Self::new(crate::ipc::bridge::state_client())
    }

    pub fn refresh(&mut self) -> Result<(), String> {
        self.tasks = self
            .rpc
            .request(method::TASKS_GET_CONFIG, None)
            .map_err(|error| error.message)?;
        self.devices = self
            .rpc
            .request(method::DEVICE_LIST, None)
            .map_err(|error| error.message)?;
        self.execution = self
            .rpc
            .request(method::EXECUTION_GET_STATE, None)
            .map_err(|error| error.message)?;
        self.stats = self
            .rpc
            .request(method::STATS_GET_SUMMARY, None)
            .map_err(|error| error.message)?;
        self.error = None;
        Ok(())
    }

    /// Apply the console's five task cards to the contract's task config.
    pub fn configure(&mut self, selection: Value) -> Result<(), String> {
        let daily = selection.get("daily");
        self.tasks.enabledTasks.daily_task = enabled(daily);
        let config = &mut self.tasks.daily_task;
        config.set_EXP_count = number(daily, "exp", 1);
        config.set_thread_count = number(daily, "thread", 3);
        config.daily_teams = number(daily, "team", 1);
        config.use_continuous_combat = flag(daily, "chain", false);
        config.use_continuous_combat_select = number(daily, "chainCount", 1);
        config.targeted_teaming_EXP = flag(daily, "targetedExp", false);
        config.targeted_teaming_thread = flag(daily, "targetedThread", false);
        config.EXP_day_1_2 = number(daily, "expSlash", 1);
        config.EXP_day_3_4 = number(daily, "expPierce", 1);
        config.EXP_day_5_6 = number(daily, "expBlunt", 1);
        config.EXP_day_7 = number(daily, "expAll", 1);
        config.thread_day_1 = number(daily, "threadMon", 1);
        config.thread_day_2 = number(daily, "threadTue", 1);
        config.thread_day_3 = number(daily, "threadWed", 1);
        config.thread_day_4 = number(daily, "threadThu", 1);
        config.thread_day_5 = number(daily, "threadFri", 1);
        config.thread_day_6 = number(daily, "threadSat", 1);
        config.thread_day_7 = number(daily, "threadSun", 1);

        let rewards = selection.get("rewards");
        self.tasks.enabledTasks.get_reward = enabled(rewards);
        self.tasks.get_reward.set_get_prize = match text(rewards, "mode", "all").as_str() {
            "lunacy" => 1,
            "mail" => 2,
            _ => 0,
        };

        let enkephalin = selection.get("enkephalin");
        self.tasks.enabledTasks.buy_enkephalin = enabled(enkephalin);
        self.tasks.buy_enkephalin.set_lunacy_to_enkephalin = number(enkephalin, "times", 2);
        self.tasks.buy_enkephalin.Dr_Grandet_mode = flag(enkephalin, "grandet", false);
        self.tasks.buy_enkephalin.skip_enkephalin = flag(enkephalin, "skip", false);

        let mirror = selection.get("mirror");
        self.tasks.enabledTasks.mirror = enabled(mirror);
        let config = &mut self.tasks.mirror;
        config.set_mirror_count = number(mirror, "runs", 1);
        config.hard_mirror = text(mirror, "difficulty", "normal") == "hard";
        config.hard_mirror_target_floors = count(mirror, "hardTarget", 5);
        config.infinite_dungeons = flag(mirror, "infinite", false);
        config.no_weekly_bonuses = flag(mirror, "noWeekly", false);
        config.floor_3_exit = flag(mirror, "floorThree", false);
        config.save_rewards = flag(mirror, "saveRewards", false);
        config.hard_mirror_single_bonuses = flag(mirror, "singleBonus", false);
        config.select_event_pack = flag(mirror, "selectEvent", false);
        config.skip_event_pack = flag(mirror, "skipEvent", false);
        config.re_claim_rewards = flag(mirror, "reclaim", false);
        config.not_skip_whitegossypium = flag(mirror, "cotton", false);
        config.fight_to_last_man = flag(mirror, "lastMan", false);
        config.mirror_keyboard_navigation = flag(mirror, "keyboard", false);
        config.mirror_keyboard_simple_pathfinding = flag(mirror, "simpleKeys", false);
        config.mirror_prefer_hatred_and_despair = flag(mirror, "hatred", false);
        config.mirror_minimize_non_boss_combat = flag(mirror, "minimize", false);

        // The card's own switch is the whole of this task's configuration.
        let resonance = enabled(selection.get("resonance"));
        self.tasks.enabledTasks.resonate_with_Ahab = resonance;
        self.tasks.resonate_with_Ahab.enabled = resonance;

        let value = serde_json::to_value(&self.tasks).map_err(|error| error.to_string())?;
        self.rpc
            .request_value(method::TASKS_SET_CONFIG, Some(value))
            .map_err(|error| error.message)?;
        Ok(())
    }

    /// The console's five cards, derived from the contract's task config.
    pub fn selection(&self) -> Value {
        let daily = &self.tasks.daily_task;
        let enkephalin = &self.tasks.buy_enkephalin;
        let mirror = &self.tasks.mirror;
        json!({
            "daily": {
                "enabled": self.tasks.enabledTasks.daily_task,
                "exp": daily.set_EXP_count.to_string(),
                "thread": daily.set_thread_count.to_string(),
                "team": daily.daily_teams.to_string(),
                "chain": onoff(daily.use_continuous_combat),
                "chainCount": daily.use_continuous_combat_select.to_string(),
                "targetedExp": onoff(daily.targeted_teaming_EXP),
                "targetedThread": onoff(daily.targeted_teaming_thread),
                "expSlash": daily.EXP_day_1_2.to_string(),
                "expPierce": daily.EXP_day_3_4.to_string(),
                "expBlunt": daily.EXP_day_5_6.to_string(),
                "expAll": daily.EXP_day_7.to_string(),
                "threadMon": daily.thread_day_1.to_string(),
                "threadTue": daily.thread_day_2.to_string(),
                "threadWed": daily.thread_day_3.to_string(),
                "threadThu": daily.thread_day_4.to_string(),
                "threadFri": daily.thread_day_5.to_string(),
                "threadSat": daily.thread_day_6.to_string(),
                "threadSun": daily.thread_day_7.to_string(),
            },
            "rewards": {
                "enabled": self.tasks.enabledTasks.get_reward,
                "mode": match self.tasks.get_reward.set_get_prize {
                    1 => "lunacy",
                    2 => "mail",
                    _ => "all",
                },
            },
            "enkephalin": {
                "enabled": self.tasks.enabledTasks.buy_enkephalin,
                "times": enkephalin.set_lunacy_to_enkephalin.to_string(),
                "grandet": onoff(enkephalin.Dr_Grandet_mode),
                "skip": onoff(enkephalin.skip_enkephalin),
            },
            "mirror": {
                "enabled": self.tasks.enabledTasks.mirror,
                "runs": mirror.set_mirror_count.to_string(),
                "difficulty": if mirror.hard_mirror { "hard" } else { "normal" },
                "hardTarget": mirror.hard_mirror_target_floors.to_string(),
                "infinite": onoff(mirror.infinite_dungeons),
                "noWeekly": onoff(mirror.no_weekly_bonuses),
                "floorThree": onoff(mirror.floor_3_exit),
                "saveRewards": onoff(mirror.save_rewards),
                "singleBonus": onoff(mirror.hard_mirror_single_bonuses),
                "selectEvent": onoff(mirror.select_event_pack),
                "skipEvent": onoff(mirror.skip_event_pack),
                "reclaim": onoff(mirror.re_claim_rewards),
                "cotton": onoff(mirror.not_skip_whitegossypium),
                "lastMan": onoff(mirror.fight_to_last_man),
                "keyboard": onoff(mirror.mirror_keyboard_navigation),
                "simpleKeys": onoff(mirror.mirror_keyboard_simple_pathfinding),
                "hatred": onoff(mirror.mirror_prefer_hatred_and_despair),
                "minimize": onoff(mirror.mirror_minimize_non_boss_combat),
            },
            "resonance": {
                "enabled": self.tasks.enabledTasks.resonate_with_Ahab,
            },
        })
    }

    /// Applies one of the window card's settings and persists the tasks.
    /// `field` is `size`, `position`, `reduceMiscontact`, `screenshotInterval`,
    /// `mouseInterval` or `asyncInput`; anything else is rejected.
    pub fn set_window(&mut self, field: &str, value: &Value) -> Result<(), String> {
        let windows = &mut self.tasks.set_windows;
        match field {
            "size" => {
                // The card's `Select` hands its value over as text.
                let requested = value
                    .as_u64()
                    .or_else(|| value.as_str().and_then(|value| value.parse::<u64>().ok()));
                windows.set_win_size = match requested {
                    Some(720) => 720,
                    Some(1440) => 1440,
                    _ => 1080,
                }
            }
            "position" => {
                let position = value.as_str().unwrap_or("free");
                windows.set_win_position = match position {
                    "center" | "left_top" | "right_top" => position.to_owned(),
                    _ => "free".to_owned(),
                }
            }
            "reduceMiscontact" => windows.set_reduce_miscontact = value.as_bool().unwrap_or(true),
            "screenshotInterval" => {
                windows.screenshot_interval = interval(value, windows.screenshot_interval)?
            }
            "mouseInterval" => {
                windows.mouse_action_interval = interval(value, windows.mouse_action_interval)?
            }
            "asyncInput" => windows.use_post_message = value.as_bool().unwrap_or(false),
            other => return Err(format!("未知的窗口设置：{other}")),
        }
        self.save_tasks()
    }

    fn save_tasks(&mut self) -> Result<(), String> {
        let value = serde_json::to_value(&self.tasks).map_err(|error| error.to_string())?;
        self.rpc
            .request_value(method::TASKS_SET_CONFIG, Some(value))
            .map_err(|error| error.message)?;
        Ok(())
    }

    /// The "after the queue finishes" actions, as the console edits them.
    pub fn after_completion(&self) -> Value {
        let config = &self.tasks.afterCompletion;
        json!({
            "actions": config.actions.iter().map(|action| exit_key(*action)).collect::<Vec<_>>(),
            "power": power_key(config.powerAction),
            "keep": config.keepAfterCompletion,
        })
    }

    /// Replaces the after-run actions and persists the tasks. The actions are
    /// the console's keys; `keep` marks them as the saved default, and leaves
    /// them as a choice for this run only when it is false.
    pub fn set_after_completion(
        &mut self,
        actions: Vec<String>,
        power: String,
        keep: bool,
    ) -> Result<(), String> {
        let mut parsed = Vec::with_capacity(actions.len());
        for action in &actions {
            parsed.push(match action.as_str() {
                "exit_game" => AfterExitAction::ExitGame,
                "exit_emulator" => AfterExitAction::ExitEmulator,
                "exit_aalc" => AfterExitAction::ExitAalc,
                other => return Err(format!("未知的退出动作：{other}")),
            });
        }
        let power_action = match power.as_str() {
            "none" => AfterPowerAction::None,
            "sleep" => AfterPowerAction::Sleep,
            "hibernate" => AfterPowerAction::Hibernate,
            "lock" => AfterPowerAction::Lock,
            "shutdown" => AfterPowerAction::Shutdown,
            other => return Err(format!("未知的电源动作：{other}")),
        };
        self.tasks.afterCompletion = AfterCompletionConfig {
            actions: parsed,
            powerAction: power_action,
            keepAfterCompletion: keep,
        };
        self.save_tasks()
    }

    /// Window settings shown on the console.
    pub fn window_summary(&self) -> Value {
        json!({
            "size": self.tasks.set_windows.set_win_size,
            "resolution": format!("{}P", self.tasks.set_windows.set_win_size),
            "position": self.tasks.set_windows.set_win_position,
            "reduceMiscontact": self.tasks.set_windows.set_reduce_miscontact,
            "screenshotInterval": self.tasks.set_windows.screenshot_interval,
            "mouseInterval": self.tasks.set_windows.mouse_action_interval,
            "asyncInput": self.tasks.set_windows.use_post_message,
        })
    }

    pub fn selected_task_count(&self) -> usize {
        [
            self.tasks.enabledTasks.daily_task,
            self.tasks.enabledTasks.get_reward,
            self.tasks.enabledTasks.buy_enkephalin,
            self.tasks.enabledTasks.mirror,
            self.tasks.enabledTasks.resonate_with_Ahab,
        ]
        .into_iter()
        .filter(|enabled| *enabled)
        .count()
    }

    fn has_executable_task(&self) -> bool {
        self.tasks.enabledTasks.daily_task
            || self.tasks.enabledTasks.get_reward
            || self.tasks.enabledTasks.buy_enkephalin
            || self.tasks.enabledTasks.mirror
    }

    /// Start the queue. Returns whether the backend accepted it.
    pub fn start(&mut self) -> Result<bool, String> {
        if self.execution.state != ExecutionState::Idle {
            self.feedback = Some("任务正在运行，请先停止".to_owned());
            return Ok(false);
        }
        if !self.has_executable_task() {
            self.feedback = Some("没有选择可执行任务".to_owned());
            self.push_log("warn", "没有选择可执行任务");
            return Ok(false);
        }
        self.next_client_request = self.next_client_request.wrapping_add(1);
        let request_id = format!(
            "{}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_millis())
                .unwrap_or_default(),
            self.next_client_request
        );
        let response = self
            .rpc
            .request_value(
                method::EXECUTION_START,
                Some(json!({ "clientRequestId": request_id })),
            )
            .map_err(|error| error.message)?;
        self.drain_events();
        let accepted = response
            .as_ref()
            .and_then(|value| value.get("accepted"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        self.feedback = Some(if accepted {
            "已提交启动请求".to_owned()
        } else {
            response
                .as_ref()
                .and_then(|value| value.get("message"))
                .and_then(Value::as_str)
                .unwrap_or("启动请求未被接受")
                .to_owned()
        });
        Ok(accepted)
    }

    pub fn pause_or_resume(&mut self) -> Result<(), String> {
        let method_name = match self.execution.state {
            ExecutionState::Running => method::EXECUTION_PAUSE,
            ExecutionState::Paused => method::EXECUTION_RESUME,
            _ => return Ok(()),
        };
        self.rpc
            .request_value(method_name, Some(self.run_params()))
            .map_err(|error| error.message)?;
        self.drain_events();
        Ok(())
    }

    pub fn stop(&mut self) -> Result<(), String> {
        if self.execution.state == ExecutionState::Idle {
            return Ok(());
        }
        self.rpc
            .request_value(method::EXECUTION_STOP, Some(self.run_params()))
            .map_err(|error| error.message)?;
        self.drain_events();
        Ok(())
    }

    fn run_params(&self) -> Value {
        json!({ "runId": self.execution.runId })
    }

    pub fn refresh_devices(&mut self) -> Result<(), String> {
        let devices = self
            .rpc
            .request(method::DEVICE_LIST, None)
            .map_err(|error| error.message)?;
        self.apply_devices(devices);
        Ok(())
    }

    /// Replaces the cached device list after a scan performed off the engine
    /// thread.
    pub fn apply_devices(&mut self, devices: Vec<DeviceInfo>) {
        self.devices = devices;
        self.feedback = Some("已刷新设备列表".to_owned());
    }

    pub fn connect_device(&mut self, device_id: &str) -> Result<(), String> {
        if !self.begin_connect(device_id) {
            return Ok(());
        }
        let response = match self
            .rpc
            .request_value(method::DEVICE_CONNECT, Some(json!({ "id": device_id })))
        {
            Ok(response) => response,
            Err(error) => {
                self.fail_connect(&error.message);
                return Err(error.message);
            }
        };
        self.apply_connected(device_id, response.as_ref());
        Ok(())
    }

    /// Marks the selection as connecting. Returns whether a connect RPC is
    /// needed; an empty id only clears the selection.
    pub fn begin_connect(&mut self, device_id: &str) -> bool {
        if device_id.is_empty() {
            self.selected_device = None;
            self.device_status = ConnectionStatus::Disconnected;
            return false;
        }
        self.device_status = ConnectionStatus::Connecting;
        true
    }

    /// Applies a successful `device.connect` response after the RPC completes
    /// off the engine thread.
    pub fn apply_connected(&mut self, device_id: &str, response: Option<&Value>) {
        self.selected_device = response
            .and_then(|value| value.get("deviceId"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| Some(device_id.to_owned()));
        self.device_status = ConnectionStatus::Connected;
        self.feedback = Some("设备已连接".to_owned());
    }

    /// Clears the transient connecting state when `device.connect` fails.
    pub fn fail_connect(&mut self, message: &str) {
        self.device_status = ConnectionStatus::Disconnected;
        self.feedback = Some(message.to_owned());
    }

    /// Ask the sidecar to start or stop the preview stream.
    pub fn set_preview_enabled(&mut self, enabled: bool) -> Result<bool, String> {
        let response = self
            .rpc
            .request_value(
                method::PREVIEW_SET_ENABLED,
                Some(json!({ "enabled": enabled })),
            )
            .map_err(|error| error.message)?
            .unwrap_or_default();
        self.preview_status = if enabled {
            if response
                .get("running")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                PreviewStatus::Running
            } else {
                PreviewStatus::Starting
            }
        } else {
            PreviewStatus::Stopped
        };
        self.drain_events();
        Ok(self.preview_status == PreviewStatus::Running)
    }

    /// Drain the event queue and return the latest preview state.
    pub fn poll_preview(&mut self) -> (PreviewStatus, Option<&PreviewFrame>) {
        self.drain_events();
        (self.preview_status, self.preview_frame.as_ref())
    }

    /// The full statistics payload, including mirror history, for the detail
    /// dialogs.
    pub fn stats_payload(&self) -> Value {
        serde_json::to_value(&self.stats).unwrap_or(Value::Null)
    }

    /// Drains the event queue first, then returns the payload: a page that shows
    /// the mirror history sees a finished run without waiting for the console's
    /// poll.
    pub fn poll_stats(&mut self) -> Value {
        self.drain_events();
        self.stats_payload()
    }

    /// Load the daily statistics for the detail dialog.
    pub fn load_daily_stats(&mut self) -> Result<DailyStatsPayload, String> {
        self.rpc
            .request(method::STATS_GET_DAILY_SUMMARY, None)
            .map_err(|error| error.message)
    }

    pub fn clear_logs(&mut self) {
        self.logs.clear();
        self.feedback = Some("日志已清空".to_owned());
    }

    pub fn set_feedback(&mut self, message: impl Into<String>) {
        self.feedback = Some(message.into());
    }

    pub fn clear_feedback(&mut self) {
        self.feedback = None;
    }

    fn push_log(&mut self, level: &str, message: &str) {
        let level = match level {
            "warn" => crate::model::LogLevel::Warn,
            "error" => crate::model::LogLevel::Error,
            "debug" => crate::model::LogLevel::Debug,
            _ => crate::model::LogLevel::Info,
        };
        self.logs.push_back(LogEntryPayload {
            ts: 0,
            level,
            message: message.to_owned(),
            runId: None,
        });
        while self.logs.len() > MAX_LOGS {
            self.logs.pop_front();
        }
    }

    fn drain_events(&mut self) {
        for envelope in self.rpc.take_events() {
            match envelope.event.as_str() {
                event::LOG_ENTRY => {
                    if let Ok(entry) = serde_json::from_value::<LogEntryPayload>(envelope.payload) {
                        self.logs.push_back(entry);
                        while self.logs.len() > MAX_LOGS {
                            self.logs.pop_front();
                        }
                    }
                }
                event::EXECUTION_STATUS => {
                    if let Ok(status) =
                        serde_json::from_value::<ExecutionStatusPayload>(envelope.payload)
                    {
                        // A new run starts with no progress of its own; a
                        // finished one keeps no floor for the next task to
                        // inherit. The card reads these under the run id gate.
                        if matches!(
                            status.state,
                            ExecutionState::Idle | ExecutionState::Starting
                        ) {
                            self.mirror_progress = None;
                            self.mirror_floor = None;
                        }
                        self.execution = status;
                    }
                }
                event::EXECUTION_MIRROR_PROGRESS => {
                    if let Ok(progress) =
                        serde_json::from_value::<MirrorProgressPayload>(envelope.payload)
                        && self.same_run(progress.runId.as_deref())
                    {
                        self.mirror_progress = Some(progress);
                    }
                }
                event::EXECUTION_MIRROR_FLOOR => {
                    if let Ok(floor) =
                        serde_json::from_value::<MirrorFloorPayload>(envelope.payload)
                        && self.same_run(floor.runId.as_deref())
                    {
                        self.mirror_floor = Some(floor);
                    }
                }
                event::EXECUTION_STATS => {
                    if let Ok(stats) =
                        serde_json::from_value::<ExecutionStatsPayload>(envelope.payload)
                    {
                        self.stats = stats;
                    }
                }
                event::DEVICE_STATUS => {
                    if let Ok(status) = serde_json::from_value::<crate::model::DeviceStatusPayload>(
                        envelope.payload,
                    ) {
                        self.device_status = status.status;
                        if status.deviceId.is_some() {
                            self.selected_device = status.deviceId;
                        }
                    }
                }
                event::PREVIEW_STATUS => {
                    if let Ok(status) =
                        serde_json::from_value::<PreviewStatusPayload>(envelope.payload)
                    {
                        if let Some(error) = status.error {
                            self.feedback = Some(error);
                        }
                        self.preview_status = status.status;
                    }
                }
                event::SCREENSHOT_FRAME => {
                    if let Some(binary) = envelope.binary
                        && !binary.is_empty()
                    {
                        let width = envelope
                            .payload
                            .get("width")
                            .and_then(Value::as_u64)
                            .unwrap_or(0) as u32;
                        let height = envelope
                            .payload
                            .get("height")
                            .and_then(Value::as_u64)
                            .unwrap_or(0) as u32;
                        self.preview_revision = self.preview_revision.wrapping_add(1);
                        self.preview_frame = Some(PreviewFrame {
                            jpeg: binary,
                            width,
                            height,
                            revision: self.preview_revision,
                        });
                        self.preview_status = PreviewStatus::Running;
                    }
                }
                _ => {}
            }
        }
    }
}

impl HomeState {
    /// Whether a run-scoped event belongs to the run this console shows. A
    /// payload without a run id predates run-scoped events and is accepted.
    fn same_run(&self, run_id: Option<&str>) -> bool {
        match run_id {
            None => true,
            Some(id) => self.execution.runId.as_deref() == Some(id),
        }
    }

    /// Milliseconds since the current run started, when the backend stamped it.
    pub fn run_elapsed_ms(&self) -> Option<u64> {
        let started = self.stats.currentRun.startedAt?;
        if started <= 0 {
            return None;
        }
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_millis() as i64;
        Some((now - started).max(0) as u64)
    }
}

fn enabled(value: Option<&Value>) -> bool {
    value
        .and_then(|value| value.get("enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn text(value: Option<&Value>, key: &str, fallback: &str) -> String {
    value
        .and_then(|value| value.get(key))
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_owned()
}

/// A switch on a card, as `"on"` / `"off"`.
fn flag(value: Option<&Value>, key: &str, fallback: bool) -> bool {
    match value
        .and_then(|value| value.get(key))
        .and_then(Value::as_str)
    {
        Some("on") => true,
        Some("off") => false,
        _ => fallback,
    }
}

fn onoff(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}

fn exit_key(action: AfterExitAction) -> &'static str {
    match action {
        AfterExitAction::ExitGame => "exit_game",
        AfterExitAction::ExitEmulator => "exit_emulator",
        AfterExitAction::ExitAalc => "exit_aalc",
    }
}

fn power_key(action: AfterPowerAction) -> &'static str {
    match action {
        AfterPowerAction::None => "none",
        AfterPowerAction::Sleep => "sleep",
        AfterPowerAction::Hibernate => "hibernate",
        AfterPowerAction::Lock => "lock",
        AfterPowerAction::Shutdown => "shutdown",
    }
}

fn count(value: Option<&Value>, key: &str, fallback: u32) -> u32 {
    value
        .and_then(|value| value.get(key))
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<u32>().ok())
        .unwrap_or(fallback)
}

fn number(value: Option<&Value>, key: &str, fallback: u8) -> u8 {
    value
        .and_then(|value| value.get(key))
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(fallback)
}

/// A window interval in seconds; the sidecar rejects one it cannot use, so the
/// range is kept to what the console offers (0.05 to 5s).
fn interval(value: &Value, fallback: f32) -> Result<f32, String> {
    let seconds = value
        .as_f64()
        .or_else(|| value.as_str().and_then(|value| value.parse::<f64>().ok()))
        .ok_or_else(|| format!("不是有效的间隔：{value}"))? as f32;
    if !(0.05..=5.0).contains(&seconds) {
        return Err(format!("间隔超出范围（0.05~5s）：{seconds}"));
    }
    Ok(if seconds.is_finite() {
        seconds
    } else {
        fallback
    })
}

#[cfg(test)]
mod tests;
