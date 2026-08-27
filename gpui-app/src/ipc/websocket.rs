use std::{
    collections::HashMap,
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
    contract::{EventEnvelope, RequestId, RpcError, RpcRequest, RpcResponse},
};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(180);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const READ_POLL_TIMEOUT: Duration = Duration::from_millis(100);

/// A local Python sidecar client.
///
/// The WebSocket itself is owned by a worker thread.  UI calls only enqueue a
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
    events: Arc<Mutex<Vec<EventEnvelope>>>,
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

enum WorkerCommand {
    Request(RpcRequest, Sender<RpcResponse>),
    Shutdown,
}

struct SidecarGuard {
    child: Mutex<Child>,
}

impl Drop for SidecarGuard {
    fn drop(&mut self) {
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl WebSocketClient {
    /// Preserve the old constructor shape for callers while exposing a
    /// structured unavailable response if the sidecar cannot be started.
    pub fn new() -> Self {
        Self::try_new().unwrap_or_else(|error| Self::failed(error))
    }

    /// Start the configured Python sidecar and connect to its loopback socket.
    pub fn try_new() -> Result<Self, String> {
        if let Ok(url) = env::var("AHAB_BACKEND_URL") {
            let token = env::var("AHAB_BACKEND_TOKEN")
                .map_err(|_| "AHAB_BACKEND_TOKEN is required with AHAB_BACKEND_URL".to_owned())?;
            return Self::connect_url(&url, &token, None);
        }

        let (sidecar, port, token) = spawn_sidecar()?;
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
                events: Arc::new(Mutex::new(Vec::new())),
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
        std::mem::take(&mut *events)
    }

    pub fn request(&self, method: impl Into<String>, params: Option<Value>) -> RpcResponse {
        let request = RpcRequest::new(
            self.shared.next_id.fetch_add(1, Ordering::Relaxed),
            method,
            params,
        );
        self.send_request(request)
    }

    fn connect_url(
        url: &str,
        token: &str,
        sidecar: Option<Arc<SidecarGuard>>,
    ) -> Result<Self, String> {
        let address = parse_loopback_address(url)?;
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
            message_text(hello).ok_or_else(|| "sidecar 鉴权响应不是文本".to_owned())?;
        let hello_value: Value = serde_json::from_str(&hello_text)
            .map_err(|error| format!("sidecar 鉴权响应无效：{error}"))?;
        if hello_value.get("type") != Some(&Value::String("hello".into()))
            || hello_value.get("ok") != Some(&Value::Bool(true))
        {
            return Err("sidecar 鉴权失败".to_owned());
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
        let events = Arc::new(Mutex::new(Vec::new()));
        let connected_flag = Arc::new(AtomicBool::new(true));
        let error = Arc::new(Mutex::new(None));
        let worker_events = Arc::clone(&events);
        let worker_connected = Arc::clone(&connected_flag);
        let worker_error = Arc::clone(&error);

        thread::Builder::new()
            .name("AhabSidecarWebSocket".into())
            .spawn(move || {
                run_worker(
                    socket,
                    commands_rx,
                    worker_events,
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
                commands: Mutex::new(Some(commands_tx)),
                _sidecar: sidecar,
            }),
        })
    }

    pub(crate) fn send_request(&self, request: RpcRequest) -> RpcResponse {
        let id = request.id;
        if !self.is_connected() {
            return self.unavailable_response(id);
        }
        let Some(commands) = self
            .shared
            .commands
            .lock()
            .ok()
            .and_then(|mut commands| commands.as_mut().cloned())
        else {
            return self.unavailable_response(id);
        };
        let (response_tx, response_rx) = mpsc::channel();
        if let Err(error) = commands.send(WorkerCommand::Request(request, response_tx)) {
            return RpcResponse::failure(
                id,
                RpcError::new(-32000, format!("sidecar 请求发送失败：{error}")),
            );
        }
        match response_rx.recv_timeout(REQUEST_TIMEOUT) {
            Ok(response) => response,
            Err(error) => RpcResponse::failure(
                id,
                RpcError::new(-32000, format!("sidecar 请求超时或已断开：{error}")),
            ),
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

fn run_worker(
    mut socket: tungstenite::WebSocket<TcpStream>,
    commands: Receiver<WorkerCommand>,
    events: Arc<Mutex<Vec<EventEnvelope>>>,
    connected: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
) {
    let mut pending: HashMap<RequestId, Sender<RpcResponse>> = HashMap::new();
    let mut fatal_error: Option<String> = None;

    loop {
        loop {
            let command = match commands.try_recv() {
                Ok(command) => command,
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => {
                    fatal_error = Some("sidecar client dropped".to_owned());
                    break;
                }
            };
            match command {
                WorkerCommand::Request(request, response_tx) => {
                    let id = request.id;
                    match serde_json::to_string(&request)
                        .map_err(|error| error.to_string())
                        .and_then(|payload| {
                            socket
                                .send(Message::text(payload))
                                .map_err(|error| error.to_string())
                        }) {
                        Ok(()) => {
                            pending.insert(id, response_tx);
                        }
                        Err(send_error) => {
                            let _ = response_tx.send(RpcResponse::failure(
                                id,
                                RpcError::new(-32000, format!("sidecar 写入失败：{send_error}")),
                            ));
                            fatal_error = Some(send_error);
                            break;
                        }
                    }
                }
                WorkerCommand::Shutdown => {
                    let _ = socket.close(None);
                    fatal_error = Some("sidecar client closed".to_owned());
                    break;
                }
            }
        }
        if fatal_error.is_some() {
            break;
        }

        match socket.read() {
            Ok(message) => {
                if let Some(text) = message_text(message) {
                    match serde_json::from_str::<Value>(&text) {
                        Ok(value) if value.get("event").and_then(Value::as_str).is_some() => {
                            if let Ok(event) = serde_json::from_value::<EventEnvelope>(value) {
                                if let Ok(mut queue) = events.lock() {
                                    queue.push(event);
                                }
                            }
                        }
                        Ok(value) => {
                            if let Some(id) = value.get("id").and_then(Value::as_u64) {
                                match serde_json::from_value::<RpcResponse>(value) {
                                    Ok(response) => {
                                        if let Some(waiter) = pending.remove(&id) {
                                            let _ = waiter.send(response);
                                        }
                                    }
                                    Err(parse_error) => {
                                        fatal_error =
                                            Some(format!("sidecar 响应无效：{parse_error}"));
                                        break;
                                    }
                                }
                            }
                        }
                        Err(parse_error) => {
                            fatal_error = Some(format!("sidecar 消息无效：{parse_error}"));
                            break;
                        }
                    }
                }
            }
            Err(error) if is_read_timeout(&error) => {}
            Err(error) => {
                fatal_error = Some(format!("sidecar 连接断开：{error}"));
                break;
            }
        }
    }

    let message = fatal_error.unwrap_or_else(|| "sidecar 连接已关闭".to_owned());
    connected.store(false, Ordering::Release);
    if let Ok(mut stored_error) = error.lock() {
        *stored_error = Some(message.clone());
    }
    for (id, waiter) in pending {
        let _ = waiter.send(RpcResponse::failure(
            id,
            RpcError::new(-32000, message.clone()),
        ));
    }
}

fn is_read_timeout(error: &TungsteniteError) -> bool {
    matches!(
        error,
        TungsteniteError::Io(io_error)
            if matches!(
                io_error.kind(),
                std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
            )
    )
}

fn message_text(message: Message) -> Option<String> {
    match message {
        Message::Text(text) => Some(text.to_string()),
        Message::Binary(bytes) => String::from_utf8(bytes.to_vec()).ok(),
        Message::Close(_) => None,
        Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => None,
    }
}

fn parse_loopback_address(url: &str) -> Result<SocketAddr, String> {
    let authority = url
        .strip_prefix("ws://")
        .ok_or_else(|| "sidecar URL 必须使用 ws://".to_owned())?
        .split('/')
        .next()
        .unwrap_or_default();
    let host_port = authority.split('?').next().unwrap_or_default().to_owned();
    let address: SocketAddr = host_port
        .parse()
        .map_err(|error| format!("无效的 sidecar 地址：{error}"))?;
    if !address.ip().is_loopback() {
        return Err("sidecar 只允许连接 loopback 地址".to_owned());
    }
    Ok(address)
}

fn spawn_sidecar() -> Result<(Arc<SidecarGuard>, u16, String), String> {
    let token = format!(
        "{:x}-{:x}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos()
    );
    let mut command = if let Ok(executable) = env::var("AHAB_BACKEND_EXE") {
        Command::new(executable)
    } else {
        let python = env::var("AHAB_PYTHON").unwrap_or_else(|_| "python".to_owned());
        let mut command = Command::new(python);
        command.arg("-u").arg(find_backend_script()?);
        command
    };
    command
        .arg("--host")
        .arg("127.0.0.1")
        .arg("--port")
        .arg("0")
        .arg("--token")
        .arg(&token)
        .arg("--parent-pid")
        .arg(std::process::id().to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = command
        .spawn()
        .map_err(|error| format!("启动 Python sidecar 失败：{error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "无法读取 Python sidecar 启动信息".to_owned())?;
    let ready = match read_ready_line(stdout) {
        Ok(ready) => ready,
        Err(error) => {
            terminate_child(&mut child);
            return Err(error);
        }
    };
    let Some(port) = ready.get("port").and_then(Value::as_u64) else {
        terminate_child(&mut child);
        return Err("sidecar 启动信息缺少端口".to_owned());
    };
    if port == 0 || port > u16::MAX as u64 {
        terminate_child(&mut child);
        return Err(format!("sidecar 返回了无效端口：{port}"));
    }
    if ready.get("ready") != Some(&Value::Bool(true)) {
        terminate_child(&mut child);
        return Err("sidecar 未报告 ready".to_owned());
    }
    Ok((
        Arc::new(SidecarGuard {
            child: Mutex::new(child),
        }),
        port as u16,
        token,
    ))
}

fn terminate_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn read_ready_line(stdout: impl std::io::Read + Send + 'static) -> Result<Value, String> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let result = reader
            .read_line(&mut line)
            .map_err(|error| format!("读取 sidecar 启动信息失败：{error}"))
            .and_then(|_| {
                serde_json::from_str::<Value>(line.trim())
                    .map_err(|error| format!("sidecar 启动信息无效：{error}"))
            });
        let _ = sender.send(result);
    });
    receiver
        .recv_timeout(Duration::from_secs(20))
        .map_err(|error| format!("等待 sidecar 启动超时：{error}"))?
}

fn find_backend_script() -> Result<PathBuf, String> {
    let candidates = [
        env::var_os("AHAB_BACKEND_SCRIPT").map(PathBuf::from),
        Some(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("main_backend.py"),
        ),
        env::current_dir()
            .ok()
            .map(|directory| directory.join("main_backend.py")),
    ];
    candidates
        .into_iter()
        .flatten()
        .map(|path| if path.is_file() { Ok(path) } else { Err(path) })
        .find_map(Result::ok)
        .ok_or_else(|| "找不到 main_backend.py，请设置 AHAB_BACKEND_SCRIPT".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_device_sidecar_ids_are_not_hardware_handles() {
        assert!(parse_loopback_address("ws://127.0.0.1:1234").is_ok());
        assert!(parse_loopback_address("ws://192.168.0.1:1234").is_err());
        assert!(parse_loopback_address("http://127.0.0.1:1234").is_err());
    }

    #[test]
    fn failed_client_returns_structured_error() {
        let client = WebSocketClient::failed("test unavailable");
        let response = client.request("device.list", None);
        assert_eq!(response.error.unwrap().code, -32000);
    }

    #[test]
    fn sidecar_round_trip_can_be_run_explicitly() {
        if std::env::var_os("AHAB_RUN_SIDECAR_INTEGRATION").is_none() {
            return;
        }
        let client = WebSocketClient::try_new().expect("sidecar should start");
        let response = client.request("device.list", None);
        assert!(response.error.is_none(), "sidecar response: {response:?}");
        assert!(response.result.unwrap().is_array());
    }
}
