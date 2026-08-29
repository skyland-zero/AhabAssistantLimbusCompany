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
        self.log("任务已开始");
    }

    pub fn stop(&mut self) {
        if !self.is_busy() {
            return;
        }
        self.send(crate::ipc::contract::method::EXECUTION_STOP, None);
        self.log("任务已停止");
    }

    pub fn pause_or_resume(&mut self) {
        let method = match self.execution.state {
            ExecutionState::Running => crate::ipc::contract::method::EXECUTION_PAUSE,
            ExecutionState::Paused => crate::ipc::contract::method::EXECUTION_RESUME,
            ExecutionState::Idle => return,
        };
        self.send(method, None);
        self.log(if self.execution.state == ExecutionState::Paused {
            "任务已暂停"
        } else {
            "任务已继续"
        });
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
        self.poll_events();
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
        self.poll_events();
    }

    pub(super) fn save_tasks(&mut self) {
        let value = serde_json::to_value(&self.tasks).expect("TasksConfig is serializable");
        self.send(crate::ipc::contract::method::TASKS_SET_CONFIG, Some(value));
    }

    fn send(&mut self, method: &str, params: Option<serde_json::Value>) {
        if self.rpc.is_sidecar() {
            let _ = self.rpc.request_async(method, params);
            return;
        }
        let result = self.rpc.request_value(method, params);
        self.apply_rpc_result(result);
    }

    /// Drain events from either the shared test backend or the sidecar.
    /// Returns whether the caller should request a repaint.
    pub fn poll_events(&mut self) -> bool {
        let events = self.rpc.take_events();
        if events.is_empty() {
            return false;
        }
        self.apply_events(events);
        true
    }

    pub(crate) fn apply_events(&mut self, events: Vec<EventEnvelope>) {
        for event in events {
            if let Some(sequence) = event.seq {
                if sequence <= self.last_event_sequence {
                    continue;
                }
                self.last_event_sequence = sequence;
            }
            match event.event.as_str() {
                crate::ipc::contract::event::EXECUTION_STATUS => {
                    if let Ok(status) = serde_json::from_value(event.payload) {
                        self.execution = status;
                        if self.execution.state == ExecutionState::Idle {
                            self.mirror_progress = None;
                        }
                    }
                }
                crate::ipc::contract::event::EXECUTION_MIRROR_PROGRESS => {
                    if let Ok(progress) = serde_json::from_value(event.payload) {
                        self.mirror_progress = Some(progress);
                    }
                }
                crate::ipc::contract::event::EXECUTION_STATS => {
                    if let Ok(stats) = serde_json::from_value(event.payload) {
                        self.stats = stats;
                    }
                }
                crate::ipc::contract::event::SCREENSHOT_FRAME => {
                    if let Ok(frame) = serde_json::from_value::<ScreenshotFrame>(event.payload)
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
                    if let Ok(status) =
                        serde_json::from_value::<PreviewStatusPayload>(event.payload)
                        && (status.deviceId.is_none()
                            || self.selected_device.as_deref() == status.deviceId.as_deref())
                    {
                        self.preview_status = status.status;
                        self.preview_error = status.error;
                    }
                }
                crate::ipc::contract::event::DEVICE_STATUS => {
                    if let Ok(status) = serde_json::from_value::<DeviceStatusPayload>(event.payload)
                    {
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
                    if let Ok(entry) = serde_json::from_value::<LogEntryPayload>(event.payload) {
                        self.push_log(entry);
                    }
                }
                crate::ipc::contract::event::APP_NOTICE => {
                    let level = match event.payload.get("level").and_then(|value| value.as_str()) {
                        Some("error") => LogLevel::Error,
                        Some("warn") => LogLevel::Warn,
                        _ => LogLevel::Info,
                    };
                    if let Some(message) = event
                        .payload
                        .get("message")
                        .and_then(|value| value.as_str())
                    {
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
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
            .unwrap_or_default();
        self.push_log(LogEntryPayload {
            ts,
            level,
            message: message.to_owned(),
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
}
