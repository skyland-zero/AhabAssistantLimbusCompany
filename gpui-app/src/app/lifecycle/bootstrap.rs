use super::*;

impl AhabApp {
    pub fn start_backend_hydration(&mut self, cx: &mut Context<Self>) {
        if !self.home.rpc.is_sidecar() {
            return;
        }

        let backend_epoch = self.backend_epoch;
        let home_rpc = self.home.rpc.clone();
        let teams_rpc = self.teams.rpc.clone();
        let themes_rpc = self.theme_packs.rpc.clone();
        let resources_rpc = self.resources.rpc.clone();
        let settings_rpc = self.settings_page.rpc.clone();
        cx.spawn(async move |this, cx| {
            let tasks_request = home_rpc.request_async(method::TASKS_GET_CONFIG, None);
            let devices_request = home_rpc.request_async(method::DEVICE_LIST, None);
            let execution_request = home_rpc.request_async(method::EXECUTION_GET_STATE, None);
            let stats_request = home_rpc.request_async(method::STATS_GET_SUMMARY, None);
            let teams_request = teams_rpc.request_async(method::TEAM_LIST, None);
            let sinners_request = teams_rpc.request_async(method::SINNER_LIST, None);
            let presets_request = teams_rpc.request_async(method::TEAM_PRESET_LIST, None);
            let themes_request = themes_rpc.request_async(method::THEME_PACK_LIST, None);
            let resources_request = resources_rpc.request_async(method::RESOURCE_STATUS, None);
            let hotkey_request = settings_rpc.request_async(method::HOTKEY_GET, None);
            let system_request = settings_rpc.request_async(method::SYSTEM_SETTINGS_GET, None);

            // Submit every read before awaiting the receivers.  The websocket
            // worker can pipeline these requests and the sidecar read pool
            // handles them concurrently, reducing first-paint hydration time
            // without touching GPUI state from background threads.
            let tasks_response = cx
                .background_executor()
                .spawn(async move { tasks_request.recv().await.ok() });
            let devices_response = cx
                .background_executor()
                .spawn(async move { devices_request.recv().await.ok() });
            let execution_response = cx
                .background_executor()
                .spawn(async move { execution_request.recv().await.ok() });
            let stats_response = cx
                .background_executor()
                .spawn(async move { stats_request.recv().await.ok() });
            let teams_response = cx
                .background_executor()
                .spawn(async move { teams_request.recv().await.ok() });
            let sinners_response = cx
                .background_executor()
                .spawn(async move { sinners_request.recv().await.ok() });
            let presets_response = cx
                .background_executor()
                .spawn(async move { presets_request.recv().await.ok() });
            let themes_response = cx
                .background_executor()
                .spawn(async move { themes_request.recv().await.ok() });
            let resources_response = cx
                .background_executor()
                .spawn(async move { resources_request.recv().await.ok() });
            let hotkey_response = cx
                .background_executor()
                .spawn(async move { hotkey_request.recv().await.ok() });
            let system_response = cx
                .background_executor()
                .spawn(async move { system_request.recv().await.ok() });

            let tasks_response = tasks_response.await;
            let devices_response = devices_response.await;
            let execution_response = execution_response.await;
            let stats_response = stats_response.await;
            let teams_response = teams_response.await;
            let sinners_response = sinners_response.await;
            let presets_response = presets_response.await;
            let themes_response = themes_response.await;
            let resources_response = resources_response.await;
            let hotkey_response = hotkey_response.await;
            let system_response = system_response.await;

            let _ = this.update(cx, |view, cx| {
                if view.backend_epoch != backend_epoch {
                    return;
                }
                if let Some(response) = tasks_response
                    && let Ok(Some(value)) =
                        RpcGateway::decode_response(method::TASKS_GET_CONFIG, response)
                    && let Ok(tasks) = serde_json::from_value(value)
                {
                    view.home.tasks = tasks;
                }
                if let Some(response) = devices_response
                    && let Ok(Some(value)) =
                        RpcGateway::decode_response(method::DEVICE_LIST, response)
                    && let Ok(devices) = serde_json::from_value(value)
                {
                    view.home.devices = devices;
                }
                if let Some(response) = execution_response
                    && let Ok(Some(value)) =
                        RpcGateway::decode_response(method::EXECUTION_GET_STATE, response)
                    && let Ok(execution) = serde_json::from_value(value)
                {
                    view.home.apply_execution_status(execution);
                }
                if let Some(response) = stats_response
                    && let Ok(Some(value)) =
                        RpcGateway::decode_response(method::STATS_GET_SUMMARY, response)
                {
                    view.home.apply_stats_summary(value);
                }
                if let Some(response) = teams_response
                    && let Ok(Some(value)) =
                        RpcGateway::decode_response(method::TEAM_LIST, response)
                    && let Ok(teams) = serde_json::from_value(value)
                {
                    view.teams.teams = teams;
                }
                if let Some(response) = sinners_response
                    && let Ok(Some(value)) =
                        RpcGateway::decode_response(method::SINNER_LIST, response)
                    && let Ok(sinners) = serde_json::from_value(value)
                {
                    view.teams.sinners = sinners;
                }
                if let Some(response) = presets_response
                    && let Ok(Some(value)) =
                        RpcGateway::decode_response(method::TEAM_PRESET_LIST, response)
                    && let Ok(presets) = serde_json::from_value(value)
                {
                    view.teams.presets = presets;
                }
                if let Some(response) = themes_response
                    && let Ok(Some(value)) =
                        RpcGateway::decode_response(method::THEME_PACK_LIST, response)
                    && let Ok(data) = serde_json::from_value(value)
                {
                    view.theme_packs.data = data;
                }
                if let Some(response) = resources_response
                    && let Ok(Some(value)) =
                        RpcGateway::decode_response(method::RESOURCE_STATUS, response)
                    && let Ok(groups) = serde_json::from_value(value)
                {
                    view.resources.groups = groups;
                }
                if let Some(response) = hotkey_response
                    && let Ok(Some(value)) =
                        RpcGateway::decode_response(method::HOTKEY_GET, response)
                    && let Ok(hotkey) = serde_json::from_value(value)
                {
                    view.settings_page.hotkey = hotkey;
                }
                if let Some(response) = system_response
                    && let Ok(Some(value)) =
                        RpcGateway::decode_response(method::SYSTEM_SETTINGS_GET, response)
                    && let Ok(system) = serde_json::from_value(value)
                {
                    view.settings_page.system = system;
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn backend_attempt_is_current(&self, attempt_id: u64) -> bool {
        match self.backend_operation {
            BackendOperation::Connecting {
                attempt_id: current,
                ..
            }
            | BackendOperation::WaitingRetry {
                attempt_id: current,
                ..
            } => current == attempt_id,
            BackendOperation::Idle => false,
        }
    }

    pub(crate) fn install_backend_client(&mut self, client: BackendClient) {
        self.home.rpc = RpcGateway::new(client.shared());
        self.teams.rpc = RpcGateway::new(client.shared());
        self.theme_packs.rpc = RpcGateway::new(client.shared());
        self.toolbox.rpc = RpcGateway::new(client.shared());
        self.resources.rpc = RpcGateway::new(client.shared());
        self.settings_page.rpc = RpcGateway::new(client);
    }

    /// Start a backend connection after the first GPUI frame. Every blocking
    /// process/network operation stays on the background executor, while this
    /// entity only owns the state transitions and UI notifications.
    pub fn start_backend_bootstrap(&mut self, cx: &mut Context<Self>) {
        if self.backend_status.phase != BackendPhase::WaitingForFirstFrame {
            return;
        }
        self.start_backend_connection(cx, BackendStartReason::Initial);
    }

    /// Start a new automatic/manual recovery cycle after the automatic retry
    /// budget has been exhausted or the live connection has been lost.
    pub fn retry_backend(&mut self, cx: &mut Context<Self>) {
        let reason = match self.backend_status.phase {
            BackendPhase::Failed => BackendStartReason::ManualRetry,
            BackendPhase::Disconnected => BackendStartReason::Reconnect,
            _ => return,
        };
        self.start_backend_connection(cx, reason);
    }

    pub(crate) fn start_backend_connection(
        &mut self,
        cx: &mut Context<Self>,
        reason: BackendStartReason,
    ) {
        if self.backend_status.phase == BackendPhase::Mock || !self.backend_operation.is_idle() {
            return;
        }

        self.backend_attempt_id = self.backend_attempt_id.wrapping_add(1);
        let attempt_id = self.backend_attempt_id;
        let recovery = reason != BackendStartReason::Initial;
        let terminal_phase = match reason {
            BackendStartReason::Reconnect => BackendPhase::Disconnected,
            BackendStartReason::Initial | BackendStartReason::ManualRetry => BackendPhase::Failed,
        };
        self.backend_operation = BackendOperation::Connecting {
            attempt_id,
            retry_no: 0,
            recovery,
            terminal_phase,
        };
        self.backend_status.phase = if recovery {
            BackendPhase::Restarting
        } else {
            BackendPhase::Starting
        };
        self.backend_status.retry_no = None;
        self.backend_status.last_error = None;
        self.preview_control.backend_changed();
        self.reconcile_preview_without_window(cx);

        match reason {
            BackendStartReason::Initial => self.log_backend_localized(
                LogLevel::Info,
                "正在启动 Python 后端 sidecar",
                "Starting the Python sidecar",
            ),
            BackendStartReason::ManualRetry => self.log_backend_localized(
                LogLevel::Info,
                "正在手动重试启动 Python 后端",
                "Manually retrying the Python backend",
            ),
            BackendStartReason::Reconnect => self.log_backend_localized(
                LogLevel::Info,
                "正在恢复 Python 后端连接",
                "Recovering the Python backend connection",
            ),
        }
        cx.notify();

        let rpc = self.home.rpc.clone();
        cx.spawn(async move |this, cx| {
            let mut retry_no = 0;
            loop {
                let attempt_result = {
                    let rpc = rpc.clone();
                    cx.background_executor()
                        .spawn(async move { rpc.start_or_connect() })
                        .await
                };

                match attempt_result {
                    Ok(attach) => {
                        let _ = this.update(cx, |view, cx| {
                            if !view.backend_attempt_is_current(attempt_id) {
                                return;
                            }

                            if let BackendAttach::New(client) = attach {
                                view.install_backend_client(client);
                            }
                            view.backend_operation = BackendOperation::Idle;
                            view.backend_status.phase = BackendPhase::Ready;
                            view.backend_status.retry_no = None;
                            view.backend_status.last_error = None;
                            view.backend_epoch = view.backend_epoch.wrapping_add(1);
                            view.preview_control.backend_changed();
                            if recovery {
                                view.home.reset_after_sidecar_restart();
                            }
                            view.log_backend_localized(
                                LogLevel::Info,
                                "Python 后端已就绪，开始加载主控台数据",
                                "Python backend is ready; loading console data",
                            );
                            view.start_backend_hydration(cx);
                            view.reconcile_preview_without_window(cx);
                            cx.notify();
                        });
                        break;
                    }
                    Err(error) => {
                        let should_retry = this
                            .update(cx, |view, cx| {
                                if !view.backend_attempt_is_current(attempt_id) {
                                    return false;
                                }

                                view.backend_status.last_error = Some(error.clone());
                                if retry_no < MAX_AUTO_RETRIES {
                                    let next_retry = retry_no + 1;
                                    let delay = retry_delay(next_retry).as_secs();
                                    view.backend_operation = BackendOperation::WaitingRetry {
                                        attempt_id,
                                        retry_no: next_retry,
                                        recovery,
                                        terminal_phase,
                                    };
                                    view.backend_status.phase = BackendPhase::RetryWaiting;
                                    view.backend_status.retry_no = Some(next_retry);
                                    view.log_backend(
                                        LogLevel::Warn,
                                        format!(
                                            "Python 后端第 {} 次启动失败：{}；将在 {} 秒后自动重试（{}/{}）",
                                            retry_no + 1,
                                            error,
                                            delay,
                                            next_retry,
                                            MAX_AUTO_RETRIES,
                                        ),
                                    );
                                    cx.notify();
                                    true
                                } else {
                                    view.backend_operation = BackendOperation::Idle;
                                    view.backend_status.phase = terminal_phase;
                                    view.backend_status.retry_no = None;
                                    view.log_backend(
                                        LogLevel::Error,
                                        format!(
                                            "Python 后端自动重试已耗尽，请手动重试：{}",
                                            error
                                        ),
                                    );
                                    cx.notify();
                                    false
                                }
                            })
                            .unwrap_or(false);

                        if !should_retry {
                            break;
                        }

                        let next_retry = retry_no + 1;
                        cx.background_executor()
                            .timer(retry_delay(next_retry))
                            .await;
                        retry_no = next_retry;

                        let should_start_next = this
                            .update(cx, |view, cx| {
                                if !view.backend_attempt_is_current(attempt_id) {
                                    return false;
                                }
                                view.backend_operation = BackendOperation::Connecting {
                                    attempt_id,
                                    retry_no,
                                    recovery,
                                    terminal_phase,
                                };
                                view.backend_status.phase = if recovery {
                                    BackendPhase::Restarting
                                } else {
                                    BackendPhase::Starting
                                };
                                view.backend_status.retry_no = Some(retry_no);
                                view.log_backend(
                                    LogLevel::Info,
                                    format!(
                                        "开始第 {}/{} 次自动重试启动 Python 后端",
                                        retry_no, MAX_AUTO_RETRIES
                                    ),
                                );
                                cx.notify();
                                true
                            })
                            .unwrap_or(false);
                        if !should_start_next {
                            break;
                        }
                    }
                }
            }
        })
        .detach();
    }
}
