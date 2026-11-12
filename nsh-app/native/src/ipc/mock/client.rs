//! The public mock handle: cheap to clone, one shared state per backend.

use super::*;

use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct MockClient {
    inner: Arc<Mutex<MockState>>,
}

impl Default for MockClient {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(MockState::default())),
        }
    }
}

impl MockClient {
    /// Return another handle to the same backend state.
    pub fn shared(&self) -> Self {
        self.clone()
    }

    pub fn call(&self, method_name: &str, params: Option<Value>) -> RpcResponse {
        self.inner
            .lock()
            .expect("mock backend lock poisoned")
            .call(method_name, params)
    }

    pub fn submit(&self, method_name: &str, params: Option<Value>) {
        self.inner
            .lock()
            .expect("mock backend lock poisoned")
            .submit(method_name, params);
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
