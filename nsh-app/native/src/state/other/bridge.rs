//! TSX-facing stores for settings, theme packs, resources and the toolbox.

use nsh_core::prelude::*;

use crate::model::{ResourceGroup, ThemePack, ToolId, UpdateSource};

use super::{
    resources::ResourcesState,
    settings::{HotkeyTarget, SettingsState, SystemBool},
    theme_packs::ThemePacksState,
    toolbox::ToolboxState,
};

fn tool_id(value: &str) -> ToolId {
    match value {
        "enkephalin" => ToolId::Enkephalin,
        "screenshot" => ToolId::Screenshot,
        "resolution" => ToolId::Resolution,
        _ => ToolId::InfiniteBattle,
    }
}

#[derive(Clone, Debug, JsObject)]
pub struct HotkeyView {
    pub enabled: bool,
    pub start_stop: String,
    pub pause_resume: Option<String>,
}

#[derive(Clone, Debug, JsObject)]
pub struct SystemView {
    pub memory_protection: bool,
    pub minimize_to_tray: bool,
    pub autostart: bool,
    pub experimental_keep_screen_awake: bool,
    pub experimental_hdr_warning: bool,
    pub enable_template_blur: bool,
    pub update_prerelease_enable: bool,
    pub update_source: String,
    pub mirrorchyan_cdk: String,
    pub wxpusher_spt: String,
}

#[derive(Clone, Debug, JsObject)]
pub struct ThemePackView {
    pub id: String,
    pub name: String,
    pub weight: u32,
    pub enabled: bool,
    pub tier: String,
}

#[derive(Clone, Debug, JsObject)]
pub struct ResourceGroupView {
    pub id: String,
    pub name: String,
    pub local_version: String,
    pub remote_version: Option<String>,
    pub last_sync_at: Option<i64>,
}

/// Settings flags, update source, CDK/SPT and global hotkeys.
#[class]
pub struct SettingsStore {
    state: SettingsState,
}

#[class]
impl SettingsStore {
    /// Loads the settings synchronously (mock/offline use).
    pub fn new() -> Self {
        Self {
            state: SettingsState::from_shared(),
        }
    }

    /// Loads the settings off the engine thread.
    pub async fn connect() -> Result<SettingsStore, String> {
        blocking(SettingsStore::try_new)
            .await
            .map_err(|error| error.to_string())?
    }

    fn try_new() -> Result<Self, String> {
        let state = SettingsState::from_shared();
        match state.error.clone() {
            Some(error) => Err(error),
            None => Ok(Self { state }),
        }
    }

    /// The global-hotkey configuration.
    pub fn hotkeys(&self) -> HotkeyView {
        HotkeyView {
            enabled: self.state.hotkey.enabled,
            start_stop: self.state.hotkey.startStop.clone(),
            pause_resume: self.state.hotkey.pauseResume.clone(),
        }
    }

    /// The system flags, update source and saved secrets.
    pub fn system(&self) -> SystemView {
        let system = &self.state.system;
        SystemView {
            memory_protection: system.memory_protection,
            minimize_to_tray: system.minimize_to_tray,
            autostart: system.autostart,
            experimental_keep_screen_awake: system.experimental_keep_screen_awake,
            experimental_hdr_warning: system.experimental_hdr_warning,
            enable_template_blur: system.enable_template_blur,
            update_prerelease_enable: system.update_prerelease_enable,
            update_source: match system.update_source {
                UpdateSource::GitHub => "github",
                UpdateSource::MirrorChyan => "mirrorChyan",
            }
            .into(),
            mirrorchyan_cdk: system.mirrorchyan_cdk.clone(),
            wxpusher_spt: system.wxpusher_spt.clone(),
        }
    }

    /// The last state-layer feedback line.
    pub fn feedback(&self) -> Option<String> {
        self.state.feedback.clone()
    }

    /// Flips one system flag and saves the visible settings patch.
    pub fn set_system_flag(&mut self, flag: String, value: bool) -> Result<SystemView, String> {
        let field = match flag.as_str() {
            "memoryProtection" => SystemBool::MemoryProtection,
            "minimizeToTray" => SystemBool::MinimizeToTray,
            "autostart" => SystemBool::Autostart,
            "keepScreenAwake" => SystemBool::KeepScreenAwake,
            "hdrWarning" => SystemBool::HdrWarning,
            "templateBlur" => SystemBool::TemplateBlur,
            "prerelease" => SystemBool::Prerelease,
            _ => return Err(format!("未知设置项：{flag}")),
        };
        self.state.set_system_bool(field, value)?;
        Ok(self.system())
    }

