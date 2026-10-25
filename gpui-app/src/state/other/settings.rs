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
        #[allow(unused_mut)]
        let mut state = Self {
            rpc: RpcGateway::new(client),
            hotkey: HotkeyConfig::default(),
            system: SystemSettingsConfig::default(),
            capturing: None,
            open_select: None,
            feedback: None,
        };
        #[cfg(test)]
        {
            state.hotkey = state
                .rpc
                .request(method::HOTKEY_GET, None)
                .unwrap_or_default();
            state.system = state
                .rpc
                .request(method::SYSTEM_SETTINGS_GET, None)
                .unwrap_or_default();
        }
        state
    }

    #[cfg(test)]
    pub(crate) fn pump_mock(&mut self) {
        for _ in 0..8 {
            let completions = self.rpc.take_completions();
            if completions.is_empty() {
                break;
            }
            for completion in completions {
                let result = crate::ipc::RpcGateway::decode_response(
                    &completion.method,
                    completion.response,
                );
                self.apply_rpc_result(&completion.method, result);
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
            SystemBool::MemoryProtection => self.system.memory_protection = value,
            SystemBool::MinimizeToTray => self.system.minimize_to_tray = value,
            SystemBool::Autostart => self.system.autostart = value,
            SystemBool::KeepScreenAwake => self.system.experimental_keep_screen_awake = value,
            SystemBool::HdrWarning => self.system.experimental_hdr_warning = value,
            SystemBool::TemplateBlur => self.system.enable_template_blur = value,
            SystemBool::Prerelease => self.system.update_prerelease_enable = value,
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

    /// Record that the external repository link was requested.
    ///
    /// Pages call this instead of writing the feedback field directly, keeping
    /// business state behind the state-layer boundary.
    pub fn note_repository_opened(&mut self) {
        self.feedback = Some("已请求打开 GitHub 仓库".to_owned());
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

    pub fn set_wxpusher_spt(&mut self, spt: String) {
        self.system.wxpusher_spt = spt.trim().to_owned();
        self.persist_system();
    }

    pub fn test_notification(&mut self, spt: String) {
        self.rpc.submit(
            method::NOTIFICATION_TEST,
            Some(serde_json::json!({"spt": spt})),
        );
        self.feedback = Some("正在发送测试通知".to_owned());
    }

    pub fn check_update(&mut self) {
        self.rpc.submit(method::APP_CHECK_UPDATE, None);
        self.feedback = Some("正在检查更新".to_owned());
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
        self.rpc.submit(
            method::HOTKEY_SET,
            Some(serde_json::to_value(&self.hotkey).expect("HotkeyConfig is serializable")),
        );
        self.feedback = Some("正在保存设置".to_owned());
    }

    fn persist_system(&mut self) {
        self.rpc.submit(
            method::SYSTEM_SETTINGS_SET,
            Some(self.visible_system_settings_patch()),
        );
        self.feedback = Some("正在保存设置".to_owned());
    }

    pub(crate) fn visible_system_settings_patch(&self) -> serde_json::Value {
        serde_json::json!({
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
            method::NOTIFICATION_TEST => {
                self.feedback = Some("测试通知已发送".to_owned());
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
}
