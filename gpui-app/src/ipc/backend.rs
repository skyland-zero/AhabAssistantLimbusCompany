use std::sync::{
    Arc, RwLock,
    atomic::{AtomicBool, Ordering},
    mpsc::{self, Receiver},
};

use serde_json::Value;

use super::{
    EventEnvelope, MockClient, RpcClient, RpcCompletion, RpcError, RpcRequest, RpcResponse,
    TransportActivity, websocket::WebSocketClient,
};

#[derive(Clone)]
pub struct SidecarSupervisor {
    client: Arc<RwLock<WebSocketClient>>,
    activity: TransportActivity,
    restartable: bool,
    restarting: Arc<AtomicBool>,
}

impl SidecarSupervisor {
    fn try_new() -> Result<Self, String> {
        Self::try_new_with_activity(TransportActivity::new())
    }

    fn try_new_with_activity(activity: TransportActivity) -> Result<Self, String> {
        let restartable = std::env::var_os("AHAB_BACKEND_URL").is_none();
        Ok(Self {
            client: Arc::new(RwLock::new(WebSocketClient::try_new_with_activity(
                activity.clone(),
            )?)),
            activity,
            restartable,
            restarting: Arc::new(AtomicBool::new(false)),
        })
    }

    fn unavailable(error: impl Into<String>) -> Self {
        let activity = TransportActivity::new();
        Self {
            client: Arc::new(RwLock::new(WebSocketClient::failed_with_activity(
                error,
                activity.clone(),
            ))),
            activity,
            restartable: std::env::var_os("AHAB_BACKEND_URL").is_none(),
            restarting: Arc::new(AtomicBool::new(false)),
        }
    }

    fn with_client<T>(
        &self,
        fallback: impl FnOnce() -> T,
        call: impl FnOnce(&WebSocketClient) -> T,
    ) -> T {
        self.client
            .read()
            .map(|client| call(&client))
            .unwrap_or_else(|_| fallback())
    }

    fn restarting_response(id: u64) -> RpcResponse {
        RpcResponse::failure(id, RpcError::new(-32001, "Python sidecar 正在重启"))
    }

    pub fn restart(&self) -> Result<(), String> {
        if !self.restartable {
            return Err("外部 sidecar 不能由 GPUI 重启".to_owned());
        }
        if self.restarting.swap(true, Ordering::AcqRel) {
            return Err("Python sidecar 已在重启".to_owned());
        }
        let result =
            WebSocketClient::try_new_with_activity(self.activity.clone()).and_then(|replacement| {
                let mut client = self
                    .client
                    .write()
                    .map_err(|_| "sidecar supervisor lock poisoned".to_owned())?;
                *client = replacement;
                Ok(())
            });
        self.restarting.store(false, Ordering::Release);
        self.activity.notify();
        result
    }

    fn is_connected(&self) -> bool {
        !self.restarting.load(Ordering::Acquire)
            && self.with_client(|| false, WebSocketClient::is_connected)
    }

    fn call(&self, method: &str, params: Option<Value>) -> RpcResponse {
        if self.restarting.load(Ordering::Acquire) {
            return Self::restarting_response(0);
        }
        self.with_client(
            || RpcResponse::failure(0, RpcError::new(-32000, "sidecar 状态锁不可用")),
            |client| client.request(method.to_owned(), params),
        )
    }

    fn request_async(&self, method: &str, params: Option<Value>) -> Receiver<RpcResponse> {
        if self.restarting.load(Ordering::Acquire) {
            return ready_receiver(Self::restarting_response(0));
        }
        self.with_client(
            || {
                ready_receiver(RpcResponse::failure(
                    0,
                    RpcError::new(-32000, "sidecar 状态锁不可用"),
                ))
            },
            |client| client.request_async(method.to_owned(), params),
        )
    }

    fn submit(&self, method: &str, params: Option<Value>) {
        if self.restarting.load(Ordering::Acquire) {
            return;
        }
        self.with_client(|| (), |client| client.submit(method.to_owned(), params));
    }

    fn take_events(&self) -> Vec<EventEnvelope> {
        self.with_client(Vec::new, WebSocketClient::take_events)
    }

