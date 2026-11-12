//! The settings page's backend flags and global-hotkey configuration.

use serde_json::json;

use crate::{
    ipc::{BackendClient, RpcGateway, contract::method},
    model::{HotkeyConfig, SystemSettingsConfig, UpdateSource},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SystemBool {
    MemoryProtection,
    MinimizeToTray,
    Autostart,
    KeepScreenAwake,
    HdrWarning,
    TemplateBlur,
    Prerelease,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HotkeyTarget {
    StartStop,
    PauseResume,
}

pub struct SettingsState {
    rpc: RpcGateway,
    pub hotkey: HotkeyConfig,
    pub system: SystemSettingsConfig,
    pub feedback: Option<String>,
    pub error: Option<String>,
}

impl SettingsState {
    pub fn new(client: impl Into<BackendClient>) -> Self {
        let mut state = Self {
            rpc: RpcGateway::new(client),
            hotkey: HotkeyConfig::default(),
            system: SystemSettingsConfig::default(),
            feedback: None,
            error: None,
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
        self.hotkey = self
            .rpc
            .request(method::HOTKEY_GET, None)
            .map_err(|error| error.message)?;
        self.system = self
            .rpc
            .request(method::SYSTEM_SETTINGS_GET, None)
            .map_err(|error| error.message)?;
        self.error = None;
        Ok(())
    }

    pub fn set_system_bool(&mut self, field: SystemBool, value: bool) -> Result<(), String> {
        match field {
            SystemBool::MemoryProtection => self.system.memory_protection = value,
            SystemBool::MinimizeToTray => self.system.minimize_to_tray = value,
            SystemBool::Autostart => self.system.autostart = value,
            SystemBool::KeepScreenAwake => self.system.experimental_keep_screen_awake = value,
            SystemBool::HdrWarning => self.system.experimental_hdr_warning = value,
            SystemBool::TemplateBlur => self.system.enable_template_blur = value,
            SystemBool::Prerelease => self.system.update_prerelease_enable = value,
        }
        self.persist_system()
    }

    pub fn set_update_source(&mut self, source: UpdateSource) -> Result<(), String> {
        self.system.update_source = source;
        self.persist_system()
    }

    pub fn set_cdk(&mut self, cdk: String) {
        self.system.mirrorchyan_cdk = cdk;
    }

    pub fn set_wxpusher_spt(&mut self, spt: String) {
        self.system.wxpusher_spt = spt.trim().to_owned();
    }

    pub fn save_cdk(&mut self) -> Result<(), String> {
        self.persist_system()
    }

    pub fn save_wxpusher_spt(&mut self) -> Result<(), String> {
        self.persist_system()
    }

    pub fn set_hotkeys_enabled(&mut self, enabled: bool) -> Result<(), String> {
        self.hotkey.enabled = enabled;
        self.persist_hotkey()
    }

    pub fn set_hotkey(
        &mut self,
        target: HotkeyTarget,
        value: Option<String>,
    ) -> Result<(), String> {
        match target {
            HotkeyTarget::StartStop => self.hotkey.startStop = value.unwrap_or_default(),
            HotkeyTarget::PauseResume => self.hotkey.pauseResume = value,
        }
        self.persist_hotkey()
    }

    pub fn test_notification(&mut self, spt: String) -> Result<(), String> {
        self.rpc
            .request_value(method::NOTIFICATION_TEST, Some(json!({ "spt": spt })))
            .map_err(|error| {
                self.feedback = Some(error.message.clone());
                error.message
            })?;
        self.feedback = Some("测试通知已发送".to_owned());
        Ok(())
    }

    pub fn check_update(&mut self) -> Result<String, String> {
        let result = self
            .rpc
            .request_value(method::APP_CHECK_UPDATE, None)
            .map_err(|error| error.message)?
            .unwrap_or_default();
        let message = match result.get("status").and_then(|value| value.as_str()) {
            Some("available") => format!(
                "发现新版本：{}",
                result
                    .get("latest")
                    .and_then(|value| value.as_str())
                    .unwrap_or("未知")
            ),
            Some("up_to_date") => format!(
                "当前已是最新版本：{}",
                result
                    .get("latest")
                    .and_then(|value| value.as_str())
                    .unwrap_or("未知")
            ),
            _ => result
                .get("error")
                .and_then(|value| value.as_str())
                .unwrap_or("更新检查失败")
                .to_owned(),
        };
        self.feedback = Some(message.clone());
        Ok(message)
    }

    fn persist_hotkey(&mut self) -> Result<(), String> {
        self.rpc
            .request_value(
                method::HOTKEY_SET,
                Some(serde_json::to_value(&self.hotkey).map_err(|error| error.to_string())?),
            )
            .map_err(|error| error.message)?;
        self.feedback = Some("设置已保存".to_owned());
        Ok(())
    }

    fn persist_system(&mut self) -> Result<(), String> {
        self.rpc
            .request_value(method::SYSTEM_SETTINGS_SET, Some(self.visible_patch()))
            .map_err(|error| error.message)?;
        self.feedback = Some("设置已保存".to_owned());
        Ok(())
    }

    fn visible_patch(&self) -> serde_json::Value {
        json!({
            "memory_protection": self.system.memory_protection,
            "minimize_to_tray": self.system.minimize_to_tray,
            "autostart": self.system.autostart,
            "experimental_keep_screen_awake": self.system.experimental_keep_screen_awake,
            "experimental_hdr_warning": self.system.experimental_hdr_warning,
            "enable_template_blur": self.system.enable_template_blur,
            "update_prerelease_enable": self.system.update_prerelease_enable,
            "update_source": self.system.update_source,
            "mirrorchyan_cdk": self.system.mirrorchyan_cdk,
            "wxpusher_spt": self.system.wxpusher_spt,
        })
    }
}
