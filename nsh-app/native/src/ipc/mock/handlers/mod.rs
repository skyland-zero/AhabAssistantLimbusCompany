//! The deterministic mock's request router and its shared helpers.

use super::super::contract::{event, method};
use super::presets::builtin_team_presets;
use super::*;

mod device;
mod execution;
mod settings;
mod teams;

impl MockState {
    pub fn call(&mut self, method_name: &str, params: Option<Value>) -> RpcResponse {
        let request = RpcRequest::new(self.sequence.next(), method_name, params);
        self.handle(request)
    }

    pub fn submit(&mut self, method_name: &str, params: Option<Value>) {
        let request = RpcRequest::new(self.sequence.next(), method_name, params);
        let request_id = request.id;
        let method = request.method.clone();
        let params = request.params.clone();
        let response = self.handle(request);
        if self.completions.len() >= 512 {
            self.completions.pop_front();
        }
        self.completions
            .push_back(super::super::contract::RpcCompletion {
                request_id,
                method,
                params,
                response,
            });
    }

    pub fn take_events(&mut self) -> Vec<EventEnvelope> {
        std::mem::take(&mut self.events)
    }

    pub fn take_completions(&mut self) -> Vec<super::super::contract::RpcCompletion> {
        self.completions.drain(..).collect()
    }

    fn params<T: serde::de::DeserializeOwned>(
        params: Option<Value>,
        description: &str,
    ) -> Result<T, RpcError> {
        serde_json::from_value(params.unwrap_or(Value::Null))
            .map_err(|error| RpcError::invalid_params(format!("{description}: {error}")))
    }

    pub(super) fn emit<T: Serialize>(&mut self, name: &str, payload: &T) {
        if let Ok(value) = serde_json::to_value(payload) {
            self.event_sequence = self.event_sequence.wrapping_add(1);
            self.events
                .push(EventEnvelope::new(name, value).with_sequence(self.event_sequence.max(1)));
        }
    }

    fn emit_stats(&mut self) {
        let stats = self.stats.clone();
        self.emit(event::EXECUTION_STATS, &stats);
    }

    fn advance_execution_revision(&mut self) {
        self.execution.stateRevision = self.execution.stateRevision.saturating_add(1);
    }

    fn execution_result(&self, accepted: bool, reason: Option<&str>) -> Value {
        let mut result = serde_json::to_value(&self.execution).unwrap_or_else(|_| json!({}));
        if let Some(object) = result.as_object_mut() {
            object.insert("accepted".into(), json!(accepted));
            if let Some(reason) = reason {
                object.insert("reason".into(), json!(reason));
            }
        }
        result
    }

    fn requested_run_id(params: Option<&Value>) -> Option<&str> {
        params
            .and_then(Value::as_object)
            .and_then(|params| params.get("runId"))
            .and_then(Value::as_str)
    }

    fn validate_run_id(&self, params: Option<&Value>) -> Result<(), RpcError> {
        let Some(requested) = Self::requested_run_id(params) else {
            return Ok(());
        };
        if self.execution.runId.as_deref() == Some(requested) {
            Ok(())
        } else {
            Err(RpcError::with_data(
                -32013,
                "STALE_RUN",
                json!({"code": "STALE_RUN", "runId": requested}),
            ))
        }
    }