    /// Chooses the update mirror and saves it.
    pub fn set_update_source(&mut self, source: String) -> Result<SystemView, String> {
        let source = match source.as_str() {
            "mirrorChyan" => UpdateSource::MirrorChyan,
            _ => UpdateSource::GitHub,
        };
        self.state.set_update_source(source)?;
        Ok(self.system())
    }

    /// Stores the CDK in the draft settings (call `saveCdk` to persist).
    pub fn set_cdk(&mut self, value: String) {
        self.state.set_cdk(value);
    }

    /// Persists the CDK with the visible settings patch.
    pub fn save_cdk(&mut self) -> Result<SystemView, String> {
        self.state.save_cdk()?;
        Ok(self.system())
    }

    /// Stores the WxPusher SPT in the draft settings.
    pub fn set_spt(&mut self, value: String) {
        self.state.set_wxpusher_spt(value);
    }

    /// Persists the WxPusher SPT with the visible settings patch.
    pub fn save_spt(&mut self) -> Result<SystemView, String> {
        self.state.save_wxpusher_spt()?;
        Ok(self.system())
    }

    /// Sends a test notification with the current SPT.
    pub fn test_notification(&mut self) -> Result<String, String> {
        let spt = self.state.system.wxpusher_spt.clone();
        self.state.test_notification(spt)?;
        Ok(self.state.feedback.clone().unwrap_or_default())
    }

    /// Checks for an update and returns the feedback line.
    pub fn check_update(&mut self) -> Result<String, String> {
        self.state.check_update()
    }

    /// Turns global hotkeys on or off and saves the config.
    pub fn set_hotkeys_enabled(&mut self, enabled: bool) -> Result<HotkeyView, String> {
        self.state.set_hotkeys_enabled(enabled)?;
        Ok(self.hotkeys())
    }

    /// Clears one hotkey and saves the config.
    pub fn clear_hotkey(&mut self, target: String) -> Result<HotkeyView, String> {
        let target = if target == "pauseResume" {
            HotkeyTarget::PauseResume
        } else {
            HotkeyTarget::StartStop
        };
        self.state.set_hotkey(target, None)?;
        Ok(self.hotkeys())
    }

    /// Sets one hotkey combination and saves the config.
    pub fn set_hotkey(
        &mut self,
        target: String,
        value: Option<String>,
    ) -> Result<HotkeyView, String> {
        let target = if target == "pauseResume" {
            HotkeyTarget::PauseResume
        } else {
            HotkeyTarget::StartStop
        };
        self.state.set_hotkey(target, value)?;
        Ok(self.hotkeys())
    }
}

/// Mirror Dungeon theme-pack weights.
#[class]
pub struct ThemePacksStore {
    state: ThemePacksState,
}

#[class]
impl ThemePacksStore {
    /// Loads the packs synchronously (mock/offline use).
    pub fn new() -> Self {
        Self {
            state: ThemePacksState::from_shared(),
        }
    }

    /// Loads the packs off the engine thread.
    pub async fn connect() -> Result<ThemePacksStore, String> {
        blocking(ThemePacksStore::try_new)
            .await
            .map_err(|error| error.to_string())?
    }

    fn try_new() -> Result<Self, String> {
        let state = ThemePacksState::from_shared();
        match state.error.clone() {
            Some(error) => Err(error),
            None => Ok(Self { state }),
        }
    }

    /// The packs in draw order.
    pub fn packs(&self) -> Vec<ThemePackView> {
        self.state.sorted_packs().iter().map(view).collect()
    }

    /// Whether the Mirror Dungeon is in its hard cycle.
    pub fn hard_mirror_active(&self) -> bool {
        self.state.data.hardMirrorActive
    }

    /// The weight of the enabled packs.
    pub fn total_weight(&self) -> u32 {
        self.state.total_weight()
    }

    /// Whether the list is sorted by weight.
    pub fn sorted_by_weight(&self) -> bool {
        self.state.sort_by_weight
    }

    /// The last state-layer feedback line.
    pub fn feedback(&self) -> Option<String> {
        self.state.feedback.clone()
    }

    /// Turns the weight sort on or off.
    pub fn set_sort_by_weight(&mut self, enabled: bool) {
        self.state.set_sort_by_weight(enabled);
    }

    /// Turns one pack on or off and saves the list.
    pub fn toggle(&mut self, id: String) -> Result<Vec<ThemePackView>, String> {
        self.state.toggle_enabled(&id)?;
        Ok(self.packs())
    }

