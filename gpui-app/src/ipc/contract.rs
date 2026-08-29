#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};

pub type RequestId = u64;
pub const RPC_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub id: RequestId,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

impl RpcRequest {
    pub fn new(id: RequestId, method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            method: method.into(),
            params,
        }
    }
    pub fn without_params(id: RequestId, method: impl Into<String>) -> Self {
        Self::new(id, method, None)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RpcResponse {
    pub jsonrpc: String,
    pub id: RequestId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<RpcError>,
}

impl RpcResponse {
    pub fn success(id: RequestId, result: impl Into<Value>) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result.into()),
            error: None,
        }
    }
    pub fn failure(id: RequestId, error: RpcError) -> Self {
        Self {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(error),
        }
    }
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}
impl RpcError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }
    pub fn with_data(code: i32, message: impl Into<String>, data: Value) -> Self {
        Self {
            code,
            message: message.into(),
            data: Some(data),
        }
    }
    pub fn method_not_found(method: &str) -> Self {
        Self::new(-32601, format!("Method not found: {method}"))
    }
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self::new(-32602, message)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct EventEnvelope {
    pub event: String,
    pub payload: Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seq: Option<u64>,
}
impl EventEnvelope {
    pub fn new(event: impl Into<String>, payload: impl Into<Value>) -> Self {
        Self {
            event: event.into(),
            payload: payload.into(),
            seq: None,
        }
    }

    pub fn with_sequence(mut self, sequence: u64) -> Self {
        self.seq = Some(sequence);
        self
    }
}

#[derive(Debug, Default)]
pub struct RequestSequence(AtomicU64);
impl RequestSequence {
    pub fn next(&self) -> RequestId {
        self.0.fetch_add(1, Ordering::Relaxed) + 1
    }
    pub fn request(&self, method: impl Into<String>, params: Option<Value>) -> RpcRequest {
        RpcRequest::new(self.next(), method, params)
    }
}

pub mod method {
    pub const APP_PING: &str = "app.ping";
    pub const APP_VERSION: &str = "app.version";
    pub const APP_CHECK_UPDATE: &str = "app.checkUpdate";
    pub const STATS_GET_SUMMARY: &str = "stats.getSummary";
    pub const STATS_GET_DAILY_SUMMARY: &str = "stats.getDailySummary";
    pub const TASKS_GET_CONFIG: &str = "tasks.getConfig";
    pub const TASKS_SET_CONFIG: &str = "tasks.setConfig";
    pub const EXECUTION_GET_STATE: &str = "execution.getState";
    pub const EXECUTION_START: &str = "execution.start";
    pub const EXECUTION_STOP: &str = "execution.stop";
    pub const EXECUTION_PAUSE: &str = "execution.pause";
    pub const EXECUTION_RESUME: &str = "execution.resume";
    pub const TEAM_LIST: &str = "team.list";
    pub const TEAM_SAVE: &str = "team.save";
    pub const TEAM_DELETE: &str = "team.delete";
    pub const SINNER_LIST: &str = "sinner.list";
    pub const THEME_PACK_LIST: &str = "themePack.list";
    pub const THEME_PACK_UPDATE_ALL: &str = "themePack.updateAll";
    pub const THEME_PACK_RESET_WEIGHTS: &str = "themePack.resetWeights";
    pub const RESOURCE_STATUS: &str = "resource.status";
    pub const RESOURCE_CHECK_UPDATE: &str = "resource.checkUpdate";
    pub const RESOURCE_SYNC_START: &str = "resource.sync.start";
    pub const TOOL_START: &str = "tool.start";
    pub const TOOL_STOP: &str = "tool.stop";
    pub const TOOL_SCREENSHOT: &str = "tool.screenshot";
    pub const HOTKEY_GET: &str = "hotkey.get";
    pub const HOTKEY_SET: &str = "hotkey.set";
    pub const SYSTEM_SETTINGS_GET: &str = "systemSettings.get";
    pub const SYSTEM_SETTINGS_SET: &str = "systemSettings.set";
    pub const DEVICE_LIST: &str = "device.list";
    pub const DEVICE_CONNECT: &str = "device.connect";
    pub const DEVICE_DISCONNECT: &str = "device.disconnect";
}

pub mod event {
    pub const SCREENSHOT_FRAME: &str = "screenshot.frame";
    pub const PREVIEW_STATUS: &str = "preview.status";
    pub const EXECUTION_STATUS: &str = "execution.status";
    pub const EXECUTION_MIRROR_PROGRESS: &str = "execution.mirrorProgress";
    pub const EXECUTION_STATS: &str = "execution.stats";
    pub const TOOL_STATUS: &str = "tool.status";
    pub const DEVICE_STATUS: &str = "device.status";
    pub const LOG_ENTRY: &str = "log.entry";
    pub const RESOURCE_SYNC_PROGRESS: &str = "resource.sync.progress";
    pub const APP_NOTICE: &str = "app.notice";
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn request_and_event_use_canonical_json() {
        let request = RpcRequest::without_params(7, method::APP_PING);
        assert_eq!(
            serde_json::to_string(&request).unwrap(),
            r#"{"jsonrpc":"2.0","id":7,"method":"app.ping"}"#
        );
        let event = EventEnvelope::new(event::APP_NOTICE, serde_json::json!({"level":"info"}));
        assert_eq!(event.event, "app.notice");
    }
    #[test]
    fn sequence_is_monotonic() {
        let sequence = RequestSequence::default();
        assert_eq!(sequence.next(), 1);
        assert_eq!(sequence.next(), 2);
    }
}
