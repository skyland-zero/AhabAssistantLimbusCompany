//! The saved-team owner: lists, formation slots and mutations.

use serde_json::{Value, json};

use crate::{
    ipc::{BackendClient, RpcGateway, contract::method},
    model::{
        SinnerInfo, TEAM_SLOT_COUNT, TeamDetail, TeamPreset, TeamPurpose, TeamStats,
        team_number_from_id,
    },
};

use super::types::{TeamFilter, TeamSaveInput, TeamSlot};

pub struct TeamsState {
    rpc: RpcGateway,
    pub teams: Vec<TeamDetail>,
    pub sinners: Vec<SinnerInfo>,
    pub presets: Vec<TeamPreset>,
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
        self.teams = teams;
        self.sinners = sinners;
        self.presets = presets;
        self.error = None;
        Ok(())
    }

    pub fn count_for(&self, filter: TeamFilter) -> usize {
        match filter.purpose() {
            None => self.teams.len(),
            Some(purpose) => self
                .teams
                .iter()
                .filter(|team| team.purpose == purpose)
                .count(),
        }
    }

    /// Fixed formation numbers: every slot for "all", 2–20 for mirror,
    /// slot 1 for experience dungeons, and none for general teams.
    pub fn slots_for(&self, filter: TeamFilter) -> Vec<TeamSlot> {
        let numbers: Vec<u32> = match filter {
            TeamFilter::All => (1..=TEAM_SLOT_COUNT).collect(),
            TeamFilter::Mirror => (2..=TEAM_SLOT_COUNT).collect(),
            TeamFilter::Luxcavation => vec![1],
            TeamFilter::General => Vec::new(),
        };
        let purpose = filter.purpose();
        numbers
            .into_iter()
            .filter_map(|number| {
                let team = self
                    .teams
                    .iter()
                    .find(|team| team_number_from_id(&team.id) == Some(number))
                    .cloned();
                if let Some(team) = team.as_ref()
                    && purpose.is_some()
                    && Some(team.purpose) != purpose
                {
                    return None;
                }
                Some(TeamSlot { number, team })
            })
            .collect()
    }

    /// Teams outside the fixed formation numbers the filter shows.
    pub fn extra_teams_for(&self, filter: TeamFilter) -> Vec<TeamDetail> {
        let fixed_numbers: Vec<u32> = match filter {
            TeamFilter::All => (1..=TEAM_SLOT_COUNT).collect(),
            TeamFilter::Mirror => (2..=TEAM_SLOT_COUNT).collect(),
            TeamFilter::Luxcavation => vec![1],
            TeamFilter::General => Vec::new(),
        };
        let purpose = filter.purpose();
        self.teams
            .iter()
            .filter(|team| purpose.is_none() || Some(team.purpose) == purpose)
            .filter(|team| {
                team_number_from_id(&team.id)
                    .map(|number| !fixed_numbers.contains(&number))
                    .unwrap_or(true)
            })
            .cloned()
            .collect()
    }

    /// Create or update one team and return the stored payload.
    pub fn save(&mut self, input: TeamSaveInput) -> Result<TeamDetail, String> {
        let mut submitted = input.team;
        if submitted.purpose == TeamPurpose::Luxcavation {
            submitted.enabled = false;
            submitted.mirrorConfig = None;
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
        Ok(saved)
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
        self.teams.retain(|team| team.id != team_id);
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
            return Err("经验本队伍不参与镜牢启用队列".to_owned());
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
        Ok(saved)
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
