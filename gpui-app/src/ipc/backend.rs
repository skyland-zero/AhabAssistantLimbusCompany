use std::sync::mpsc::{self, Receiver};
use std::thread;

use serde_json::Value;

use super::{
    EventEnvelope, MockClient, RpcClient, RpcError, RpcRequest, RpcResponse,
    websocket::WebSocketClient,
};

/// The one client handle shared by all GPUI pages.
///
/// Keeping the transport choice here makes the page states usable in tests
/// without allowing production code to silently substitute a mock backend.
#[derive(Clone)]
pub enum BackendClient {
    Mock(MockClient),
    Sidecar(WebSocketClient),
}

impl BackendClient {
    pub fn mock() -> Self {
        Self::Mock(MockClient::default())
    }

    pub fn try_sidecar() -> Result<Self, String> {
        Ok(Self::Sidecar(WebSocketClient::try_new()?))
    }

    /// Keep a failed sidecar as a real transport boundary. Requests return a
    /// structured error and the UI can expose retry/offline feedback instead
    /// of rendering fabricated business data.
    pub fn unavailable(error: impl Into<String>) -> Self {
        Self::Sidecar(WebSocketClient::failed(error))
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
        }
    }

    pub fn call(&self, method: &str, params: Option<Value>) -> RpcResponse {
        match self {
            Self::Mock(client) => client.call(method, params),
            Self::Sidecar(client) => client.request(method.to_owned(), params),
        }
    }

    /// Submit a request on a worker thread. This is the API used by runtime
    /// page actions; it never waits for the socket on the GPUI render thread.
    pub fn request_async(&self, method: &str, params: Option<Value>) -> Receiver<RpcResponse> {
        let client = self.clone();
        let method = method.to_owned();
        let (sender, receiver) = mpsc::channel();
        thread::Builder::new()
            .name("AhabBackendRequest".into())
            .spawn(move || {
                let _ = sender.send(client.call(&method, params));
            })
            .expect("failed to start backend request thread");
        receiver
    }

    pub fn take_events(&self) -> Vec<EventEnvelope> {
        match self {
            Self::Mock(client) => client.take_events(),
            Self::Sidecar(client) => client.take_events(),
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
        match self {
            Self::Mock(client) => client.send(request),
            Self::Sidecar(client) => client.send(request),
        }
    }
}

/// Decode a response returned by either transport at one shared boundary.
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
