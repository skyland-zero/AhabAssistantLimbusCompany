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
