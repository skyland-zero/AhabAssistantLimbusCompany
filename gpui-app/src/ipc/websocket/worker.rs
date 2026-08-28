use super::*;

pub(super) enum WorkerCommand {
    Request(RpcRequest, Sender<RpcResponse>),
    Shutdown,
}

pub(super) fn run_worker(
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
                            if let Ok(event) = serde_json::from_value::<EventEnvelope>(value)
                                && let Ok(mut queue) = events.lock()
                            {
                                queue.push(event);
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

pub(super) fn message_text(message: Message) -> Option<String> {
    match message {
        Message::Text(text) => Some(text.to_string()),
        Message::Binary(bytes) => String::from_utf8(bytes.to_vec()).ok(),
        Message::Close(_) => None,
        Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => None,
    }
}
