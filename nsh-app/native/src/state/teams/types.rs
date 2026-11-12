//! Payloads and small selection types for the teams state.

use nsh_core::prelude::*;
use serde_json::Value;

use crate::model::{TeamDetail, TeamPurpose};

/// One formation filter: all teams or one purpose.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TeamFilter {
    #[default]
    All,
    Mirror,
    Luxcavation,
    General,
}

impl TeamFilter {
    pub fn parse(value: &str) -> Self {
        match value {
            "mirror" => Self::Mirror,
            "luxcavation" => Self::Luxcavation,
            "general" => Self::General,
            _ => Self::All,
        }
    }

    pub const fn purpose(self) -> Option<TeamPurpose> {
        match self {
            Self::All => None,
            Self::Mirror => Some(TeamPurpose::Mirror),
            Self::Luxcavation => Some(TeamPurpose::Luxcavation),
            Self::General => Some(TeamPurpose::General),
        }
    }
}

/// A fixed formation number and the team saved there, if any.
#[derive(Clone, Debug, PartialEq)]
pub struct TeamSlot {
    pub number: u32,
    pub team: Option<TeamDetail>,
}

/// One saved team as TSX sees it.
#[derive(Clone, Debug, JsObject)]
pub struct TeamView {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub sinners: Vec<String>,
    pub purpose: String,
    pub accessory_scheme: String,
    pub enabled: bool,
    pub team_code: String,
    /// The mirror-run configuration, or `null` for an experience-dungeon team.
    pub mirror_config: Value,
}

impl From<&TeamDetail> for TeamView {
    fn from(team: &TeamDetail) -> Self {
        Self {
            schema_version: team.schemaVersion,
            id: team.id.clone(),
            name: team.name.clone(),
            sinners: team.sinners.clone(),
            purpose: match team.purpose {
                TeamPurpose::Mirror => "mirror",
                TeamPurpose::Luxcavation => "luxcavation",
                TeamPurpose::General => "general",
            }
            .into(),
            accessory_scheme: team.accessoryScheme.clone(),
            enabled: team.enabled,
            team_code: team.teamCode.clone(),
            mirror_config: team
                .mirrorConfig
                .as_ref()
                .and_then(|config| serde_json::to_value(config).ok())
                .unwrap_or(Value::Null),
        }
    }
}

/// One fixed formation slot as TSX sees it.
#[derive(Clone, Debug, JsObject)]
pub struct TeamSlotView {
    pub number: u32,
    pub team: Option<TeamView>,
}

impl From<&TeamSlot> for TeamSlotView {
    fn from(slot: &TeamSlot) -> Self {
        Self {
            number: slot.number,
            team: slot.team.as_ref().map(TeamView::from),
        }
    }
}

/// One sinner roster entry.
#[derive(Clone, Debug, JsObject)]
pub struct SinnerView {
    pub id: String,
    pub name: String,
}

/// A team payload from TSX plus the requested number for a new team.
#[derive(Clone, Debug)]
pub struct TeamSaveInput {
    pub team: TeamDetail,
    pub team_number: Option<u32>,
}

impl TeamSaveInput {
    pub fn parse(value: Value) -> Result<Self, String> {
        let team_number = value
            .get("teamNumber")
            .and_then(Value::as_u64)
            .map(|number| number as u32);
        if team_number == Some(0) {
            return Err("队伍编号必须大于 0".to_owned());
        }
        let team: TeamDetail =
            serde_json::from_value(value).map_err(|error| format!("队伍数据无效：{error}"))?;
        if team.name.trim().is_empty() {
            return Err("队伍名称不能为空".to_owned());
        }
        if team.sinners.len() > 12 {
            return Err("队伍最多选择 12 名人格".to_owned());
        }
        Ok(Self { team, team_number })
    }
}
