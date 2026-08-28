use serde::de::DeserializeOwned;
use serde_json::Value;

use super::{EventEnvelope, MockClient, RpcError, RpcResponse};

/// Application-facing RPC adapter.
///
/// State objects depend on this small boundary instead of knowing which
/// transport serves a method. MockClient can still route device methods to
/// the sidecar while the rest of the app remains deterministic in tests.
#[derive(Clone)]
pub struct RpcGateway {
    client: MockClient,
}

impl RpcGateway {
    pub fn new(client: MockClient) -> Self {
        Self { client }
    }

    pub fn shared(&self) -> Self {
        Self::new(self.client.shared())
    }

    pub fn is_sidecar(&self) -> bool {
        self.client.is_sidecar()
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
        let response = self.call(method, params);
        if let Some(error) = response.error {
            Err(error)
        } else {
            Ok(response.result)
        }
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ipc::contract::method, model::TasksConfig};

    #[test]
    fn typed_requests_decode_through_the_gateway() {
        let mut gateway = RpcGateway::new(MockClient::default());
        let config: TasksConfig = gateway
            .request(method::TASKS_GET_CONFIG, None)
            .expect("mock task config should decode");
        assert_eq!(config.set_windows.set_win_size, 1080);
    }

    #[test]
    fn gateway_preserves_structured_errors() {
        let mut gateway = RpcGateway::new(MockClient::default());
        let error = gateway
            .request_value("not.implemented", None)
            .expect_err("unknown methods should remain structured errors");
        assert_eq!(error.code, -32601);
    }
}
