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
            self.stopping_since = Some(std::time::Instant::now());
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

    pub(super) fn save_tasks(&mut self) {
        let value = serde_json::to_value(&self.tasks).expect("TasksConfig is serializable");
        self.send(crate::ipc::contract::method::TASKS_SET_CONFIG, Some(value));
    }

    fn send(&mut self, method: &str, params: Option<serde_json::Value>) {
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
                    self.stopping_since = None;
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
    fn apply_command_snapshot(&mut self, value: Option<&serde_json::Value>) {
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
            self.stopping_since = None;
        }
    }

    pub(crate) fn mark_stop_timeout_handled(&mut self) {
        self.stopping_since = None;
    }

    pub(crate) fn suspend_preview(&mut self) -> bool {
        let changed = self.latest_screenshot.is_some()
            || self.preview_status != PreviewStatus::Stopped
            || self.preview_error.is_some();
        self.latest_screenshot = None;
        self.preview_status = PreviewStatus::Stopped;
        self.preview_error = None;
        if changed {
            self.screenshot_revision = self.screenshot_revision.wrapping_add(1);
        }
        changed
    }

    pub(crate) fn start_preview(&mut self) -> bool {
        let changed =
            self.preview_status != PreviewStatus::Starting || self.preview_error.is_some();
        self.preview_status = PreviewStatus::Starting;
        self.preview_error = None;
        changed
    }

    pub(crate) fn reset_after_sidecar_restart(&mut self) {
        self.execution = ExecutionStatusPayload::default();
        self.stopping_since = None;
        self.state_before_stopping = None;
        self.last_event_sequence = 0;
        self.execution_event_sequences.clear();
        self.next_client_request_id = 0;
        self.selected_device = None;
        self.device_status = ConnectionStatus::Disconnected;
        self.latest_screenshot = None;
        self.preview_status = PreviewStatus::Stopped;
        self.screenshot_revision = self.screenshot_revision.wrapping_add(1);
        self.preview_identity = None;
        self.preview_generation_floor.clear();
        self.preview_recent_run_id = None;
    }

    fn current_execution_run_id(&self) -> Option<&str> {
        self.execution
            .runId
            .as_deref()
            .or(self.stats.currentRun.runId.as_deref())
    }

    /// Runner-derived events must never be allowed to resurrect a completed
    /// run or overwrite a newer run.  A run id is optional only for legacy
    /// schema-2 events; once schema-3 state has identified a run, an explicit
    /// mismatching id is rejected.
    fn accepts_derived_run(&self, run_id: Option<&str>) -> bool {
        let Some(run_id) = run_id else {
            return true;
        };
        match self.current_execution_run_id() {
            Some(current) if current == run_id => self.execution.state != ExecutionState::Idle,
            Some(_) => {
                self.execution.state == ExecutionState::Idle && self.execution.stateRevision == 0
            }
            None => true,
        }
    }

    fn preview_execution_is_active(&self) -> bool {
        self.execution.state != ExecutionState::Idle
            || self.execution.deviceLease != DeviceLeaseState::None
    }

    /// Extract the identity fields that Python emits on every preview event.
    /// Missing fields are deliberately rejected rather than defaulted: a
    /// partial handshake must never overwrite the last known frame.
    fn preview_metadata(payload: &serde_json::Value) -> Option<(String, Option<String>, u64)> {
        let device_id = payload.get("deviceId")?.as_str()?.to_owned();
        if device_id.is_empty() {
            return None;
        }
        let run_id = match payload.get("runId")? {
            serde_json::Value::Null => None,
            serde_json::Value::String(value) if !value.is_empty() => Some(value.clone()),
            _ => return None,
        };
        let generation = payload.get("generation")?.as_u64()?;
        Some((device_id, run_id, generation))
    }

    /// Accept only the currently leased run while execution is active.  Once
    /// idle, the completed run remains a short-lived compatibility allowance,
    /// while a null-run sidecar preview may establish a fresh baseline.
    fn preview_run_is_allowed(&self, run_id: Option<&str>) -> bool {
        let current = self.current_execution_run_id();
        if self.preview_execution_is_active() {
            return run_id.is_some_and(|run_id| current == Some(run_id));
        }
        match run_id {
            Some(run_id) => {
                current == Some(run_id) || self.preview_recent_run_id.as_deref() == Some(run_id)
            }
            None => true,
        }
    }

    fn accepts_preview_identity(
        &self,
        device_id: &str,
        run_id: Option<&str>,
        generation: u64,
    ) -> bool {
        if self.selected_device.as_deref() != Some(device_id)
            || !self.preview_run_is_allowed(run_id)
        {
            return false;
        }

        // After a null-run sidecar preview has established a baseline, a late
        // event from the completed Runner run must not reclaim the surface.
        if !self.preview_execution_is_active()
            && self
                .preview_identity
                .as_ref()
                .is_some_and(|identity| identity.run_id.is_none() && run_id.is_some())
        {
            return false;
        }

        let key = (device_id.to_owned(), run_id.map(str::to_owned));
        self.preview_generation_floor
            .get(&key)
            .is_none_or(|floor| generation >= *floor)
    }

    fn record_preview_identity(&mut self, device_id: &str, run_id: Option<&str>, generation: u64) {
        let run_id = run_id.map(str::to_owned);
        let key = (device_id.to_owned(), run_id.clone());
        self.preview_generation_floor
            .entry(key)
            .and_modify(|floor| *floor = (*floor).max(generation))
            .or_insert(generation);
        self.preview_identity = Some(PreviewEventIdentity {
            device_id: device_id.to_owned(),
            run_id,
            generation,
        });
    }

    fn reconcile_preview_execution_boundary(
        &mut self,
        previous: &ExecutionStatusPayload,
        current: &ExecutionStatusPayload,
    ) {
        let was_active = previous.state != ExecutionState::Idle
            || previous.deviceLease != DeviceLeaseState::None;
        let is_active =
            current.state != ExecutionState::Idle || current.deviceLease != DeviceLeaseState::None;
        if !is_active {
            if let Some(run_id) = current.runId.clone().or_else(|| previous.runId.clone()) {
                self.preview_recent_run_id = Some(run_id);
            }
            // The next null-run sidecar event is allowed to establish its own
            // generation baseline after lease restoration reaches idle.
            self.preview_identity = None;
            if self.latest_screenshot.take().is_some() {
                self.screenshot_revision = self.screenshot_revision.wrapping_add(1);
            }
        } else if !was_active || previous.runId != current.runId {
            // An active execution boundary invalidates any sidecar preview
            // identity left over from the previous idle period.
            self.preview_identity = None;
            if self.latest_screenshot.take().is_some() {
                self.screenshot_revision = self.screenshot_revision.wrapping_add(1);
            }
        }
    }

    /// `app.exitRequested` is emitted after the sidecar has finalized a run,
    /// so the normal derived-event gate (which intentionally rejects events
    /// for a run already in `idle`) cannot be used for it.  Keep the explicit
    /// run association, however, so a delayed completion from an older run
    /// cannot close the current application.
    pub(crate) fn accepts_exit_request(&self, run_id: Option<&str>) -> bool {
        run_id.is_none_or(|run_id| self.current_execution_run_id() == Some(run_id))
    }

    fn accepts_stats_event(&self, stats: &ExecutionStatsPayload) -> bool {
        let run_id = stats.currentRun.runId.as_deref();
        self.accepts_derived_run(run_id)
            || (stats.currentRun.state == ExecutionState::Idle
                && self.execution.state == ExecutionState::Idle
                && self.execution.runId.as_deref() == run_id)
    }

    fn derived_run_id<'a>(event: &str, payload: &'a serde_json::Value) -> Option<&'a str> {
        payload
            .get("runId")
            .and_then(serde_json::Value::as_str)
            .or_else(|| {
                (event == crate::ipc::contract::event::EXECUTION_STATS)
                    .then(|| payload.pointer("/currentRun/runId"))
                    .flatten()
                    .and_then(serde_json::Value::as_str)
            })
    }

    fn is_runner_derived_event(event: &str) -> bool {
        matches!(
            event,
            crate::ipc::contract::event::EXECUTION_STATUS
                | crate::ipc::contract::event::EXECUTION_MIRROR_PROGRESS
                | crate::ipc::contract::event::EXECUTION_MIRROR_FLOOR
                | crate::ipc::contract::event::EXECUTION_STATS
                | crate::ipc::contract::event::SCREENSHOT_FRAME
                | crate::ipc::contract::event::PREVIEW_STATUS
                | crate::ipc::contract::event::LOG_ENTRY
                | crate::ipc::contract::event::APP_NOTICE
        )
    }

    /// Apply a status only if its monotonic revision and run id are current.
    /// Revision zero is retained for old schema-2 peers and follows the old
    /// arrival-order behavior.
    pub(crate) fn apply_execution_status(&mut self, status: ExecutionStatusPayload) -> bool {
        let current = &self.execution;
        if status.stateRevision < current.stateRevision {
            return false;
        }
        if let (Some(current_run), Some(status_run)) = (&current.runId, &status.runId) {
            if current_run != status_run {
                // There is only one active execution.  A status for another
                // run is stale while the current run is non-idle.  Once a
                // finished run is idle, only a strictly newer revision may
                // introduce a new run; this prevents late old-run events from
                // resurrecting the completed execution.
                let is_new_run = current.state == ExecutionState::Idle
                    && status.stateRevision > current.stateRevision;
                if !is_new_run {
                    return false;
                }
            } else if current.state == ExecutionState::Idle && status.state != ExecutionState::Idle
            {
                // The final idle snapshot remains authoritative for this run.
                return false;
            }
        }
        if status.stateRevision > 0
            && current.stateRevision > 0
            && status.stateRevision == current.stateRevision
            && (status.runId != current.runId || status.state != current.state)
        {
            return false;
        }
        let previous_execution = self.execution.clone();
        self.execution = status;
        let current_execution = self.execution.clone();
        self.reconcile_preview_execution_boundary(&previous_execution, &current_execution);
        if self.execution.state == ExecutionState::Idle {
            self.mirror_progress = None;
            self.mirror_floor = None;
            self.stopping_since = None;
            self.state_before_stopping = None;
        }
        true
    }

    pub(crate) fn apply_events(&mut self, events: Vec<EventEnvelope>) {
        for event in events {
            let EventEnvelope {
                event: event_name,
                payload,
                seq,
                binary,
            } = event;
            let runner_derived = Self::is_runner_derived_event(&event_name);
            let run_id = runner_derived.then(|| Self::derived_run_id(&event_name, &payload));
            if let Some(sequence) = seq {
                let duplicate = if let Some(Some(run_id)) = run_id {
                    self.execution_event_sequences
                        .get(run_id)
                        .is_some_and(|last| sequence <= *last)
                } else {
                    sequence <= self.last_event_sequence
                };
                if duplicate {
                    continue;
                }
                if let Some(Some(run_id)) = run_id {
                    self.execution_event_sequences
                        .insert(run_id.to_owned(), sequence);
                } else {
                    self.last_event_sequence = sequence;
                }
            }
            match event_name.as_str() {
                crate::ipc::contract::event::EXECUTION_STATUS => {
                    if let Ok(status) = serde_json::from_value(payload) {
                        self.apply_execution_status(status);
                    }
                }
                crate::ipc::contract::event::EXECUTION_MIRROR_PROGRESS => {
                    if let Ok(progress) = serde_json::from_value(payload) {
                        let progress: MirrorProgressPayload = progress;
                        if self.accepts_derived_run(progress.runId.as_deref()) {
                            self.mirror_progress = Some(progress);
                        }
                    }
                }
                crate::ipc::contract::event::EXECUTION_MIRROR_FLOOR => {
                    if let Ok(progress) = serde_json::from_value(payload) {
                        let progress: MirrorFloorPayload = progress;
                        if self.accepts_derived_run(progress.runId.as_deref()) && progress.floor > 0
                        {
                            self.mirror_floor = Some(progress);
                        }
                    }
                }
                crate::ipc::contract::event::EXECUTION_STATS => {
                    if let Ok(stats) = serde_json::from_value(payload) {
                        let stats: ExecutionStatsPayload = stats;
                        if self.accepts_stats_event(&stats) {
                            self.stats = stats;
                        }
                    }
                }
                crate::ipc::contract::event::SCREENSHOT_FRAME => {
                    let Some((device_id, run_id, generation)) = Self::preview_metadata(&payload)
                    else {
                        continue;
                    };
                    if !self.accepts_preview_identity(&device_id, run_id.as_deref(), generation) {
                        continue;
                    }
                    let frame = binary.map(|jpeg| ScreenshotFrame {
                        instanceId: payload
                            .get("instanceId")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        jpeg,
                        width: payload
                            .get("width")
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or_default() as u32,
                        height: payload
                            .get("height")
                            .and_then(serde_json::Value::as_u64)
                            .unwrap_or_default() as u32,
                        deviceId: Some(device_id.clone()),
                        runId: run_id.clone(),
                        generation: Some(generation),
                    });
                    let frame =
                        frame.or_else(|| serde_json::from_value::<ScreenshotFrame>(payload).ok());
                    if let Some(frame) = frame
                        && frame.deviceId.as_deref() == Some(device_id.as_str())
                        && frame.runId == run_id
                        && frame.generation == Some(generation)
                        && (frame.instanceId == "default"
                            || self.selected_device.as_deref() == Some(frame.instanceId.as_str()))
                    {
                        self.record_preview_identity(&device_id, run_id.as_deref(), generation);
                        let changed = self
                            .latest_screenshot
                            .as_ref()
                            .is_none_or(|current| current != &frame);
                        if changed {
                            self.latest_screenshot = Some(frame);
                            self.screenshot_revision = self.screenshot_revision.wrapping_add(1);
                        }
                        self.preview_status = PreviewStatus::Running;
                        self.preview_error = None;
                    }
                }
                crate::ipc::contract::event::PREVIEW_STATUS => {
                    let Some((device_id, run_id, generation)) = Self::preview_metadata(&payload)
                    else {
                        continue;
                    };
                    if let Ok(status) =
                        serde_json::from_value::<PreviewStatusPayload>(payload.clone())
                        && status.deviceId.as_deref() == Some(device_id.as_str())
                        && status.runId == run_id
                        && status.generation == Some(generation)
                        && self.accepts_preview_identity(&device_id, run_id.as_deref(), generation)
                    {
                        self.record_preview_identity(&device_id, run_id.as_deref(), generation);
                        self.preview_status = status.status;
                        self.preview_error = status.error;
                        if status.status == PreviewStatus::Stopped
                            && self.latest_screenshot.take().is_some()
                        {
                            self.screenshot_revision = self.screenshot_revision.wrapping_add(1);
                        }
                    }
                }
                crate::ipc::contract::event::DEVICE_STATUS => {
                    if let Ok(status) = serde_json::from_value::<DeviceStatusPayload>(payload) {
                        self.selected_device = status.deviceId;
                        self.device_status = status.status;
                        match status.status {
                            ConnectionStatus::Connected => {
                                self.device_error = None;
                                self.latest_screenshot = None;
                                self.screenshot_revision = self.screenshot_revision.wrapping_add(1);
                                self.preview_status = PreviewStatus::Starting;
                                self.preview_error = None;
                            }
                            ConnectionStatus::Connecting => {
                                self.latest_screenshot = None;
                                self.screenshot_revision = self.screenshot_revision.wrapping_add(1);
                                self.preview_status = PreviewStatus::Starting;
                                self.preview_error = None;
                            }
                            ConnectionStatus::Disconnected => {
                                self.latest_screenshot = None;
                                self.screenshot_revision = self.screenshot_revision.wrapping_add(1);
                                self.preview_status = PreviewStatus::Stopped;
                                self.preview_error = None;
                            }
                        }
                    }
                }
                crate::ipc::contract::event::LOG_ENTRY => {
                    if let Ok(entry) = serde_json::from_value::<LogEntryPayload>(payload)
                        && self.accepts_derived_run(entry.runId.as_deref())
                    {
                        self.push_log(entry);
                    }
                }
                crate::ipc::contract::event::APP_NOTICE => {
                    let notice_run_id = payload.get("runId").and_then(serde_json::Value::as_str);
                    if !self.accepts_derived_run(notice_run_id) {
                        continue;
                    }
                    let level = match payload.get("level").and_then(|value| value.as_str()) {
                        Some("error") => LogLevel::Error,
                        Some("warn") => LogLevel::Warn,
                        _ => LogLevel::Info,
                    };
                    if let Some(message) = payload.get("message").and_then(|value| value.as_str()) {
                        if level == LogLevel::Error
                            && (message.contains("设备")
                                || message.contains("device")
                                || message.contains("窗口")
                                || message.contains("HWND"))
                        {
                            self.device_error = Some(message.to_owned());
                        }
                        self.log_level(level, message);
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn log(&mut self, message: &str) {
        self.log_level(LogLevel::Info, message);
    }

    fn log_level(&mut self, level: LogLevel, message: &str) {
        self.append_local_log(level, message.to_owned());
    }

    pub(crate) fn append_local_log(&mut self, level: LogLevel, message: impl Into<String>) {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
            .unwrap_or_default();
        self.push_log(LogEntryPayload {
            ts,
            level,
            message: message.into(),
            runId: None,
        });
    }

    fn push_log(&mut self, entry: LogEntryPayload) {
        self.logs.push_back(entry);
        while self.logs.len() > 300 {
            self.logs.pop_front();
        }
        self.log_revision = self.log_revision.wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn preview_frames_follow_selected_device_and_clear_on_disconnect() {
        let mut home = HomeState::default();
        home.apply_events(vec![
            EventEnvelope::new(
                crate::ipc::contract::event::DEVICE_STATUS,
                json!({"deviceId":"pc:limbus","status":"connected"}),
            )
            .with_sequence(1),
            EventEnvelope::new(
                crate::ipc::contract::event::SCREENSHOT_FRAME,
                json!({
                    "instanceId":"pc:limbus",
                    "deviceId":"pc:limbus",
                    "runId":null,
                    "generation":1,
                    "jpeg":[255,216,255,217],
                    "width":720,
                    "height":405
                }),
            )
            .with_sequence(2),
        ]);

        assert_eq!(home.preview_status, PreviewStatus::Running);
        assert_eq!(home.latest_screenshot.as_ref().unwrap().width, 720);

        home.apply_events(vec![
            EventEnvelope::new(
                crate::ipc::contract::event::DEVICE_STATUS,
                json!({"deviceId":null,"status":"disconnected"}),
            )
            .with_sequence(3),
        ]);

        assert_eq!(home.preview_status, PreviewStatus::Stopped);
        assert!(home.latest_screenshot.is_none());
    }

    #[test]
    fn binary_preview_frames_keep_metadata_and_jpeg_bytes_separate() {
        let mut home = HomeState::default();
        home.apply_events(vec![
            EventEnvelope::new(
                crate::ipc::contract::event::DEVICE_STATUS,
                json!({"deviceId":"fixture","status":"connected"}),
            )
            .with_sequence(1),
            EventEnvelope::new(
                crate::ipc::contract::event::SCREENSHOT_FRAME,
                json!({
                    "instanceId":"fixture",
                    "deviceId":"fixture",
                    "runId":null,
                    "generation":1,
                    "width":2,
                    "height":1
                }),
            )
            .with_binary(vec![0xff, 0xd8, 0xff, 0xd9])
            .with_sequence(2),
        ]);

        let frame = home.latest_screenshot.expect("binary preview frame");
        assert_eq!(frame.instanceId, "fixture");
        assert_eq!(frame.jpeg, vec![0xff, 0xd8, 0xff, 0xd9]);
    }

    #[test]
    fn preview_events_follow_run_generation_and_idle_recovery_boundaries() {
        let mut home = HomeState::default();
        home.apply_events(vec![
            EventEnvelope::new(
                crate::ipc::contract::event::DEVICE_STATUS,
                json!({"deviceId":"pc:limbus","status":"connected"}),
            ),
            EventEnvelope::new(
                crate::ipc::contract::event::EXECUTION_STATUS,
                json!({
                    "state":"running",
                    "stateRevision":1,
                    "runId":"run-a",
                    "deviceLease":"runner"
                }),
            ),
            EventEnvelope::new(
                crate::ipc::contract::event::SCREENSHOT_FRAME,
                json!({
                    "instanceId":"pc:limbus",
                    "deviceId":"pc:limbus",
                    "runId":"run-a",
                    "generation":4,
                    "jpeg":[1],
                    "width":1,
                    "height":1
                }),
            ),
        ]);

        assert_eq!(home.latest_screenshot.as_ref().unwrap().generation, Some(4));
        assert_eq!(
            home.preview_identity.as_ref().unwrap().run_id.as_deref(),
            Some("run-a")
        );

        // An older generation and a different active run cannot replace the
        // current frame; status events use the same high-water mark.
        home.apply_events(vec![
            EventEnvelope::new(
                crate::ipc::contract::event::SCREENSHOT_FRAME,
                json!({
                    "instanceId":"pc:limbus",
                    "deviceId":"pc:limbus",
                    "runId":"run-a",
                    "generation":3,
                    "jpeg":[2],
                    "width":1,
                    "height":1
                }),
            ),
            EventEnvelope::new(
                crate::ipc::contract::event::PREVIEW_STATUS,
                json!({
                    "deviceId":"pc:limbus",
                    "runId":"run-a",
                    "generation":3,
                    "status":"error",
                    "error":"stale"
                }),
            ),
            EventEnvelope::new(
                crate::ipc::contract::event::SCREENSHOT_FRAME,
                json!({
                    "instanceId":"pc:limbus",
                    "deviceId":"pc:limbus",
                    "runId":"run-b",
                    "generation":1,
                    "jpeg":[3],
                    "width":1,
                    "height":1
                }),
            ),
            EventEnvelope::new(
                crate::ipc::contract::event::SCREENSHOT_FRAME,
                json!({
                    "instanceId":"pc:limbus",
                    "deviceId":"pc:limbus",
                    "runId":null,
                    "generation":9,
                    "jpeg":[4],
                    "width":1,
                    "height":1
                }),
            ),
        ]);
        assert_eq!(home.latest_screenshot.as_ref().unwrap().jpeg, vec![1]);
        assert_eq!(home.preview_status, PreviewStatus::Running);

        // Once the execution reaches idle, the completed Runner run is kept
        // only as a recent-run allowance while a fresh null-run sidecar
        // generation establishes a new baseline.
        home.apply_events(vec![EventEnvelope::new(
            crate::ipc::contract::event::EXECUTION_STATUS,
            json!({
                "state":"idle",
                "stateRevision":2,
                "runId":"run-a",
                "deviceLease":"none"
            }),
        )]);
        assert!(home.latest_screenshot.is_none());
        assert_eq!(home.preview_recent_run_id.as_deref(), Some("run-a"));

        home.apply_events(vec![
            EventEnvelope::new(
                crate::ipc::contract::event::PREVIEW_STATUS,
                json!({
                    "deviceId":"pc:limbus",
                    "runId":null,
                    "generation":1,
                    "status":"starting"
                }),
            ),
            EventEnvelope::new(
                crate::ipc::contract::event::SCREENSHOT_FRAME,
                json!({
                    "instanceId":"pc:limbus",
                    "deviceId":"pc:limbus",
                    "runId":null,
                    "generation":1,
                    "jpeg":[5],
                    "width":1,
                    "height":1
                }),
            ),
            EventEnvelope::new(
                crate::ipc::contract::event::SCREENSHOT_FRAME,
                json!({
                    "instanceId":"pc:limbus",
                    "deviceId":"pc:limbus",
                    "runId":"run-a",
                    "generation":99,
                    "jpeg":[6],
                    "width":1,
                    "height":1
                }),
            ),
        ]);
        assert_eq!(home.latest_screenshot.as_ref().unwrap().jpeg, vec![5]);
        assert_eq!(home.latest_screenshot.as_ref().unwrap().runId, None);
        assert_eq!(home.latest_screenshot.as_ref().unwrap().generation, Some(1));
    }

    #[test]
    fn preview_events_missing_identity_metadata_fail_safe() {
        let mut home = HomeState::default();
        home.apply_events(vec![
            EventEnvelope::new(
                crate::ipc::contract::event::DEVICE_STATUS,
                json!({"deviceId":"pc:limbus","status":"connected"}),
            ),
            EventEnvelope::new(
                crate::ipc::contract::event::EXECUTION_STATUS,
                json!({"state":"running","stateRevision":1,"runId":"run-a"}),
            ),
            EventEnvelope::new(
                crate::ipc::contract::event::SCREENSHOT_FRAME,
                json!({
                    "instanceId":"pc:limbus",
                    "deviceId":"pc:limbus",
                    "runId":"run-a",
                    "generation":2,
                    "jpeg":[7],
                    "width":1,
                    "height":1
                }),
            ),
        ]);
        let frame_before = home.latest_screenshot.clone();

        // Each event below is a plausible handshake-time partial payload, but
        // none is allowed to clear or replace the established frame.
        home.apply_events(vec![
            EventEnvelope::new(
                crate::ipc::contract::event::SCREENSHOT_FRAME,
                json!({
                    "instanceId":"pc:limbus",
                    "runId":"run-a",
                    "generation":3,
                    "jpeg":[8],
                    "width":1,
                    "height":1
                }),
            ),
            EventEnvelope::new(
                crate::ipc::contract::event::SCREENSHOT_FRAME,
                json!({
                    "instanceId":"pc:limbus",
                    "deviceId":"pc:limbus",
                    "generation":3,
                    "jpeg":[9],
                    "width":1,
                    "height":1
                }),
            ),
            EventEnvelope::new(
                crate::ipc::contract::event::PREVIEW_STATUS,
                json!({"deviceId":"pc:limbus","runId":"run-a","status":"error"}),
            ),
        ]);
        assert_eq!(home.latest_screenshot, frame_before);
        assert_eq!(home.preview_status, PreviewStatus::Running);
    }

    #[test]
    fn local_backend_logs_use_the_same_bounded_queue() {
        let mut home = HomeState::default();
        home.append_local_log(LogLevel::Error, "backend failed");

        assert_eq!(home.logs.back().unwrap().message, "backend failed");
        assert_eq!(home.logs.back().unwrap().level, LogLevel::Error);

        for index in 0..301 {
            home.append_local_log(LogLevel::Info, format!("log {index}"));
        }

        assert_eq!(home.logs.len(), 300);
        assert_eq!(home.logs.front().unwrap().message, "log 1");
    }

    #[test]
    fn execution_task_changes_follow_status_and_stats_events() {
        let mut home = HomeState::default();
        let status_event = |seq, task| {
            EventEnvelope::new(
                crate::ipc::contract::event::EXECUTION_STATUS,
                json!({"state":"running","currentTaskId":task}),
            )
            .with_sequence(seq)
        };
        let stats_event = |seq, task| {
            let mut stats = ExecutionStatsPayload::default();
            stats.currentRun.state = ExecutionState::Running;
            stats.currentRun.currentTaskId = Some(task);
            EventEnvelope::new(
                crate::ipc::contract::event::EXECUTION_STATS,
                serde_json::to_value(stats).unwrap(),
            )
            .with_sequence(seq)
        };

        home.apply_events(vec![
            status_event(1, "daily_task"),
            stats_event(2, FixedTaskId::DailyTask),
            status_event(3, "get_reward"),
            stats_event(4, FixedTaskId::GetReward),
            status_event(5, "mirror"),
            stats_event(6, FixedTaskId::Mirror),
        ]);

        assert_eq!(home.execution.currentTaskId, Some(FixedTaskId::Mirror));
        assert_eq!(
            home.stats.currentRun.currentTaskId,
            Some(FixedTaskId::Mirror)
        );

        home.apply_events(vec![
            EventEnvelope::new(
                crate::ipc::contract::event::EXECUTION_STATUS,
                json!({"state":"idle","currentTaskId":null}),
            )
            .with_sequence(7),
            EventEnvelope::new(
                crate::ipc::contract::event::EXECUTION_STATS,
                serde_json::to_value(ExecutionStatsPayload::default()).unwrap(),
            )
            .with_sequence(8),
        ]);

        assert_eq!(home.execution.currentTaskId, None);
        assert_eq!(home.stats.currentRun.currentTaskId, None);
    }

    #[test]
    fn execution_snapshots_reject_older_revisions_and_run_derived_events() {
        let mut home = HomeState::default();
        home.apply_events(vec![EventEnvelope::new(
            crate::ipc::contract::event::EXECUTION_STATUS,
            json!({
                "schemaVersion": 3,
                "state": "running",
                "stateRevision": 4,
                "runId": "run-new"
            }),
        )]);

        home.apply_events(vec![
            EventEnvelope::new(
                crate::ipc::contract::event::EXECUTION_STATUS,
                json!({
                    "state": "paused",
                    "stateRevision": 3,
                    "runId": "run-new"
                }),
            ),
            EventEnvelope::new(
                crate::ipc::contract::event::EXECUTION_MIRROR_PROGRESS,
                json!({
                    "current": 99,
                    "total": 99,
                    "isHard": false,
                    "isInfinite": false,
                    "runId": "run-old"
                }),
            ),
            EventEnvelope::new(
                crate::ipc::contract::event::EXECUTION_STATS,
                json!({
                    "schemaVersion": 1,
                    "currentRun": {
                        "runId": "run-old",
                        "state": "running",
                        "currentTaskId": null,
                        "startedAt": null,
                        "targets": {"exp": 0, "thread": 0, "mirror": 0},
                        "completed": {"exp": 0, "thread": 0, "mirror": 0},
                        "isMirrorInfinite": false,
                        "updatedAt": null
                    },
                    "today": {"exp": 0, "thread": 0, "mirror": 0},
                    "week": {"exp": 0, "thread": 0, "mirror": 0},
                    "updatedAt": 0
                }),
            ),
        ]);

        assert_eq!(home.execution.state, ExecutionState::Running);
        assert_eq!(home.execution.stateRevision, 4);
        assert!(home.mirror_progress.is_none());
        assert_eq!(home.stats.currentRun.runId, None);

        // A different run cannot replace an active one, even with a larger
        // revision.  It becomes eligible only after the current run reaches
        // its final idle snapshot.
        home.apply_events(vec![EventEnvelope::new(
            crate::ipc::contract::event::EXECUTION_STATUS,
            json!({
                "state": "starting",
                "stateRevision": 5,
                "runId": "run-next"
            }),
        )]);
        assert_eq!(home.execution.state, ExecutionState::Running);
        home.apply_events(vec![EventEnvelope::new(
            crate::ipc::contract::event::EXECUTION_STATUS,
            json!({
                "state": "idle",
                "stateRevision": 5,
                "runId": "run-new"
            }),
        )]);
        home.apply_events(vec![EventEnvelope::new(
            crate::ipc::contract::event::EXECUTION_STATUS,
            json!({
                "state": "starting",
                "stateRevision": 6,
                "runId": "run-next"
            }),
        )]);
        assert_eq!(home.execution.state, ExecutionState::Starting);
        assert_eq!(home.execution.runId.as_deref(), Some("run-next"));
    }

    #[test]
    fn execution_status_does_not_overwrite_active_or_finalized_runs() {
        let mut home = HomeState::default();
        home.apply_execution_status(ExecutionStatusPayload {
            state: ExecutionState::Running,
            stateRevision: 4,
            runId: Some("run-current".into()),
            ..ExecutionStatusPayload::default()
        });
        assert!(!home.apply_execution_status(ExecutionStatusPayload {
            state: ExecutionState::Running,
            stateRevision: 5,
            runId: Some("run-old".into()),
            ..ExecutionStatusPayload::default()
        }));
        assert_eq!(home.execution.runId.as_deref(), Some("run-current"));

        assert!(home.apply_execution_status(ExecutionStatusPayload {
            state: ExecutionState::Idle,
            stateRevision: 6,
            runId: Some("run-current".into()),
            ..ExecutionStatusPayload::default()
        }));
        assert!(!home.apply_execution_status(ExecutionStatusPayload {
            state: ExecutionState::Running,
            stateRevision: 7,
            runId: Some("run-current".into()),
            ..ExecutionStatusPayload::default()
        }));
        assert_eq!(home.execution.state, ExecutionState::Idle);
    }

    #[test]
    fn final_idle_stats_are_applied_after_the_idle_status_snapshot() {
        let mut home = HomeState::default();
        home.apply_execution_status(ExecutionStatusPayload {
            state: ExecutionState::Running,
            stateRevision: 1,
            runId: Some("run-finished".into()),
            ..ExecutionStatusPayload::default()
        });
        home.apply_execution_status(ExecutionStatusPayload {
            state: ExecutionState::Idle,
            stateRevision: 2,
            runId: Some("run-finished".into()),
            outcome: Some(crate::model::ExecutionOutcome::Completed),
            ..ExecutionStatusPayload::default()
        });

        let mut stats = ExecutionStatsPayload::default();
        stats.currentRun.runId = Some("run-finished".into());
        stats.currentRun.state = ExecutionState::Idle;
        stats.currentRun.completed.mirror = 3;
        home.apply_events(vec![EventEnvelope::new(
            crate::ipc::contract::event::EXECUTION_STATS,
            serde_json::to_value(stats).unwrap(),
        )]);

        assert_eq!(home.stats.currentRun.runId.as_deref(), Some("run-finished"));
        assert_eq!(home.stats.currentRun.completed.mirror, 3);
    }

    #[test]
    fn app_exit_requests_accept_only_the_latest_run() {
        let mut home = HomeState::default();
        home.apply_execution_status(ExecutionStatusPayload {
            state: ExecutionState::Idle,
            stateRevision: 8,
            runId: Some("run-latest".into()),
            outcome: Some(crate::model::ExecutionOutcome::Completed),
            ..ExecutionStatusPayload::default()
        });

        assert!(home.accepts_exit_request(Some("run-latest")));
        assert!(!home.accepts_exit_request(Some("run-old")));
        // Legacy sidecars did not include a run association on lifecycle
        // events; keep that compatibility path explicit.
        assert!(home.accepts_exit_request(None));
    }

    #[test]
    fn idle_state_is_busy_when_a_device_lease_is_still_present() {
        let mut home = HomeState::default();
        assert!(!home.is_busy());
        home.execution.deviceLease = crate::model::DeviceLeaseState::Restoring;
        assert!(home.is_busy());
    }
}
