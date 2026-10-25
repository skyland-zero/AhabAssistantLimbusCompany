use super::*;

use std::sync::{Arc, Mutex};

// The public client is a cheap handle so every page in one GPUI window
// observes the same mock backend, just like the React app observes one IPC
// service.
#[derive(Clone)]
pub struct MockClient {
    inner: Arc<Mutex<MockState>>,
    activity: crate::ipc::TransportActivity,
}

impl Default for MockClient {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(MockState::default())),
            activity: crate::ipc::TransportActivity::new(),
        }
    }
}

impl MockClient {
    /// Return another handle to the same backend state.
    pub fn shared(&self) -> Self {
        self.clone()
    }

    pub fn call(&self, method_name: &str, params: Option<Value>) -> RpcResponse {
        let response = self
            .inner
            .lock()
            .expect("mock backend lock poisoned")
            .call(method_name, params);
        // Hydration requests can emit mock state/notice events; wake the app
        // pump so they are applied through the same path as real events.
        self.activity.notify();
        response
    }

    pub fn submit(&self, method_name: &str, params: Option<Value>) {
        self.inner
            .lock()
            .expect("mock backend lock poisoned")
            .submit(method_name, params);
        self.activity.notify();
    }

    pub fn activity(&self) -> crate::ipc::TransportActivity {
        self.activity.clone()
    }

    pub fn take_events(&self) -> Vec<EventEnvelope> {
        self.inner
            .lock()
            .expect("mock backend lock poisoned")
            .take_events()
    }

    pub fn take_completions(&self) -> Vec<super::super::contract::RpcCompletion> {
        self.inner
            .lock()
            .expect("mock backend lock poisoned")
            .take_completions()
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
        self.inner
            .lock()
            .expect("mock backend lock poisoned")
            .handle(request)
    }
}
