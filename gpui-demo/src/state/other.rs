use std::collections::HashMap;

use serde_json::json;

use crate::{
    ipc::{
        EventEnvelope, MockClient,
        contract::{RpcResponse, event, method},
    },
    model::{
        HotkeyConfig, ResourceGroup, SystemSettingsConfig, ThemePack, ThemePackState, ToolId,
        ToolStatusPayload,
    },
};

pub struct ThemePacksState {
    pub client: MockClient,
    pub data: ThemePackState,
    pub sort_by_weight: bool,
    pub feedback: Option<String>,
}

impl Default for ThemePacksState {
    fn default() -> Self {
        Self::new()
    }
}

impl ThemePacksState {
    pub fn new() -> Self {
        let mut state = Self {
            client: MockClient::default(),
            data: ThemePackState::default(),
            sort_by_weight: false,
            feedback: None,
        };
        state.reload();
        state
    }

    pub fn reload(&mut self) {
        if let Some(value) = self.request(method::THEME_PACK_LIST, None) {
            if let Ok(data) = serde_json::from_value(value) {
                self.data = data;
            }
        }
    }

    pub fn sorted_packs(&self) -> Vec<ThemePack> {
        let mut packs = self.data.packs.clone();
        if self.sort_by_weight {
            packs
                .sort_by(|left, right| right.weight.cmp(&left.weight).then(left.id.cmp(&right.id)));
        }
        packs
    }

    pub fn total_weight(&self) -> u32 {
        self.data
            .packs
            .iter()
            .filter(|pack| pack.enabled)
            .map(|pack| u32::from(pack.weight))
            .sum()
    }

    pub fn set_sort_by_weight(&mut self, enabled: bool) {
        self.sort_by_weight = enabled;
    }

    pub fn set_all_enabled(&mut self, enabled: bool) {
        let mut packs = self.data.packs.clone();
        for pack in &mut packs {
            pack.enabled = enabled;
        }
        self.persist_packs(packs);
    }

    pub fn toggle_enabled(&mut self, id: &str) {
        if let Some(pack) = self.data.packs.iter().find(|pack| pack.id == id) {
            let enabled = !pack.enabled;
            self.patch_pack(id, |pack| pack.enabled = enabled);
        }
    }

    pub fn cycle_weight(&mut self, id: &str) {
        if let Some(pack) = self.data.packs.iter().find(|pack| pack.id == id) {
            if pack.enabled {
                let weight = if pack.weight >= 10 {
                    0
                } else {
                    pack.weight + 1
                };
                self.patch_pack(id, |pack| pack.weight = weight);
            }
        }
    }

    pub fn reset_weights(&mut self) {
        let response = self.client.call(method::THEME_PACK_RESET_WEIGHTS, None);
        if let Some(error) = response.error {
            self.feedback = Some(error.message);
        } else if let Some(value) = response.result {
            if let Ok(data) = serde_json::from_value(value) {
                self.data = data;
                self.feedback = Some("已恢复默认权重".to_owned());
            }
        }
    }

    fn patch_pack(&mut self, id: &str, patch: impl FnOnce(&mut ThemePack)) {
        let mut packs = self.data.packs.clone();
        if let Some(pack) = packs.iter_mut().find(|pack| pack.id == id) {
            patch(pack);
            self.persist_packs(packs);
        }
    }

    fn persist_packs(&mut self, packs: Vec<ThemePack>) {
        let response = self.client.call(
            method::THEME_PACK_UPDATE_ALL,
            Some(json!({ "packs": packs })),
        );
        if let Some(error) = response.error {
            self.feedback = Some(error.message);
            return;
        }
        self.data.packs = packs;
        self.feedback = Some("主题包设置已保存".to_owned());
    }

    fn request(
        &mut self,
        method_name: &str,
        params: Option<serde_json::Value>,
    ) -> Option<serde_json::Value> {
        let response = self.client.call(method_name, params);
        if let Some(error) = response.error {
            self.feedback = Some(error.message);
            None
        } else {
            response.result
        }
    }
}

pub struct ToolboxState {
    pub client: MockClient,
    pub running: HashMap<ToolId, bool>,
    pub feedback: Option<String>,
}

impl Default for ToolboxState {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolboxState {
    pub fn new() -> Self {
        Self {
            client: MockClient::default(),
            running: HashMap::new(),
            feedback: None,
        }
    }