    fn take_completions(&self) -> Vec<RpcCompletion> {
        self.with_client(Vec::new, WebSocketClient::take_completions)
    }

    fn activity(&self) -> TransportActivity {
        self.activity.clone()
    }
}

fn ready_receiver(response: RpcResponse) -> Receiver<RpcResponse> {
    let (sender, receiver) = mpsc::channel();
    let _ = sender.send(response);
    receiver
}

/// Describes whether starting the backend reused the shared owned-sidecar
/// supervisor or produced a new client for an externally managed sidecar.
///
/// Owned sidecars must be restarted through the existing supervisor so every
/// page keeps the same transport and an old child process is not left behind.
/// External sidecars cannot be replaced by that supervisor, so a new client is
/// handed back to the app after a successful connection.
pub enum BackendAttach {
    Reused,
    New(BackendClient),
}

/// The one client handle shared by all GPUI pages.
#[derive(Clone)]
pub enum BackendClient {
    Mock(MockClient),
    Sidecar(SidecarSupervisor),
}

impl BackendClient {
    pub fn mock() -> Self {
        Self::Mock(MockClient::default())
    }

    pub fn try_sidecar() -> Result<Self, String> {
        Ok(Self::Sidecar(SidecarSupervisor::try_new()?))
    }

    pub fn unavailable(error: impl Into<String>) -> Self {
        Self::Sidecar(SidecarSupervisor::unavailable(error))
    }

    /// Start or reconnect the configured sidecar without changing the page
    /// layer's transport contract.
    ///
    /// For a child owned by GPUI, the supervisor replaces its WebSocket client
    /// in place. For an externally managed sidecar, the connection attempt
    /// creates a new client that the app must install into all page gateways.
    pub fn start_or_connect(&self) -> Result<BackendAttach, String> {
        match self {
            Self::Mock(_) => Err("mock backend does not start a sidecar".to_owned()),
            Self::Sidecar(supervisor) if supervisor.restartable => {
                supervisor.restart()?;
                Ok(BackendAttach::Reused)
            }
            Self::Sidecar(supervisor) => Ok(BackendAttach::New(Self::Sidecar(
                SidecarSupervisor::try_new_with_activity(supervisor.activity())?,
            ))),
        }
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
            Self::Sidecar(supervisor) => supervisor.is_connected(),
        }
    }

    pub fn restart_sidecar(&self) -> Result<(), String> {
        match self {
            Self::Mock(_) => Err("mock backend does not restart".to_owned()),
            Self::Sidecar(supervisor) => supervisor.restart(),
        }
    }

    pub fn call(&self, method: &str, params: Option<Value>) -> RpcResponse {
        match self {
            Self::Mock(client) => client.call(method, params),
            Self::Sidecar(supervisor) => supervisor.call(method, params),
        }
    }

    pub fn request_async(&self, method: &str, params: Option<Value>) -> Receiver<RpcResponse> {
        match self {
            Self::Mock(client) => ready_receiver(client.call(method, params)),
            Self::Sidecar(supervisor) => supervisor.request_async(method, params),
        }
    }

    pub fn submit(&self, method: &str, params: Option<Value>) {
        match self {
            Self::Mock(_) => {}
            Self::Sidecar(supervisor) => supervisor.submit(method, params),
        }
    }

    pub fn take_events(&self) -> Vec<EventEnvelope> {
        match self {
            Self::Mock(client) => client.take_events(),
            Self::Sidecar(supervisor) => supervisor.take_events(),
        }
    }

    pub fn take_completions(&self) -> Vec<RpcCompletion> {
        match self {
            Self::Mock(_) => Vec::new(),
            Self::Sidecar(supervisor) => supervisor.take_completions(),
        }
    }

    pub(crate) fn activity(&self) -> TransportActivity {
        match self {
            Self::Mock(_) => TransportActivity::new(),
            Self::Sidecar(supervisor) => supervisor.activity(),
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
        let activity = client.activity();
        Self::Sidecar(SidecarSupervisor {
            client: Arc::new(RwLock::new(client)),
            activity,
            restartable: false,
            restarting: Arc::new(AtomicBool::new(false)),
        })
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
