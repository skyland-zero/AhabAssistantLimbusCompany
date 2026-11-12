//! The toolbox page's four tool actions and their running state.

use std::collections::HashMap;

use serde_json::json;

use crate::{
    ipc::{BackendClient, RpcGateway, contract::method},
    model::{ToolId, ToolStatusPayload},
};

pub struct ToolboxState {
    rpc: RpcGateway,
    running: HashMap<ToolId, bool>,
    pub feedback: Option<String>,
    pub error: Option<String>,
}

impl ToolboxState {
    pub fn new(client: impl Into<BackendClient>) -> Self {
        Self {
            rpc: RpcGateway::new(client),
            running: HashMap::new(),
            feedback: None,
            error: None,
        }
    }

    pub fn from_shared() -> Self {
        Self::new(crate::ipc::bridge::state_client())
    }

    pub fn is_running(&self, tool: ToolId) -> bool {
        self.running.get(&tool).copied().unwrap_or(false)
    }

    pub fn toggle(&mut self, tool: ToolId) -> Result<bool, String> {
        let running = !self.is_running(tool);
        let method_name = if running {
            method::TOOL_START
        } else {
            method::TOOL_STOP
        };
        self.rpc
            .request_value(method_name, Some(json!({ "id": tool })))
            .map_err(|error| {
                self.error = Some(error.message.clone());
                error.message
            })?;
        self.drain_tool_events();
        self.running.insert(tool, running);
        self.feedback = Some(if running {
            "工具已启动".to_owned()
        } else {
            "工具已停止".to_owned()
        });
        Ok(running)
    }

    pub fn screenshot(&mut self) -> Result<String, String> {
        let result = self
            .rpc
            .request_value(method::TOOL_SCREENSHOT, None)
            .map_err(|error| error.message)?
            .unwrap_or_default();
        let path = result
            .get("path")
            .and_then(|value| value.as_str())
            .unwrap_or("未知路径");
        let message = format!("截图完成：{path}");
        self.feedback = Some(message.clone());
        Ok(message)
    }

    pub fn apply_resolution(&mut self) -> Result<String, String> {
        self.resolution(method::TOOL_RESOLUTION_SET, false)
    }

    pub fn reset_resolution(&mut self) -> Result<String, String> {
        self.resolution(method::TOOL_RESOLUTION_RESET, true)
    }

    fn resolution(&mut self, method_name: &str, reset: bool) -> Result<String, String> {
        let result = self
            .rpc
            .request_value(method_name, None)
            .map_err(|error| error.message)?
            .unwrap_or_default();
        let reconnected = result
            .get("reconnected")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        let message = if reset {
            "已还原设备分辨率与 DPI"
        } else {
            "已修改分辨率为 1080P (240 DPI)"
        };
        let message = if reconnected {
            format!("{message}，Scrcpy 已重连")
        } else {
            message.to_owned()
        };
        self.feedback = Some(message.clone());
        Ok(message)
    }

    fn drain_tool_events(&mut self) {
        for event in self.rpc.take_events() {
            if event.event == crate::ipc::contract::event::TOOL_STATUS
                && let Ok(status) = serde_json::from_value::<ToolStatusPayload>(event.payload)
            {
                self.running.insert(status.toolId, status.running);
            }
        }
    }
}