    pub(super) fn handle(&mut self, request: RpcRequest) -> RpcResponse {
        let id = request.id;
        let result: Result<Value, RpcError> = (|| match request.method.as_str() {
            method::APP_PING => Ok(json!("pong")),
            method::APP_VERSION => Ok(json!({
                "schemaVersion": 3,
                "ui": "nsh",
                "backend": "mock-1.0.0"
            })),
            method::APP_CHECK_UPDATE => Ok(json!({
                "schemaVersion": 3,
                "status": "up_to_date",
                "updateAvailable": false,
                "latest": env!("CARGO_PKG_VERSION")
            })),
            method::APP_SHUTDOWN => Ok(json!(true)),
            method::APP_DIAGNOSE => Ok(json!({
                "schemaVersion": 3,
                "version": env!("CARGO_PKG_VERSION"),
                "bootId": "mock-boot",
                "pid": std::process::id(),
                "capabilities": [
                    "core.rpc.v3",
                    "app.shutdown",
                    "app.diagnose",
                    "event.bootId",
                    "telemetry.backpressure"
                ],
                "clients": 1,
                "inflightRequests": 0,
                "outboundDepth": 0,
                "droppedEvents": 0,
                "uptimeMs": 0
            })),
            method::STATS_GET_SUMMARY => Ok(serde_json::to_value(&self.stats).unwrap()),
            method::STATS_GET_DAILY_SUMMARY => Ok(serde_json::to_value(DailyStatsPayload {
                schemaVersion: 1,
                dateFrom: "2026-08-25".into(),
                dateTo: "2026-08-31".into(),
                days: vec![
                    DailyStatEntry {
                        date: "2026-08-27".into(),
                        exp: 2,
                        thread: 1,
                        mirror: 0,
                        total: 3,
                    },
                    DailyStatEntry {
                        date: "2026-08-29".into(),
                        exp: 2,
                        thread: 2,
                        mirror: 2,
                        total: 6,
                    },
                    DailyStatEntry {
                        date: "2026-08-30".into(),
                        exp: 1,
                        thread: 1,
                        mirror: 2,
                        total: 4,
                    },
                    DailyStatEntry {
                        date: "2026-08-31".into(),
                        exp: 2,
                        thread: 1,
                        mirror: 2,
                        total: 5,
                    },
                ],
                updatedAt: 0,
            })
            .unwrap()),
            method::TASKS_GET_CONFIG => Ok(serde_json::to_value(&self.tasks).unwrap()),
            method::TASKS_SET_CONFIG => {
                self.tasks = Self::params(request.params, "tasks.setConfig requires TasksConfig")?;
                Ok(json!(true))
            }
            method::EXECUTION_GET_STATE
            | method::EXECUTION_START
            | method::EXECUTION_STOP
            | method::EXECUTION_PAUSE
            | method::EXECUTION_RESUME => self.handle_execution(&request),
            method::TEAM_LIST
            | method::TEAM_SELECTION_GET
            | method::TEAM_SELECTION_SET
            | method::TEAM_STATS_GET
            | method::TEAM_STATS_CLEAR
            | method::TEAM_PRESET_LIST
            | method::TEAM_SAVE
            | method::TEAM_DELETE
            | method::SINNER_LIST => self.handle_teams(&request),
            method::THEME_PACK_LIST
            | method::THEME_PACK_UPDATE_ALL
            | method::THEME_PACK_RESET_WEIGHTS
            | method::RESOURCE_STATUS
            | method::RESOURCE_CHECK_UPDATE
            | method::RESOURCE_SYNC_START
            | method::HOTKEY_GET
            | method::HOTKEY_SET
            | method::SYSTEM_SETTINGS_GET
            | method::SYSTEM_SETTINGS_SET
            | method::NOTIFICATION_TEST
            | method::PREVIEW_SET_ENABLED => self.handle_settings(&request),
            method::TOOL_START
            | method::TOOL_STOP
            | method::TOOL_SCREENSHOT
            | method::TOOL_RESOLUTION_SET
            | method::TOOL_RESOLUTION_RESET
            | method::DEVICE_LIST
            | method::DEVICE_CONNECT
            | method::DEVICE_DISCONNECT => self.handle_device(&request),
            unknown => Err(RpcError::method_not_found(unknown)),
        })();
        match result {
            Ok(value) => RpcResponse::success(id, value),
            Err(error) => RpcResponse::failure(id, error),
        }
    }
}

fn has_executable_task(tasks: &TasksConfig) -> bool {
    first_executable_task(tasks).is_some()
}

fn first_executable_task(tasks: &TasksConfig) -> Option<FixedTaskId> {
    let enabled = &tasks.enabledTasks;
    if enabled.daily_task {
        Some(FixedTaskId::DailyTask)
    } else if enabled.get_reward {
        Some(FixedTaskId::GetReward)
    } else if enabled.buy_enkephalin {
        Some(FixedTaskId::BuyEnkephalin)
    } else if enabled.mirror {
        Some(FixedTaskId::Mirror)
    } else {
        None
    }
}

fn merge_json<T: Serialize + serde::de::DeserializeOwned>(
    current: &T,
    patch: Option<Value>,
    label: &str,
) -> Result<T, RpcError> {
    let mut value = serde_json::to_value(current)
        .map_err(|error| RpcError::invalid_params(format!("{label}: {error}")))?;
    let patch =
        patch.ok_or_else(|| RpcError::invalid_params(format!("{label} requires an object")))?;
    let (Some(base), Some(changes)) = (value.as_object_mut(), patch.as_object()) else {
        return Err(RpcError::invalid_params(format!(
            "{label} requires an object"
        )));
    };
    base.extend(changes.clone());
    serde_json::from_value(value)
        .map_err(|error| RpcError::invalid_params(format!("{label}: {error}")))
}
