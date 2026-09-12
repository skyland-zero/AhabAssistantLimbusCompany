use std::time::Duration;

use gpui::Context;

use super::{AhabApp, HomeInvalidation};
use crate::{
    i18n::paired,
    ipc::{RpcGateway, contract::method},
    model::{ConnectionStatus, ExecutionState, ExecutionStatusPayload, LogLevel},
    shell::ToastKind,
    state::HomeSelect,
};

/// Time after which a local optimistic stop asks the sidecar for an
/// authoritative state snapshot.  This is a reconciliation deadline, not a
/// backend restart timeout: a responsive sidecar remains authoritative while
/// it reports `stopping` or `restoring`.
const STOP_RECONCILE_TIMEOUT: Duration = Duration::from_secs(5);
/// Interval between follow-up reconciliations while the backend still reports
/// `stopping`/`restoring`.
const STOP_RECONCILE_INTERVAL: Duration = Duration::from_secs(3);
/// Bound the follow-up probes so a permanently stuck stop is surfaced instead
/// of polling forever.
const STOP_RECONCILE_ATTEMPTS: u32 = 3;
const THEME_PERSIST_DEBOUNCE: Duration = Duration::from_millis(200);

impl AhabApp {
    /// Validate the current selection and start a run, connecting the last
    /// used device when necessary.
    ///
    /// The execution toolbar's mouse and keyboard paths both call this so the
    /// two input methods report identical validation feedback.
    pub fn start_execution(&mut self, cx: &mut Context<Self>) {
        let language = self.state.settings.language;
        if self.home.selected_task_count() == 0 {
            self.show_toast(
                ToastKind::Warning,
                paired(
                    "请至少勾选一个要执行的任务",
                    "Select at least one task to run",
                )
                .get(language),
                cx,
            );
        } else if self.home.device_status != ConnectionStatus::Connected {
            if self.home.devices.is_empty() {
                self.home.open_select = Some(HomeSelect::Device);
                self.home.device_error = Some(
                    paired(
                        "未检测到可用的游戏窗口或模拟器，请先启动游戏并连接",
                        "No game window or emulator detected, please launch and connect first",
                    )
                    .get(language)
                    .to_owned(),
                );
                self.show_toast(
                    ToastKind::Warning,
                    paired(
                        "未连接设备，请先选择游戏窗口或模拟器",
                        "Device not connected, please select game window first",
                    )
                    .get(language),
                    cx,
                );
            } else {
                let last_id_opt = self
                    .state
                    .settings
                    .lastDeviceId
                    .clone()
                    .filter(|id| self.home.devices.iter().any(|d| &d.id == id));
                if let Some(last_id) = last_id_opt {
                    self.show_toast(
                        ToastKind::Info,
                        paired(
                            "正在自动连接上次使用的设备...",
                            "Auto-connecting to last used device...",
                        )
                        .get(language),
                        cx,
                    );
                    self.select_device(last_id, cx);
                } else {
                    self.home.open_select = Some(HomeSelect::Device);
                    self.show_toast(
                        ToastKind::Warning,
                        paired(
                            "请先选择并连接游戏窗口或模拟器",
                            "Please select and connect a device first",
                        )
                        .get(language),
                        cx,
                    );
                }
            }
        } else {
            self.home.start();
        }
    }

    pub fn stop_execution(&mut self, cx: &mut Context<Self>) {
        if !self.home.stop() {
            return;
        }

        self.stop_timeout_generation = self.stop_timeout_generation.wrapping_add(1);
        let generation = self.stop_timeout_generation;
        let rpc = self.home.rpc.clone();
        cx.spawn(async move |this, cx| {
            let mut delay = STOP_RECONCILE_TIMEOUT;
            for attempt in 0..STOP_RECONCILE_ATTEMPTS {
                cx.background_executor().timer(delay).await;
                delay = STOP_RECONCILE_INTERVAL;
                let should_reconcile = this
                    .update(cx, |view, _cx| {
                        stop_timeout_is_current(
                            generation,
                            view.stop_timeout_generation,
                            view.home.execution.state,
                        ) && view.backend_operation.is_idle()
                    })
                    .unwrap_or(false);
                if !should_reconcile {
                    return;
                }

                let request = rpc.request_async(method::EXECUTION_GET_STATE, None);
                let response = cx
                    .background_executor()
                    .spawn(async move { request.recv().await.ok() })
                    .await;
                let spent = attempt + 1 >= STOP_RECONCILE_ATTEMPTS;
                let keep_probing = this
                    .update(cx, |view, cx| {
                        if !stop_timeout_is_current(
                            generation,
                            view.stop_timeout_generation,
                            view.home.execution.state,
                        ) || !view.backend_operation.is_idle()
                        {
                            return false;
                        }

                        let still_pending = reconcile_stop_state(view, response, cx);
                        if still_pending && spent {
                            view.log_backend_localized(
                                LogLevel::Warn,
                                "停止状态对账已达重试上限，后端仍报告停止中；不会在此时重启后端",
                                "Stop-state reconciliation exhausted its retries while the backend still reports stopping; the backend will not be restarted here",
                            );
                        }
                        cx.notify();
                        still_pending && !spent
                    })
                    .unwrap_or(false);
                if !keep_probing {
                    return;
                }
            }
        })
        .detach();
    }

