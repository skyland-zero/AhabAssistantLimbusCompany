use gpui::Context;

use super::AhabApp;

impl AhabApp {
    /// Start a lightweight GPUI-side event pump for unsolicited sidecar
    /// events. Visual/mock runs do not need a timer.
    pub fn start_event_pump(&mut self, cx: &mut Context<Self>) {
        if !self.home.rpc.is_sidecar() {
            return;
        }
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(50))
                    .await;
                if this
                    .update(cx, |view, cx| {
                        if view.poll_backend_events() {
                            cx.notify();
                        }
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    pub(crate) fn poll_backend_events(&mut self) -> bool {
        let events = self.home.rpc.take_events();
        if events.is_empty() {
            return false;
        }
        self.home.apply_events(events.clone());
        self.toolbox.apply_events(events.clone());
        self.resources.apply_events(events);
        true
    }

    pub fn select_device(&mut self, id: String, cx: &mut Context<Self>) {
        self.state.settings.lastDeviceId = Some(id.clone());
        let _ = self.state.save();
        self.home.device_error = None;
        self.home.close_select();
        if !self.home.rpc.is_sidecar() {
            self.home.select_device(id);
            cx.notify();
            return;
        }
        self.home.device_status = crate::model::ConnectionStatus::Connecting;
        cx.notify();
        self.spawn_device_request(
            crate::ipc::contract::method::DEVICE_CONNECT,
            Some(serde_json::json!({ "id": id })),
            cx,
        );
    }

    pub fn disconnect_device(&mut self, cx: &mut Context<Self>) {
        self.home.close_select();
        self.home.device_error = None;
        if !self.home.rpc.is_sidecar() {
            self.home.disconnect_device();
            cx.notify();
            return;
        }
        self.spawn_device_request(crate::ipc::contract::method::DEVICE_DISCONNECT, None, cx);
    }

    pub fn refresh_devices(&mut self, cx: &mut Context<Self>) {
        self.home.is_scanning_devices = true;
        self.home.device_error = None;
        cx.notify();

        let rpc = self.home.rpc.clone();
        cx.spawn(async move |this, cx| {
            // Keep scanning state active for at least 400ms so the user sees clear feedback
            cx.background_executor()
                .timer(std::time::Duration::from_millis(400))
                .await;
            let request = rpc.request_async(crate::ipc::contract::method::DEVICE_LIST, None);
            let result = cx
                .background_executor()
                .spawn(async move { request.recv().ok() })
                .await
                .map(|response| {
                    crate::ipc::RpcGateway::decode_response(
                        crate::ipc::contract::method::DEVICE_LIST,
                        response,
                    )
                })
                .unwrap_or_else(|| Err(crate::ipc::RpcError::new(-32000, "后端连接已断开")));
            let _ = this.update(cx, |view, cx| {
                view.home.apply_device_list_result(result);
                view.home.is_scanning_devices = false;

                let count = view.home.devices.len();
                let language = view.state.settings.language;
                if count > 0 {
                    view.show_toast(
                        crate::shell::ToastKind::Success,
                        match language {
                            crate::model::Language::ZhCn => {
                                format!("已刷新设备列表，发现 {} 个可用设备", count)
                            }
                            crate::model::Language::EnUs => {
                                format!("Device list refreshed, found {} available devices", count)
                            }
                        },
                        cx,
                    );
                } else {
                    view.show_toast(
                        crate::shell::ToastKind::Warning,
                        match language {
                            crate::model::Language::ZhCn => {
                                "已刷新，未检测到游戏窗口或模拟器".to_string()
                            }
                            crate::model::Language::EnUs => {
                                "Refreshed, no game window or emulator detected".to_string()
                            }
                        },
                        cx,
                    );
                }

                if view.home.device_status == crate::model::ConnectionStatus::Disconnected {
                    let last_id_opt = view
                        .state
                        .settings
                        .lastDeviceId
                        .clone()
                        .filter(|id| view.home.devices.iter().any(|d| &d.id == id));
                    if let Some(last_id) = last_id_opt {
                        view.select_device(last_id, cx);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn spawn_device_request(
        &mut self,
        method: &'static str,
        params: Option<serde_json::Value>,
        cx: &mut Context<Self>,
    ) {
        let rpc = self.home.rpc.clone();
        cx.spawn(async move |this, cx| {
            let request = rpc.request_async(method, params);
            let result = cx
                .background_executor()
                .spawn(async move { request.recv().ok() })
                .await
                .map(|response| crate::ipc::RpcGateway::decode_response(method, response))
                .unwrap_or_else(|| Err(crate::ipc::RpcError::new(-32000, "后端连接已断开")));
            let _ = this.update(cx, |view, cx| {
                view.home.apply_rpc_result(result);
                cx.notify();
            });
        })
        .detach();
    }
}
