use super::*;

impl HomeState {
    pub fn start(&mut self) {
        if self.is_busy() {
            return;
        }
        if self.selected_task_count() == 0 {
            self.log("警告：没有选择任务，无法开始");
            return;
        }
        self.send(crate::ipc::contract::method::EXECUTION_START, None);
    }

    pub fn stop(&mut self) {
        if !self.is_busy() {
            return;
        }
        if self.rpc.is_sidecar() {
            self.state_before_stopping = Some(self.execution.state);
            self.execution.state = ExecutionState::Stopping;
            self.stopping_since = Some(std::time::Instant::now());
        }
        self.send(crate::ipc::contract::method::EXECUTION_STOP, None);
    }

    pub fn pause_or_resume(&mut self) {
        let method = match self.execution.state {
            ExecutionState::Running => crate::ipc::contract::method::EXECUTION_PAUSE,
            ExecutionState::Paused => crate::ipc::contract::method::EXECUTION_RESUME,
            ExecutionState::Idle | ExecutionState::Stopping => return,
        };
        self.send(method, None);
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
            crate::ipc::contract::method::EXECUTION_STOP => self.log("正在停止任务"),
            crate::ipc::contract::method::EXECUTION_PAUSE => self.log("任务已暂停"),
            crate::ipc::contract::method::EXECUTION_RESUME => self.log("任务已继续"),
            crate::ipc::contract::method::TASKS_SET_CONFIG => {}
            _ => self.apply_rpc_result(Ok(value)),
        }
    }

    pub(crate) fn stop_timed_out(&self) -> bool {
        self.execution.state == ExecutionState::Stopping
            && self
                .stopping_since
                .is_some_and(|started| started.elapsed() >= std::time::Duration::from_secs(5))
    }

    pub(crate) fn mark_stop_timeout_handled(&mut self) {
        self.stopping_since = None;
    }

    pub(crate) fn reset_after_sidecar_restart(&mut self) {
        self.execution = ExecutionStatusPayload::default();
        self.stopping_since = None;
        self.state_before_stopping = None;
        self.last_event_sequence = 0;
        self.selected_device = None;
        self.device_status = ConnectionStatus::Disconnected;
        self.latest_screenshot = None;
        self.preview_status = PreviewStatus::Stopped;
        self.screenshot_revision = self.screenshot_revision.wrapping_add(1);
    }

    pub(crate) fn apply_events(&mut self, events: Vec<EventEnvelope>) {
        for event in events {
            let EventEnvelope {
                event: event_name,
                payload,
                seq,
                binary,
            } = event;
            if let Some(sequence) = seq {
                if sequence <= self.last_event_sequence {
                    continue;
                }
                self.last_event_sequence = sequence;
            }
            match event_name.as_str() {
                crate::ipc::contract::event::EXECUTION_STATUS => {
                    if let Ok(status) = serde_json::from_value(payload) {
                        self.execution = status;
                        if self.execution.state == ExecutionState::Idle {
                            self.mirror_progress = None;
                            self.stopping_since = None;
                            self.state_before_stopping = None;
                        }
                    }
                }
                crate::ipc::contract::event::EXECUTION_MIRROR_PROGRESS => {
                    if let Ok(progress) = serde_json::from_value(payload) {
                        self.mirror_progress = Some(progress);
                    }
                }
                crate::ipc::contract::event::EXECUTION_STATS => {
                    if let Ok(stats) = serde_json::from_value(payload) {
                        self.stats = stats;
                    }
                }
                crate::ipc::contract::event::SCREENSHOT_FRAME => {
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
                    });
                    let frame =
                        frame.or_else(|| serde_json::from_value::<ScreenshotFrame>(payload).ok());
                    if let Some(frame) = frame
                        && (frame.instanceId == "default"
                            || self.selected_device.as_deref() == Some(frame.instanceId.as_str()))
                    {
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
                    if let Ok(status) = serde_json::from_value::<PreviewStatusPayload>(payload)
                        && (status.deviceId.is_none()
                            || self.selected_device.as_deref() == status.deviceId.as_deref())
                    {
                        self.preview_status = status.status;
                        self.preview_error = status.error;
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
                    if let Ok(entry) = serde_json::from_value::<LogEntryPayload>(payload) {
                        self.push_log(entry);
                    }
                }
                crate::ipc::contract::event::APP_NOTICE => {
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
                json!({"instanceId":"fixture","width":2,"height":1}),
            )
            .with_binary(vec![0xff, 0xd8, 0xff, 0xd9])
            .with_sequence(2),
        ]);

        let frame = home.latest_screenshot.expect("binary preview frame");
        assert_eq!(frame.instanceId, "fixture");
        assert_eq!(frame.jpeg, vec![0xff, 0xd8, 0xff, 0xd9]);
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
}
