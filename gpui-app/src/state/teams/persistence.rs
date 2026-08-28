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

    pub fn with_client(client: MockClient) -> Self {
        let mut state = Self {
            rpc: RpcGateway::new(client),
            teams: Vec::new(),
            sinners: Vec::new(),
            filter: TeamFilter::All,
            editor: None,
            delete_target: None,
            open_select: None,
            feedback: None,
        };
        state.reload();
        state
    }

    pub fn reload(&mut self) {
        self.teams = self
            .request_value(method::TEAM_LIST, None)
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default();
        self.sinners = self
            .request_value(method::SINNER_LIST, None)
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default();
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

        let mut mirror = current.mirrorConfig.clone().unwrap_or_default();
        if let Some(mirror_patch) = object.get("mirrorConfig") {
            let Some(patch) = mirror_patch.as_object() else {
                return Err("mirrorConfig 必须是对象".to_owned());
            };
            let mut value = serde_json::to_value(&mirror).map_err(|error| error.to_string())?;
            let Some(base) = value.as_object_mut() else {
                return Err("mirrorConfig 默认值无效".to_owned());
            };
            let mut patch = patch.clone();
            if let Some(discard_patch) = patch.get("discard_systems").and_then(Value::as_object) {
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

        editor.team = TeamDetail {
            id,
            name: name.to_owned(),
            sinners,
            purpose,
            accessoryScheme: accessory_scheme,
            enabled,
            mirrorConfig: Some(mirror),
        };
        editor.json_import_open = false;
        self.feedback = Some("已导入队伍 JSON（尚未保存）".to_owned());
        Ok(())
    }

    pub fn save_editor(&mut self) -> Result<(), String> {
        let Some(editor) = self.editor.as_mut() else {
            return Err("当前没有打开队伍编辑器".to_owned());
        };
        if editor.team.name.trim().is_empty() {
            return Err("队伍名称不能为空".to_owned());
        }
        let value = serde_json::to_value(&editor.team).map_err(|error| error.to_string())?;
        if let Err(error) = self.rpc.request_value(method::TEAM_SAVE, Some(value)) {
            return Err(error.message);
        }
        self.reload_teams_only();
        self.editor = None;
        self.feedback = Some("队伍已保存".to_owned());
        Ok(())
    }

    pub fn request_delete(&mut self, team: TeamDetail) {
        self.close_select();
        self.delete_target = Some(team);
    }

    pub fn cancel_delete(&mut self) {
        self.delete_target = None;
    }

    pub fn confirm_delete(&mut self) -> Result<(), String> {
        let Some(team) = self.delete_target.take() else {
            return Ok(());
        };
        if let Err(error) = self
            .rpc
            .request_value(method::TEAM_DELETE, Some(json!({"id": team.id})))
        {
            self.delete_target = Some(team);
            return Err(error.message);
        }
        self.reload_teams_only();
        self.feedback = Some("队伍已删除".to_owned());
        Ok(())
    }

    #[allow(dead_code)]
    pub fn take_feedback(&mut self) -> Option<String> {
        self.feedback.take()
    }

    fn reload_teams_only(&mut self) {
        self.teams = self
            .request_value(method::TEAM_LIST, None)
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default();
    }

    fn request_value(&mut self, method_name: &str, params: Option<Value>) -> Option<Value> {
        match self.rpc.request_value(method_name, params) {
            Ok(value) => value,
            Err(error) => {
                self.feedback = Some(error.message);
                None
            }
        }
    }
}