    pub fn is_running(&self, tool: ToolId) -> bool {
        self.running.get(&tool).copied().unwrap_or(false)
    }

    pub fn toggle(&mut self, tool: ToolId) {
        let method_name = if self.is_running(tool) {
            method::TOOL_STOP
        } else {
            method::TOOL_START
        };
        let response = self.client.call(method_name, Some(json!({ "id": tool })));
        let events = self.client.take_events();
        self.apply_events(events);
        if let Some(error) = response.error {
            self.feedback = Some(error.message);
        }
    }

    pub fn screenshot(&mut self) {
        let response = self.client.call(method::TOOL_SCREENSHOT, None);
        if let Some(error) = response.error {
            self.feedback = Some(error.message);
        } else {
            let path = response
                .result
                .and_then(|value| {
                    value
                        .get("path")
                        .and_then(|value| value.as_str())
                        .map(str::to_owned)
                })
                .unwrap_or_else(|| "AALC/screenshots/mock.png".to_owned());
            self.feedback = Some(format!("截图完成：{path}"));
        }
    }

    fn apply_events(&mut self, events: Vec<EventEnvelope>) {
        for event in events {
            if event.event == event::TOOL_STATUS {
                if let Ok(status) = serde_json::from_value::<ToolStatusPayload>(event.payload) {
                    self.running.insert(status.toolId, status.running);
                }
            }
        }
    }
}

pub struct ResourcesState {
    pub client: MockClient,
    pub groups: Vec<ResourceGroup>,
    pub sync_progress: Option<u8>,
    pub feedback: Option<String>,
}

impl Default for ResourcesState {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourcesState {
    pub fn new() -> Self {
        let mut state = Self {
            client: MockClient::default(),
            groups: Vec::new(),
            sync_progress: None,
            feedback: None,
        };
        state.reload();
        state
    }

    pub fn reload(&mut self) {
        if let Some(value) = self.request(method::RESOURCE_STATUS, None) {
            if let Ok(groups) = serde_json::from_value(value) {
                self.groups = groups;
            }
        }
    }

    pub fn check_update(&mut self) {
        let response = self.client.call(method::RESOURCE_CHECK_UPDATE, None);
        if let Some(error) = response.error {
            self.feedback = Some(error.message);
            return;
        }
        let groups = response
            .result
            .and_then(|value| serde_json::from_value::<Vec<ResourceGroup>>(value).ok())
            .unwrap_or_default();
        let has_update = groups.iter().any(|group| {
            group
                .remoteVersion
                .as_ref()
                .is_some_and(|remote| remote != &group.localVersion)
        });
        self.groups = groups;
        self.feedback = Some(if has_update {
            "发现资源更新".to_owned()
        } else {
            "资源已是最新版本".to_owned()
        });
    }

    pub fn sync_now(&mut self) {
        if self.sync_progress.is_some() {
            return;
        }
        let response = self.client.call(method::RESOURCE_SYNC_START, None);
        let events = self.client.take_events();
        self.apply_events(events);
        if let Some(error) = response.error {
            self.sync_progress = None;
            self.feedback = Some(error.message);
            return;
        }
        self.reload();
        self.sync_progress = None;
        self.feedback = Some("资源同步完成".to_owned());
    }

    fn apply_events(&mut self, events: Vec<EventEnvelope>) {
        for event in events {
            if event.event == event::RESOURCE_SYNC_PROGRESS {
                self.sync_progress = event
                    .payload
                    .get("progress")
                    .and_then(|value| value.as_u64())
                    .map(|progress| progress.min(100) as u8);
            }
        }
    }

    fn request(
        &mut self,
        method_name: &str,
        params: Option<serde_json::Value>,
    ) -> Option<serde_json::Value> {
        let response = self.client.call(method_name, params);
        if let Some(error) = response.error {
            self.feedback = Some(error.message);
            None
        } else {
            response.result
        }
    }
}

pub struct SettingsPageState {
    pub client: MockClient,
    pub hotkey: HotkeyConfig,
    pub system: SystemSettingsConfig,
    pub capturing: Option<HotkeyTarget>,
    pub feedback: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HotkeyTarget {
    StartStop,
    PauseResume,
}

impl Default for SettingsPageState {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsPageState {
    pub fn new() -> Self {
        let mut state = Self {
            client: MockClient::default(),
            hotkey: HotkeyConfig::default(),
            system: SystemSettingsConfig::default(),
            capturing: None,
            feedback: None,
        };
        state.reload();
        state
    }

