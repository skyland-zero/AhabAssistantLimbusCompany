//! The authoritative execution snapshot and mirror progress events.

#![allow(non_snake_case)]

use serde::{Deserialize, Serialize};

use super::enums::{
    DeviceLeaseState, DeviceRestoreState, ExecutionErrorPayload, ExecutionOutcome,
    ExecutionRequestedBy, ExecutionState, FixedTaskId,
};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ExecutionStatusPayload {
    /// External RPC schema carried by authoritative execution snapshots.
    #[serde(default = "execution_schema_version")]
    pub schemaVersion: u32,
    #[serde(default)]
    pub state: ExecutionState,
    #[serde(default)]
    pub currentTaskId: Option<FixedTaskId>,
    /// Monotonically increasing sidecar revision. Zero is reserved for legacy
    /// payloads that predate schema 3.
    #[serde(default)]
    pub stateRevision: u64,
    #[serde(default)]
    pub runId: Option<String>,
    #[serde(default)]
    pub runnerPid: Option<u32>,
    #[serde(default)]
    pub deviceLease: DeviceLeaseState,
    #[serde(default)]
    pub outcome: Option<ExecutionOutcome>,
    #[serde(default)]
    pub forced: bool,
    #[serde(default)]
    pub requestedBy: Option<ExecutionRequestedBy>,
    #[serde(default)]
    pub error: Option<ExecutionErrorPayload>,
    #[serde(default)]
    pub deviceRestore: DeviceRestoreState,
}

impl Default for ExecutionStatusPayload {
    fn default() -> Self {
        Self {
            schemaVersion: execution_schema_version(),
            state: ExecutionState::Idle,
            currentTaskId: None,
            stateRevision: 0,
            runId: None,
            runnerPid: None,
            deviceLease: DeviceLeaseState::None,
            outcome: None,
            forced: false,
            requestedBy: None,
            error: None,
            deviceRestore: DeviceRestoreState::NotNeeded,
        }
    }
}

fn execution_schema_version() -> u32 {
    3
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct MirrorProgressPayload {
    pub current: u32,
    pub total: u32,
    pub isHard: bool,
    pub isInfinite: bool,
    #[serde(default)]
    pub runId: Option<String>,
}

/// Current in-run floor reported by the automation (1-based).
/// `floorTotal` is the route length (5/15), independent from the run-count
/// `total` in [`MirrorProgressPayload`]. All fields defaulted so payloads from
/// older backends (or mock events) still deserialize.
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct MirrorFloorPayload {
    #[serde(default)]
    pub floor: u32,
    #[serde(default)]
    pub floorTotal: u32,
    #[serde(default)]
    pub runId: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execution_status_defaults_schema_three_fields_for_legacy_payloads() {
        let status: ExecutionStatusPayload =
            serde_json::from_value(serde_json::json!({"state": "starting"})).unwrap();

        assert_eq!(status.schemaVersion, 3);
        assert_eq!(status.state, ExecutionState::Starting);
        assert_eq!(status.stateRevision, 0);
        assert_eq!(status.runId, None);
        assert_eq!(status.runnerPid, None);
        assert_eq!(status.deviceLease, DeviceLeaseState::None);
        assert_eq!(status.outcome, None);
        assert!(!status.forced);
        assert_eq!(status.requestedBy, None);
        assert_eq!(status.error, None);
        assert_eq!(status.deviceRestore, DeviceRestoreState::NotNeeded);
    }

    #[test]
    fn execution_status_round_trips_runner_and_recovery_details() {
        let status = ExecutionStatusPayload {
            state: ExecutionState::Restoring,
            stateRevision: 27,
            runId: Some("run-27".into()),
            runnerPid: Some(1234),
            deviceLease: DeviceLeaseState::Restoring,
            outcome: Some(ExecutionOutcome::Stopped),
            forced: true,
            requestedBy: Some(ExecutionRequestedBy::User),
            error: Some(ExecutionErrorPayload {
                code: "DEVICE_RESTORE_FAILED".into(),
                message: "restore failed".into(),
                phase: "restoring".into(),
                recovery: "reconnect_device".into(),
            }),
            deviceRestore: DeviceRestoreState::Failed,
            ..ExecutionStatusPayload::default()
        };
        let encoded = serde_json::to_value(&status).unwrap();
        assert_eq!(encoded["schemaVersion"], 3);
        assert_eq!(encoded["state"], "restoring");
        assert_eq!(encoded["deviceLease"], "restoring");
        assert_eq!(
            serde_json::from_value::<ExecutionStatusPayload>(encoded).unwrap(),
            status
        );
    }
}
