use std::time::Duration;

use gpui::{Context, Window};

use super::AhabApp;
use crate::{i18n, shell::ToastKind};

/// Outer bound for the sidecar's graceful close.  The sidecar owns its own
/// Runner stop/device cleanup deadlines; GPUI only prevents an unbounded quit.
const SIDECAR_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

impl AhabApp {
    /// Start a non-blocking graceful exit.
    ///
    /// The window close is vetoed until the sidecar has been asked to close and
    /// has either exited or exhausted the shutdown budget.  This keeps the UI
    /// responsive while `application.close()` stops a Runner, releases the
    /// device and flushes configuration.
    pub(crate) fn request_app_exit(&mut self, cx: &mut Context<Self>) {
        if self.shutdown_started {
            return;
        }
        self.shutdown_started = true;
        self.exit_requested = true;
        let message = i18n::paired(
            "正在关闭 Python 后端...",
            "Shutting down the Python backend...",
        )
        .get(self.state.settings.language)
        .to_owned();
        self.show_toast(ToastKind::Info, message, cx);
        cx.notify();

        let rpc = self.home.rpc.clone();
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .spawn(async move { rpc.shutdown(SIDECAR_SHUTDOWN_TIMEOUT) })
                .await
                .ok();
            let _ = this.update(cx, |view, cx| {
                view.allow_close = true;
                cx.quit();
            });
        })
        .detach();
    }

    /// Fallback for quit paths that bypass [`Self::request_app_exit`], such as
    /// platform-initiated termination.  This runs synchronously from the GPUI
    /// quit observer and therefore needs its own bounded wait.
    pub(crate) fn shutdown_backend(&mut self) {
        if self.shutdown_started || !self.home.rpc.is_sidecar() {
            return;
        }
        self.shutdown_started = true;
        let _ = self.home.rpc.shutdown(SIDECAR_SHUTDOWN_TIMEOUT);
    }

    pub(crate) fn register_app_quit(&mut self, cx: &mut Context<Self>) {
        if self.app_quit_subscription.is_some() {
            return;
        }
        self.app_quit_subscription = Some(cx.on_app_quit(|view, _cx| {
            view.shutdown_backend();
            async {}
        }));
    }

    pub(crate) fn register_window_close_guard(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let handle = cx.entity().downgrade();
        window.on_window_should_close(cx, move |_window, cx| {
            let Some(view) = handle.upgrade() else {
                return true;
            };
            let mut allow = false;
            view.update(cx, |view, cx| {
                if view.allow_close {
                    allow = true;
                } else {
                    view.request_app_exit(cx);
                }
            });
            allow
        });
    }
}
