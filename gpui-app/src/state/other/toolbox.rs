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
            let _ = self
                .rpc
                .request_async(method_name, Some(json!({ "id": tool })));
            self.running.insert(tool, !self.is_running(tool));
            self.feedback = Some("工具请求已提交".to_owned());
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
            let _ = self.rpc.request_async(method::TOOL_SCREENSHOT, None);
            self.feedback = Some("截图请求已提交".to_owned());
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

    pub(crate) fn apply_events(&mut self, events: Vec<EventEnvelope>) {
        for event in events {
            if event.event == event::TOOL_STATUS
                && let Ok(status) = serde_json::from_value::<ToolStatusPayload>(event.payload)
            {
                self.running.insert(status.toolId, status.running);
            }
        }
    }
}
