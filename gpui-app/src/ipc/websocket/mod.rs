//! WebSocket transport for the optional Python sidecar.
//!
//! The client lifecycle stays here, while socket event processing and sidecar
//! process management live in dedicated modules.

mod sidecar;
mod worker;

use sidecar::SidecarGuard;
use worker::{WorkerCommand, run_worker};

use std::{
    collections::{HashMap, VecDeque},
    env,
    io::{BufRead, BufReader},
    net::{SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::Duration,
};

use serde_json::{Value, json};
use tungstenite::{Error as TungsteniteError, Message};

use super::{
    RpcClient,
    contract::{
        EventEnvelope, RPC_SCHEMA_VERSION, RequestId, RpcCompletion, RpcError, RpcRequest,
        RpcResponse,
    },
};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const READ_POLL_TIMEOUT: Duration = Duration::from_millis(100);
const MAX_EVENT_QUEUE: usize = 512;

/// A local Python sidecar client.
///
/// The WebSocket itself is owned by a worker thread. UI calls only enqueue a
/// request and wait for its matching response; unsolicited events are queued
/// separately and can be drained by the GPUI entity without touching the
/// network from the render thread.
#[derive(Clone)]
pub struct WebSocketClient {
    shared: Arc<ClientShared>,
}

struct ClientShared {
    next_id: AtomicU64,
    connected: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
    events: Arc<Mutex<VecDeque<EventEnvelope>>>,
    completions: Arc<Mutex<VecDeque<RpcCompletion>>>,
    commands: Mutex<Option<Sender<WorkerCommand>>>,
    _sidecar: Option<Arc<SidecarGuard>>,
}

impl Drop for ClientShared {
    fn drop(&mut self) {
        if let Ok(mut commands) = self.commands.lock()
            && let Some(sender) = commands.take()
        {
            let _ = sender.send(WorkerCommand::Shutdown);
        }
    }
}

impl WebSocketClient {
    /// Preserve the old constructor shape for callers while exposing a
    /// structured unavailable response if the sidecar cannot be started.
    pub fn new() -> Self {
        Self::try_new().unwrap_or_else(Self::failed)
    }

    /// Start the configured Python sidecar and connect to its loopback socket.
    pub fn try_new() -> Result<Self, String> {
        if let Ok(url) = env::var("AHAB_BACKEND_URL") {
            let token = env::var("AHAB_BACKEND_TOKEN")
                .map_err(|_| "AHAB_BACKEND_TOKEN is required with AHAB_BACKEND_URL".to_owned())?;
            return Self::connect_url(&url, &token, None);
        }

        let (sidecar, port, token) = sidecar::spawn_sidecar()?;
        let url = format!("ws://127.0.0.1:{port}");
        Self::connect_url(&url, &token, Some(sidecar))
    }

    /// Return a client that reports an unavailable transport without panicking.
    pub fn failed(error: impl Into<String>) -> Self {
        Self {
            shared: Arc::new(ClientShared {
                next_id: AtomicU64::new(1),
                connected: Arc::new(AtomicBool::new(false)),
                error: Arc::new(Mutex::new(Some(error.into()))),
                events: Arc::new(Mutex::new(VecDeque::new())),
                completions: Arc::new(Mutex::new(VecDeque::new())),
                commands: Mutex::new(None),
                _sidecar: None,
            }),
        }
    }

    pub fn is_connected(&self) -> bool {
        self.shared.connected.load(Ordering::Acquire)
    }

    pub fn take_events(&self) -> Vec<EventEnvelope> {
        let Ok(mut events) = self.shared.events.lock() else {
            return Vec::new();
        };
        events.drain(..).collect()
    }

    pub fn take_completions(&self) -> Vec<RpcCompletion> {
        let Ok(mut completions) = self.shared.completions.lock() else {
            return Vec::new();
        };
        completions.drain(..).collect()
    }

    pub fn request(&self, method: impl Into<String>, params: Option<Value>) -> RpcResponse {
        let request = RpcRequest::new(
            self.shared.next_id.fetch_add(1, Ordering::Relaxed),
            method,
            params,
        );
        self.send_request(request)
    }

    pub fn request_async(
        &self,
        method: impl Into<String>,
        params: Option<Value>,
    ) -> Receiver<RpcResponse> {
        let request = RpcRequest::new(
            self.shared.next_id.fetch_add(1, Ordering::Relaxed),
            method,
            params,
        );
        let (sender, receiver) = mpsc::channel();
        self.enqueue_request(request, Some(sender), false);
        receiver
    }

    pub fn submit(&self, method: impl Into<String>, params: Option<Value>) {
        let request = RpcRequest::new(
            self.shared.next_id.fetch_add(1, Ordering::Relaxed),
            method,
            params,
        );
        self.enqueue_request(request, None, true);
    }

    fn connect_url(
        url: &str,
        token: &str,
        sidecar: Option<Arc<SidecarGuard>>,
    ) -> Result<Self, String> {
        let address = sidecar::parse_loopback_address(url)?;
        let stream = TcpStream::connect_timeout(&address, CONNECT_TIMEOUT)
            .map_err(|error| format!("连接 Python sidecar 失败：{error}"))?;
        stream
            .set_read_timeout(Some(CONNECT_TIMEOUT))
            .map_err(|error| format!("设置 sidecar 读取超时失败：{error}"))?;
        stream
            .set_write_timeout(Some(CONNECT_TIMEOUT))
            .map_err(|error| format!("设置 sidecar 写入超时失败：{error}"))?;

        let (mut socket, _) = tungstenite::client(url, stream)
            .map_err(|error| format!("WebSocket 握手失败：{error}"))?;
        socket
            .send(Message::text(
                json!({"type": "hello", "token": token}).to_string(),
            ))
            .map_err(|error| format!("sidecar 鉴权发送失败：{error}"))?;
        let hello = socket
            .read()
            .map_err(|error| format!("sidecar 鉴权读取失败：{error}"))?;
        let hello_text =
            worker::message_text(hello).ok_or_else(|| "sidecar 鉴权响应不是文本".to_owned())?;
        let hello_value: Value = serde_json::from_str(&hello_text)
            .map_err(|error| format!("sidecar 鉴权响应无效：{error}"))?;
        if hello_value.get("type") != Some(&Value::String("hello".into()))
            || hello_value.get("ok") != Some(&Value::Bool(true))
        {
            return Err("sidecar 鉴权失败".to_owned());
        }
        if hello_value.get("schemaVersion") != Some(&json!(RPC_SCHEMA_VERSION)) {
            return Err("sidecar RPC 协议版本不兼容".to_owned());
        }

        socket
            .get_mut()
            .set_read_timeout(Some(READ_POLL_TIMEOUT))
            .map_err(|error| format!("设置 sidecar 轮询超时失败：{error}"))?;
        socket
            .get_mut()
            .set_write_timeout(Some(CONNECT_TIMEOUT))
            .map_err(|error| format!("设置 sidecar 写入超时失败：{error}"))?;

        let (commands_tx, commands_rx) = mpsc::channel();
        let events = Arc::new(Mutex::new(VecDeque::new()));
        let completions = Arc::new(Mutex::new(VecDeque::new()));
        let connected_flag = Arc::new(AtomicBool::new(true));
        let error = Arc::new(Mutex::new(None));
        let worker_events = Arc::clone(&events);
        let worker_completions = Arc::clone(&completions);
        let worker_connected = Arc::clone(&connected_flag);
        let worker_error = Arc::clone(&error);

        thread::Builder::new()
            .name("AhabSidecarWebSocket".into())
            .spawn(move || {
                run_worker(
                    socket,
                    commands_rx,
                    worker_events,
                    worker_completions,
                    worker_connected,
                    worker_error,
                )
            })
            .map_err(|error| format!("启动 sidecar 网络线程失败：{error}"))?;

        Ok(Self {
            shared: Arc::new(ClientShared {
                next_id: AtomicU64::new(1),
                connected: connected_flag,
                error,
                events,
                completions,
                commands: Mutex::new(Some(commands_tx)),
                _sidecar: sidecar,
            }),
        })
    }

    pub(crate) fn send_request(&self, request: RpcRequest) -> RpcResponse {
        let id = request.id;
        let (response_tx, response_rx) = mpsc::channel();
        self.enqueue_request(request, Some(response_tx), false);
        match response_rx.recv_timeout(REQUEST_TIMEOUT) {
            Ok(response) => response,
            Err(error) => RpcResponse::failure(
                id,
                RpcError::new(-32000, format!("sidecar 请求超时或已断开：{error}")),
            ),
        }
    }

    fn enqueue_request(
        &self,
        request: RpcRequest,
        response_tx: Option<Sender<RpcResponse>>,
        report_completion: bool,
    ) {
        let id = request.id;
        if !self.is_connected() {
            let response = self.unavailable_response(id);
            if let Some(sender) = response_tx {
                let _ = sender.send(response.clone());
            }
            if report_completion {
                self.push_completion(&request, response);
            }
            return;
        }
        let Some(commands) = self
            .shared
            .commands
            .lock()
            .ok()
            .and_then(|mut commands| commands.as_mut().cloned())
        else {
            let response = self.unavailable_response(id);
            if let Some(sender) = response_tx {
                let _ = sender.send(response.clone());
            }
            if report_completion {
                self.push_completion(&request, response);
            }
            return;
        };
        let method = request.method.clone();
        let params = request.params.clone();
        if let Err(error) = commands.send(WorkerCommand::Request {
            request,
            response_tx,
            report_completion,
        }) {
            let response = RpcResponse::failure(
                id,
                RpcError::new(-32000, format!("sidecar 请求发送失败：{error}")),
            );
            if let WorkerCommand::Request { response_tx, .. } = error.0
                && let Some(sender) = response_tx
            {
                let _ = sender.send(response.clone());
            }
            if report_completion && let Ok(mut queue) = self.shared.completions.lock() {
                if queue.len() >= MAX_EVENT_QUEUE {
                    queue.pop_front();
                }
                queue.push_back(RpcCompletion {
                    method,
                    params,
                    response,
                });
            }
        }
    }

    fn push_completion(&self, request: &RpcRequest, response: RpcResponse) {
        if let Ok(mut queue) = self.shared.completions.lock() {
            if queue.len() >= MAX_EVENT_QUEUE {
                queue.pop_front();
            }
            queue.push_back(RpcCompletion {
                method: request.method.clone(),
                params: request.params.clone(),
                response,
            });
        }
    }

    fn unavailable_response(&self, id: RequestId) -> RpcResponse {
        let message = self
            .shared
            .error
            .lock()
            .ok()
            .and_then(|error| error.clone())
            .unwrap_or_else(|| "Python sidecar 不可用".to_owned());
        RpcResponse::failure(id, RpcError::new(-32000, message))
    }
}

impl Default for WebSocketClient {
    fn default() -> Self {
        Self::new()
    }
}

impl RpcClient for WebSocketClient {
    fn send(&mut self, request: RpcRequest) -> RpcResponse {
        self.send_request(request)
    }
}

#[cfg(test)]
mod tests;
