#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};

pub type RequestId = u64;
/// Version of the external GPUI ↔ sidecar JSON-RPC contract.
///
/// Runner IPC has its own, independent protocol version.  Keep this value in
/// the transport contract so the WebSocket handshake and the deterministic
/// mock advertise the same schema.
pub const RPC_SCHEMA_VERSION: u32 = 3;

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
    #[serde(skip)]
    pub binary: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RpcCompletion {
    pub method: String,
    pub params: Option<Value>,
    pub response: RpcResponse,
}
impl EventEnvelope {
    pub fn new(event: impl Into<String>, payload: impl Into<Value>) -> Self {
        Self {
            event: event.into(),
            payload: payload.into(),
            seq: None,
            binary: None,
        }
    }

    pub fn with_sequence(mut self, sequence: u64) -> Self {
        self.seq = Some(sequence);
        self
    }

    pub fn with_binary(mut self, binary: Vec<u8>) -> Self {
        self.binary = Some(binary);
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
    pub const TEAM_STATS_GET: &str = "team.stats.get";
    pub const TEAM_STATS_CLEAR: &str = "team.stats.clear";
    pub const TEAM_PRESET_LIST: &str = "team.preset.list";
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
    pub const TOOL_RESOLUTION_SET: &str = "tool.resolution.set";
    pub const TOOL_RESOLUTION_RESET: &str = "tool.resolution.reset";
    pub const HOTKEY_GET: &str = "hotkey.get";
    pub const HOTKEY_SET: &str = "hotkey.set";
    pub const SYSTEM_SETTINGS_GET: &str = "systemSettings.get";
    pub const SYSTEM_SETTINGS_SET: &str = "systemSettings.set";
    pub const NOTIFICATION_TEST: &str = "notification.test";
    pub const PREVIEW_SET_ENABLED: &str = "preview.setEnabled";
    pub const DEVICE_LIST: &str = "device.list";
    pub const DEVICE_CONNECT: &str = "device.connect";
    pub const DEVICE_DISCONNECT: &str = "device.disconnect";
}

pub mod event {
    pub const SCREENSHOT_FRAME: &str = "screenshot.frame";
    pub const PREVIEW_STATUS: &str = "preview.status";
    pub const EXECUTION_STATUS: &str = "execution.status";
    pub const EXECUTION_MIRROR_PROGRESS: &str = "execution.mirrorProgress";
    pub const EXECUTION_MIRROR_FLOOR: &str = "execution.mirrorFloor";
    pub const EXECUTION_STATS: &str = "execution.stats";
    pub const TOOL_STATUS: &str = "tool.status";
    pub const DEVICE_STATUS: &str = "device.status";
    pub const LOG_ENTRY: &str = "log.entry";
    pub const RESOURCE_SYNC_PROGRESS: &str = "resource.sync.progress";
    pub const APP_NOTICE: &str = "app.notice";
    pub const APP_EXIT_REQUESTED: &str = "app.exitRequested";
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        ExecutionStatsPayload, ExecutionStatusPayload, LogEntryPayload, PreviewStatusPayload,
        ScreenshotFrame,
    };

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

    #[test]
    fn python_schema_three_fixtures_decode_at_the_rust_boundary() {
        let status = serde_json::from_str::<ExecutionStatusPayload>(
            r#"{
                "schemaVersion":3,
                "state":"restoring",
                "stateRevision":27,
                "currentTaskId":"mirror",
                "runId":"run-fixture",
                "runnerPid":1234,
                "deviceLease":"restoring",
                "outcome":"completed",
                "forced":false,
                "requestedBy":null,
                "error":null,
                "deviceRestore":"pending"
            }"#,
        )
        .expect("Python execution.status fixture should decode");
        assert_eq!(status.schemaVersion, 3);
        assert_eq!(status.stateRevision, 27);
        assert_eq!(status.runId.as_deref(), Some("run-fixture"));

        let stats = serde_json::from_str::<ExecutionStatsPayload>(
            r#"{
                "schemaVersion":3,
                "currentRun":{
                    "runId":"run-fixture",
                    "state":"running",
                    "currentTaskId":"mirror",
                    "startedAt":1725000000000,
                    "targets":{"exp":0,"thread":0,"mirror":1},
                    "completed":{"exp":0,"thread":0,"mirror":0},
                    "isMirrorInfinite":false,
                    "updatedAt":1725000000000
                },
                "lastMirror":null,
                "mirrorHistory":[],
                "today":{"exp":0,"thread":0,"mirror":0},
                "week":{"exp":0,"thread":0,"mirror":0},
                "updatedAt":1725000000000
            }"#,
        )
        .expect("Python execution.stats fixture should decode");
        assert_eq!(stats.currentRun.runId.as_deref(), Some("run-fixture"));

        let preview = serde_json::from_str::<PreviewStatusPayload>(
            r#"{"deviceId":"pc:limbus","runId":"run-fixture","generation":9,"status":"running"}"#,
        )
        .expect("Python preview.status fixture should decode");
        assert_eq!(preview.deviceId.as_deref(), Some("pc:limbus"));
        assert_eq!(preview.generation, Some(9));

        let frame = serde_json::from_str::<ScreenshotFrame>(
            r#"{"instanceId":"pc:limbus","jpeg":[255,216,255,217],"width":2,"height":1,"deviceId":"pc:limbus","runId":"run-fixture","generation":9}"#,
        )
        .expect("Python screenshot.frame fixture should decode");
        assert_eq!(frame.deviceId.as_deref(), Some("pc:limbus"));
        assert_eq!(frame.runId.as_deref(), Some("run-fixture"));
        assert_eq!(frame.generation, Some(9));

        let log = serde_json::from_str::<LogEntryPayload>(
            r#"{"ts":1725000000000,"level":"warn","message":"runner warning","runId":"run-fixture"}"#,
        )
        .expect("Python log.entry fixture should decode");
        assert_eq!(log.runId.as_deref(), Some("run-fixture"));
    }

    #[test]
    fn python_structured_error_fixture_preserves_data_code() {
        let response = serde_json::from_str::<RpcResponse>(
            r#"{
                "jsonrpc":"2.0",
                "id":8,
                "error":{
                    "code":-32013,
                    "message":"STALE_RUN",
                    "data":{"code":"STALE_RUN","retryable":false,"userMessage":"STALE_RUN","runId":"old-run"}
                }
            }"#,
        )
        .expect("Python structured error fixture should decode");
        let data = response
            .error
            .expect("error should be present")
            .data
            .expect("error data");
        assert_eq!(data.get("code").and_then(Value::as_str), Some("STALE_RUN"));
        assert_eq!(data.get("retryable"), Some(&Value::Bool(false)));
    }
}
