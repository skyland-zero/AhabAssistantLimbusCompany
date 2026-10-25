use std::{
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use serde_json::Value;

use super::contract::SessionInfo;
use super::{
    EventEnvelope, MockClient, RpcClient, RpcCompletion, RpcError, RpcRequest, RpcResponse,
    TransportActivity, contract, websocket::WebSocketClient,
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

    fn has_live_process(&self) -> bool {
        self.with_client(|| false, WebSocketClient::has_live_sidecar)
    }

    /// Reconnect to the current sidecar without replacing its process.
    fn reconnect(&self) -> Result<(), String> {
        if self.restarting.swap(true, Ordering::AcqRel) {
            return Err("Python sidecar 正在重连或重启".to_owned());
        }
        let result = (|| {
            let replacement = self
                .client
                .read()
                .map_err(|_| "sidecar 状态锁不可用".to_owned())?
                .reconnect()?;
            let mut client = self
                .client
                .write()
                .map_err(|_| "sidecar supervisor lock poisoned".to_owned())?;
            *client = replacement;
            Ok(())
        })();
        self.restarting.store(false, Ordering::Release);
        self.activity.notify();
        result
    }

    /// Ask the owned sidecar to close gracefully and wait briefly for the
    /// process to exit.  External sidecars are left under their owner's control.
    pub fn shutdown(&self, timeout: Duration) -> Result<(), String> {
        if !self.restartable {
            return Ok(());
        }
        if self.restarting.load(Ordering::Acquire) {
            return Ok(());
        }
        self.with_client(
            || Ok(()),
            |client| {
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
            },
        )
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

    fn request_async(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> async_channel::Receiver<RpcResponse> {
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

    fn session_info(&self) -> Option<SessionInfo> {
        self.with_client(|| None, WebSocketClient::session_info)
    }

    fn activity(&self) -> TransportActivity {
        self.activity.clone()
    }
}

fn ready_receiver(response: RpcResponse) -> async_channel::Receiver<RpcResponse> {
    let (sender, receiver) = async_channel::bounded(1);
    let _ = sender.send_blocking(response);
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
    /// A replacement sidecar process was started.
    Reused,
    /// The existing sidecar process was kept and only its transport was
    /// reconnected. UI state must not be reset in this case.
    Reconnected,
    /// An externally managed sidecar was attached through a new client.
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
                if supervisor.has_live_process() {
                    // A transport failure is not a process failure.  Keep the
                    // sidecar (and any Runner it owns) alive and retry the
                    // session before considering a process replacement.
                    supervisor.reconnect()?;
                    Ok(BackendAttach::Reconnected)
                } else {
                    supervisor.restart()?;
                    Ok(BackendAttach::Reused)
                }
            }
            Self::Sidecar(supervisor) => Ok(BackendAttach::New(Self::Sidecar(
                SidecarSupervisor::try_new_with_activity(supervisor.activity())?,
            ))),
        }
    }

    /// Ask the owned sidecar to close gracefully.  This is a no-op for Mock
    /// and for externally managed sidecars.
    pub fn shutdown(&self, timeout: Duration) -> Result<(), String> {
        match self {
            Self::Mock(_) => Ok(()),
            Self::Sidecar(supervisor) => supervisor.shutdown(timeout),
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

    pub fn call(&self, method: &str, params: Option<Value>) -> RpcResponse {
        match self {
            Self::Mock(client) => client.call(method, params),
            Self::Sidecar(supervisor) => supervisor.call(method, params),
        }
    }

    pub fn request_async(
        &self,
        method: &str,
        params: Option<Value>,
    ) -> async_channel::Receiver<RpcResponse> {
        match self {
            Self::Mock(client) => ready_receiver(client.call(method, params)),
            Self::Sidecar(supervisor) => supervisor.request_async(method, params),
        }
    }

    pub fn submit(&self, method: &str, params: Option<Value>) {
        match self {
            Self::Mock(client) => client.submit(method, params),
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
            Self::Mock(client) => client.take_completions(),
            Self::Sidecar(supervisor) => supervisor.take_completions(),
        }
    }

    pub fn session_info(&self) -> Option<SessionInfo> {
        match self {
            Self::Mock(_) => None,
            Self::Sidecar(supervisor) => supervisor.session_info(),
        }
    }

    pub(crate) fn activity(&self) -> TransportActivity {
        match self {
            Self::Mock(client) => client.activity(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn owned_sidecar_reconnects_without_restarting_the_process() {
        if std::env::var_os("AHAB_RUN_SIDECAR_INTEGRATION").is_none() {
            return;
        }
        let client = BackendClient::unavailable("integration test");
        let first = client.start_or_connect().expect("sidecar should start");
        assert!(matches!(first, BackendAttach::Reused));
        let second = client.start_or_connect().expect("sidecar should reconnect");
        assert!(matches!(second, BackendAttach::Reconnected));

        let response = client.call(contract::method::APP_PING, None);
        assert!(response.error.is_none(), "ping response: {response:?}");
        client
            .shutdown(Duration::from_secs(5))
            .expect("sidecar should shut down");
    }
}
