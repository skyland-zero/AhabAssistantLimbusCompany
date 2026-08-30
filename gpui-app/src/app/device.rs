use gpui::Context;

use super::AhabApp;
use crate::ipc::{
    RpcCompletion, RpcGateway,
    contract::{event, method},
};

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
                        let changed = view.poll_backend_events();
                        let recovering = view.maybe_recover_backend(cx);
                        if changed || recovering {
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
        let mut changed = false;
        if !events.is_empty() {
            let mut home_events = Vec::new();
            let mut toolbox_events = Vec::new();
            let mut resource_events = Vec::new();
            for event_value in events {
                match event_value.event.as_str() {
                    event::TOOL_STATUS => toolbox_events.push(event_value),
                    event::RESOURCE_SYNC_PROGRESS => resource_events.push(event_value),
                    event::EXECUTION_STATUS
                    | event::EXECUTION_MIRROR_PROGRESS
                    | event::EXECUTION_STATS
                    | event::SCREENSHOT_FRAME
                    | event::PREVIEW_STATUS
                    | event::DEVICE_STATUS
                    | event::LOG_ENTRY
                    | event::APP_NOTICE => home_events.push(event_value),
                    _ => {}
                }
            }
            self.home.apply_events(home_events);
            self.toolbox.apply_events(toolbox_events);
            self.resources.apply_events(resource_events);
            changed = true;
        }

        for completion in self.home.rpc.take_completions() {
            self.apply_backend_completion(completion);
            changed = true;
        }
        if self.theme_packs.flush_debounced() {
            changed = true;
        }
        changed
    }

    fn apply_backend_completion(&mut self, completion: RpcCompletion) {
        let method_name = completion.method.as_str();
        let result = RpcGateway::decode_response(method_name, completion.response);
        match method_name {
            method::EXECUTION_START
            | method::EXECUTION_STOP
            | method::EXECUTION_PAUSE
            | method::EXECUTION_RESUME
            | method::TASKS_SET_CONFIG => self.home.apply_command_result(method_name, result),
            method::APP_CHECK_UPDATE
            | method::HOTKEY_GET
            | method::HOTKEY_SET
            | method::SYSTEM_SETTINGS_GET
            | method::SYSTEM_SETTINGS_SET
            | method::NOTIFICATION_TEST => self.settings_page.apply_rpc_result(method_name, result),
            method::RESOURCE_STATUS
            | method::RESOURCE_CHECK_UPDATE
            | method::RESOURCE_SYNC_START => self.resources.apply_rpc_result(method_name, result),
            method::TOOL_START | method::TOOL_STOP | method::TOOL_SCREENSHOT => self
                .toolbox
                .apply_rpc_result(method_name, completion.params, result),
            method::THEME_PACK_LIST
            | method::THEME_PACK_UPDATE_ALL
            | method::THEME_PACK_RESET_WEIGHTS => {
                self.theme_packs.apply_rpc_result(method_name, result)
            }
            _ => {}
        }
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
