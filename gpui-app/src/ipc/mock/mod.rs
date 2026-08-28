//! Deterministic in-process RPC backend used by the GPUI migration and tests.
//!
//! State defaults, request handlers, and the public client are separated so
//! adding a new contract method does not require editing one large transport
//! file.

mod client;
mod handlers;
mod state;

pub use client::MockClient;

use super::{
    RpcClient,
    contract::{EventEnvelope, RpcError, RpcRequest, RpcResponse},
};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

// A compact re-export avoids making ipc users know which model submodule owns
// a payload.
mod model_types {
    pub use crate::model::*;
}
use model_types::*;

struct MockState {
    next_team_id: u64,
    sequence: super::contract::RequestSequence,
    tasks: TasksConfig,
    execution: ExecutionStatusPayload,
    teams: Vec<TeamDetail>,
    sinners: Vec<SinnerInfo>,
    packs: ThemePackState,
    resources: Vec<ResourceGroup>,
    hotkey: HotkeyConfig,
    system_settings: SystemSettingsConfig,
    devices: Vec<DeviceInfo>,
    device_status: ConnectionStatus,
    tools: HashMap<ToolId, bool>,
    events: Vec<EventEnvelope>,
}

#[cfg(test)]
mod tests;
