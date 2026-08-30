use gpui::{Context, Window};
use serde::Deserialize;
use serde_json::Value;

use super::{AhabApp, BackendPhase, HomeInvalidation, Page};
use crate::{
    ipc::{RpcError, contract::method},
    model::{ConnectionStatus, PreviewStatus},
    shell,
};

#[derive(Default)]
pub(crate) struct PreviewControlState {
    desired_enabled: bool,
    acknowledged_enabled: Option<bool>,
    in_flight: Option<bool>,
    failed_enabled: Option<bool>,
    unsupported: bool,
}

enum PreviewControlOutcome {
    Applied { enabled: bool, running: bool },
    Unsupported,
    Failed(String),
    Ignored,
}

#[derive(Deserialize)]
struct PreviewControlPayload {
    enabled: bool,
    running: bool,
}

impl PreviewControlState {
    pub(crate) fn backend_changed(&mut self) {
        self.acknowledged_enabled = None;
        self.in_flight = None;
        self.failed_enabled = None;
        self.unsupported = false;
    }

    fn set_desired(&mut self, enabled: bool) -> bool {
        if self.desired_enabled == enabled {
            return false;
        }
        self.desired_enabled = enabled;
        self.failed_enabled = None;
        true
    }

    fn desired_enabled(&self) -> bool {
        self.desired_enabled
    }

    fn next_request(&mut self, backend_ready: bool) -> Option<bool> {
        if !backend_ready
            || self.unsupported
            || self.in_flight.is_some()
            || self.acknowledged_enabled == Some(self.desired_enabled)
            || self.failed_enabled == Some(self.desired_enabled)
        {
            return None;
        }
        self.in_flight = Some(self.desired_enabled);
        Some(self.desired_enabled)
    }

    fn complete(&mut self, result: Result<Option<Value>, RpcError>) -> PreviewControlOutcome {
        let Some(requested) = self.in_flight.take() else {
            return PreviewControlOutcome::Ignored;
        };
        match result {
            Ok(Some(value)) => match serde_json::from_value::<PreviewControlPayload>(value) {
                Ok(payload) => {
                    self.acknowledged_enabled = Some(payload.enabled);
                    self.failed_enabled = None;
                    PreviewControlOutcome::Applied {
                        enabled: payload.enabled,
                        running: payload.running,
                    }
                }
                Err(error) => {
                    self.failed_enabled = Some(requested);
                    PreviewControlOutcome::Failed(format!("预览控制响应无效：{error}"))
                }
            },
            Ok(None) => {
                self.failed_enabled = Some(requested);
                PreviewControlOutcome::Failed("预览控制响应为空".to_owned())
            }
            Err(error) if error.code == -32601 => {
                self.unsupported = true;
                PreviewControlOutcome::Unsupported
            }
            Err(error) => {
                self.failed_enabled = Some(requested);
                PreviewControlOutcome::Failed(error.message)
            }
        }
    }
}

impl AhabApp {
    pub(crate) fn attach_window(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.window_minimized = shell::is_window_minimized(window);
        self.window_subscriptions
            .push(cx.observe_window_activation(window, |view, window, cx| {
                view.update_window_state(window, cx)
            }));
        self.window_subscriptions
            .push(cx.observe_window_bounds(window, |view, window, cx| {
                view.update_window_state(window, cx)
            }));
        self.reconcile_preview(Some(window), cx);
    }

    fn update_window_state(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let minimized = shell::is_window_minimized(window);
        if minimized == self.window_minimized {
            return;
        }
        self.window_minimized = minimized;
        self.reconcile_preview(Some(window), cx);
        cx.notify();
    }

    pub(crate) fn reconcile_preview_without_window(&mut self, cx: &mut Context<Self>) {
        self.reconcile_preview(None, cx);
    }

