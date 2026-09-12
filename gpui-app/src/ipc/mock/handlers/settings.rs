use super::*;

impl MockState {
    pub(super) fn handle_settings(&mut self, request: &RpcRequest) -> Result<Value, RpcError> {
        match request.method.as_str() {
            method::THEME_PACK_LIST => Ok(serde_json::to_value(&self.packs).unwrap()),
            method::THEME_PACK_UPDATE_ALL => {
                let value: Value = Self::params(
                    request.params.clone(),
                    "themePack.updateAll requires {packs}",
                )?;
                let packs: Vec<ThemePack> =
                    Self::params(value.get("packs").cloned(), "themePack.updateAll packs")?;
                self.packs.packs = packs;
                Ok(json!(true))
            }
            method::THEME_PACK_RESET_WEIGHTS => {
                self.packs.packs.iter_mut().for_each(|pack| pack.weight = 1);
                Ok(json!(self.packs.clone()))
            }
            method::RESOURCE_STATUS => Ok(serde_json::to_value(&self.resources).unwrap()),
            method::RESOURCE_CHECK_UPDATE => {
                for resource in &mut self.resources {
                    resource.remoteVersion = Some("v2025.06.2".into());
                }
                // Checking updates returns the same resource collection the
                // UI renders, matching the canonical `resource.status`
                // response shape and avoiding a second incompatible payload.
                Ok(serde_json::to_value(&self.resources).unwrap())
            }
            method::RESOURCE_SYNC_START => {
                let scope = request
                    .params
                    .as_ref()
                    .and_then(|value| value.get("scope"))
                    .and_then(Value::as_str)
                    .unwrap_or("all")
                    .to_owned();
                self.emit(
                    event::RESOURCE_SYNC_PROGRESS,
                    &SyncProgressPayload {
                        scope: scope.clone(),
                        progress: 100,
                    },
                );
                for resource in &mut self.resources {
                    if scope == "all" || resource.id == scope {
                        resource.localVersion = "v2025.06.2".into();
                        resource.lastSyncAt = Some(0);
                    }
                }
                Ok(json!({"accepted": true, "runId": "mock-resource-sync"}))
            }
            method::HOTKEY_GET => Ok(serde_json::to_value(&self.hotkey).unwrap()),
            method::HOTKEY_SET => {
                self.hotkey = merge_json(&self.hotkey, request.params.clone(), "hotkey.set")?;
                Ok(json!(true))
            }
            method::SYSTEM_SETTINGS_GET => Ok(serde_json::to_value(&self.system_settings).unwrap()),
            method::SYSTEM_SETTINGS_SET => {
                self.system_settings = merge_json(
                    &self.system_settings,
                    request.params.clone(),
                    "systemSettings.set",
                )?;
                Ok(json!(true))
            }
            method::NOTIFICATION_TEST => {
                let value: Value =
                    Self::params(request.params.clone(), "notification.test requires {spt}")?;
                let spt = match value.get("spt") {
                    Some(Value::String(value)) => value.trim(),
                    Some(_) => return Err(RpcError::invalid_params("SPT 必须是字符串")),
                    None => self.system_settings.wxpusher_spt.trim(),
                };
                if spt.is_empty() {
                    return Err(RpcError::invalid_params("SPT 未配置"));
                }
                if spt.len() <= 4 || !spt.starts_with("SPT_") {
                    return Err(RpcError::invalid_params("SPT 格式无效"));
                }
                Ok(json!({"accepted": true}))
            }
            method::PREVIEW_SET_ENABLED => {
                let value: Value = Self::params(
                    request.params.clone(),
                    "preview.setEnabled requires {enabled}",
                )?;
                let enabled = value
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .ok_or_else(|| {
                        RpcError::invalid_params("preview.setEnabled.enabled must be a boolean")
                    })?;
                self.preview_enabled = enabled;
                Ok(json!({
                    "enabled": enabled,
                    "running": enabled && self.device_status == ConnectionStatus::Connected
                }))
            }
            _ => Err(RpcError::method_not_found(&request.method)),
        }
    }
}
