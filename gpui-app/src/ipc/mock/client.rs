use super::*;

use super::super::contract::method;

use std::sync::{Arc, Mutex};

// The public client is a cheap handle so every page in one GPUI window
// observes the same mock backend, just like the React app observes one IPC
// service.
#[derive(Clone)]
pub struct MockClient {
    inner: Arc<Mutex<MockState>>,
    /// Device calls can be routed to the real Python sidecar while all other
    /// pages continue to use the deterministic mock during migration.
    remote: Option<Arc<super::super::websocket::WebSocketClient>>,
}

impl Default for MockClient {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(MockState::default())),
            remote: None,
        }
    }
}

impl MockClient {
    /// Create a hybrid client: real Python device IPC, deterministic mock for
    /// the remaining pages that have not migrated yet.
    pub fn try_sidecar() -> Result<Self, String> {
        Ok(Self {
            remote: Some(Arc::new(
                super::super::websocket::WebSocketClient::try_new()?
            )),
            ..Self::default()
        })
    }

    pub fn is_sidecar(&self) -> bool {
        self.remote.is_some()
    }

    /// Return another handle to the same backend state.
    pub fn shared(&self) -> Self {
        self.clone()
    }

    pub fn call(&mut self, method_name: &str, params: Option<Value>) -> RpcResponse {
        if is_remote_device_method(method_name)
            && let Some(remote) = self.remote.as_ref()
        {
            return remote.request(method_name, params);
        }
        self.inner
            .lock()
            .expect("mock backend lock poisoned")
            .call(method_name, params)
    }

    pub fn take_events(&mut self) -> Vec<EventEnvelope> {
        let mut events = self
            .inner
            .lock()
            .expect("mock backend lock poisoned")
            .take_events();
        if let Some(remote) = self.remote.as_ref() {
            events.extend(remote.take_events());
        }
        events
    }

    #[allow(dead_code)]
    pub fn device_status(&self) -> ConnectionStatus {
        self.inner
            .lock()
            .expect("mock backend lock poisoned")
            .device_status
    }
}

impl RpcClient for MockClient {
    fn send(&mut self, request: RpcRequest) -> RpcResponse {
        if is_remote_device_method(&request.method)
            && let Some(remote) = self.remote.as_ref()
        {
            return remote.send_request(request);
        }
        self.inner
            .lock()
            .expect("mock backend lock poisoned")
            .handle(request)
    }
}

fn is_remote_device_method(method_name: &str) -> bool {
    matches!(
        method_name,
        method::DEVICE_LIST | method::DEVICE_CONNECT | method::DEVICE_DISCONNECT
    )
}
