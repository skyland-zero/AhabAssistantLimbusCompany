use std::time::Duration;

use gpui::Context;

use super::{AhabApp, BackendOperation, BackendPhase, BackendStatus, Page, VisualState};
use crate::{
    app_inputs::{SettingsInputs, TeamInputs},
    ipc::{BackendAttach, BackendClient, RpcGateway, contract::method},
    model::{Language, LogLevel},
    state::{
        AppState, HomeState, ResourcesState, SettingsPageState, TeamsState, ThemePacksState,
        ToolboxState,
    },
};

const MAX_AUTO_RETRIES: u8 = 3;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BackendStartReason {
    Initial,
    ManualRetry,
    Reconnect,
}

fn runtime_client() -> BackendClient {
    let mode = std::env::var("AHAB_BACKEND")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let visual_run = std::env::var_os("AHAB_VISUAL_PAGE").is_some()
        || std::env::var_os("AHAB_VISUAL_STATE").is_some();
    let use_mock = mode == "mock" || (mode.is_empty() && visual_run);
    if use_mock {
        return BackendClient::mock();
    }
    BackendClient::unavailable("Python 后端正在等待 GPUI 首帧启动")
}

fn retry_delay(retry_no: u8) -> Duration {
    Duration::from_secs(match retry_no {
        1 => 1,
        2 => 2,
        _ => 4,
    })
}

fn localized(language: Language, zh: &'static str, en: &'static str) -> String {
    match language {
        Language::ZhCn => zh.to_owned(),
        Language::EnUs => en.to_owned(),
    }
}

fn apply_visual_overrides(settings: &mut crate::model::AppSettings) {
    if let Ok(theme) = std::env::var("AHAB_VISUAL_THEME") {
        settings.themeMode = match theme.to_ascii_lowercase().as_str() {
            "light" => crate::model::ThemeMode::Light,
            "dark" => crate::model::ThemeMode::Dark,
            "system" => crate::model::ThemeMode::System,
            _ => settings.themeMode,
        };
    }
    if let Ok(language) = std::env::var("AHAB_VISUAL_LANGUAGE") {
        settings.language = match language.as_str() {
            "en-US" | "en-us" | "en" => crate::model::Language::EnUs,
            "zh-CN" | "zh-cn" | "zh" => crate::model::Language::ZhCn,
            _ => settings.language,
        };
    }
    if let Ok(accent) = std::env::var("AHAB_VISUAL_ACCENT")
        && !accent.trim().is_empty()
    {
        settings.accentId = accent;
    }
}

impl AhabApp {
    /// Hydrate the first renderable state from the sidecar away from the
    /// render thread. This is called only after the backend handshake has
    /// completed, so constructors and the first frame never wait on network
    /// responses.
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
                .spawn(async move { tasks_request.recv().ok() });
            let devices_response = cx
                .background_executor()
                .spawn(async move { devices_request.recv().ok() });
            let execution_response = cx
                .background_executor()
                .spawn(async move { execution_request.recv().ok() });
            let stats_response = cx
                .background_executor()
                .spawn(async move { stats_request.recv().ok() });
            let teams_response = cx
                .background_executor()
                .spawn(async move { teams_request.recv().ok() });
            let sinners_response = cx
                .background_executor()
                .spawn(async move { sinners_request.recv().ok() });
            let themes_response = cx
                .background_executor()
                .spawn(async move { themes_request.recv().ok() });
            let resources_response = cx
                .background_executor()
                .spawn(async move { resources_request.recv().ok() });
            let hotkey_response = cx
                .background_executor()
                .spawn(async move { hotkey_request.recv().ok() });
            let system_response = cx
                .background_executor()
                .spawn(async move { system_request.recv().ok() });

            let tasks_response = tasks_response.await;
            let devices_response = devices_response.await;
            let execution_response = execution_response.await;
            let stats_response = stats_response.await;
            let teams_response = teams_response.await;
            let sinners_response = sinners_response.await;
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
                    view.home.execution = execution;
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

    fn log_backend(&mut self, level: LogLevel, message: impl Into<String>) {
        self.home.append_local_log(level, message);
    }

    pub(crate) fn log_backend_localized(
        &mut self,
        level: LogLevel,
        zh: &'static str,
        en: &'static str,
    ) {
        let message = localized(self.state.settings.language, zh, en);
        self.log_backend(level, message);
    }

