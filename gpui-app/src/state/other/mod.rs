mod resources;
mod settings;
mod theme_packs;
mod toolbox;

use std::{collections::HashMap, time::Instant};

use serde_json::json;

use crate::{
    ipc::{
        EventEnvelope, MockClient, RpcGateway,
        contract::{event, method},
    },
    model::{
        HotkeyConfig, ResourceGroup, SystemSettingsConfig, ThemePack, ThemePackState, ToolId,
        ToolStatusPayload,
    },
};

pub struct ThemePacksState {
    pub rpc: RpcGateway,
    pub data: ThemePackState,
    pub sort_by_weight: bool,
    pub feedback: Option<String>,
    pub(crate) confirmed_packs: Vec<ThemePack>,
    pub(crate) persist_due: Option<Instant>,
}

pub struct ToolboxState {
    pub rpc: RpcGateway,
    pub running: HashMap<ToolId, bool>,
    pub feedback: Option<String>,
}

pub struct ResourcesState {
    pub rpc: RpcGateway,
    pub groups: Vec<ResourceGroup>,
    pub sync_progress: Option<u8>,
    pub sync_finish_scheduled: bool,
    pub feedback: Option<String>,
}

pub struct SettingsPageState {
    pub rpc: RpcGateway,
    pub hotkey: HotkeyConfig,
    pub system: SystemSettingsConfig,
    pub capturing: Option<HotkeyTarget>,
    pub open_select: Option<SettingsSelect>,
    pub feedback: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsSelect {
    UpdateSource,
    SimulatorType,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HotkeyTarget {
    StartStop,
    PauseResume,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SystemBool {
    Simulator,
    MemoryProtection,
    MinimizeToTray,
    Autostart,
    KeepScreenAwake,
    HdrWarning,
    Prerelease,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SystemU16 {
    SimulatorType,
    SimulatorPort,
    StartTimeout,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_pack_weight_and_batch_updates_are_deterministic() {
        let mut state = ThemePacksState::default();
        assert_eq!(state.total_weight(), 28);
        state.cycle_weight("pk-1");
        assert_eq!(
            state
                .data
                .packs
                .iter()
                .find(|pack| pack.id == "pk-1")
                .unwrap()
                .weight,
            6
        );
        state.adjust_weight("pk-1", -2);
        assert_eq!(
            state
                .data
                .packs
                .iter()
                .find(|pack| pack.id == "pk-1")
                .unwrap()
                .weight,
            4
        );
        state.adjust_weight("pk-1", 99);
        assert_eq!(
            state
                .data
                .packs
                .iter()
                .find(|pack| pack.id == "pk-1")
                .unwrap()
                .weight,
            10
        );
        state.set_all_enabled(false);
        assert_eq!(state.total_weight(), 0);
    }

    #[test]
    fn toolbox_events_update_running_state() {
        let mut state = ToolboxState::default();
        assert!(!state.is_running(ToolId::InfiniteBattle));
        state.toggle(ToolId::InfiniteBattle);
        assert!(state.is_running(ToolId::InfiniteBattle));
        state.toggle(ToolId::InfiniteBattle);
        assert!(!state.is_running(ToolId::InfiniteBattle));
    }

    #[test]
    fn resources_check_and_sync_share_canonical_names() {
        let mut state = ResourcesState::default();
        state.check_update();
        assert!(
            state
                .groups
                .iter()
                .all(|group| group.remoteVersion.is_some())
        );
        state.sync_now();
        assert!(state.groups.iter().all(|group| group.lastSyncAt == Some(0)));
        assert_eq!(state.sync_progress, Some(100));
        state.finish_sync();
        assert!(state.sync_progress.is_none());
    }

    #[test]
    fn settings_changes_are_sent_to_mock_and_capture_ignores_empty_combo() {
        let mut state = SettingsPageState::default();
        state.capture(HotkeyTarget::StartStop);
        state.finish_capture(None);
        assert_eq!(state.hotkey.startStop, "F10");
        state.finish_capture(Some("Ctrl+F10".into()));
        // No target remains after the empty capture.
        assert_eq!(state.hotkey.startStop, "F10");
        state.capture(HotkeyTarget::StartStop);
        state.finish_capture(Some("Ctrl+F10".into()));
        assert_eq!(state.hotkey.startStop, "Ctrl+F10");
    }
}
