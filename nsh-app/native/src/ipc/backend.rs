//! The one client handle the app shares: a deterministic mock, the real
//! loopback sidecar, or an explicit unavailable transport.

use std::time::{Duration, Instant};

use serde_json::Value;

use super::{
    EventEnvelope, MockClient, RpcClient, RpcCompletion, RpcError, RpcRequest, RpcResponse,
    contract, websocket::WebSocketClient,
};

/// The transport behind every state object.
#[derive(Clone)]
pub enum BackendClient {
    Mock(MockClient),
    Sidecar(WebSocketClient),
    /// A sidecar that could not be started. Every call fails with the stored
    /// message instead of panicking, so the UI can show the reason.
    Unavailable(String),
}

impl BackendClient {
    pub fn mock() -> Self {
        Self::Mock(MockClient::default())
    }

    pub fn try_sidecar() -> Result<Self, String> {
        Ok(Self::Sidecar(WebSocketClient::try_new()?))
    }

    pub fn unavailable(error: impl Into<String>) -> Self {
        Self::Unavailable(error.into())
    }

    /// Ask the owned sidecar to close gracefully. Mock and externally managed
    /// sidecars are left alone.
    pub fn shutdown(&self, timeout: Duration) -> Result<(), String> {
        let Self::Sidecar(client) = self else {
            return Ok(());
        };
        let deadline = Instant::now() + timeout;
        let result = if client.is_connected() {
            decode_response(
                contract::method::APP_SHUTDOWN,
                client.request_with_timeout(contract::method::APP_SHUTDOWN, None, timeout),
            )
            .map(|_| ())
            .map_err(|error| error.message)
        } else {
            Ok(())
        };
        if result.is_ok() {
            let remaining = deadline.saturating_duration_since(Instant::now());
            let _ = client.wait_for_exit(remaining);
        }
        result
    }

    pub fn is_sidecar(&self) -> bool {
        matches!(self, Self::Sidecar(_))
    }

    pub fn shared(&self) -> Self {
        self.clone()
    }

    pub fn is_connected(&self) -> bool {
        match self {
            Self::Mock(_) => true,
            Self::Sidecar(client) => client.is_connected(),
            Self::Unavailable(_) => false,
        }
    }

    pub fn call(&self, method: &str, params: Option<Value>) -> RpcResponse {
        match self {
            Self::Mock(client) => client.call(method, params),
            Self::Sidecar(client) => client.request(method.to_owned(), params),
            Self::Unavailable(error) => {
                RpcResponse::failure(0, RpcError::new(-32000, error.clone()))
            }
        }
    }

    pub fn submit(&self, method: &str, params: Option<Value>) {
        match self {
            Self::Mock(client) => client.submit(method, params),
            Self::Sidecar(client) => client.submit(method.to_owned(), params),
            Self::Unavailable(_) => {}
        }
    }

    pub fn take_events(&self) -> Vec<EventEnvelope> {
        match self {
            Self::Mock(client) => client.take_events(),
            Self::Sidecar(client) => client.take_events(),
            Self::Unavailable(_) => Vec::new(),
        }
    }

    pub fn take_completions(&self) -> Vec<RpcCompletion> {
        match self {
            Self::Mock(client) => client.take_completions(),
            Self::Sidecar(client) => client.take_completions(),
            Self::Unavailable(_) => Vec::new(),
        }
    }

    pub fn session_info(&self) -> Option<contract::SessionInfo> {
        match self {
            Self::Mock(_) | Self::Unavailable(_) => None,
            Self::Sidecar(client) => client.session_info(),
        }
    }

    /// A short label for diagnostics (`mock`, `sidecar`, `unavailable`).
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Mock(_) => "mock",
            Self::Sidecar(_) => "sidecar",
            Self::Unavailable(_) => "unavailable",
        }
    }
}

impl From<MockClient> for BackendClient {
    fn from(client: MockClient) -> Self {
        Self::Mock(client)
    }
}

impl From<WebSocketClient> for BackendClient {
    fn from(client: WebSocketClient) -> Self {
        Self::Sidecar(client)
    }
}

impl RpcClient for BackendClient {
    fn send(&mut self, request: RpcRequest) -> RpcResponse {
        let RpcRequest { method, params, .. } = request;
        self.call(&method, params)
    }
}

pub fn decode_response(method: &str, response: RpcResponse) -> Result<Option<Value>, RpcError> {
    if let Some(error) = response.error {
        Err(error)
    } else {
        response
            .result
            .map(Some)
            .ok_or_else(|| RpcError::new(-32603, format!("{method} returned no result")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::contract::method;
    use serde_json::json;

    #[test]
    fn mock_round_trips_through_the_backend_boundary() {
        let client = BackendClient::mock();
        assert!(client.is_connected());
        assert_eq!(client.kind(), "mock");
        let response = client.call(method::APP_PING, None);
        assert_eq!(response.result, Some(json!("pong")));
    }

    #[test]
    fn unavailable_is_a_structured_error_not_a_panic() {
        let client = BackendClient::unavailable("sidecar missing");
        assert!(!client.is_connected());
        let error = decode_response(method::APP_PING, client.call(method::APP_PING, None))
            .expect_err("unavailable backend must fail");
        assert_eq!(error.code, -32000);
        assert_eq!(error.message, "sidecar missing");
    }

    #[test]
    fn real_sidecar_round_trips_when_the_integration_flag_is_set() {
        if std::env::var_os("AHAB_RUN_SIDECAR_INTEGRATION").is_none() {
            return;
        }
        let client = BackendClient::try_sidecar().expect("sidecar should start");
        assert!(client.is_connected());
        let response = client.call(method::APP_PING, None);
        assert_eq!(response.result, Some(json!("pong")));
        let version = client.call(method::APP_VERSION, None);
        assert!(!version.is_error(), "version response: {version:?}");
        client
            .shutdown(std::time::Duration::from_secs(5))
            .expect("sidecar should shut down");
    }
}
