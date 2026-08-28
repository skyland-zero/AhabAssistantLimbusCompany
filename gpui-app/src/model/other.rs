#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ToolStatusPayload {
    pub toolId: ToolId,
    pub running: bool,
}
#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolId {
    InfiniteBattle,
    Enkephalin,
    Screenshot,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ThemePack {
    pub id: String,
    pub name: String,
    pub weight: u8,
    pub enabled: bool,
    pub tier: String,
}
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ThemePackState {
    pub hardMirrorActive: bool,
    pub packs: Vec<ThemePack>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ResourceGroup {
    pub id: String,
    pub name: String,
    pub localVersion: String,
    pub remoteVersion: Option<String>,
    pub lastSyncAt: Option<i64>,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SyncProgressPayload {
    pub scope: String,
    pub progress: u8,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct LogEntryPayload {
    pub ts: i64,
    pub level: LogLevel,
    pub message: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[allow(non_snake_case)]
pub struct HotkeyConfig {
    pub startStop: String,
    pub pauseResume: Option<String>,
    pub enabled: bool,
}
impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            startStop: "F10".into(),
            pauseResume: Some("F11".into()),
            enabled: true,
        }
    }
}
impl HotkeyConfig {
    pub fn mock_default() -> Self {
        Self::default()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum UpdateSource {
    GitHub,
    MirrorChyan,
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[allow(non_snake_case)]
pub struct SystemSettingsConfig {
    pub simulator: bool,
    pub simulator_type: u8,
    pub simulator_port: u16,
    pub start_emulator_timeout: u16,
    pub memory_protection: bool,
    pub minimize_to_tray: bool,
    pub autostart: bool,
    pub experimental_keep_screen_awake: bool,
    pub experimental_hdr_warning: bool,
    pub update_prerelease_enable: bool,
    pub update_source: UpdateSource,
    pub mirrorchyan_cdk: String,
}
impl Default for SystemSettingsConfig {
    fn default() -> Self {
        Self {
            simulator: true,
            simulator_type: 0,
            simulator_port: 16384,
            start_emulator_timeout: 60,
            memory_protection: true,
            minimize_to_tray: true,
            autostart: false,
            experimental_keep_screen_awake: true,
            experimental_hdr_warning: true,
            update_prerelease_enable: false,
            update_source: UpdateSource::GitHub,
            mirrorchyan_cdk: String::new(),
        }
    }
}
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct UpdateInfo {
    pub updateAvailable: bool,
    pub latest: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceKind {
    PcWindow,
    MumuEmulator,
    AdbGeneric,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeviceInfo {
    pub id: String,
    pub name: String,
    pub detail: Option<String>,
}

impl DeviceInfo {
    pub fn kind(&self) -> DeviceKind {
        if self.id.starts_with("pc:") {
            DeviceKind::PcWindow
        } else if self.id.starts_with("mumu:") {
            DeviceKind::MumuEmulator
        } else {
            DeviceKind::AdbGeneric
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Connected,
}
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeviceStatusPayload {
    pub deviceId: Option<String>,
    pub status: ConnectionStatus,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ScreenshotFrame {
    pub instanceId: String,
    pub jpeg: Vec<u8>,
    pub width: u32,
    pub height: u32,
}
#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AppNotice {
    pub level: NoticeLevel,
    pub message: String,
}
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum NoticeLevel {
    Info,
    Warn,
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn settings_defaults_match_mock() {
        let settings = SystemSettingsConfig::default();
        assert!(settings.simulator && settings.memory_protection);
        assert_eq!(settings.simulator_port, 16384);
        assert_eq!(
            serde_json::to_string(&UpdateSource::GitHub).unwrap(),
            "\"GitHub\""
        );
    }
    #[test]
    fn event_payload_round_trips_json() {
        let payload = DeviceStatusPayload {
            deviceId: Some("device".into()),
            status: ConnectionStatus::Connected,
        };
        let json = serde_json::to_string(&payload).unwrap();
        assert_eq!(
            serde_json::from_str::<DeviceStatusPayload>(&json).unwrap(),
            payload
        );
    }
}
