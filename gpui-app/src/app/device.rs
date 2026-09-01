use gpui::{Context, Window};

use super::{AhabApp, HomeInvalidation};
use crate::ipc::{
    RpcCompletion, RpcGateway,
    contract::{event, method},
};

impl AhabApp {
    /// Wait for transport activity and drain every queued sidecar update in a
    /// single GPUI turn. Visual/mock runs apply their events synchronously.
    pub fn start_event_pump(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !self.home.rpc.is_sidecar() {
            return;
        }
        let activity_rpc = self.home.rpc.clone();
        cx.spawn_in(window, async move |this, cx| {
            loop {
                if !activity_rpc.wait_for_activity().await {
                    break;
                }
                if this
                    .update_in(cx, |view, window, cx| {
                        let invalidation = view.poll_backend_events();
                        let recovering = view.maybe_recover_backend(cx);
                        view.reconcile_preview(Some(window), cx);
                        view.notify_home_views(invalidation, cx);
                        if invalidation.root || recovering {
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

    pub(crate) fn poll_backend_events(&mut self) -> HomeInvalidation {
        let events = self.home.rpc.take_events();
        let mut invalidation = HomeInvalidation::default();
        if !events.is_empty() {
            let mut home_events = Vec::new();
            let mut toolbox_events = Vec::new();
            let mut resource_events = Vec::new();
            for event_value in events {
                match event_value.event.as_str() {
                    event::TOOL_STATUS => {
                        invalidation.root = true;
                        toolbox_events.push(event_value);
                    }
                    event::RESOURCE_SYNC_PROGRESS => {
                        invalidation.root = true;
                        resource_events.push(event_value);
                    }
                    event::EXECUTION_STATUS => {
                        invalidation.root = true;
                        invalidation.stats = true;
                        home_events.push(event_value);
                    }
                    event::EXECUTION_MIRROR_PROGRESS => {
                        // Mirror progress is rendered inside the task card. It
                        // is not a frame-rate event, so a root refresh here is
                        // acceptable and keeps all existing task controls intact.
                        invalidation.root = true;
                        home_events.push(event_value);
                    }
                    event::EXECUTION_STATS => {
                        invalidation.stats = true;
                        home_events.push(event_value);
                    }
                    event::SCREENSHOT_FRAME => {
                        if self.preview_accepts_frames() {
                            invalidation.preview = true;
                            home_events.push(event_value);
                        }
                    }
                    event::PREVIEW_STATUS => {
                        invalidation.preview = true;
                        home_events.push(event_value);
                    }
                    event::DEVICE_STATUS => {
                        invalidation.root = true;
                        invalidation.preview = true;
                        home_events.push(event_value);
                    }
                    event::LOG_ENTRY => {
                        invalidation.logs = true;
                        home_events.push(event_value);
                    }
                    event::APP_NOTICE => {
                        invalidation.root = true;
                        invalidation.logs = true;
                        home_events.push(event_value);
                    }
                    _ => {}
                }
            }
            self.home.apply_events(home_events);
            self.toolbox.apply_events(toolbox_events);
            self.resources.apply_events(resource_events);
        }

        for completion in self.home.rpc.take_completions() {
            invalidation.merge(self.apply_backend_completion(completion));
        }
        invalidation
    }

    fn apply_backend_completion(&mut self, completion: RpcCompletion) -> HomeInvalidation {
        let method_name = completion.method.as_str();
        let result = RpcGateway::decode_response(method_name, completion.response);
        let mut invalidation = HomeInvalidation::default();
        match method_name {
            method::EXECUTION_START
            | method::EXECUTION_STOP
            | method::EXECUTION_PAUSE
            | method::EXECUTION_RESUME
            | method::TASKS_SET_CONFIG => {
                self.home.apply_command_result(method_name, result);
                invalidation.root = true;
                invalidation.stats = true;
            }
            method::APP_CHECK_UPDATE
            | method::HOTKEY_GET
            | method::HOTKEY_SET
            | method::SYSTEM_SETTINGS_GET
            | method::SYSTEM_SETTINGS_SET
            | method::NOTIFICATION_TEST => {
                self.settings_page.apply_rpc_result(method_name, result);
                invalidation.root = true;
            }
            method::RESOURCE_STATUS
            | method::RESOURCE_CHECK_UPDATE
            | method::RESOURCE_SYNC_START => {
                self.resources.apply_rpc_result(method_name, result);
                invalidation.root = true;
            }
            method::TOOL_START | method::TOOL_STOP | method::TOOL_SCREENSHOT => {
                self.toolbox
                    .apply_rpc_result(method_name, completion.params, result);
                invalidation.root = true;
            }
            method::THEME_PACK_LIST
            | method::THEME_PACK_UPDATE_ALL
            | method::THEME_PACK_RESET_WEIGHTS => {
                self.theme_packs.apply_rpc_result(method_name, result);
                invalidation.root = true;
            }
            method::PREVIEW_SET_ENABLED => {
                invalidation.merge(self.apply_preview_control_completion(result));
            }
            _ => {}
        }
        invalidation
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
                    let auto_device_id = preferred_device_id(
                        &view.home.devices,
                        view.state.settings.lastDeviceId.as_deref(),
                    );
                    if let Some(device_id) = auto_device_id {
                        view.select_device(device_id, cx);
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

fn preferred_device_id(
    devices: &[crate::model::DeviceInfo],
    last_device_id: Option<&str>,
) -> Option<String> {
    if let Some(last_device_id) = last_device_id
        && devices
            .iter()
            .any(|device| device.id.as_str() == last_device_id)
    {
        return Some(last_device_id.to_owned());
    }

    (devices.len() == 1).then(|| devices[0].id.clone())
}

#[cfg(test)]
mod tests {
    use super::preferred_device_id;
    use crate::model::DeviceInfo;

    fn devices(ids: &[&str]) -> Vec<DeviceInfo> {
        ids.iter()
            .map(|id| DeviceInfo {
                id: (*id).to_owned(),
                name: (*id).to_owned(),
                detail: None,
            })
            .collect()
    }

    #[test]
    fn keeps_a_valid_last_device_preference() {
        let available = devices(&["mumu:2", "mumu:3"]);
        assert_eq!(
            preferred_device_id(&available, Some("mumu:3")),
            Some("mumu:3".to_owned())
        );
    }

    #[test]
    fn selects_the_only_device_when_the_last_device_is_missing() {
        let available = devices(&["mumu:2"]);
        assert_eq!(
            preferred_device_id(&available, Some("mumu:0")),
            Some("mumu:2".to_owned())
        );
    }

    #[test]
    fn does_not_guess_when_multiple_devices_are_available() {
        let available = devices(&["mumu:2", "mumu:3"]);
        assert_eq!(preferred_device_id(&available, Some("mumu:0")), None);
    }
}
