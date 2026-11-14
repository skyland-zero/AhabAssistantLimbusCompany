//! TSX-facing console store: task selection, execution, devices and logs.

use nsh_core::prelude::*;

use crate::model::{ConnectionStatus, ExecutionState, FixedTaskId, PreviewStatus};

use super::HomeState;

#[derive(Clone, Debug, JsObject)]
pub struct LogView {
    pub ts: i64,
    pub level: String,
    pub message: String,
}

#[derive(Clone, Debug, JsObject)]
pub struct DeviceView {
    pub id: String,
    pub name: String,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, JsObject)]
pub struct WindowView {
    /// `"720"`, `"1080"` or `"1440"`: the `Select`'s value for the window's size.
    pub size: String,
    /// `"1080P"`, for the card's summary line.
    pub resolution: String,
    /// `center`, `left_top`, `right_top` or `free`.
    pub position: String,
    pub reduce_miscontact: bool,
    /// Seconds, one decimal (`"0.5"`), so the `Select` matches its option.
    pub screenshot_interval: String,
    pub mouse_interval: String,
    pub async_input: bool,
}

#[derive(Clone, Debug, JsObject)]
pub struct AfterCompletionView {
    /// `exit_game`, `exit_emulator`, `exit_aalc`, in the order they run.
    pub actions: Vec<String>,
    /// `none`, `sleep`, `hibernate`, `lock` or `shutdown`.
    pub power: String,
    /// Saved as the default, rather than chosen for this run only.
    pub keep: bool,
}

#[derive(Clone, Debug, JsObject)]
pub struct PreviewView {
    pub status: String,
    /// A `data:image/jpeg;base64,...` URL, or `null` before the first frame.
    pub frame: Option<String>,
    pub revision: u64,
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Debug, JsObject)]
pub struct StatsSummaryView {
    pub today_exp: u32,
    pub today_thread: u32,
    pub today_mirror: u32,
    pub week_exp: u32,
    pub week_thread: u32,
    pub week_mirror: u32,
    pub last_mirror: Option<String>,
    /// The last mirror run's time, for the console's "this time vs last time".
    pub last_mirror_seconds: Option<f64>,
    /// The last mirror run's floors, shown beside its time.
    pub last_mirror_floors: Option<u32>,
}

/// The current run's progress: the console card's running state.
#[derive(Clone, Debug, JsObject)]
pub struct RunProgressView {
    /// `idle`, `starting`, `running`, `paused`, `stopping` or `restoring`.
    pub state: String,
    /// `daily_task`, `get_reward`, `buy_enkephalin`, `mirror`, ...
    pub task: Option<String>,
    /// A task is running or paused: the card shows this run, not the totals.
    pub live: bool,
    pub exp_done: u32,
    pub exp_target: u32,
    pub thread_done: u32,
    pub thread_target: u32,
    pub mirror_done: u32,
    pub mirror_target: u32,
    /// The run's mirror target is unbounded (`targets.mirror` is zero).
    pub infinite: bool,
    pub hard: bool,
    /// The mirror run's floor (1-based) and the route's length, when it runs.
    pub floor: Option<u32>,
    pub floor_total: Option<u32>,
    /// Milliseconds since the run started.
    pub elapsed_ms: Option<u64>,
}

/// The console's backend state.
#[class]
pub struct HomeStore {
    state: HomeState,
}

#[class]
impl HomeStore {
    /// Loads the console state synchronously (mock/offline use).
    pub fn new() -> Self {
        Self {
            state: HomeState::from_shared(),
        }
    }

    /// Loads the console state off the engine thread.
    pub async fn connect() -> Result<HomeStore, String> {
        blocking(HomeStore::try_new)
            .await
            .map_err(|error| error.to_string())?
    }

    fn try_new() -> Result<Self, String> {
        let state = HomeState::from_shared();
        match state.error.clone() {
            Some(error) => Err(error),
            None => Ok(Self { state }),
        }
    }

    /// The five task cards, derived from the contract's task config.
    pub fn selection(&self) -> nsh_core::JsonValue {
        self.state.selection()
    }

    /// Applies the five task cards to the contract's task config.
    pub fn configure(&mut self, selection: nsh_core::JsonValue) -> Result<(), String> {
        self.state.configure(selection)
    }