    pub(crate) fn reconcile_preview(
        &mut self,
        window: Option<&mut Window>,
        cx: &mut Context<Self>,
    ) {
        let desired = preview_should_run(
            self.backend_status.phase,
            self.home.device_status,
            self.current_page,
            self.home.right_panel_collapsed,
            self.window_minimized,
            self.home.rpc.is_sidecar(),
        );
        let desired_changed = self.preview_control.set_desired(desired);
        let local_changed = if desired {
            desired_changed && self.home.start_preview()
        } else {
            self.home.suspend_preview()
        };

        if !desired
            && (desired_changed || local_changed)
            && let Some(window) = window
            && let Some(views) = self.home_views.clone()
        {
            views.clear_preview_render_resources(window, cx);
        }
        if local_changed {
            self.notify_home_views(
                HomeInvalidation {
                    preview: true,
                    ..HomeInvalidation::default()
                },
                cx,
            );
        }
        self.submit_preview_request();
    }

    pub(crate) fn preview_accepts_frames(&self) -> bool {
        self.preview_control.desired_enabled()
    }

    fn submit_preview_request(&mut self) {
        let Some(enabled) = self
            .preview_control
            .next_request(self.backend_status.is_ready() && self.home.rpc.is_sidecar())
        else {
            return;
        };
        self.home.rpc.submit(
            method::PREVIEW_SET_ENABLED,
            Some(serde_json::json!({"enabled": enabled})),
        );
    }

    pub(crate) fn apply_preview_control_completion(
        &mut self,
        result: Result<Option<Value>, RpcError>,
    ) -> HomeInvalidation {
        let outcome = self.preview_control.complete(result);
        let mut invalidation = HomeInvalidation::default();
        match outcome {
            PreviewControlOutcome::Applied { enabled, running } => {
                if self.preview_control.desired_enabled() == enabled {
                    if enabled && running {
                        self.home.preview_status = PreviewStatus::Running;
                        self.home.preview_error = None;
                    } else if enabled {
                        self.home.start_preview();
                    } else {
                        self.home.suspend_preview();
                    }
                    invalidation.preview = true;
                }
            }
            PreviewControlOutcome::Unsupported => {
                let message = match self.state.settings.language {
                    crate::model::Language::ZhCn => {
                        "当前 Python 后端不支持实时预览按需启停，已启用本地降载兼容模式"
                    }
                    crate::model::Language::EnUs => {
                        "The Python backend does not support preview gating; local frame rendering is still gated"
                    }
                };
                self.home
                    .append_local_log(crate::model::LogLevel::Warn, message);
                invalidation.logs = true;
            }
            PreviewControlOutcome::Failed(error) => {
                if self.preview_control.desired_enabled() {
                    self.home.preview_status = PreviewStatus::Error;
                    self.home.preview_error = Some(error.clone());
                    invalidation.preview = true;
                }
                self.home
                    .append_local_log(crate::model::LogLevel::Error, error);
                invalidation.logs = true;
            }
            PreviewControlOutcome::Ignored => {}
        }
        self.submit_preview_request();
        invalidation
    }
}

pub(crate) fn preview_should_run(
    backend_phase: BackendPhase,
    device_status: ConnectionStatus,
    page: Page,
    right_panel_collapsed: bool,
    window_minimized: bool,
    sidecar: bool,
) -> bool {
    sidecar
        && backend_phase == BackendPhase::Ready
        && device_status == ConnectionStatus::Connected
        && page == Page::Home
        && !right_panel_collapsed
        && !window_minimized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visible_background_home_keeps_preview_running() {
        assert!(preview_should_run(
            BackendPhase::Ready,
            ConnectionStatus::Connected,
            Page::Home,
            false,
            false,
            true,
        ));
    }

    #[test]
    fn only_minimize_or_visibility_state_disables_preview() {
        let cases = [
            (Page::Teams, false, false),
            (Page::Home, true, false),
            (Page::Home, false, true),
        ];
        for (page, collapsed, minimized) in cases {
            assert!(!preview_should_run(
                BackendPhase::Ready,
                ConnectionStatus::Connected,
                page,
                collapsed,
                minimized,
                true,
            ));
        }
    }
}
