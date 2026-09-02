use std::time::Duration;

use gpui::Context;

use super::AhabApp;
use crate::model::{ExecutionState, LogLevel};

const STOP_TIMEOUT: Duration = Duration::from_secs(15);
const THEME_PERSIST_DEBOUNCE: Duration = Duration::from_millis(200);

impl AhabApp {
    pub fn stop_execution(&mut self, cx: &mut Context<Self>) {
        if !self.home.stop() {
            return;
        }

        self.stop_timeout_generation = self.stop_timeout_generation.wrapping_add(1);
        let generation = self.stop_timeout_generation;
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(STOP_TIMEOUT).await;
            let _ = this.update(cx, |view, cx| {
                if !stop_timeout_is_current(
                    generation,
                    view.stop_timeout_generation,
                    view.home.execution.state,
                ) || !view.backend_operation.is_idle()
                {
                    return;
                }

                view.home.mark_stop_timeout_handled();
                view.log_backend_localized(
                    LogLevel::Warn,
                    "任务停止超时，保持 Python 后端和设备连接，等待任务退出",
                    "Task stop timed out; keeping the Python backend and device connected while the task exits",
                );
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn schedule_theme_pack_persist(&mut self, cx: &mut Context<Self>) {
        let Some(generation) = self.theme_packs.pending_persist_generation() else {
            return;
        };
        if self.theme_persist_timer_generation == Some(generation) {
            return;
        }
        self.theme_persist_timer_generation = Some(generation);

        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(THEME_PERSIST_DEBOUNCE).await;
            let _ = this.update(cx, |view, cx| {
                if view.theme_persist_timer_generation != Some(generation) {
                    return;
                }
                view.theme_persist_timer_generation = None;
                if view.theme_packs.flush_debounced(generation) {
                    cx.notify();
                }
            });
        })
        .detach();
    }
}

fn stop_timeout_is_current(
    expected_generation: u64,
    current_generation: u64,
    state: ExecutionState,
) -> bool {
    expected_generation == current_generation && state == ExecutionState::Stopping
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_or_completed_stop_timeouts_do_nothing() {
        assert!(stop_timeout_is_current(4, 4, ExecutionState::Stopping));
        assert!(!stop_timeout_is_current(3, 4, ExecutionState::Stopping));
        assert!(!stop_timeout_is_current(4, 4, ExecutionState::Idle));
        assert!(!stop_timeout_is_current(4, 4, ExecutionState::Running));
    }
}