    /// Sets one pack's weight and saves the list.
    pub fn set_weight(&mut self, id: String, weight: u32) -> Result<Vec<ThemePackView>, String> {
        self.state.set_weight(&id, weight.min(10) as u8)?;
        Ok(self.packs())
    }

    /// Turns every pack on or off and saves the list.
    pub fn set_all(&mut self, enabled: bool) -> Result<Vec<ThemePackView>, String> {
        self.state.set_all_enabled(enabled)?;
        Ok(self.packs())
    }

    /// Restores the backend's default weights.
    pub fn reset_weights(&mut self) -> Result<Vec<ThemePackView>, String> {
        self.state.reset_weights()?;
        Ok(self.packs())
    }
}

fn view(pack: &ThemePack) -> ThemePackView {
    ThemePackView {
        id: pack.id.clone(),
        name: pack.name.clone(),
        weight: u32::from(pack.weight),
        enabled: pack.enabled,
        tier: pack.tier.clone(),
    }
}

/// Resource groups and their sync progress.
#[class]
pub struct ResourcesStore {
    state: ResourcesState,
}

#[class]
impl ResourcesStore {
    /// Loads the resource groups synchronously (mock/offline use).
    pub fn new() -> Self {
        Self {
            state: ResourcesState::from_shared(),
        }
    }

    /// Loads the resource groups off the engine thread.
    pub async fn connect() -> Result<ResourcesStore, String> {
        blocking(ResourcesStore::try_new)
            .await
            .map_err(|error| error.to_string())?
    }

    fn try_new() -> Result<Self, String> {
        let state = ResourcesState::from_shared();
        match state.error.clone() {
            Some(error) => Err(error),
            None => Ok(Self { state }),
        }
    }

    /// The resource groups.
    pub fn groups(&self) -> Vec<ResourceGroupView> {
        self.state.groups.iter().map(group_view).collect()
    }

    /// The sync's progress, or `null` when idle.
    pub fn sync_progress(&self) -> Option<u32> {
        self.state.sync_progress.map(u32::from)
    }

    /// The last state-layer feedback line.
    pub fn feedback(&self) -> Option<String> {
        self.state.feedback.clone()
    }

    /// Re-reads the remote versions.
    pub fn check(&mut self) -> Result<Vec<ResourceGroupView>, String> {
        self.state.check_update()?;
        Ok(self.groups())
    }

    /// Starts a sync and returns the progress the backend reported.
    pub fn sync_now(&mut self) -> Result<u32, String> {
        self.state.sync_now().map(u32::from)
    }

    /// Hides a completed sync after its progress was shown.
    pub fn finish_sync(&mut self) -> Vec<ResourceGroupView> {
        self.state.finish_sync();
        self.groups()
    }
}

fn group_view(group: &ResourceGroup) -> ResourceGroupView {
    ResourceGroupView {
        id: group.id.clone(),
        name: group.name.clone(),
        local_version: group.localVersion.clone(),
        remote_version: group.remoteVersion.clone(),
        last_sync_at: group.lastSyncAt,
    }
}

/// The toolbox's four tool actions.
#[class]
pub struct ToolboxStore {
    state: ToolboxState,
}

#[class]
impl ToolboxStore {
    /// A toolbox bound to the configured backend.
    pub fn new() -> Self {
        Self {
            state: ToolboxState::from_shared(),
        }
    }

    /// Connects off the engine thread.
    pub async fn connect() -> Result<ToolboxStore, String> {
        blocking(|| Self {
            state: ToolboxState::from_shared(),
        })
        .await
        .map_err(|error| error.to_string())
    }

    /// Whether `id` is running.
    pub fn running(&self, id: String) -> bool {
        self.state.is_running(tool_id(&id))
    }

    /// The last state-layer feedback line.
    pub fn feedback(&self) -> Option<String> {
        self.state.feedback.clone()
    }

    /// Starts or stops a tool.
    pub fn toggle(&mut self, id: String) -> Result<bool, String> {
        self.state.toggle(tool_id(&id))
    }

    /// Captures the game window.
    pub fn screenshot(&mut self) -> Result<String, String> {
        self.state.screenshot()
    }

    /// Switches the device to 1080P landscape.
    pub fn apply_resolution(&mut self) -> Result<String, String> {
        self.state.apply_resolution()
    }

    /// Restores the device's resolution and DPI.
    pub fn reset_resolution(&mut self) -> Result<String, String> {
        self.state.reset_resolution()
    }
}