    fn backend_attempt_is_current(&self, attempt_id: u64) -> bool {
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

    fn install_backend_client(&mut self, client: BackendClient) {
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

    pub(crate) fn maybe_recover_backend(&mut self, cx: &mut Context<Self>) -> bool {
        if !self.backend_operation.is_idle() || !self.home.rpc.is_sidecar() {
            return false;
        }

        if self.backend_status.is_ready() && !self.home.rpc.is_connected() {
            self.log_backend_localized(
                LogLevel::Warn,
                "Python 后端连接已断开，开始自动恢复",
                "Python backend connection lost; starting automatic recovery",
            );
            self.start_backend_connection(cx, BackendStartReason::Reconnect);
            return true;
        }

        false
    }

    pub fn new() -> Self {
        let mut state = AppState::load();
        apply_visual_overrides(&mut state.settings);
        let current_page = std::env::var("AHAB_VISUAL_PAGE")
            .ok()
            .and_then(|page| Page::parse(&page))
            .unwrap_or(Page::Home);
        let visual_state = std::env::var("AHAB_VISUAL_STATE")
            .ok()
            .and_then(|state| VisualState::parse(&state));
        let client = runtime_client();
        let backend_status = if client.is_sidecar() {
            BackendStatus::waiting_for_first_frame()
        } else {
            BackendStatus::mock()
        };
        let home = HomeState::with_client(
            client.shared(),
            state.settings.rightPanelWidth,
            state.settings.rightPanelCollapsed,
        );

        let mut app = Self {
            current_page,
            state,
            home,
            teams: TeamsState::with_client(client.shared()),
            theme_packs: ThemePacksState::with_client(client.shared()),
            toolbox: ToolboxState::with_client(client.shared()),
            resources: ResourcesState::with_client(client.shared()),
            settings_page: SettingsPageState::with_client(client),
            team_inputs: TeamInputs::default(),
            settings_inputs: SettingsInputs::default(),
            visual_state,
            settings_scroll: gpui::ScrollHandle::new(),
            settings_active_section: 0,
            help_scroll: gpui::ScrollHandle::new(),
            help_active_section: 0,
            toast: None,
            toast_generation: 0,
            home_views: None,
            titlebar_status_dot: None,
            backend_status,
            backend_operation: BackendOperation::Idle,
            backend_attempt_id: 0,
            backend_epoch: 0,
            stop_timeout_generation: 0,
            theme_persist_timer_generation: None,
            preview_control: Default::default(),
            window_minimized: false,
            window_subscriptions: Vec::new(),
        };

        if app.backend_status.phase == BackendPhase::WaitingForFirstFrame {
            app.log_backend_localized(
                LogLevel::Info,
                "GPUI 窗口已创建，等待首帧后启动 Python 后端",
                "GPUI window created; Python backend will start after the first frame",
            );
        }

        if let Some(last_id) = app.state.settings.lastDeviceId.as_ref()
            && app.home.devices.iter().any(|d| &d.id == last_id)
            && app.home.device_status == crate::model::ConnectionStatus::Disconnected
        {
            app.home.select_device(last_id.clone());
        }

        app
    }

    pub(crate) fn apply_visual_state(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.visual_state.take() else {
            return;
        };
        self.current_page = state.page();
        match state {
            VisualState::HomeExpanded => {
                self.home
                    .expanded_tasks
                    .insert(crate::model::FixedTaskId::DailyTask);
            }
            VisualState::HomeSelect => {
                self.home
                    .expanded_tasks
                    .insert(crate::model::FixedTaskId::GetReward);
                self.home.open_select = Some(crate::state::HomeSelect::RewardMode);
            }
            VisualState::HomeRunning => {
                self.home.start();
            }
            VisualState::HomePaused => {
                self.home.start();
                self.home.pause_or_resume();
            }
            VisualState::HomeAfterCompletion => {
                self.home.set_after_completion_open(true);
            }
            VisualState::TeamsEditor => {
                if let Some(team) = self.teams.teams.first().cloned() {
                    self.teams.open_edit(&team);
                    self.create_team_inputs(cx);
                }
            }
            VisualState::TeamsDelete => {
                if let Some(team) = self.teams.teams.first().cloned() {
                    self.teams.request_delete(team);
                }
            }
            VisualState::TeamsSelect => {
                if let Some(team) = self.teams.teams.first().cloned() {
                    self.teams.open_edit(&team);
                    self.create_team_inputs(cx);
                    self.teams.open_select = Some(crate::state::TeamSelect::Purpose);
                }
            }
            VisualState::SettingsHotkey => {
                self.settings_page.capturing = Some(crate::state::HotkeyTarget::StartStop);
            }
            VisualState::SettingsSelect => {
                self.settings_page.open_select = Some(crate::state::SettingsSelect::UpdateSource);
            }
            VisualState::SettingsLatest => {
                self.settings_page.check_update();
            }
            VisualState::ToolboxRunning => {
                self.toolbox.toggle(crate::model::ToolId::InfiniteBattle);
            }
            VisualState::ResourcesSyncing => {
                self.resources.sync_progress = Some(42);
                self.resources.sync_finish_scheduled = false;
            }
            VisualState::HelpScrolled => {
                self.help_scroll.scroll_to_top_of_item(6);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_retry_delays_use_exponential_backoff() {
        assert_eq!(retry_delay(1), Duration::from_secs(1));
        assert_eq!(retry_delay(2), Duration::from_secs(2));
        assert_eq!(retry_delay(3), Duration::from_secs(4));
        assert_eq!(retry_delay(0), Duration::from_secs(4));
    }

    #[test]
    fn retry_budget_allows_three_retries_after_the_initial_attempt() {
        assert_eq!(MAX_AUTO_RETRIES, 3);
        assert_eq!(usize::from(MAX_AUTO_RETRIES) + 1, 4);
    }
}