    /// Applies one of the window card's settings (`size`, `position`,
    /// `reduceMiscontact`, `screenshotInterval`, `mouseInterval`, `asyncInput`)
    /// and returns the settings as they are now.
    pub fn set_window(
        &mut self,
        field: String,
        value: nsh_core::JsonValue,
    ) -> Result<WindowView, String> {
        self.state.set_window(&field, &value)?;
        Ok(self.window())
    }

    /// The "after the queue finishes" actions.
    pub fn after_completion(&self) -> AfterCompletionView {
        let view = self.state.after_completion();
        AfterCompletionView {
            actions: view
                .get("actions")
                .and_then(|value| value.as_array())
                .map(|actions| {
                    actions
                        .iter()
                        .filter_map(|action| action.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default(),
            power: view
                .get("power")
                .and_then(|value| value.as_str())
                .unwrap_or("none")
                .to_owned(),
            keep: view
                .get("keep")
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
        }
    }

    /// Replaces the after-run actions and returns them as they are now.
    pub fn set_after_completion(
        &mut self,
        actions: Vec<String>,
        power: String,
        keep: bool,
    ) -> Result<AfterCompletionView, String> {
        self.state.set_after_completion(actions, power, keep)?;
        Ok(self.after_completion())
    }

    /// The window settings shown on the console.
    pub fn window(&self) -> WindowView {
        let summary = self.state.window_summary();
        let text = |key: &str, fallback: &str| {
            summary
                .get(key)
                .and_then(|value| value.as_str())
                .unwrap_or(fallback)
                .to_owned()
        };
        let flag = |key: &str, fallback: bool| {
            summary
                .get(key)
                .and_then(|value| value.as_bool())
                .unwrap_or(fallback)
        };
        let seconds = |key: &str, fallback: f64| {
            format!(
                "{:.1}",
                summary
                    .get(key)
                    .and_then(|value| value.as_f64())
                    .unwrap_or(fallback)
            )
        };
        WindowView {
            size: summary
                .get("size")
                .and_then(|value| value.as_u64())
                .unwrap_or(1080)
                .to_string(),
            resolution: text("resolution", "1080P"),
            position: text("position", "free"),
            reduce_miscontact: flag("reduceMiscontact", true),
            screenshot_interval: seconds("screenshotInterval", 0.5),
            mouse_interval: seconds("mouseInterval", 0.3),
            async_input: flag("asyncInput", false),
        }
    }

    /// Starts the queue; the feedback line says whether it was accepted.
    pub fn start(&mut self) -> Result<bool, String> {
        self.state.start()
    }

    /// Pauses a running queue or resumes a paused one.
    pub fn pause_or_resume(&mut self) -> Result<(), String> {
        self.state.pause_or_resume()
    }

    /// Stops the running queue.
    pub fn stop(&mut self) -> Result<(), String> {
        self.state.stop()
    }

    /// `idle`, `starting`, `running`, `paused`, `stopping` or `restoring`.
    pub fn execution_state(&self) -> String {
        match self.state.execution.state {
            ExecutionState::Idle => "idle",
            ExecutionState::Starting => "starting",
            ExecutionState::Running => "running",
            ExecutionState::Paused => "paused",
            ExecutionState::Stopping => "stopping",
            ExecutionState::Restoring => "restoring",
        }
        .into()
    }

    /// The device list.
    pub fn devices(&self) -> Vec<DeviceView> {
        self.state
            .devices
            .iter()
            .map(|device| DeviceView {
                id: device.id.clone(),
                name: device.name.clone(),
                detail: device.detail.clone(),
            })
            .collect()
    }

    /// `disconnected`, `connecting` or `connected`.
    pub fn device_state(&self) -> String {
        match self.state.device_status {
            ConnectionStatus::Disconnected => "disconnected",
            ConnectionStatus::Connecting => "connecting",
            ConnectionStatus::Connected => "connected",
        }
        .into()
    }

    /// Re-reads the device list.
    pub fn refresh_devices(&mut self) -> Result<Vec<DeviceView>, String> {
        self.state.refresh_devices()?;
        Ok(self.devices())
    }

    /// Connects one device by id; an empty id disconnects.
    pub fn connect_device(&mut self, id: String) -> Result<String, String> {
        self.state.connect_device(&id)?;
        Ok(self.device_state())
    }

    /// Starts or stops the preview stream.
    pub fn set_preview_enabled(&mut self, enabled: bool) -> Result<bool, String> {
        self.state.set_preview_enabled(enabled)
    }

    /// Drains pending events and returns the latest preview frame and status.
    pub fn poll_preview(&mut self) -> PreviewView {
        let (status, frame) = self.state.poll_preview();
        PreviewView {
            status: match status {
                PreviewStatus::Stopped => "stopped",
                PreviewStatus::Starting => "starting",
                PreviewStatus::Running => "running",
                PreviewStatus::Error => "error",
            }
            .into(),
            frame: frame.map(|frame| format!("data:image/jpeg;base64,{}", base64(&frame.jpeg))),
            revision: frame.map(|frame| frame.revision).unwrap_or(0),
            width: frame.map(|frame| frame.width).unwrap_or(0),
            height: frame.map(|frame| frame.height).unwrap_or(0),
        }
    }

    /// The run log, oldest first.
    pub fn logs(&self) -> Vec<LogView> {
        self.state
            .logs
            .iter()
            .map(|entry| LogView {
                ts: entry.ts,
                level: match entry.level {
                    crate::model::LogLevel::Debug => "debug",
                    crate::model::LogLevel::Info => "info",
                    crate::model::LogLevel::Warn => "warn",
                    crate::model::LogLevel::Error => "error",
                }
                .into(),
                message: entry.message.clone(),
            })
            .collect()
    }

    /// Empties the run log.
    pub fn clear_logs(&mut self) {
        self.state.clear_logs();
    }

    /// The full statistics payload, including the mirror history.
    pub fn stats_payload(&self) -> nsh_core::JsonValue {
        self.state.stats_payload()
    }

    /// Drains events, then the statistics payload (the stats page's poll).
    pub fn poll_stats(&mut self) -> nsh_core::JsonValue {
        self.state.poll_stats()
    }

    /// Loads the daily statistics for the detail dialog.
    pub fn daily_stats(&mut self) -> Result<nsh_core::JsonValue, String> {
        let stats = self.state.load_daily_stats()?;
        serde_json::to_value(stats).map_err(|error| error.to_string())
    }

    /// The statistics summary shown on the console.
    pub fn stats(&self) -> StatsSummaryView {
        let stats = &self.state.stats;
        StatsSummaryView {
            today_exp: stats.today.exp,
            today_thread: stats.today.thread,
            today_mirror: stats.today.mirror,
            week_exp: stats.week.exp,
            week_thread: stats.week.thread,
            week_mirror: stats.week.mirror,
            last_mirror: stats
                .lastMirror
                .as_ref()
                .map(|mirror| mirror.completedAt.clone()),
            last_mirror_seconds: stats.lastMirror.as_ref().map(|mirror| mirror.totalSeconds),
            last_mirror_floors: stats.lastMirror.as_ref().map(|mirror| mirror.floorCount),
        }
    }

    /// The current run's counts, floor and elapsed time: the console card's
    /// running state. Empty while nothing runs, so the card shows the totals.
    pub fn run_progress(&self) -> RunProgressView {
        let execution = &self.state.execution;
        let run = &self.state.stats.currentRun;
        let task = execution.currentTaskId.map(task_id_name);
        let live = matches!(
            execution.state,
            ExecutionState::Running | ExecutionState::Paused
        ) && task.is_some();
        let mirror = task.as_deref() == Some("mirror");
        RunProgressView {
            state: self.execution_state(),
            task,
            live,
            exp_done: run.completed.exp,
            exp_target: run.targets.exp,
            thread_done: run.completed.thread,
            thread_target: run.targets.thread,
            mirror_done: run.completed.mirror,
            mirror_target: run.targets.mirror,
            infinite: run.isMirrorInfinite,
            hard: self
                .state
                .mirror_progress
                .as_ref()
                .map(|progress| progress.isHard)
                .unwrap_or(self.state.tasks.mirror.hard_mirror),
            floor: if mirror && live {
                self.state.mirror_floor.as_ref().map(|floor| floor.floor)
            } else {
                None
            },
            floor_total: if mirror && live {
                self.state
                    .mirror_floor
                    .as_ref()
                    .map(|floor| floor.floorTotal)
            } else {
                None
            },
            elapsed_ms: if live { self.state.run_elapsed_ms() } else { None },
        }
    }

    /// The connected device's name, for the running card's status line.
    pub fn selected_device_name(&self) -> Option<String> {
        let id = self.state.selected_device.as_ref()?;
        self.state
            .devices
            .iter()
            .find(|device| &device.id == id)
            .map(|device| device.name.clone())
    }

    /// The last state-layer feedback line.
    pub fn feedback(&self) -> Option<String> {
        self.state.feedback.clone()
    }

    /// Writes a feedback line (for actions that stay in TSX).
    pub fn set_feedback(&mut self, message: String) {
        self.state.set_feedback(message);
    }

    /// Clears the feedback line.
    pub fn clear_feedback(&mut self) {
        self.state.clear_feedback();
    }
}

impl Default for HomeStore {
    fn default() -> Self {
        Self::new()
    }
}

fn task_id_name(task: FixedTaskId) -> String {
    match task {
        FixedTaskId::SetWindows => "set_windows",
        FixedTaskId::DailyTask => "daily_task",
        FixedTaskId::GetReward => "get_reward",
        FixedTaskId::BuyEnkephalin => "buy_enkephalin",
        FixedTaskId::Mirror => "mirror",
        FixedTaskId::ResonateWithAhab => "resonate_with_Ahab",
    }
    .to_owned()
}

const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let a = chunk[0];
        let b = chunk.get(1).copied().unwrap_or(0);
        let c = chunk.get(2).copied().unwrap_or(0);
        out.push(BASE64[(a >> 2) as usize] as char);
        out.push(BASE64[(((a & 0b11) << 4) | (b >> 4)) as usize] as char);
        out.push(if chunk.len() > 1 {
            BASE64[(((b & 0b1111) << 2) | (c >> 6)) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            BASE64[(c & 0b111111) as usize] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::BackendClient;
    use crate::model::FixedTaskId;
    use serde_json::json;

    fn store() -> HomeStore {
        let state = HomeState::new(BackendClient::mock());
        assert!(state.error.is_none(), "mock must load: {:?}", state.error);
        HomeStore { state }
    }

    /// A queue of two EXP runs, four thread runs and three mirrors.
    fn queue(store: &mut HomeStore) {
        store
            .state
            .configure(json!({
                "daily": {"enabled": true, "exp": "2", "thread": "4"},
                "rewards": {"enabled": false},
                "enkephalin": {"enabled": false},
                "mirror": {"enabled": true, "runs": "3", "difficulty": "normal"},
                "resonance": {"enabled": false},
            }))
            .unwrap();
    }

    #[test]
    fn an_idle_console_reports_no_run() {
        let store = store();
        let progress = store.run_progress();
        assert!(!progress.live);
        assert_eq!(progress.task, None);
        assert_eq!(progress.floor, None);
        assert_eq!(progress.elapsed_ms, None);
    }

    #[test]
    fn a_daily_run_reports_its_counts_and_no_floor() {
        let mut store = store();
        queue(&mut store);
        store.state.start().unwrap();
        store.state.poll_preview();

        let progress = store.run_progress();
        assert!(progress.live);
        assert_eq!(progress.task.as_deref(), Some("daily_task"));
        assert_eq!(progress.exp_done, 0);
        assert_eq!(progress.exp_target, 2);
        assert_eq!(progress.thread_target, 4);
        assert_eq!(progress.mirror_target, 3);
        // The mirror floor belongs to the mirror task only.
        assert_eq!(progress.floor, None);
        assert!(progress.elapsed_ms.is_some(), "the run is stamped");
    }

    #[test]
    fn a_mirror_run_reports_its_floor_until_the_run_ends() {
        let mut store = store();
        queue(&mut store);
        store.state.start().unwrap();
        store.state.poll_preview();
        store.state.execution.currentTaskId = Some(FixedTaskId::Mirror);

        let progress = store.run_progress();
        assert_eq!(progress.task.as_deref(), Some("mirror"));
        assert_eq!(progress.floor, Some(1));
        assert_eq!(progress.floor_total, Some(5));
        assert!(!progress.hard);

        store.state.stop().unwrap();
        store.state.poll_preview();
        let progress = store.run_progress();
        assert!(!progress.live, "a finished run goes back to the totals");
        assert_eq!(progress.floor, None);
    }

    #[test]
    fn base64_matches_the_standard_padding() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(&[0xff, 0xd8, 0xff, 0xd9]), "/9j/2Q==");
    }
}
