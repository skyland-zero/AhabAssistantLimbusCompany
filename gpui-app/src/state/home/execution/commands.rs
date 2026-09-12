use super::*;

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

impl HomeState {
    pub fn start(&mut self) {
        if self.is_busy() {
            return;
        }
        if self.selected_task_count() == 0 {
            self.log("警告：没有选择任务，无法开始");
            return;
        }
        self.next_client_request_id = self.next_client_request_id.wrapping_add(1);
        let task_id = self
            .execution
            .currentTaskId
            .or_else(|| first_executable_task(&self.tasks));
        let params = json!({
            "clientRequestId": format!("gpui-start-{}", self.next_client_request_id),
            "taskId": task_id,
        });
        self.send(crate::ipc::contract::method::EXECUTION_START, Some(params));
    }

    pub fn stop(&mut self) -> bool {
        if !self.is_busy() {
            return false;
        }
        let needs_timeout = self.rpc.is_sidecar();
        let run_id = self.execution.runId.clone();
        if needs_timeout {
            self.state_before_stopping = Some(self.execution.state);
            self.execution.state = ExecutionState::Stopping;
            self.execution.requestedBy = Some(crate::model::ExecutionRequestedBy::User);
        }
        let params = run_id.map(|run_id| json!({ "runId": run_id }));
        self.send(crate::ipc::contract::method::EXECUTION_STOP, params);
        needs_timeout
    }

    pub fn pause_or_resume(&mut self) {
        let method = match self.execution.state {
            ExecutionState::Running => crate::ipc::contract::method::EXECUTION_PAUSE,
            ExecutionState::Paused => crate::ipc::contract::method::EXECUTION_RESUME,
            ExecutionState::Idle
            | ExecutionState::Starting
            | ExecutionState::Stopping
            | ExecutionState::Restoring => return,
        };
        let params = self
            .execution
            .runId
            .clone()
            .map(|run_id| json!({ "runId": run_id }));
        self.send(method, params);
    }

    pub fn clear_logs(&mut self) {
        self.logs.clear();
        self.log_revision = self.log_revision.wrapping_add(1);
    }

    pub fn dismiss_device_error(&mut self) {
        self.device_error = None;
    }

    pub fn select_device(&mut self, id: String) {
        self.device_error = None;
        self.device_status = ConnectionStatus::Connecting;
        self.close_select();
        self.send(
            crate::ipc::contract::method::DEVICE_CONNECT,
            Some(json!({ "id": id })),
        );
    }

    pub fn disconnect_device(&mut self) {
        self.device_error = None;
        self.close_select();
        self.send(crate::ipc::contract::method::DEVICE_DISCONNECT, None);
    }

    pub fn apply_device_list_result(
        &mut self,
        result: Result<Option<serde_json::Value>, crate::ipc::RpcError>,
    ) {
        self.is_scanning_devices = false;
        match result {
            Err(error) => {
                self.device_error = Some(error.message.clone());
                self.log_level(LogLevel::Error, &format!("IPC 错误：{}", error.message));
            }
            Ok(Some(value)) => {
                if let Ok(devices) = serde_json::from_value(value) {
                    self.devices = devices;
                }
            }
            Ok(None) => {}
        }
    }

    pub fn apply_rpc_result(
        &mut self,
        result: Result<Option<serde_json::Value>, crate::ipc::RpcError>,
    ) {
        if let Err(error) = result {
            if self.device_status == ConnectionStatus::Connecting {
                self.device_status = ConnectionStatus::Disconnected;
                self.device_error = Some(error.message.clone());
            }
            self.log_level(LogLevel::Error, &format!("IPC 错误：{}", error.message));
        }
    }

    pub(crate) fn save_tasks(&mut self) {
        let value = serde_json::to_value(&self.tasks).expect("TasksConfig is serializable");
        self.send(crate::ipc::contract::method::TASKS_SET_CONFIG, Some(value));
    }

    pub(crate) fn send(&mut self, method: &str, params: Option<serde_json::Value>) {
        if self.rpc.is_sidecar() {
            self.rpc.submit(method, params);
            return;
        }
        let result = self.rpc.request_value(method, params);
        self.apply_rpc_result(result);
        let events = self.rpc.take_events();
        self.apply_events(events);
    }

    pub(crate) fn apply_command_result(
        &mut self,
        method: &str,
        result: Result<Option<serde_json::Value>, crate::ipc::RpcError>,
    ) {
        let value = match result {
            Ok(value) => value,
            Err(error) => {
                if method == crate::ipc::contract::method::EXECUTION_STOP {
                    if let Some(previous) = self.state_before_stopping.take() {
                        self.execution.state = previous;
                    }
                    self.execution.requestedBy = None;
                }
                self.log_level(LogLevel::Error, &format!("IPC 错误：{}", error.message));
                return;
            }
        };
        match method {
            crate::ipc::contract::method::EXECUTION_START => {
                let accepted = value
                    .as_ref()
                    .and_then(|value| value.get("accepted"))
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false);
                if accepted {
                    self.apply_command_snapshot(value.as_ref());
                    self.log("任务已开始");
                } else {
                    let reason = value
                        .as_ref()
                        .and_then(|value| value.get("reason"))
                        .and_then(|value| value.as_str())
                        .unwrap_or("后端未接受任务");
                    self.log_level(LogLevel::Warn, reason);
                }
            }
            crate::ipc::contract::method::EXECUTION_STOP => {
                self.apply_command_snapshot(value.as_ref());
                self.log("正在停止任务");
            }
            crate::ipc::contract::method::EXECUTION_PAUSE => {
                self.apply_command_snapshot(value.as_ref());
                self.log("任务已暂停");
            }
            crate::ipc::contract::method::EXECUTION_RESUME => {
                self.apply_command_snapshot(value.as_ref());
                self.log("任务已继续");
            }
            crate::ipc::contract::method::TASKS_SET_CONFIG => {}
            _ => self.apply_rpc_result(Ok(value)),
        }
    }

    /// Apply the minimal authoritative snapshot optionally returned by an
    /// execution command.  Older sidecars only returned `accepted` and
    /// `state`; the defaults in [`ExecutionStatusPayload`] preserve those
    /// responses while schema-3 sidecars provide the full revision/run data.
    pub(crate) fn apply_command_snapshot(&mut self, value: Option<&serde_json::Value>) {
        let Some(value) = value else {
            return;
        };
        let Some(state) = value.get("state") else {
            return;
        };
        let mut snapshot = value.clone();
        if !snapshot.is_object() {
            return;
        }
        // Command responses contain `accepted` alongside the status fields;
        // serde ignores that extension.  Keep the explicit schema marker when
        // a sidecar omits it so legacy responses remain treated as snapshots.
        if snapshot.get("schemaVersion").is_none() {
            snapshot["schemaVersion"] = serde_json::json!(3);
        }
        let Ok(status) = serde_json::from_value::<ExecutionStatusPayload>(snapshot) else {
            self.log_level(LogLevel::Warn, "后端返回了无效的执行状态快照");
            return;
        };
        // A command response can race an event.  Use the same revision/run
        // gate as unsolicited status events instead of blindly overwriting a
        // newer state.
        if self.apply_execution_status(status) && state.as_str() == Some("idle") {
            self.state_before_stopping = None;
        }
    }
}