    pub fn reload(&mut self) {
        if let Some(value) = self.request(method::HOTKEY_GET, None) {
            if let Ok(hotkey) = serde_json::from_value(value) {
                self.hotkey = hotkey;
            }
        }
        if let Some(value) = self.request(method::SYSTEM_SETTINGS_GET, None) {
            if let Ok(system) = serde_json::from_value(value) {
                self.system = system;
            }
        }
    }

    pub fn set_hotkey(&mut self, target: HotkeyTarget, value: Option<String>) {
        match target {
            HotkeyTarget::StartStop => self.hotkey.startStop = value.unwrap_or_default(),
            HotkeyTarget::PauseResume => self.hotkey.pauseResume = value,
        }
        self.persist_hotkey();
    }

    pub fn set_hotkeys_enabled(&mut self, enabled: bool) {
        self.hotkey.enabled = enabled;
        self.persist_hotkey();
    }

    pub fn set_system_bool(&mut self, field: SystemBool, value: bool) {
        match field {
            SystemBool::Simulator => self.system.simulator = value,
            SystemBool::MemoryProtection => self.system.memory_protection = value,
            SystemBool::MinimizeToTray => self.system.minimize_to_tray = value,
            SystemBool::Autostart => self.system.autostart = value,
            SystemBool::KeepScreenAwake => self.system.experimental_keep_screen_awake = value,
            SystemBool::HdrWarning => self.system.experimental_hdr_warning = value,
            SystemBool::Prerelease => self.system.update_prerelease_enable = value,
        }
        self.persist_system();
    }

    pub fn set_system_u16(&mut self, field: SystemU16, value: u16) {
        match field {
            SystemU16::SimulatorType => {
                self.system.simulator_type = value.min(u16::from(u8::MAX)) as u8
            }
            SystemU16::SimulatorPort => self.system.simulator_port = value,
            SystemU16::StartTimeout => self.system.start_emulator_timeout = value,
        }
        self.persist_system();
    }

    pub fn toggle_update_source(&mut self) {
        self.system.update_source = match self.system.update_source {
            crate::model::UpdateSource::GitHub => crate::model::UpdateSource::MirrorChyan,
            crate::model::UpdateSource::MirrorChyan => crate::model::UpdateSource::GitHub,
        };
        self.persist_system();
    }

    pub fn set_cdk(&mut self, cdk: String) {
        self.system.mirrorchyan_cdk = cdk;
        self.persist_system();
    }

    pub fn check_update(&mut self) {
        let response = self.client.call(method::APP_CHECK_UPDATE, None);
        if let Some(error) = response.error {
            self.feedback = Some(error.message);
        } else {
            let latest = response
                .result
                .and_then(|value| {
                    value
                        .get("latest")
                        .and_then(|value| value.as_str())
                        .map(str::to_owned)
                })
                .unwrap_or_else(|| "未知".to_owned());
            self.feedback = Some(format!("当前已是最新版本：{latest}"));
        }
    }

    pub fn capture(&mut self, target: HotkeyTarget) {
        self.capturing = Some(target);
    }

    pub fn finish_capture(&mut self, combo: Option<String>) {
        if let (Some(target), Some(combo)) = (self.capturing.take(), combo) {
            self.set_hotkey(target, Some(combo));
        }
    }

    fn persist_hotkey(&mut self) {
        let response = self.client.call(
            method::HOTKEY_SET,
            Some(serde_json::to_value(&self.hotkey).expect("HotkeyConfig is serializable")),
        );
        self.set_feedback(response);
    }

    fn persist_system(&mut self) {
        let response = self.client.call(
            method::SYSTEM_SETTINGS_SET,
            Some(serde_json::to_value(&self.system).expect("SystemSettingsConfig is serializable")),
        );
        self.set_feedback(response);
    }

    fn set_feedback(&mut self, response: RpcResponse) {
        if let Some(error) = response.error {
            self.feedback = Some(error.message);
        } else {
            self.feedback = Some("设置已保存".to_owned());
        }
    }

    fn request(
        &mut self,
        method_name: &str,
        params: Option<serde_json::Value>,
    ) -> Option<serde_json::Value> {
        let response = self.client.call(method_name, params);
        if let Some(error) = response.error {
            self.feedback = Some(error.message);
            None
        } else {
            response.result
        }
    }
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
