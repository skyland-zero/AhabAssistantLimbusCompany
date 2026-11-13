//! The external JSON-RPC contract's serializable payloads (crate `aalc-model`).
//!
//! These are the field-for-field Rust `serde` types the Python sidecar and the
//! deterministic mock both speak (schema 3). They are pure data: no RPC, no
//! signals, no rendering, and no `nsh-core` dependency, so they build and test
//! on their own (`cargo test -p aalc-model`). The app crate re-exports this
//! crate as `crate::model`, which is the path its code uses.

#![allow(dead_code)]

pub mod other;
pub mod settings;
pub mod stats;
pub mod tasks;
pub mod team_stats;
pub mod teams;

pub use other::{
    AppNotice, ConnectionStatus, DeviceInfo, DeviceKind, DeviceStatusPayload, HotkeyConfig,
    LogEntryPayload, LogLevel, PreviewStatus, PreviewStatusPayload, ResourceGroup, ScreenshotFrame,
    SyncProgressPayload, SystemSettingsConfig, ThemePack, ThemePackState, ToolId,
    ToolStatusPayload, UpdateInfo, UpdateSource,
};
pub use settings::{AppSettings, Language, ThemeMode};
pub use stats::{
    CurrentRunStats, DailyStatEntry, DailyStatsPayload, ExecutionStatsPayload,
    MirrorCompletionStats, MirrorTeamStats, StatCounts,
};
pub use tasks::{
    AfterCompletionConfig, AfterExitAction, AfterPowerAction, BuyEnkephalinConfig, DailyTaskConfig,
    DeviceLease, DeviceLeaseState, DeviceRestore, DeviceRestoreState, EnabledTasks, ExecutionError,
    ExecutionErrorPayload, ExecutionOutcome, ExecutionRequestedBy, ExecutionState,
    ExecutionStatusPayload, FixedTaskId, GetRewardConfig, MirrorConfig, MirrorFloorPayload,
    MirrorProgressPayload, RequestedBy, ResonateWithAhabConfig, SetWindowsConfig, TasksConfig,
};
pub use team_stats::{TeamStats, TeamStatsBucket};
pub use teams::{
    DiscardSystems, LocalizedText, MAX_OBSERVE_EGO_GIFTS, SPIDERWEB_ENTANGLED_IN_RED_GIFT_ID,
    SinnerInfo, TEAM_SLOT_COUNT, TeamDetail, TeamMirrorConfig, TeamPreset, TeamPurpose,
    TeamSummary, team_number_from_id,
};
