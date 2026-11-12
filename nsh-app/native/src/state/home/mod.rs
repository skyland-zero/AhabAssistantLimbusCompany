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
        ConnectionStatus, DeviceInfo, ExecutionState, ExecutionStatsPayload,
        ExecutionStatusPayload, LogEntryPayload, TasksConfig,
    },
};

const MAX_LOGS: usize = 300;

pub struct HomeState {
    rpc: RpcGateway,
    pub tasks: TasksConfig,
    pub execution: ExecutionStatusPayload,
    pub stats: ExecutionStatsPayload,
    pub logs: VecDeque<LogEntryPayload>,
    pub devices: Vec<DeviceInfo>,
    pub selected_device: Option<String>,
    pub device_status: ConnectionStatus,
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
        self.tasks.daily_task.set_EXP_count = number(daily, "exp", 1);
        self.tasks.daily_task.set_thread_count = number(daily, "thread", 3);
        self.tasks.daily_task.use_continuous_combat = text(daily, "chain", "off") == "on";

        let rewards = selection.get("rewards");
        self.tasks.enabledTasks.get_reward = enabled(rewards);
        self.tasks.get_reward.set_get_prize = match text(rewards, "mode", "all").as_str() {
            "daily" => 1,
            "weekly" => 2,
            _ => 0,
        };

        let enkephalin = selection.get("enkephalin");
        self.tasks.enabledTasks.buy_enkephalin = enabled(enkephalin);
        self.tasks.buy_enkephalin.set_lunacy_to_enkephalin = number(enkephalin, "times", 2);
        self.tasks.buy_enkephalin.Dr_Grandet_mode = text(enkephalin, "grandet", "off") == "on";

        let mirror = selection.get("mirror");
        self.tasks.enabledTasks.mirror = enabled(mirror);
        self.tasks.mirror.set_mirror_count = number(mirror, "runs", 1);
        self.tasks.mirror.hard_mirror = text(mirror, "difficulty", "normal") == "hard";

        let resonance = selection.get("resonance");
        self.tasks.enabledTasks.resonate_with_Ahab = enabled(resonance);
        self.tasks.resonate_with_Ahab.enabled = text(resonance, "voice", "on") == "on";

        let value = serde_json::to_value(&self.tasks).map_err(|error| error.to_string())?;
        self.rpc
            .request_value(method::TASKS_SET_CONFIG, Some(value))
            .map_err(|error| error.message)?;
        Ok(())
    }

    /// The console's five cards, derived from the contract's task config.
    pub fn selection(&self) -> Value {
        json!({
            "daily": {
                "enabled": self.tasks.enabledTasks.daily_task,
                "exp": self.tasks.daily_task.set_EXP_count.to_string(),
                "thread": self.tasks.daily_task.set_thread_count.to_string(),
                "chain": if self.tasks.daily_task.use_continuous_combat { "on" } else { "off" },
            },
            "rewards": {
                "enabled": self.tasks.enabledTasks.get_reward,
                "mode": match self.tasks.get_reward.set_get_prize {
                    1 => "daily",
                    2 => "weekly",
                    _ => "all",
                },
            },
            "enkephalin": {
                "enabled": self.tasks.enabledTasks.buy_enkephalin,
                "times": self.tasks.buy_enkephalin.set_lunacy_to_enkephalin.to_string(),
                "grandet": if self.tasks.buy_enkephalin.Dr_Grandet_mode { "on" } else { "off" },
            },
            "mirror": {
                "enabled": self.tasks.enabledTasks.mirror,
                "runs": self.tasks.mirror.set_mirror_count.to_string(),
                "difficulty": if self.tasks.mirror.hard_mirror { "hard" } else { "normal" },
            },
            "resonance": {
                "enabled": self.tasks.enabledTasks.resonate_with_Ahab,
                "voice": if self.tasks.resonate_with_Ahab.enabled { "on" } else { "off" },
            },
        })
    }

    /// Window settings shown on the console.
    pub fn window_summary(&self) -> Value {
        json!({
            "resolution": format!("{}P", self.tasks.set_windows.set_win_size),
            "asyncInput": self.tasks.set_windows.use_post_message,
            "position": self.tasks.set_windows.set_win_position,
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
        self.devices = self
            .rpc
            .request(method::DEVICE_LIST, None)
            .map_err(|error| error.message)?;
        self.feedback = Some("已刷新设备列表".to_owned());
        Ok(())
    }

    pub fn connect_device(&mut self, device_id: &str) -> Result<(), String> {
        if device_id.is_empty() {
            self.selected_device = None;
            self.device_status = ConnectionStatus::Disconnected;
            return Ok(());
        }
        self.device_status = ConnectionStatus::Connecting;
        let response = self
            .rpc
            .request_value(method::DEVICE_CONNECT, Some(json!({ "id": device_id })))
            .map_err(|error| error.message)?;
        self.selected_device = response
            .as_ref()
            .and_then(|value| value.get("deviceId"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| Some(device_id.to_owned()));
        self.device_status = ConnectionStatus::Connected;
        self.feedback = Some("设备已连接".to_owned());
        Ok(())
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
                        self.execution = status;
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
                _ => {}
            }
        }
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

fn number(value: Option<&Value>, key: &str, fallback: u8) -> u8 {
    value
        .and_then(|value| value.get(key))
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<u8>().ok())
        .unwrap_or(fallback)
}

#[cfg(test)]
mod tests;
