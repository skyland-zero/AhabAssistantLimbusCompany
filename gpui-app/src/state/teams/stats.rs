use super::*;

use serde_json::{Value, json};

impl TeamsState {
    pub fn begin_team_stats_load(&mut self) -> Result<Option<(String, Value)>, String> {
        let Some(editor) = self.editor.as_mut() else {
            return Ok(None);
        };
        if editor.team.id.is_empty() || editor.team.purpose == TeamPurpose::Luxcavation {
            return Ok(None);
        }
        if editor.stats_loading || editor.stats_clearing {
            return Ok(None);
        }
        let team_id = editor.team.id.clone();
        editor.stats_loading = true;
        editor.stats_error = None;
        Ok(Some((team_id.clone(), json!({ "id": team_id }))))
    }

    pub fn refresh_team_stats(&mut self) -> Result<(), String> {
        let Some((team_id, params)) = self.begin_team_stats_load()? else {
            return Ok(());
        };
        if self.rpc.is_sidecar() {
            return Ok(());
        }
        let result = self
            .rpc
            .request::<TeamStats>(crate::ipc::contract::method::TEAM_STATS_GET, Some(params))
            .map_err(|error| error.message);
        match result {
            Ok(stats) => {
                self.apply_team_stats(&team_id, stats);
                Ok(())
            }
            Err(error) => {
                self.fail_team_stats(&team_id, error.clone());
                Err(error)
            }
        }
    }

    pub fn apply_team_stats(&mut self, team_id: &str, mut stats: TeamStats) -> bool {
        let Some(editor) = self.editor.as_mut() else {
            return false;
        };
        if editor.team.id != team_id {
            return false;
        }
        if stats.teamId.is_empty() {
            stats.teamId = team_id.to_owned();
        }
        editor.stats = stats;
        editor.stats_loading = false;
        editor.stats_clearing = false;
        editor.stats_error = None;
        true
    }

    pub fn fail_team_stats(&mut self, team_id: &str, error: String) {
        let current_editor = if let Some(editor) = self
            .editor
            .as_mut()
            .filter(|editor| editor.team.id == team_id)
        {
            editor.stats_loading = false;
            editor.stats_clearing = false;
            editor.stats_error = Some(error.clone());
            true
        } else {
            false
        };
        if current_editor {
            self.feedback = Some(error);
        }
    }

    pub fn request_clear_team_stats(&mut self) -> bool {
        let Some(editor) = self.editor.as_mut() else {
            return false;
        };
        if editor.team.id.is_empty()
            || editor.team.purpose == TeamPurpose::Luxcavation
            || editor.stats_loading
            || editor.stats_clearing
        {
            return false;
        }
        editor.stats_clear_confirmation = true;
        self.close_select();
        true
    }

    pub fn cancel_clear_team_stats(&mut self) {
        if let Some(editor) = self.editor.as_mut().filter(|editor| !editor.stats_clearing) {
            editor.stats_clear_confirmation = false;
        }
    }

    pub fn begin_team_stats_clear(&mut self) -> Result<Option<(String, Value)>, String> {
        let Some(editor) = self.editor.as_mut() else {
            return Err("当前没有打开队伍编辑器".to_owned());
        };
        if !editor.stats_clear_confirmation {
            return Err("当前没有待确认的统计清空操作".to_owned());
        }
        if editor.team.id.is_empty() {
            return Err("未保存的队伍没有统计数据".to_owned());
        }
        if editor.stats_loading || editor.stats_clearing {
            return Ok(None);
        }
        let team_id = editor.team.id.clone();
        editor.stats_clear_confirmation = false;
        editor.stats_clearing = true;
        editor.stats_error = None;
        Ok(Some((team_id.clone(), json!({ "id": team_id }))))
    }

    pub fn clear_team_stats(&mut self) -> Result<(), String> {
        let Some((team_id, params)) = self.begin_team_stats_clear()? else {
            return Ok(());
        };
        if self.rpc.is_sidecar() {
            return Ok(());
        }
        let result = self
            .rpc
            .request::<TeamStats>(crate::ipc::contract::method::TEAM_STATS_CLEAR, Some(params))
            .map_err(|error| error.message);
        match result {
            Ok(stats) => {
                self.apply_cleared_team_stats(&team_id, stats);
                Ok(())
            }
            Err(error) => {
                self.fail_team_stats(&team_id, error.clone());
                Err(error)
            }
        }
    }

    pub fn apply_cleared_team_stats(&mut self, team_id: &str, stats: TeamStats) -> bool {
        if !self.apply_team_stats(team_id, stats) {
            return false;
        }
        self.feedback = Some("队伍统计已清空".to_owned());
        true
    }
}
