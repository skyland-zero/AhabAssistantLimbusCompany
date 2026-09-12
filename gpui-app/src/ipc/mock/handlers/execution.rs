use super::*;

impl MockState {
    pub(super) fn handle_execution(&mut self, request: &RpcRequest) -> Result<Value, RpcError> {
        match request.method.as_str() {
            method::EXECUTION_GET_STATE => Ok(serde_json::to_value(&self.execution).unwrap()),
            method::EXECUTION_START => {
                let client_request_id = request
                    .params
                    .as_ref()
                    .and_then(|value| value.get("clientRequestId"))
                    .and_then(Value::as_str)
                    .map(str::to_owned);
                if client_request_id.is_some()
                    && client_request_id == self.last_start_client_request_id
                    && self.last_start_response.is_some()
                {
                    Ok(self.last_start_response.clone().unwrap())
                } else if self.execution.state != ExecutionState::Idle {
                    Err(RpcError::with_data(
                        -32010,
                        "EXECUTION_BUSY",
                        json!({"code": "EXECUTION_BUSY"}),
                    ))
                } else if !has_executable_task(&self.tasks) {
                    // A direct IPC caller must observe the same no-op behavior as
                    // HomeState: window settings and Ahab resonance are not
                    // executable tasks on their own.
                    let result = self.execution_result(false, Some("没有选择可执行任务"));
                    self.last_start_client_request_id = client_request_id;
                    self.last_start_response = Some(result.clone());
                    Ok(result)
                } else {
                    self.next_run_id = self.next_run_id.saturating_add(1);
                    let run_id = format!("mock-run-{}", self.next_run_id);
                    let next_revision = self.execution.stateRevision.saturating_add(1);
                    let task = request
                        .params
                        .clone()
                        .and_then(|value| value.get("taskId").cloned())
                        .and_then(|value| serde_json::from_value(value).ok())
                        .or_else(|| first_executable_task(&self.tasks));
                    self.execution = ExecutionStatusPayload {
                        state: ExecutionState::Running,
                        currentTaskId: task,
                        stateRevision: next_revision,
                        runId: Some(run_id.clone()),
                        deviceLease: DeviceLeaseState::Runner,
                        ..ExecutionStatusPayload::default()
                    };
                    self.emit(
                        event::LOG_ENTRY,
                        &LogEntryPayload {
                            ts: 0,
                            level: LogLevel::Info,
                            message: "Mock 任务已开始".to_owned(),
                            runId: Some(run_id.clone()),
                        },
                    );
                    let infinite =
                        self.tasks.enabledTasks.mirror && self.tasks.mirror.infinite_dungeons;
                    self.stats = ExecutionStatsPayload {
                        schemaVersion: 1,
                        currentRun: CurrentRunStats {
                            runId: Some(run_id.clone()),
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
                        lastMirror: self.stats.lastMirror.clone(),
                        mirrorHistory: self.stats.mirrorHistory.clone(),
                        today: self.stats.today.clone(),
                        week: self.stats.week.clone(),
                        updatedAt: 0,
                    };
                    // Publish the authoritative execution status before the
                    // run-scoped stats.  HomeState's run gates admit a new
                    // run's stats only after the status snapshot has moved on,
                    // so the reverse order would silently drop them.
                    let status = self.execution.clone();
                    self.emit(event::EXECUTION_STATUS, &status);
                    self.emit_stats();
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
                                runId: Some(run_id.clone()),
                            },
                        );
                        self.emit(
                            event::EXECUTION_MIRROR_FLOOR,
                            &MirrorFloorPayload {
                                floor: 1,
                                floorTotal: if self.tasks.mirror.hard_mirror {
                                    self.tasks.mirror.hard_mirror_target_floors
                                } else {
                                    5
                                },
                                runId: Some(run_id.clone()),
                            },
                        );
                    }
                    let result = json!({
                        "accepted": true,
                        "runId": run_id,
                        "state": "running",
                        "stateRevision": self.execution.stateRevision
                    });
                    self.last_start_client_request_id = client_request_id;
                    self.last_start_response = Some(result.clone());
                    Ok(result)
                }
            }
            method::EXECUTION_STOP => {
                self.validate_run_id(request.params.as_ref())?;
                if self.execution.state == ExecutionState::Idle {
                    let accepted = Self::requested_run_id(request.params.as_ref()).is_some()
                        && self.execution.runId.is_some();
                    Ok(self.execution_result(accepted, None))
                } else {
                    let next_revision = self.execution.stateRevision.saturating_add(1);
                    let run_id = self.execution.runId.clone();
                    self.execution = ExecutionStatusPayload::default();
                    // Preserve the monotonic revision and the last run identity
                    // when publishing the final idle snapshot.
                    self.execution.stateRevision = next_revision;
                    self.execution.runId = run_id;
                    self.execution.outcome = Some(ExecutionOutcome::Stopped);
                    self.execution.requestedBy = Some(ExecutionRequestedBy::User);
                    self.execution.deviceRestore = DeviceRestoreState::Restored;
                    self.stats.currentRun.state = ExecutionState::Idle;
                    self.stats.currentRun.currentTaskId = None;
                    // Publish the authoritative execution status before the
                    // run-scoped stats.  HomeState's run gates admit a new
                    // run's stats only after the status snapshot has moved on,
                    // so the reverse order would silently drop them.
                    let status = self.execution.clone();
                    self.emit(event::EXECUTION_STATUS, &status);
                    self.emit_stats();
                    self.last_start_client_request_id = None;
                    self.last_start_response = None;
                    Ok(self.execution_result(true, None))
                }
            }
            method::EXECUTION_PAUSE => {
                self.validate_run_id(request.params.as_ref())?;
                if self.execution.state != ExecutionState::Running {
                    Err(RpcError::with_data(
                        -32011,
                        "INVALID_EXECUTION_STATE",
                        json!({"code": "INVALID_EXECUTION_STATE"}),
                    ))
                } else {
                    self.advance_execution_revision();
                    self.execution.state = ExecutionState::Paused;
                    self.stats.currentRun.state = ExecutionState::Paused;
                    // Publish the authoritative execution status before the
                    // run-scoped stats.  HomeState's run gates admit a new
                    // run's stats only after the status snapshot has moved on,
                    // so the reverse order would silently drop them.
                    let status = self.execution.clone();
                    self.emit(event::EXECUTION_STATUS, &status);
                    self.emit_stats();
                    Ok(self.execution_result(true, None))
                }
            }
            method::EXECUTION_RESUME => {
                self.validate_run_id(request.params.as_ref())?;
                if self.execution.state != ExecutionState::Paused {
                    Err(RpcError::with_data(
                        -32011,
                        "INVALID_EXECUTION_STATE",
                        json!({"code": "INVALID_EXECUTION_STATE"}),
                    ))
                } else {
                    self.advance_execution_revision();
                    self.execution.state = ExecutionState::Running;
                    self.stats.currentRun.state = ExecutionState::Running;
                    // Publish the authoritative execution status before the
                    // run-scoped stats.  HomeState's run gates admit a new
                    // run's stats only after the status snapshot has moved on,
                    // so the reverse order would silently drop them.
                    let status = self.execution.clone();
                    self.emit(event::EXECUTION_STATUS, &status);
                    self.emit_stats();
                    Ok(self.execution_result(true, None))
                }
            }
            _ => Err(RpcError::method_not_found(&request.method)),
        }
    }
}
