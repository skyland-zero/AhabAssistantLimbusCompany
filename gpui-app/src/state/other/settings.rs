use super::*;

impl Default for SettingsPageState {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsPageState {
    pub fn new() -> Self {
        Self::with_client(MockClient::default())
    }

    pub fn with_client(client: impl Into<crate::ipc::BackendClient>) -> Self {
        let mut state = Self {
            rpc: RpcGateway::new(client),
            hotkey: HotkeyConfig::default(),
            system: SystemSettingsConfig::default(),
            capturing: None,
            open_select: None,
            feedback: None,
        };
        if !state.rpc.is_sidecar() {
            state.reload();
        }
        state
    }

    pub fn reload(&mut self) {
        if self.rpc.is_sidecar() {
            return;
        }
        if let Some(value) = self.request(method::HOTKEY_GET, None)
            && let Ok(hotkey) = serde_json::from_value(value)
        {
            self.hotkey = hotkey;
        }
        if let Some(value) = self.request(method::SYSTEM_SETTINGS_GET, None)
            && let Ok(system) = serde_json::from_value(value)
        {
            self.system = system;
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
        self.open_select = None;
        match field {
            SystemU16::SimulatorType => {
                self.system.simulator_type = value.min(u16::from(u8::MAX)) as u8
            }
            SystemU16::SimulatorPort => self.system.simulator_port = value,
            SystemU16::StartTimeout => self.system.start_emulator_timeout = value,
        }
        self.persist_system();
    }

    pub fn set_update_source(&mut self, source: crate::model::UpdateSource) {
        self.system.update_source = source;
        self.open_select = None;
        self.persist_system();
    }

    pub fn toggle_select(&mut self, select: SettingsSelect) {
        self.open_select = if self.open_select == Some(select) {
            None
        } else {
            Some(select)
        };
    }

    pub fn close_select(&mut self) {
        self.open_select = None;
    }

    pub fn is_select_open(&self, select: SettingsSelect) -> bool {
        self.open_select == Some(select)
    }

    pub fn set_cdk(&mut self, cdk: String) {
        self.system.mirrorchyan_cdk = cdk;
        self.persist_system();
    }

    pub fn check_update(&mut self) {
        if self.rpc.is_sidecar() {
            self.rpc.submit(method::APP_CHECK_UPDATE, None);
            self.feedback = Some("正在检查更新".to_owned());
            return;
        }
        match self.rpc.request_value(method::APP_CHECK_UPDATE, None) {
            Err(error) => self.feedback = Some(error.message),
            Ok(result) => {
                let result = result.unwrap_or_default();
                let latest = result
                    .get("latest")
                    .and_then(|value| value.as_str())
                    .unwrap_or("未知");
                let update_available = result
                    .get("updateAvailable")
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false);
                self.feedback = Some(if update_available {
                    format!("发现新版本：{latest}")
                } else {
                    format!("当前已是最新版本：{latest}")
                });
            }
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
        if self.rpc.is_sidecar() {
            self.rpc.submit(
                method::HOTKEY_SET,
                Some(serde_json::to_value(&self.hotkey).expect("HotkeyConfig is serializable")),
            );
            self.feedback = Some("正在保存设置".to_owned());
            return;
        }
        let result = self.rpc.request_value(
            method::HOTKEY_SET,
            Some(serde_json::to_value(&self.hotkey).expect("HotkeyConfig is serializable")),
        );
        self.set_feedback(result);
    }

    fn persist_system(&mut self) {
        if self.rpc.is_sidecar() {
            self.rpc.submit(
                method::SYSTEM_SETTINGS_SET,
                Some(
                    serde_json::to_value(&self.system)
                        .expect("SystemSettingsConfig is serializable"),
                ),
            );
            self.feedback = Some("正在保存设置".to_owned());
            return;
        }
        let result = self.rpc.request_value(
            method::SYSTEM_SETTINGS_SET,
            Some(serde_json::to_value(&self.system).expect("SystemSettingsConfig is serializable")),
        );
        self.set_feedback(result);
    }

    fn set_feedback(&mut self, result: Result<Option<serde_json::Value>, crate::ipc::RpcError>) {
        self.feedback = Some(match result {
            Ok(_) => "设置已保存".to_owned(),
            Err(error) => error.message,
        });
    }

    pub(crate) fn apply_rpc_result(
        &mut self,
        method_name: &str,
        result: Result<Option<serde_json::Value>, crate::ipc::RpcError>,
    ) {
        let value = match result {
            Ok(value) => value,
            Err(error) => {
                self.feedback = Some(error.message);
                match method_name {
                    method::HOTKEY_SET => self.rpc.submit(method::HOTKEY_GET, None),
                    method::SYSTEM_SETTINGS_SET => {
                        self.rpc.submit(method::SYSTEM_SETTINGS_GET, None)
                    }
                    _ => {}
                }
                return;
            }
        };
        match method_name {
            method::APP_CHECK_UPDATE => {
                let result = value.unwrap_or_default();
                self.feedback = Some(
                    match result.get("status").and_then(|value| value.as_str()) {
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
                    },
                );
            }
            method::HOTKEY_SET | method::SYSTEM_SETTINGS_SET => {
                self.feedback = Some("设置已保存".to_owned());
            }
            method::HOTKEY_GET => {
                if let Some(value) = value
                    && let Ok(hotkey) = serde_json::from_value(value)
                {
                    self.hotkey = hotkey;
                }
            }
            method::SYSTEM_SETTINGS_GET => {
                if let Some(value) = value
                    && let Ok(system) = serde_json::from_value(value)
                {
                    self.system = system;
                }
            }
            _ => {}
        }
    }

    fn request(
        &mut self,
        method_name: &str,
        params: Option<serde_json::Value>,
    ) -> Option<serde_json::Value> {
        match self.rpc.request_value(method_name, params) {
            Ok(value) => value,
            Err(error) => {
                self.feedback = Some(error.message);
                None
            }
        }
    }
}