    pub fn start_stats_ticker(&mut self, cx: &mut Context<Self>) {
        self.stats_tick_generation = self.stats_tick_generation.wrapping_add(1);
        let generation = self.stats_tick_generation;
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                let should_continue = this
                    .update(cx, |view, cx| {
                        if view.stats_tick_generation != generation {
                            return false;
                        }
                        let current = &view.home.stats.currentRun;
                        let should_tick = current.runId.is_some()
                            && current.startedAt.is_some()
                            && (current.state == ExecutionState::Running
                                || view.home.execution.state == ExecutionState::Running);
                        if should_tick {
                            view.notify_home_views(
                                HomeInvalidation {
                                    stats: true,
                                    ..HomeInvalidation::default()
                                },
                                cx,
                            );
                        }
                        true
                    })
                    .unwrap_or(false);
                if !should_continue {
                    break;
                }
            }
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
    expected_generation == current_generation
        && matches!(state, ExecutionState::Stopping | ExecutionState::Restoring)
}

fn reconcile_stop_state(
    view: &mut AhabApp,
    response: Option<crate::ipc::RpcResponse>,
    cx: &mut Context<AhabApp>,
) -> bool {
    let Some(response) = response else {
        if !view.home.rpc.is_connected() {
            let _ = view.maybe_recover_backend(cx);
        } else {
            view.log_backend_localized(
                LogLevel::Warn,
                "停止状态对账未收到响应，后端仍保持连接",
                "Stop-state reconciliation received no response; keeping the backend connected",
            );
        }
        // A disconnected backend is handled by recovery; a connected one may
        // simply be slow, so allow the bounded follow-up probe.
        return view.home.rpc.is_connected();
    };

    match RpcGateway::decode_response(method::EXECUTION_GET_STATE, response) {
        Ok(Some(value)) => match serde_json::from_value::<ExecutionStatusPayload>(value) {
            Ok(status) => {
                let state = status.state;
                if !view.home.apply_execution_status(status) {
                    view.log_backend_localized(
                        LogLevel::Error,
                        "停止状态对账收到过期执行状态，已忽略",
                        "Stop-state reconciliation received a stale execution snapshot; ignored",
                    );
                    return false;
                }
                match state {
                    ExecutionState::Stopping | ExecutionState::Restoring => {
                        // A healthy sidecar may need to finish runner cleanup
                        // and device restoration.  Never restart it here.
                        view.log_backend_localized(
                            LogLevel::Info,
                            "后端仍在停止或恢复设备，继续等待权威状态",
                            "Backend is still stopping or restoring the device; continuing to wait",
                        );
                        true
                    }
                    ExecutionState::Idle => {
                        view.log_backend_localized(
                            LogLevel::Info,
                            "任务停止状态已完成对账",
                            "Task stop state reconciled successfully",
                        );
                        false
                    }
                    _ => {
                        view.log_backend_localized(
                            LogLevel::Error,
                            "停止状态对账收到矛盾执行状态，已按后端快照修正",
                            "Stop-state reconciliation received a contradictory state; corrected to the backend snapshot",
                        );
                        false
                    }
                }
            }
            Err(error) => {
                view.log_backend_localized(
                    LogLevel::Error,
                    "停止状态对账返回了无效执行状态",
                    "Stop-state reconciliation returned an invalid execution snapshot",
                );
                view.home.append_local_log(
                    LogLevel::Debug,
                    format!("execution.getState decode error: {error}"),
                );
                true
            }
        },
        Ok(None) => {
            view.log_backend_localized(
                LogLevel::Error,
                "停止状态对账返回空响应",
                "Stop-state reconciliation returned an empty response",
            );
            true
        }
        Err(error) => {
            if !view.home.rpc.is_connected() {
                let _ = view.maybe_recover_backend(cx);
            } else {
                view.log_backend_localized(
                    LogLevel::Error,
                    "停止状态对账请求失败，后端仍保持连接",
                    "Stop-state reconciliation failed while the backend remains connected",
                );
                view.home
                    .append_local_log(LogLevel::Debug, error.message.clone());
            }
            view.home.rpc.is_connected()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_or_completed_stop_timeouts_do_nothing() {
        assert!(stop_timeout_is_current(4, 4, ExecutionState::Stopping));
        assert!(stop_timeout_is_current(4, 4, ExecutionState::Restoring));
        assert!(!stop_timeout_is_current(3, 4, ExecutionState::Stopping));
        assert!(!stop_timeout_is_current(4, 4, ExecutionState::Idle));
        assert!(!stop_timeout_is_current(4, 4, ExecutionState::Running));
    }
}
