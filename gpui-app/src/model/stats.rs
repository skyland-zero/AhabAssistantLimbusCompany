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
pub struct MirrorCompletionStats {
    pub completedAt: String,
    #[serde(default)]
    pub runId: Option<String>,
    pub totalSeconds: f64,
    pub battleSeconds: f64,
    pub eventSeconds: f64,
    pub shopSeconds: f64,
    pub findRoadSeconds: f64,
    pub eventCount: u32,
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

        let decoded: ExecutionStatsPayload = serde_json::from_value(value).unwrap();

        assert_eq!(decoded.lastMirror, None);
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
}
