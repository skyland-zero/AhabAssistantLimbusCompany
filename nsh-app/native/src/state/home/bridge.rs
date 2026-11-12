//! TSX-facing console store: task selection, execution, devices and logs.

use nsh_core::prelude::*;

use crate::model::{ConnectionStatus, ExecutionState, PreviewStatus};

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
    pub resolution: String,
    pub async_input: bool,
    pub position: String,
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

    /// Sets the game window's width (`1080` or `1440`) and returns the summary.
    pub fn set_window_size(&mut self, size: u32) -> Result<WindowView, String> {
        self.state.set_window_size(size)?;
        Ok(self.window())
    }

    /// Toggles asynchronous input and returns the window summary.
    pub fn toggle_async_input(&mut self) -> Result<WindowView, String> {
        self.state.toggle_async_input()?;
        Ok(self.window())
    }

    /// The window settings shown on the console.
    pub fn window(&self) -> WindowView {
        let summary = self.state.window_summary();
        WindowView {
            resolution: summary
                .get("resolution")
                .and_then(|value| value.as_str())
                .unwrap_or("1080P")
                .to_owned(),
            async_input: summary
                .get("asyncInput")
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
            position: summary
                .get("position")
                .and_then(|value| value.as_str())
                .unwrap_or("free")
                .to_owned(),
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
        }
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
    use super::base64;

    #[test]
    fn base64_matches_the_standard_padding() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(&[0xff, 0xd8, 0xff, 0xd9]), "/9j/2Q==");
    }
}
