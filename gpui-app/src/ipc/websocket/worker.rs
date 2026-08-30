use std::time::Instant;

use super::*;

pub(super) enum WorkerCommand {
    Request {
        request: RpcRequest,
        response_tx: Option<Sender<RpcResponse>>,
        report_completion: bool,
    },
    Shutdown,
}

struct PendingRequest {
    method: String,
    params: Option<Value>,
    response_tx: Option<Sender<RpcResponse>>,
    report_completion: bool,
    deadline: Instant,
}

pub(super) fn run_worker(
    mut socket: tungstenite::WebSocket<TcpStream>,
    commands: Receiver<WorkerCommand>,
    events: Arc<Mutex<VecDeque<EventEnvelope>>>,
    completions: Arc<Mutex<VecDeque<RpcCompletion>>>,
    connected: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
    activity: TransportActivity,
) {
    let mut pending: HashMap<RequestId, PendingRequest> = HashMap::new();
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
                WorkerCommand::Request {
                    request,
                    response_tx,
                    report_completion,
                } => {
                    let id = request.id;
                    let method = request.method.clone();
                    let params = request.params.clone();
                    match serde_json::to_string(&request)
                        .map_err(|error| error.to_string())
                        .and_then(|payload| {
                            socket
                                .send(Message::text(payload))
                                .map_err(|error| error.to_string())
                        }) {
                        Ok(()) => {
                            pending.insert(
                                id,
                                PendingRequest {
                                    method,
                                    params,
                                    response_tx,
                                    report_completion,
                                    deadline: Instant::now() + REQUEST_TIMEOUT,
                                },
                            );
                        }
                        Err(send_error) => {
                            let response = RpcResponse::failure(
                                id,
                                RpcError::new(-32000, format!("sidecar 写入失败：{send_error}")),
                            );
                            complete(
                                PendingRequest {
                                    method,
                                    params,
                                    response_tx,
                                    report_completion,
                                    deadline: Instant::now(),
                                },
                                response,
                                &completions,
                                &activity,
                            );
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
            Ok(Message::Binary(bytes)) => match decode_binary_event(bytes.as_ref()) {
                Ok(event) => push_event(&events, event, &activity),
                Err(parse_error) => {
                    fatal_error = Some(format!("sidecar 二进制事件无效：{parse_error}"));
                    break;
                }
            },
            Ok(message) => {
                if let Some(text) = message_text(message) {
                    match serde_json::from_str::<Value>(&text) {
                        Ok(value) if value.get("event").and_then(Value::as_str).is_some() => {
                            if let Ok(event) = serde_json::from_value::<EventEnvelope>(value) {
                                push_event(&events, event, &activity);
                            }
                        }
                        Ok(value) => {
                            if let Some(id) = value.get("id").and_then(Value::as_u64) {
                                match serde_json::from_value::<RpcResponse>(value) {
                                    Ok(response) => {
                                        if let Some(waiter) = pending.remove(&id) {
                                            complete(waiter, response, &completions, &activity);
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
            Err(read_error) if is_read_timeout(&read_error) => {}
            Err(read_error) => {
                fatal_error = Some(format!("sidecar 连接断开：{read_error}"));
                break;
            }
        }

        let now = Instant::now();
        let expired: Vec<RequestId> = pending
            .iter()
            .filter_map(|(id, request)| (request.deadline <= now).then_some(*id))
            .collect();
        for id in expired {
            if let Some(request) = pending.remove(&id) {
                complete(
                    request,
                    RpcResponse::failure(id, RpcError::new(-32000, "sidecar 请求超时")),
                    &completions,
                    &activity,
                );
            }
        }
    }

    let message = fatal_error.unwrap_or_else(|| "sidecar 连接已关闭".to_owned());
    connected.store(false, Ordering::Release);
    if let Ok(mut stored_error) = error.lock() {
        *stored_error = Some(message.clone());
    }
    for (id, request) in pending {
        complete(
            request,
            RpcResponse::failure(id, RpcError::new(-32000, message.clone())),
            &completions,
            &activity,
        );
    }
    activity.notify();
}

fn complete(
    pending: PendingRequest,
    response: RpcResponse,
    completions: &Arc<Mutex<VecDeque<RpcCompletion>>>,
    activity: &TransportActivity,
) {
    if let Some(waiter) = pending.response_tx {
        let _ = waiter.send(response.clone());
    }
    if pending.report_completion {
        let mut queued = false;
        if let Ok(mut queue) = completions.lock() {
            if queue.len() >= MAX_EVENT_QUEUE {
                queue.pop_front();
            }
            queue.push_back(RpcCompletion {
                method: pending.method,
                params: pending.params,
                response,
            });
            queued = true;
        }
        if queued {
            activity.notify();
        }
    }
}

fn push_event(
    events: &Arc<Mutex<VecDeque<EventEnvelope>>>,
    event: EventEnvelope,
    activity: &TransportActivity,
) {
    let mut queued = false;
    if let Ok(mut queue) = events.lock() {
        if event.event == super::super::contract::event::SCREENSHOT_FRAME {
            queue.retain(|queued| queued.event != super::super::contract::event::SCREENSHOT_FRAME);
        }
        if queue.len() >= MAX_EVENT_QUEUE {
            queue.pop_front();
        }
        queue.push_back(event);
        queued = true;
    }
    if queued {
        activity.notify();
    }
}

fn decode_binary_event(bytes: &[u8]) -> Result<EventEnvelope, String> {
    let length_bytes: [u8; 4] = bytes
        .get(..4)
        .ok_or_else(|| "missing metadata length".to_owned())?
        .try_into()
        .map_err(|_| "invalid metadata length".to_owned())?;
    let metadata_length = u32::from_be_bytes(length_bytes) as usize;
    let metadata_end = 4usize
        .checked_add(metadata_length)
        .ok_or_else(|| "metadata length overflow".to_owned())?;
    let metadata = bytes
        .get(4..metadata_end)
        .ok_or_else(|| "truncated metadata".to_owned())?;
    let binary = bytes
        .get(metadata_end..)
        .ok_or_else(|| "missing binary payload".to_owned())?;
    if binary.is_empty() {
        return Err("empty binary payload".to_owned());
    }
    let event: EventEnvelope =
        serde_json::from_slice(metadata).map_err(|error| error.to_string())?;
    if event.event != super::super::contract::event::SCREENSHOT_FRAME {
        return Err("unsupported binary event".to_owned());
    }
    Ok(event.with_binary(binary.to_vec()))
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
        Message::Binary(_) | Message::Close(_) => None,
        Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_screenshot_frame_splits_metadata_and_jpeg() {
        let metadata = br#"{"event":"screenshot.frame","payload":{"instanceId":"fixture","width":2,"height":1},"seq":7}"#;
        let mut frame = (metadata.len() as u32).to_be_bytes().to_vec();
        frame.extend_from_slice(metadata);
        frame.extend_from_slice(&[0xff, 0xd8, 0xff, 0xd9]);
        let event = decode_binary_event(&frame).expect("valid frame");
        assert_eq!(event.seq, Some(7));
        assert_eq!(event.binary.as_deref(), Some(&[0xff, 0xd8, 0xff, 0xd9][..]));
    }
}
