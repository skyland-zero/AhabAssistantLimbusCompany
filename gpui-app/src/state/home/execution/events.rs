use super::*;

impl HomeState {
    /// Logs and notices may arrive after a run's final idle snapshot (for
    /// example a completion notice or a resource-sync failure with its own
    /// run id).  They never carry execution state, so while idle they are
    /// safe to display; once a run is active only that run's messages are
    /// accepted so an old run cannot pollute the current log.
    pub(crate) fn accepts_run_scoped_message(&self, run_id: Option<&str>) -> bool {
        if self.execution.state == ExecutionState::Idle {
            return true;
        }
        match self.current_execution_run_id() {
            Some(current) => run_id.is_none_or(|run_id| run_id == current),
            None => true,
        }
    }

    pub(crate) fn accepts_stats_event(&self, stats: &ExecutionStatsPayload) -> bool {
        // A stats snapshot older than the current one must never be applied:
        // it would regress the summary after a late event or a hydration race.
        if stats.updatedAt < self.stats.updatedAt {
            return false;
        }
        let run_id = stats.currentRun.runId.as_deref();
        self.accepts_derived_run(run_id)
            || (stats.currentRun.state == ExecutionState::Idle
                && self.execution.state == ExecutionState::Idle
                && self.execution.runId.as_deref() == run_id)
            // A new run's stats may arrive before its execution.status event.
            // Admit them while idle instead of silently dropping the payload;
            // the UI derives the displayed state from the authoritative
            // execution snapshot and the status event moves runId forward.
            || (self.execution.state == ExecutionState::Idle
                && stats.currentRun.state != ExecutionState::Idle
                && run_id.is_some())
    }

    pub(crate) fn derived_run_id<'a>(
        event: &str,
        payload: &'a serde_json::Value,
    ) -> Option<&'a str> {
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

    pub(crate) fn is_runner_derived_event(event: &str) -> bool {
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
            self.state_before_stopping = None;
            self.prune_run_identity_maps();
        }
        true
    }

    /// Drop per-run event/preview identity state once the run is idle.
    ///
    /// Only the live (or just-finished) run needs sequence high-water marks;
    /// retaining every historical run would grow for the sidecar's lifetime.
    pub(crate) fn prune_run_identity_maps(&mut self) {
        if let Some(current_run) = self.execution.runId.clone() {
            self.execution_event_sequences
                .retain(|run_id, _| run_id == &current_run);
            self.preview_generation_floor.retain(|(_, run_id), _| {
                run_id.as_deref() == Some(current_run.as_str()) || run_id.is_none()
            });
        } else {
            self.execution_event_sequences.clear();
            let selected = self.selected_device.clone();
            self.preview_generation_floor
                .retain(|(device_id, run_id), _| {
                    run_id.is_none() && Some(device_id) == selected.as_ref()
                });
        }
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
                        && self.accepts_run_scoped_message(entry.runId.as_deref())
                    {
                        self.push_log(entry);
                    }
                }
                crate::ipc::contract::event::APP_NOTICE => {
                    let notice_run_id = payload.get("runId").and_then(serde_json::Value::as_str);
                    if !self.accepts_run_scoped_message(notice_run_id) {
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

    pub(crate) fn log(&mut self, message: &str) {
        self.log_level(LogLevel::Info, message);
    }

    pub(crate) fn log_level(&mut self, level: LogLevel, message: &str) {
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

    pub(crate) fn push_log(&mut self, entry: LogEntryPayload) {
        self.logs.push_back(entry);
        while self.logs.len() > 300 {
            self.logs.pop_front();
        }
        self.log_revision = self.log_revision.wrapping_add(1);
    }
}
