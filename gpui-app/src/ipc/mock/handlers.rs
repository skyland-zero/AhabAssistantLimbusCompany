use super::*;

use super::super::contract::{event, method};

impl MockState {
    pub fn call(&mut self, method_name: &str, params: Option<Value>) -> RpcResponse {
        let request = RpcRequest::new(self.sequence.next(), method_name, params);
        self.handle(request)
    }

    pub fn take_events(&mut self) -> Vec<EventEnvelope> {
        std::mem::take(&mut self.events)
    }

    fn params<T: serde::de::DeserializeOwned>(
        params: Option<Value>,
        description: &str,
    ) -> Result<T, RpcError> {
        serde_json::from_value(params.unwrap_or(Value::Null))
            .map_err(|error| RpcError::invalid_params(format!("{description}: {error}")))
    }

    fn emit<T: Serialize>(&mut self, name: &str, payload: &T) {
        if let Ok(value) = serde_json::to_value(payload) {
            self.events.push(EventEnvelope::new(name, value));
        }
    }

    fn emit_stats(&mut self) {
        let stats = self.stats.clone();
        self.emit(event::EXECUTION_STATS, &stats);
    }

    pub(super) fn handle(&mut self, request: RpcRequest) -> RpcResponse {
        let id = request.id;
        let result: Result<Value, RpcError> = (|| match request.method.as_str() {
            method::APP_PING => Ok(json!("pong")),
            method::APP_VERSION => Ok(json!({
                "schemaVersion": 2,
                "ui": env!("CARGO_PKG_VERSION"),
                "backend": "mock-1.0.0"
            })),
            method::APP_CHECK_UPDATE => Ok(json!({
                "schemaVersion": 2,
                "status": "up_to_date",
                "updateAvailable": false,
                "latest": env!("CARGO_PKG_VERSION")
            })),
            method::STATS_GET_SUMMARY => Ok(serde_json::to_value(&self.stats).unwrap()),
            method::STATS_GET_DAILY_SUMMARY => Ok(serde_json::to_value(DailyStatsPayload {
                schemaVersion: 1,
                dateFrom: String::new(),
                dateTo: String::new(),
                days: Vec::new(),
                updatedAt: 0,
            })
            .unwrap()),
            method::TASKS_GET_CONFIG => Ok(serde_json::to_value(&self.tasks).unwrap()),
            method::TASKS_SET_CONFIG => {
                self.tasks = Self::params(request.params, "tasks.setConfig requires TasksConfig")?;
                Ok(json!(true))
            }
            method::EXECUTION_GET_STATE => Ok(serde_json::to_value(&self.execution).unwrap()),
            method::EXECUTION_START => {
                if self.execution.state != ExecutionState::Idle {
                    Err(RpcError::new(-32010, "execution is not idle"))
                } else if !has_executable_task(&self.tasks) {
                    // A direct IPC caller must observe the same no-op behavior as
                    // HomeState: window settings and Ahab resonance are not
                    // executable tasks on their own.
                    Ok(json!({
                        "accepted": false,
                        "runId": null,
                        "state": "idle",
                        "reason": "没有选择可执行任务"
                    }))
                } else {
                    let task = request
                        .params
                        .and_then(|value| value.get("taskId").cloned())
                        .and_then(|value| serde_json::from_value(value).ok())
                        .or_else(|| first_executable_task(&self.tasks));
                    self.execution = ExecutionStatusPayload {
                        state: ExecutionState::Running,
                        currentTaskId: task,
                    };
                    let infinite =
                        self.tasks.enabledTasks.mirror && self.tasks.mirror.infinite_dungeons;
                    self.stats = ExecutionStatsPayload {
                        schemaVersion: 1,
                        currentRun: CurrentRunStats {
                            runId: Some("mock-run".into()),
                            state: ExecutionState::Running,
                            currentTaskId: self.execution.currentTaskId,
                            startedAt: Some(0),
                            targets: StatCounts {
                                exp: if self.tasks.enabledTasks.daily_task {
                                    self.tasks.daily_task.set_EXP_count.into()
                                } else {
                                    0
                                },
                                thread: if self.tasks.enabledTasks.daily_task {
                                    self.tasks.daily_task.set_thread_count.into()
                                } else {
                                    0
                                },
                                mirror: if self.tasks.enabledTasks.mirror && !infinite {
                                    self.tasks.mirror.set_mirror_count.into()
                                } else {
                                    0
                                },
                            },
                            completed: StatCounts::default(),
                            isMirrorInfinite: infinite,
                            updatedAt: Some(0),
                        },
                        today: self.stats.today.clone(),
                        week: self.stats.week.clone(),
                        updatedAt: 0,
                    };
                    self.emit_stats();
                    let status = self.execution.clone();
                    self.emit(event::EXECUTION_STATUS, &status);
                    if self.tasks.enabledTasks.mirror {
                        self.emit(
                            event::EXECUTION_MIRROR_PROGRESS,
                            &MirrorProgressPayload {
                                current: 1,
                                total: if self.tasks.mirror.infinite_dungeons {
                                    9999
                                } else {
                                    u32::from(self.tasks.mirror.set_mirror_count)
                                },
                                isHard: self.tasks.mirror.hard_mirror,
                                isInfinite: self.tasks.mirror.infinite_dungeons,
                            },
                        );
                    }
                    Ok(json!({
                        "accepted": true,
                        "runId": "mock-run",
                        "state": "running"
                    }))
                }
            }
            method::EXECUTION_STOP => {
                self.execution = ExecutionStatusPayload::default();
                self.stats.currentRun.state = ExecutionState::Idle;
                self.stats.currentRun.currentTaskId = None;
                self.emit_stats();
                let status = self.execution.clone();
                self.emit(event::EXECUTION_STATUS, &status);
                Ok(json!({
                    "accepted": true,
                    "runId": null,
                    "state": "idle"
                }))
            }
            method::EXECUTION_PAUSE => {
                if self.execution.state != ExecutionState::Running {
                    Err(RpcError::new(-32011, "execution is not running"))
                } else {
                    self.execution.state = ExecutionState::Paused;
                    self.stats.currentRun.state = ExecutionState::Paused;
                    self.emit_stats();
                    let status = self.execution.clone();
                    self.emit(event::EXECUTION_STATUS, &status);
                    Ok(json!({
                        "accepted": true,
                        "runId": "mock-run",
                        "state": "paused"
                    }))
                }
            }
            method::EXECUTION_RESUME => {
                if self.execution.state != ExecutionState::Paused {
                    Err(RpcError::new(-32012, "execution is not paused"))
                } else {
                    self.execution.state = ExecutionState::Running;
                    self.stats.currentRun.state = ExecutionState::Running;
                    self.emit_stats();
                    let status = self.execution.clone();
                    self.emit(event::EXECUTION_STATUS, &status);
                    Ok(json!({
                        "accepted": true,
                        "runId": "mock-run",
                        "state": "running"
                    }))
                }
            }
            method::TEAM_LIST => Ok(serde_json::to_value(&self.teams).unwrap()),
            method::TEAM_SAVE => {
                let mut team: TeamDetail =
                    Self::params(request.params, "team.save requires TeamDetail")?;
                if team.name.trim().is_empty() {
                    Err(RpcError::invalid_params("team name required"))
                } else {
                    if team.id.is_empty() {
                        team.id = format!("team-{}", self.next_team_id);
                        self.next_team_id += 1;
                    }
                    if team.purpose == TeamPurpose::Luxcavation {
                        team.enabled = false;
                        team.mirrorConfig = None;
                    } else if team.mirrorConfig.is_none()
                        && let Some(existing) = self.teams.iter().find(|entry| entry.id == team.id)
                    {
                        team.mirrorConfig = existing.mirrorConfig.clone();
                    }
                    if let Some(existing) = self.teams.iter_mut().find(|entry| entry.id == team.id)
                    {
                        *existing = team.clone();
                    } else {
                        self.teams.push(team.clone());
                    }
                    Ok(json!(team))
                }
            }
            method::TEAM_DELETE => {
                let id_value: Value = Self::params(request.params, "team.delete requires {id}")?;
                let team_id = id_value
                    .get("id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| RpcError::invalid_params("team.delete requires a string id"))?;
                self.teams.retain(|team| team.id != team_id);
                Ok(json!(true))
            }
            method::SINNER_LIST => Ok(serde_json::to_value(&self.sinners).unwrap()),
            method::THEME_PACK_LIST => Ok(serde_json::to_value(&self.packs).unwrap()),
            method::THEME_PACK_UPDATE_ALL => {
                let value: Value =
                    Self::params(request.params, "themePack.updateAll requires {packs}")?;
                let packs: Vec<ThemePack> =
                    Self::params(value.get("packs").cloned(), "themePack.updateAll packs")?;
                self.packs.packs = packs;
                Ok(json!(true))
            }
            method::THEME_PACK_RESET_WEIGHTS => {
                self.packs.packs.iter_mut().for_each(|pack| pack.weight = 1);
                Ok(json!(self.packs.clone()))
            }
            method::RESOURCE_STATUS => Ok(serde_json::to_value(&self.resources).unwrap()),
            method::RESOURCE_CHECK_UPDATE => {
                for resource in &mut self.resources {
                    resource.remoteVersion = Some("v2025.06.2".into());
                }
                // Checking updates returns the same resource collection the
                // UI renders, matching the canonical `resource.status`
                // response shape and avoiding a second incompatible payload.
                Ok(serde_json::to_value(&self.resources).unwrap())
            }
            method::RESOURCE_SYNC_START => {
                let scope = request
                    .params
                    .as_ref()
                    .and_then(|value| value.get("scope"))
                    .and_then(Value::as_str)
                    .unwrap_or("all")
                    .to_owned();
                self.emit(
                    event::RESOURCE_SYNC_PROGRESS,
                    &SyncProgressPayload {
                        scope: scope.clone(),
                        progress: 100,
                    },
                );
                for resource in &mut self.resources {
                    if scope == "all" || resource.id == scope {
                        resource.localVersion = "v2025.06.2".into();
                        resource.lastSyncAt = Some(0);
                    }
                }
                Ok(json!(true))
            }
            method::TOOL_START | method::TOOL_STOP => {
                let value: Value = Self::params(request.params, "tool call requires {id}")?;
                let tool: ToolId =
                    Self::params(value.get("id").cloned(), "tool call requires a valid id")?;
                let running = request.method == method::TOOL_START;
                self.tools.insert(tool, running);
                self.emit(
                    event::TOOL_STATUS,
                    &ToolStatusPayload {
                        toolId: tool,
                        running,
                    },
                );
                Ok(json!(true))
            }
            method::TOOL_SCREENSHOT => Ok(json!({"path":"AALC/screenshots/mock.png"})),
            method::HOTKEY_GET => Ok(serde_json::to_value(&self.hotkey).unwrap()),
            method::HOTKEY_SET => {
                self.hotkey = merge_json(&self.hotkey, request.params, "hotkey.set")?;
                Ok(json!(true))
            }
            method::SYSTEM_SETTINGS_GET => Ok(serde_json::to_value(&self.system_settings).unwrap()),
            method::SYSTEM_SETTINGS_SET => {
                self.system_settings =
                    merge_json(&self.system_settings, request.params, "systemSettings.set")?;
                Ok(json!(true))
            }
            method::NOTIFICATION_TEST => {
                let value: Value =
                    Self::params(request.params, "notification.test requires {spt}")?;
                let spt = match value.get("spt") {
                    Some(Value::String(value)) => value.trim(),
                    Some(_) => return Err(RpcError::invalid_params("SPT 必须是字符串")),
                    None => self.system_settings.wxpusher_spt.trim(),
                };
                if spt.is_empty() {
                    return Err(RpcError::invalid_params("SPT 未配置"));
                }
                if spt.len() <= 4 || !spt.starts_with("SPT_") {
                    return Err(RpcError::invalid_params("SPT 格式无效"));
                }
                Ok(json!({"accepted": true}))
            }
            method::PREVIEW_SET_ENABLED => {
                let value: Value =
                    Self::params(request.params, "preview.setEnabled requires {enabled}")?;
                let enabled = value
                    .get("enabled")
                    .and_then(Value::as_bool)
                    .ok_or_else(|| {
                        RpcError::invalid_params("preview.setEnabled.enabled must be a boolean")
                    })?;
                self.preview_enabled = enabled;
                Ok(json!({
                    "enabled": enabled,
                    "running": enabled && self.device_status == ConnectionStatus::Connected
                }))
            }
            method::DEVICE_LIST => Ok(serde_json::to_value(&self.devices).unwrap()),
            method::DEVICE_CONNECT => {
                let value: Value = Self::params(request.params, "device.connect requires {id}")?;
                let device_id = value
                    .get("id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| RpcError::invalid_params("device.connect requires a string id"))?
                    .to_owned();
                self.device_status = ConnectionStatus::Connected;
                self.emit(
                    event::DEVICE_STATUS,
                    &DeviceStatusPayload {
                        deviceId: Some(device_id),
                        status: self.device_status,
                    },
                );
                Ok(json!(true))
            }
            method::DEVICE_DISCONNECT => {
                self.device_status = ConnectionStatus::Disconnected;
                self.emit(
                    event::DEVICE_STATUS,
                    &DeviceStatusPayload {
                        deviceId: None,
                        status: self.device_status,
                    },
                );
                Ok(json!(true))
            }
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
