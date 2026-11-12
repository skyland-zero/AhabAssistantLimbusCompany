//! Tasks and execution payloads.
//!
//! The original GPUI module was one 528-line serde contract with an explicit
//! file-size exception. Here it is split by payload family: the field names,
//! defaults and camelCase mapping stay together per family and are still
//! reviewed against the Python dispatcher.

mod config;
mod enums;
mod execution;

pub use config::{
    AfterCompletionConfig, BuyEnkephalinConfig, DailyTaskConfig, EnabledTasks, GetRewardConfig,
    MirrorConfig, ResonateWithAhabConfig, SetWindowsConfig, TasksConfig,
};
pub use enums::{
    AfterExitAction, AfterPowerAction, DeviceLease, DeviceLeaseState, DeviceRestore,
    DeviceRestoreState, ExecutionError, ExecutionErrorPayload, ExecutionOutcome,
    ExecutionRequestedBy, ExecutionState, FixedTaskId, RequestedBy,
};
pub use execution::{ExecutionStatusPayload, MirrorFloorPayload, MirrorProgressPayload};
