use serde::de::DeserializeOwned;
use serde_json::Value;

use super::{
    BackendAttach, BackendClient, EventEnvelope, RpcCompletion, RpcError, RpcResponse,
    decode_response,
};

/// Application-facing RPC adapter.
///
/// State objects depend on this small boundary instead of knowing which
/// transport serves a method.
#[derive(Clone)]
pub struct RpcGateway {
    client: BackendClient,
}

impl RpcGateway {
    pub fn new(client: impl Into<BackendClient>) -> Self {
        Self {
            client: client.into(),
        }
    }

    pub fn shared(&self) -> Self {
        Self::new(self.client.shared())
    }

    pub fn is_sidecar(&self) -> bool {
        self.client.is_sidecar()
    }

    pub fn is_connected(&self) -> bool {
        self.client.is_connected()
    }

    /// Keep raw responses available for state transitions that also consume
    /// events or need method-specific error handling.
    fn call(&mut self, method: &str, params: Option<Value>) -> RpcResponse {
        self.client.call(method, params)
    }

    /// Request a JSON value while normalizing transport errors in one place.
    pub fn request_value(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<Option<Value>, RpcError> {
        decode_response(method, self.call(method, params))
    }

    pub fn request_async(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> std::sync::mpsc::Receiver<RpcResponse> {
        self.client.request_async(method, params)
    }

    pub fn submit(&self, method: &str, params: Option<Value>) {
        self.client.submit(method, params);
    }

    pub fn restart_sidecar(&self) -> Result<(), String> {
        self.client.restart_sidecar()
    }

    pub fn start_or_connect(&self) -> Result<BackendAttach, String> {
        self.client.start_or_connect()
    }

    pub fn decode_response(method: &str, response: RpcResponse) -> Result<Option<Value>, RpcError> {
        decode_response(method, response)
    }

    /// Request and deserialize a typed payload at the IPC boundary.
    pub fn request<T: DeserializeOwned>(
        &mut self,
        method: &str,
        params: Option<Value>,
    ) -> Result<T, RpcError> {
        let value = self
            .request_value(method, params)?
            .ok_or_else(|| RpcError::new(-32603, format!("{method} returned no result")))?;
        serde_json::from_value(value).map_err(|error| {
            RpcError::new(
                -32603,
                format!("{method} returned an invalid payload: {error}"),
            )
        })
    }

    pub fn take_events(&mut self) -> Vec<EventEnvelope> {
        self.client.take_events()
    }

    pub fn take_completions(&self) -> Vec<RpcCompletion> {
        self.client.take_completions()
    }

    pub async fn wait_for_activity(&self) -> bool {
        self.client.activity().wait().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ipc::contract::method, model::TasksConfig};

    #[test]
    fn typed_requests_decode_through_the_gateway() {
        let mut gateway = RpcGateway::new(BackendClient::mock());
        let config: TasksConfig = gateway
            .request(method::TASKS_GET_CONFIG, None)
            .expect("mock task config should decode");
        assert_eq!(config.set_windows.set_win_size, 1080);
    }

    #[test]
    fn gateway_preserves_structured_errors() {
        let mut gateway = RpcGateway::new(BackendClient::mock());
        let error = gateway
            .request_value("not.implemented", None)
            .expect_err("unknown methods should remain structured errors");
        assert_eq!(error.code, -32601);
    }
}
