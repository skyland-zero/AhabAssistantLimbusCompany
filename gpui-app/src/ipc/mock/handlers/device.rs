use super::*;

impl MockState {
    pub(super) fn handle_device(&mut self, request: &RpcRequest) -> Result<Value, RpcError> {
        match request.method.as_str() {
            method::TOOL_START | method::TOOL_STOP => {
                let value: Value = Self::params(request.params.clone(), "tool call requires {id}")?;
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
                Ok(json!({"accepted": true, "runId": format!("mock-tool-{tool:?}")}))
            }
            method::TOOL_SCREENSHOT => Ok(json!({"path":"AALC/screenshots/mock.png"})),
            method::TOOL_RESOLUTION_SET => Ok(
                json!({"accepted": true, "size": "1920x1080", "density": 240, "reconnected": false}),
            ),
            method::TOOL_RESOLUTION_RESET => Ok(json!({"accepted": true, "reconnected": false})),
            method::DEVICE_LIST => Ok(serde_json::to_value(&self.devices).unwrap()),
            method::DEVICE_CONNECT => {
                let value: Value =
                    Self::params(request.params.clone(), "device.connect requires {id}")?;
                let device_id = value
                    .get("id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| RpcError::invalid_params("device.connect requires a string id"))?
                    .to_owned();
                self.device_status = ConnectionStatus::Connected;
                self.emit(
                    event::DEVICE_STATUS,
                    &DeviceStatusPayload {
                        deviceId: Some(device_id.clone()),
                        status: self.device_status,
                    },
                );
                Ok(json!({
                    "accepted": true,
                    "deviceId": device_id,
                    "status": "connected",
                }))
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
            _ => Err(RpcError::method_not_found(&request.method)),
        }
    }
}
