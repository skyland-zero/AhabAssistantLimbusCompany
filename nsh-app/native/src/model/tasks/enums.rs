//! Task ids, execution states and the after-run actions.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FixedTaskId {
    SetWindows,
    DailyTask,
    GetReward,
    BuyEnkephalin,
    Mirror,
    #[serde(rename = "resonate_with_Ahab")]
    ResonateWithAhab,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionState {
    #[default]
    Idle,
    Starting,
    Running,
    Paused,
    Stopping,
    Restoring,
}

/// Sidecar/device capability ownership for the current execution.
///
/// This is deliberately separate from [`ExecutionState`]: a sidecar can be
/// restoring a lease after the runner has already exited, and a lease can be
/// reserved while execution is still starting.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceLeaseState {
    #[default]
    None,
    Acquiring,
    Runner,
    Restoring,
}

/// Business result of the most recent execution.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionOutcome {
    Completed,
    Stopped,
    Failed,
    Crashed,
}

/// Actor that requested a stop. The field is optional because an idle
/// snapshot has no active stop request.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionRequestedBy {
    User,
    Shutdown,
    Watchdog,
}

/// Device cleanup/reconnection result for the most recent execution.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceRestoreState {
    #[default]
    NotNeeded,
    Pending,
    Restored,
    Disconnected,
    Failed,
}

/// Structured execution failure information supplied by the sidecar.
///
/// The fields inside an error object are defaulted so a partially populated
/// diagnostic from an older/newer sidecar remains readable. The enclosing
/// `error` field is still optional in [`super::ExecutionStatusPayload`].
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ExecutionErrorPayload {
    #[serde(default)]
    pub code: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub phase: String,
    #[serde(default)]
    pub recovery: String,
}

// Short aliases keep the public model ergonomic for callers that use the
// shorter protocol names while retaining explicit state-oriented names in the
// canonical field types.
pub type DeviceLease = DeviceLeaseState;
pub type RequestedBy = ExecutionRequestedBy;
pub type DeviceRestore = DeviceRestoreState;
pub type ExecutionError = ExecutionErrorPayload;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[allow(clippy::enum_variant_names)]
#[serde(rename_all = "snake_case")]
pub enum AfterExitAction {
    ExitGame,
    ExitEmulator,
    ExitAalc,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AfterPowerAction {
    #[default]
    None,
    Sleep,
    Hibernate,
    Lock,
    Shutdown,
}
