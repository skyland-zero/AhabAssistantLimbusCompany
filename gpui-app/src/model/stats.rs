#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};

use super::tasks::{ExecutionState, FixedTaskId};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct StatCounts {
    pub exp: u32,
    pub thread: u32,
    pub mirror: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MirrorTeamStats {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub number: u32,
    #[serde(default)]
    pub sinners: Vec<String>,
    #[serde(default)]
    pub sinnerNames: Vec<String>,
    #[serde(default)]
    pub sinnerNamesEn: Vec<String>,
    #[serde(default)]
    pub system: String,
    #[serde(default)]
    pub accessoryScheme: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct MirrorCompletionStats {
    pub completedAt: String,
    #[serde(default)]
    pub runId: Option<String>,
    pub totalSeconds: f64,
    pub battleSeconds: f64,
    pub eventSeconds: f64,
    pub shopSeconds: f64,
    pub findRoadSeconds: f64,
    #[serde(default)]
    pub themePackSeconds: f64,
    #[serde(default)]
    pub rewardCardSeconds: f64,
    #[serde(default)]
    pub egoGiftSeconds: f64,
    #[serde(default)]
    pub settlementSeconds: f64,
    #[serde(default)]
    pub otherSeconds: f64,
    pub eventCount: u32,
    #[serde(default)]
    pub failed: Option<bool>,
    #[serde(default)]
    pub failureReason: Option<String>,
    #[serde(default)]
    pub team: Option<MirrorTeamStats>,
    #[serde(default)]
    pub hardMode: bool,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub floorCount: u32,
    #[serde(default)]
    pub routeId: String,
    #[serde(default)]
    pub routeName: String,
    #[serde(default)]
    pub routeNameEn: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CurrentRunStats {
    pub runId: Option<String>,
    pub state: ExecutionState,
    pub currentTaskId: Option<FixedTaskId>,
    pub startedAt: Option<i64>,
    pub targets: StatCounts,
    pub completed: StatCounts,
    pub isMirrorInfinite: bool,
    pub updatedAt: Option<i64>,
}

impl Default for CurrentRunStats {
    fn default() -> Self {
        Self {
            runId: None,
            state: ExecutionState::Idle,
            currentTaskId: None,
            startedAt: None,
            targets: StatCounts::default(),
            completed: StatCounts::default(),
            isMirrorInfinite: false,
            updatedAt: None,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct ExecutionStatsPayload {
    pub schemaVersion: u32,
    pub currentRun: CurrentRunStats,
    #[serde(default)]
    pub lastMirror: Option<MirrorCompletionStats>,
    #[serde(default)]
    pub mirrorHistory: Vec<MirrorCompletionStats>,
    pub today: StatCounts,
    pub week: StatCounts,
    pub updatedAt: i64,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct DailyStatEntry {
    pub date: String,
    pub exp: u32,
    pub thread: u32,
    pub mirror: u32,
    pub total: u32,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct DailyStatsPayload {
    pub schemaVersion: u32,
    pub dateFrom: String,
    pub dateTo: String,
    pub days: Vec<DailyStatEntry>,
    pub updatedAt: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execution_stats_accepts_legacy_payload_without_last_mirror() {
        let mut value = serde_json::to_value(ExecutionStatsPayload::default()).unwrap();
        value.as_object_mut().unwrap().remove("lastMirror");
        value.as_object_mut().unwrap().remove("mirrorHistory");

        let decoded: ExecutionStatsPayload = serde_json::from_value(value).unwrap();

        assert_eq!(decoded.lastMirror, None);
        assert!(decoded.mirrorHistory.is_empty());
    }

    #[test]
    fn execution_stats_round_trips_last_mirror_details() {
        let mut value = serde_json::to_value(ExecutionStatsPayload::default()).unwrap();
        value["lastMirror"] = serde_json::json!({
            "completedAt": "2026-08-31T08:00:00+09:00",
            "runId": "run-1",
            "totalSeconds": 1800.5,
            "battleSeconds": 1200.25,
            "eventSeconds": 180.0,
            "shopSeconds": 90.75,
            "findRoadSeconds": 329.5,
            "eventCount": 4
        });

        let decoded: ExecutionStatsPayload = serde_json::from_value(value).unwrap();
        let record = decoded.lastMirror.unwrap();

        assert_eq!(record.completedAt, "2026-08-31T08:00:00+09:00");
        assert_eq!(record.runId.as_deref(), Some("run-1"));
        assert_eq!(record.eventCount, 4);
        assert_eq!(record.shopSeconds, 90.75);
    }

    #[test]
    fn execution_stats_round_trips_mirror_history_team_context() {
        let mut value = serde_json::to_value(ExecutionStatsPayload::default()).unwrap();
        value["mirrorHistory"] = serde_json::json!([{
            "completedAt": "2026-08-31T08:00:00+09:00",
            "totalSeconds": 1800.5,
            "battleSeconds": 1200.25,
            "eventSeconds": 180.0,
            "shopSeconds": 90.75,
            "findRoadSeconds": 329.5,
            "eventCount": 4,
            "hardMode": true,
            "mode": "hard",
            "floorCount": 15,
            "routeId": "spiderweb_family_route",
            "routeName": "蜘蛛巢默认路线",
            "routeNameEn": "Spiderweb default route",
            "team": {
                "id": "team-2",
                "name": "蜘蛛巢全家桶",
                "number": 2,
                "sinners": ["faust", "ishmael"],
                "sinnerNames": ["浮士德", "以实玛利"],
                "sinnerNamesEn": ["Faust", "Ishmael"],
                "system": "poise",
                "accessoryScheme": "poise"
            }
        }]);

        let decoded: ExecutionStatsPayload = serde_json::from_value(value).unwrap();
        let record = decoded.mirrorHistory.first().unwrap();
        let team = record.team.as_ref().unwrap();

        assert!(record.hardMode);
        assert_eq!(record.floorCount, 15);
        assert_eq!(record.routeId, "spiderweb_family_route");
        assert_eq!(team.name, "蜘蛛巢全家桶");
        assert_eq!(team.sinners, vec!["faust", "ishmael"]);
        assert_eq!(team.system, "poise");
    }
}
