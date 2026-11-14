//! The saved-team owner: lists, formation slots and mutations.

use serde_json::{Value, json};

use crate::{
    ipc::{BackendClient, RpcGateway, contract::method},
    model::{
        SinnerInfo, TEAM_SLOT_COUNT, TeamDetail, TeamPreset, TeamPurpose, TeamStats,
        team_number_from_id,
    },
};

use super::types::{TeamSaveInput, TeamSlot};

pub struct TeamsState {
    rpc: RpcGateway,
    pub teams: Vec<TeamDetail>,
    pub sinners: Vec<SinnerInfo>,
    pub presets: Vec<TeamPreset>,
    /// `select_team_by_order`: choose the physical list row instead of OCRing
    /// the in-game team name.
    pub select_team_by_order: bool,
    pub error: Option<String>,
}

impl TeamsState {
    /// Load a state from `client`, keeping the first load error for the UI.
    pub fn new(client: impl Into<BackendClient>) -> Self {
        let mut state = Self {
            rpc: RpcGateway::new(client),
            teams: Vec::new(),
            sinners: Vec::new(),
            presets: Vec::new(),
            select_team_by_order: false,
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

    /// Re-read the saved teams, the roster and the built-in presets.
    pub fn refresh(&mut self) -> Result<(), String> {
        let teams: Vec<TeamDetail> = self
            .rpc
            .request(method::TEAM_LIST, None)
            .map_err(|error| error.message)?;
        let sinners: Vec<SinnerInfo> = self
            .rpc
            .request(method::SINNER_LIST, None)
            .map_err(|error| error.message)?;
        let presets: Vec<TeamPreset> = self
            .rpc
            .request(method::TEAM_PRESET_LIST, None)
            .map_err(|error| error.message)?;
        let selection: bool = self
            .rpc
            .request(method::TEAM_SELECTION_GET, None)
            .map_err(|error| error.message)?;
        self.teams = teams;
        self.sinners = sinners;
        self.presets = presets;
        self.select_team_by_order = selection;
        self.error = None;
        Ok(())
    }

    /// Switch between OCR-by-name and physical-row selection.
    pub fn set_selection(&mut self, enabled: bool) -> Result<bool, String> {
        let response: bool = self
            .rpc
            .request(
                method::TEAM_SELECTION_SET,
                Some(json!({ "enabled": enabled })),
            )
            .map_err(|error| error.message)?;
        self.select_team_by_order = response;
        Ok(response)
    }

    /// Every game loadout number, whether or not a team is configured there.
    /// Purpose is a property of the team, not of the slot, so the page filters
    /// configured teams separately.
    pub fn slots(&self) -> Vec<TeamSlot> {
        (1..=TEAM_SLOT_COUNT)
            .map(|number| TeamSlot {
                number,
                team: self
                    .teams
                    .iter()
                    .find(|team| team_number_from_id(&team.id) == Some(number))
                    .cloned(),
            })
            .collect()
    }

    /// Legacy teams whose id falls outside the forty game loadouts. They stay
    /// visible in an "extra" section so old configs are not silently hidden.
    pub fn extra_teams(&self) -> Vec<TeamDetail> {
        self.teams
            .iter()
            .filter(|team| {
                team_number_from_id(&team.id)
                    .map(|number| number > TEAM_SLOT_COUNT)
                    .unwrap_or(true)
            })
            .cloned()
            .collect()
    }

    /// The built-in team templates, as loaded from `team.preset.list`.
    pub fn presets(&self) -> Vec<TeamPreset> {
        self.presets.clone()
    }

    /// Create or update one team and return the stored payload.
    pub fn save(&mut self, input: TeamSaveInput) -> Result<TeamDetail, String> {
        let mut submitted = input.team;
        submitted.queuePosition = 0;
        if submitted.purpose == TeamPurpose::Luxcavation {
            submitted.enabled = false;
            submitted.mirrorConfig = None;
        } else if submitted.purpose == TeamPurpose::General {
            // General teams are manual-only: their configuration stays for a
            // later switch back to mirror, but they never join the queue.
            submitted.enabled = false;
        }
        let mut value =
            serde_json::to_value(&submitted).map_err(|error| format!("队伍数据无效：{error}"))?;
        if submitted.id.is_empty()
            && let Some(number) = input.team_number
            && let Some(object) = value.as_object_mut()
        {
            object.insert("teamNumber".to_owned(), json!(number));
        }
        if submitted.purpose == TeamPurpose::Luxcavation
            && let Some(object) = value.as_object_mut()
        {
            object.insert("mirrorConfig".to_owned(), Value::Null);
            object.insert("enabled".to_owned(), Value::Bool(false));
        }
        if submitted.purpose == TeamPurpose::General
            && let Some(object) = value.as_object_mut()
        {
            object.insert("enabled".to_owned(), Value::Bool(false));
        }

        let response = self
            .rpc
            .request_value(method::TEAM_SAVE, Some(value))
            .map_err(|error| error.message)?;
        let saved = match response {
            Some(Value::Bool(true)) => {
                self.refresh()?;
                resolve_saved_team(&self.teams, &submitted)?
            }
            Some(value) => serde_json::from_value::<TeamDetail>(value)
                .map_err(|error| format!("team.save 返回了无效队伍：{error}"))?,
            None => return Err("team.save 返回了空结果".to_owned()),
        };
        self.apply_saved(&submitted, saved.clone());
        // Enabling, disabling or re-purposing a team shifts the rotation
        // positions of the other teams. Re-read the projection so the page's
        // queue strip is not left with stale `queuePosition` values.
        self.refresh()?;
        Ok(self
            .teams
            .iter()
            .find(|team| team.id == saved.id)
            .cloned()
            .unwrap_or(saved))
    }

    fn apply_saved(&mut self, submitted: &TeamDetail, saved: TeamDetail) {
        if let Some(existing) = self.teams.iter_mut().find(|item| {
            item.id == saved.id || (!submitted.id.is_empty() && item.id == submitted.id)
        }) {
            *existing = saved;
        } else {
            self.teams.push(saved);
        }
    }

    /// Delete one team and its statistics.
    pub fn delete(&mut self, team_id: &str) -> Result<(), String> {
        if team_id.is_empty() {
            return Err("未保存的队伍不能删除".to_owned());
        }
        self.rpc
            .request_value(method::TEAM_DELETE, Some(json!({ "id": team_id })))
            .map_err(|error| error.message)?;
        self.refresh()?;
        Ok(())
    }

    /// Toggle a team's enabled flag through the partial `team.save` update.
    pub fn set_enabled(&mut self, team_id: &str, enabled: bool) -> Result<TeamDetail, String> {
        let existing = self
            .teams
            .iter()
            .find(|team| team.id == team_id)
            .cloned()
            .ok_or_else(|| "未找到要更新的队伍".to_owned())?;
        if existing.purpose == TeamPurpose::Luxcavation {
            return Err("资源本队伍不参与镜牢启用队列".to_owned());
        }
        if existing.purpose == TeamPurpose::General {
            return Err("通用队伍仅手动使用，不参与镜牢启用队列".to_owned());
        }
        let response = self
            .rpc
            .request_value(
                method::TEAM_SAVE,
                Some(json!({ "id": team_id, "enabled": enabled })),
            )
            .map_err(|error| error.message)?;
        let saved = match response {
            Some(Value::Bool(true)) => {
                self.refresh()?;
                self.teams
                    .iter()
                    .find(|team| team.id == team_id)
                    .cloned()
                    .ok_or_else(|| "team.save 已成功，但无法从列表定位队伍".to_owned())?
            }
            Some(value) => serde_json::from_value::<TeamDetail>(value)
                .map_err(|error| format!("team.save 返回了无效队伍：{error}"))?,
            None => return Err("team.save 返回了空结果".to_owned()),
        };
        if let Some(existing) = self.teams.iter_mut().find(|team| team.id == saved.id) {
            *existing = saved.clone();
        }
        // The queue may have shifted other teams' positions.
        self.refresh()?;
        Ok(self
            .teams
            .iter()
            .find(|team| team.id == saved.id)
            .cloned()
            .unwrap_or(saved))
    }

    /// Read one team's accumulated statistics.
    pub fn team_stats(&mut self, team_id: &str) -> Result<TeamStats, String> {
        self.rpc
            .request(method::TEAM_STATS_GET, Some(json!({ "id": team_id })))
            .map_err(|error| error.message)
    }

    /// Clear one team's statistics and return the empty payload.
    pub fn clear_team_stats(&mut self, team_id: &str) -> Result<TeamStats, String> {
        self.rpc
            .request(method::TEAM_STATS_CLEAR, Some(json!({ "id": team_id })))
            .map_err(|error| error.message)
    }
}

fn resolve_saved_team(teams: &[TeamDetail], submitted: &TeamDetail) -> Result<TeamDetail, String> {
    teams
        .iter()
        .rev()
        .find(|team| {
            team.id.as_str() == submitted.id.as_str()
                || (submitted.id.is_empty()
                    && team.name == submitted.name
                    && team.purpose == submitted.purpose
                    && team.sinners == submitted.sinners)
        })
        .cloned()
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
