use super::*;

use serde_json::{Value, json};

use crate::{
    ipc::{MockClient, contract::method},
    model::TeamDetail,
};

impl Default for TeamsState {
    fn default() -> Self {
        Self::new()
    }
}

impl TeamsState {
    pub fn new() -> Self {
        Self::with_client(MockClient::default())
    }

    pub fn with_client(client: impl Into<crate::ipc::BackendClient>) -> Self {
        #[allow(unused_mut)]
        let mut state = Self {
            rpc: RpcGateway::new(client),
            teams: Vec::new(),
            sinners: Vec::new(),
            presets: Vec::new(),
            filter: TeamFilter::All,
            editor: None,
            delete_target: None,
            preset_picker: None,
            preset_overwrite: None,
            open_select: None,
            feedback: None,
            saving: false,
            deleting: false,
            team_toggle_in_flight: None,
            pending_save: None,
        };
        #[cfg(test)]
        {
            state.teams = state
                .rpc
                .request(method::TEAM_LIST, None)
                .unwrap_or_default();
            state.sinners = state
                .rpc
                .request(method::SINNER_LIST, None)
                .unwrap_or_default();
            state.presets = state
                .rpc
                .request(method::TEAM_PRESET_LIST, None)
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
                let result = RpcGateway::decode_response(&completion.method, completion.response);
                match completion.method.as_str() {
                    method::TEAM_SAVE | method::TEAM_DELETE | method::TEAM_LIST => {
                        self.apply_team_completion(&completion.method, result);
                    }
                    method::TEAM_STATS_GET | method::TEAM_STATS_CLEAR => {
                        self.apply_stats_completion(&completion.method, completion.params, result);
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn export_editor_json(&self) -> Option<String> {
        self.editor
            .as_ref()
            .and_then(|editor| serde_json::to_string_pretty(&editor.team).ok())
    }

    /// Merge pasted JSON with the current form and all mirror defaults. This
    /// intentionally mirrors the React editor's compatibility behavior: the
    /// current id survives imports, omitted fields keep their current values,
    /// and new mirror fields receive defaults.
    pub fn import_editor_json(&mut self, input: &str) -> Result<(), String> {
        let parsed: Value =
            serde_json::from_str(input.trim()).map_err(|error| error.to_string())?;
        let object = parsed
            .as_object()
            .ok_or_else(|| "队伍 JSON 必须是对象".to_owned())?;
        let Some(name) = object.get("name").and_then(Value::as_str) else {
            return Err("队伍 JSON 缺少 name".to_owned());
        };
        if name.trim().is_empty() {
            return Err("队伍名称不能为空".to_owned());
        }

        let Some(editor) = self.editor.as_mut() else {
            return Err("当前没有打开队伍编辑器".to_owned());
        };
        let current = &editor.team;
        let id = current.id.clone();
        let schema_version = object
            .get("schemaVersion")
            .and_then(Value::as_u64)
            .map(|value| value as u32)
            .unwrap_or(current.schemaVersion);
        let purpose = object
            .get("purpose")
            .cloned()
            .map(|value| serde_json::from_value(value).map_err(|_| "purpose 无效".to_owned()))
            .transpose()?
            .unwrap_or(current.purpose);
        let sinners = object
            .get("sinners")
            .cloned()
            .map(|value| serde_json::from_value(value).map_err(|_| "sinners 无效".to_owned()))
            .transpose()?
            .unwrap_or_else(|| current.sinners.clone());
        if sinners.len() > 12 {
            return Err("队伍最多选择 12 名人格".to_owned());
        }
        let accessory_scheme = object
            .get("accessoryScheme")
            .and_then(Value::as_str)
            .unwrap_or(&current.accessoryScheme)
            .to_owned();
        let enabled = object
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(current.enabled);

        let mirror_config = if purpose == TeamPurpose::Luxcavation {
            None
        } else {
            let mut mirror = current.mirrorConfig.clone().unwrap_or_default();
            if let Some(mirror_patch) = object.get("mirrorConfig")
                && !mirror_patch.is_null()
            {
                let Some(patch) = mirror_patch.as_object() else {
                    return Err("mirrorConfig 必须是对象".to_owned());
                };
                let mut value = serde_json::to_value(&mirror).map_err(|error| error.to_string())?;
                let Some(base) = value.as_object_mut() else {
                    return Err("mirrorConfig 默认值无效".to_owned());
                };
                let mut patch = patch.clone();
                if let Some(discard_patch) = patch.get("discard_systems").and_then(Value::as_object)
                {
                    let mut discard = serde_json::to_value(&mirror.discard_systems)
                        .map_err(|error| error.to_string())?;
                    if let Some(discard_base) = discard.as_object_mut() {
                        discard_base.extend(discard_patch.clone());
                        patch.insert("discard_systems".to_owned(), discard);
                    }
                }
                base.extend(patch);
                mirror = serde_json::from_value(value).map_err(|error| error.to_string())?;
            }
            mirror.opening_bonus.resize(10, 0);
            mirror.opening_bonus.truncate(10);
            mirror.ignore_shop.resize(5, false);
            mirror.ignore_shop.truncate(5);
            Some(mirror)
        };

        editor.team = TeamDetail {
            schemaVersion: schema_version,
            id,
            name: name.to_owned(),
            sinners,
            purpose,
            accessoryScheme: accessory_scheme,
            enabled: if purpose == TeamPurpose::Luxcavation {
                false
            } else {
                enabled
            },
            mirrorConfig: mirror_config,
        };
        editor.json_import_open = false;
        self.feedback = Some("已导入队伍 JSON（尚未保存）".to_owned());
        Ok(())
    }

    pub fn save_editor(&mut self) -> Result<(), String> {
        let (submitted, value) = self.prepare_save()?;
        self.pending_save = Some(submitted);
        self.rpc.submit(method::TEAM_SAVE, Some(value));
        Ok(())
    }

    pub fn prepare_save(&mut self) -> Result<(TeamDetail, Value), String> {
        if self.saving {
            return Err("队伍正在保存，请稍候".to_owned());
        }
        if self.team_toggle_in_flight.is_some() {
            return Err("队伍启用状态正在更新，请稍候".to_owned());
        }
        let Some(editor) = self.editor.as_ref() else {
            return Err("当前没有打开队伍编辑器".to_owned());
        };
        if editor.team.name.trim().is_empty() {
            return Err("队伍名称不能为空".to_owned());
        }
        let submitted = editor.team.clone();
        let mut value = serde_json::to_value(&submitted).map_err(|error| error.to_string())?;
        if submitted.id.is_empty()
            && let Some(team_number) = editor.requested_team_number
            && let Some(object) = value.as_object_mut()
        {
            object.insert("teamNumber".to_owned(), json!(team_number));
        }
        if submitted.purpose == TeamPurpose::Luxcavation
            && let Some(object) = value.as_object_mut()
        {
            object.insert("mirrorConfig".to_owned(), Value::Null);
            object.insert("enabled".to_owned(), Value::Bool(false));
        }
        self.saving = true;
        self.feedback = Some("队伍保存中…".to_owned());
        Ok((submitted, value))
    }

    pub fn begin_team_enabled(
        &mut self,
        team: &TeamDetail,
        enabled: bool,
    ) -> Result<Value, String> {
        if self.saving {
            return Err("队伍正在保存，请稍候".to_owned());
        }
        if self.team_toggle_in_flight.is_some() {
            return Err("队伍启用状态正在更新，请稍候".to_owned());
        }
        if team.id.is_empty() {
            return Err("未保存的队伍不能切换启用状态".to_owned());
        }
        if team.purpose == TeamPurpose::Luxcavation {
            return Err("经验本队伍不参与镜牢启用队列".to_owned());
        }
        self.team_toggle_in_flight = Some(team.id.clone());
        self.feedback = Some(if enabled {
            "正在启用队伍…".to_owned()
        } else {
            "正在停用队伍…".to_owned()
        });
        Ok(json!({"id": team.id, "enabled": enabled}))
    }

    pub fn set_team_enabled(&mut self, team: &TeamDetail, enabled: bool) -> Result<(), String> {
        let params = self.begin_team_enabled(team, enabled)?;
        self.rpc.submit(method::TEAM_SAVE, Some(params));
        Ok(())
    }

    /// Apply a `team.save`, `team.delete` or follow-up `team.list` response
    /// from the shared completion pump.
    pub(crate) fn apply_team_completion(
        &mut self,
        method_name: &str,
        result: Result<Option<Value>, crate::ipc::RpcError>,
    ) -> bool {
        match method_name {
            method::TEAM_SAVE => self.apply_team_save_result(result),
            method::TEAM_DELETE => self.apply_team_delete_result(result),
            method::TEAM_LIST => self.apply_team_list_result(result),
            _ => false,
        }
    }

    fn apply_team_save_result(
        &mut self,
        result: Result<Option<Value>, crate::ipc::RpcError>,
    ) -> bool {
        let value = match result {
            Err(error) => {
                if self.pending_save.take().is_some() {
                    self.fail_save(error.message);
                } else {
                    self.fail_team_enabled(error.message);
                }
                return true;
            }
            Ok(value) => value,
        };
        let Some(value) = value else {
            let error = "team.save 返回了空结果".to_owned();
            if self.pending_save.take().is_some() {
                self.fail_save(error);
            } else {
                self.fail_team_enabled(error);
            }
            return true;
        };
        if value.as_bool() == Some(true) {
            // The backend acknowledged without echoing the entity.  Refresh
            // the list and finish the pending operation in that completion.
            self.rpc.submit(method::TEAM_LIST, None);
            return true;
        }
        if let Some(submitted) = self.pending_save.take() {
            match serde_json::from_value::<TeamDetail>(value) {
                Ok(saved) => self.apply_saved_team(&submitted, saved),
                Err(error) => self.fail_save(format!("team.save 返回了无效队伍：{error}")),
            }
            return true;
        }
        if let Some(team_id) = self.team_toggle_in_flight.clone() {
            match serde_json::from_value::<TeamDetail>(value) {
                Ok(mut saved) => {
                    if saved.id.is_empty() {
                        saved.id = team_id;
                    }
                    self.apply_team_enabled(saved);
                }
                Err(error) => self.fail_team_enabled(format!("team.save 返回了无效队伍：{error}")),
            }
            return true;
        }
        false
    }

    fn apply_team_list_result(
        &mut self,
        result: Result<Option<Value>, crate::ipc::RpcError>,
    ) -> bool {
        let teams = match result {
            Err(error) => {
                if self.pending_save.is_some() {
                    self.fail_save(error.message);
                } else if self.team_toggle_in_flight.is_some() {
                    self.fail_team_enabled(error.message);
                }
                return true;
            }
            Ok(value) => value
                .and_then(|value| serde_json::from_value::<Vec<TeamDetail>>(value).ok())
                .unwrap_or_default(),
        };
        self.teams = teams;
        if let Some(submitted) = self.pending_save.take() {
            match resolve_saved_team(&self.teams, &submitted) {
                Ok(saved) => self.apply_saved_team(&submitted, saved),
                Err(error) => self.fail_save(error),
            }
            return true;
        }
        if let Some(team_id) = self.team_toggle_in_flight.take() {
            if let Some(saved) = self.teams.iter().find(|team| team.id == team_id).cloned() {
                self.apply_team_enabled(saved);
            } else {
                self.fail_team_enabled("team.save 已成功，但无法从列表定位队伍".to_owned());
            }
            return true;
        }
        false
    }

    fn apply_team_delete_result(
        &mut self,
        result: Result<Option<Value>, crate::ipc::RpcError>,
    ) -> bool {
        match result {
            Err(error) => {
                self.fail_delete(error.message);
                true
            }
            Ok(Some(value)) if value.as_bool() == Some(true) => {
                if let Some(team) = self.delete_target.take() {
                    self.apply_deleted_team(&team.id);
                } else {
                    self.deleting = false;
                }
                true
            }
            Ok(_) => {
                self.fail_delete("team.delete 返回了无效结果".to_owned());
                true
            }
        }
    }

    pub fn apply_team_enabled(&mut self, saved: TeamDetail) {
        self.team_toggle_in_flight = None;
        let team_id = saved.id.clone();
        if let Some(existing) = self.teams.iter_mut().find(|team| team.id == team_id) {
            *existing = saved.clone();
        }
        if let Some(editor) = self
            .editor
            .as_mut()
            .filter(|editor| editor.team.id == team_id)
        {
            editor.team.enabled = saved.enabled;
        }
        self.feedback = Some(if saved.enabled {
            "队伍已启用".to_owned()
        } else {
            "队伍已停用".to_owned()
        });
    }

    pub fn fail_team_enabled(&mut self, error: String) {
        self.team_toggle_in_flight = None;
        self.feedback = Some(error);
    }

    pub fn team_toggle_busy(&self) -> bool {
        self.team_toggle_in_flight.is_some()
    }

    pub fn apply_saved_team(&mut self, submitted: &TeamDetail, saved: TeamDetail) {
        self.saving = false;
        if let Some(existing) = self.teams.iter_mut().find(|item| {
            item.id == saved.id || (!submitted.id.is_empty() && item.id == submitted.id)
        }) {
            *existing = saved;
        } else {
            self.teams.push(saved);
        }
        self.editor = None;
        self.feedback = Some("队伍已保存".to_owned());
    }

    pub fn fail_save(&mut self, error: String) {
        self.saving = false;
        self.feedback = Some(error);
    }

    pub fn prepare_delete(&mut self) -> Result<TeamDetail, String> {
        if self.deleting {
            return Err("队伍正在删除，请稍候".to_owned());
        }
        let Some(team) = self.delete_target.clone() else {
            return Err("当前没有待删除的队伍".to_owned());
        };
        self.deleting = true;
        self.feedback = Some("队伍删除中…".to_owned());
        Ok(team)
    }

    pub fn apply_deleted_team(&mut self, team_id: &str) {
        self.deleting = false;
        self.delete_target = None;
        self.teams.retain(|item| item.id != team_id);
        self.feedback = Some("队伍已删除".to_owned());
    }

    pub fn fail_delete(&mut self, error: String) {
        self.deleting = false;
        self.feedback = Some(error);
    }

    pub fn request_delete(&mut self, team: TeamDetail) {
        if self.deleting {
            return;
        }
        self.close_select();
        self.delete_target = Some(team);
    }

    pub fn cancel_delete(&mut self) {
        if !self.deleting {
            self.delete_target = None;
        }
    }

    pub fn confirm_delete(&mut self) -> Result<(), String> {
        let team = self.prepare_delete()?;
        self.rpc
            .submit(method::TEAM_DELETE, Some(json!({"id": team.id})));
        Ok(())
    }

    #[allow(dead_code)]
    pub fn take_feedback(&mut self) -> Option<String> {
        self.feedback.take()
    }
}

fn resolve_saved_team(teams: &[TeamDetail], submitted: &TeamDetail) -> Result<TeamDetail, String> {
    let exact = teams
        .iter()
        .rev()
        .find(|team| {
            team.id.as_str() == submitted.id.as_str()
                || (submitted.id.is_empty()
                    && team.name == submitted.name
                    && team.purpose == submitted.purpose
                    && team.sinners == submitted.sinners)
        })
        .cloned();
    exact
        .or_else(|| {
            teams
                .iter()
                .rev()
                .find(|team| {
                    submitted.id.is_empty()
                        && team.name == submitted.name
                        && team.purpose == submitted.purpose
                })
                .cloned()
        })
        .ok_or_else(|| "team.save 已成功，但无法从列表定位队伍".to_owned())
}
