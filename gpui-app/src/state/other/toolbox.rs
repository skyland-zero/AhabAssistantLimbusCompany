use super::*;

impl Default for ToolboxState {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolboxState {
    pub fn new() -> Self {
        Self::with_client(MockClient::default())
    }

    pub fn with_client(client: impl Into<crate::ipc::BackendClient>) -> Self {
        Self {
            rpc: RpcGateway::new(client),
            running: HashMap::new(),
            feedback: None,
        }
    }

    pub fn is_running(&self, tool: ToolId) -> bool {
        self.running.get(&tool).copied().unwrap_or(false)
    }

    pub fn toggle(&mut self, tool: ToolId) {
        let method_name = if self.is_running(tool) {
            method::TOOL_STOP
        } else {
            method::TOOL_START
        };
        if self.rpc.is_sidecar() {
            self.rpc.submit(method_name, Some(json!({ "id": tool })));
            self.feedback = Some("正在提交工具请求".to_owned());
            return;
        }
        let result = self
            .rpc
            .request_value(method_name, Some(json!({ "id": tool })));
        let events = self.rpc.take_events();
        self.apply_events(events);
        if let Err(error) = result {
            self.feedback = Some(error.message);
        }
    }

    pub fn screenshot(&mut self) {
        if self.rpc.is_sidecar() {
            self.rpc.submit(method::TOOL_SCREENSHOT, None);
            self.feedback = Some("正在截图".to_owned());
            return;
        }
        match self.rpc.request_value(method::TOOL_SCREENSHOT, None) {
            Err(error) => self.feedback = Some(error.message),
            Ok(result) => {
                let path = result
                    .and_then(|value| {
                        value
                            .get("path")
                            .and_then(|value| value.as_str())
                            .map(str::to_owned)
                    })
                    .unwrap_or_else(|| "AALC/screenshots/mock.png".to_owned());
                self.feedback = Some(format!("截图完成：{path}"));
            }
        }
    }

    pub fn apply_resolution(&mut self) {
        if self.rpc.is_sidecar() {
            self.rpc.submit(method::TOOL_RESOLUTION_SET, None);
            self.feedback = Some("正在修改设备分辨率".to_owned());
            return;
        }
        match self.rpc.request_value(method::TOOL_RESOLUTION_SET, None) {
            Err(error) => self.feedback = Some(error.message),
            Ok(result) => self.feedback = Some(resolution_feedback(result.as_ref(), false)),
        }
    }

    pub fn reset_resolution(&mut self) {
        if self.rpc.is_sidecar() {
            self.rpc.submit(method::TOOL_RESOLUTION_RESET, None);
            self.feedback = Some("正在还原设备分辨率".to_owned());
            return;
        }
        match self.rpc.request_value(method::TOOL_RESOLUTION_RESET, None) {
            Err(error) => self.feedback = Some(error.message),
            Ok(result) => self.feedback = Some(resolution_feedback(result.as_ref(), true)),
        }
    }

    pub(crate) fn apply_events(&mut self, events: Vec<EventEnvelope>) {
        for event in events {
            if event.event == event::TOOL_STATUS
                && let Ok(status) = serde_json::from_value::<ToolStatusPayload>(event.payload)
            {
                self.running.insert(status.toolId, status.running);
            }
        }
    }

    pub(crate) fn apply_rpc_result(
        &mut self,
        method_name: &str,
        _params: Option<serde_json::Value>,
        result: Result<Option<serde_json::Value>, crate::ipc::RpcError>,
    ) {
        let value = match result {
            Ok(value) => value,
            Err(error) => {
                self.feedback = Some(error.message);
                return;
            }
        };
        self.feedback = Some(match method_name {
            method::TOOL_SCREENSHOT => {
                let path = value
                    .as_ref()
                    .and_then(|value| value.get("path"))
                    .and_then(|value| value.as_str())
                    .unwrap_or("未知路径");
                format!("截图完成：{path}")
            }
            method::TOOL_START => "工具已启动".to_owned(),
            method::TOOL_STOP => "工具已停止".to_owned(),
            method::TOOL_RESOLUTION_SET => resolution_feedback(value.as_ref(), false),
            method::TOOL_RESOLUTION_RESET => resolution_feedback(value.as_ref(), true),
            _ => return,
        });
    }
}

fn resolution_feedback(result: Option<&serde_json::Value>, reset: bool) -> String {
    let reconnected = result
        .and_then(|value| value.get("reconnected"))
        .and_then(|value| value.as_bool())
        .unwrap_or(false);
    let message = if reset {
        "已还原设备分辨率与 DPI"
    } else {
        "已修改分辨率为 1080P (240 DPI)"
    };
    if reconnected {
        format!("{message}，Scrcpy 已重连")
    } else {
        message.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::resolution_feedback;
    use serde_json::json;

    #[test]
    fn resolution_feedback_reports_scrcpy_reconnect() {
        let result = json!({"reconnected": true});

        assert_eq!(
            resolution_feedback(Some(&result), false),
            "已修改分辨率为 1080P (240 DPI)，Scrcpy 已重连"
        );
        assert_eq!(
            resolution_feedback(Some(&result), true),
            "已还原设备分辨率与 DPI，Scrcpy 已重连"
        );
        assert_eq!(
            resolution_feedback(Some(&json!({"reconnected": false})), false),
            "已修改分辨率为 1080P (240 DPI)"
        );
    }
}
