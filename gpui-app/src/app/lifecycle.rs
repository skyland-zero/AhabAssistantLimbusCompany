use gpui::Context;

use super::{AhabApp, Page, VisualState};
use crate::{
    app_inputs::{SettingsInputs, TeamInputs},
    ipc::{BackendClient, RpcGateway, contract::method},
    state::{
        AppState, HomeState, ResourcesState, SettingsPageState, TeamsState, ThemePacksState,
        ToolboxState,
    },
};

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
    match BackendClient::try_sidecar() {
        Ok(client) => client,
        Err(error) => {
            eprintln!("Python sidecar unavailable: {error}");
            BackendClient::unavailable(error)
        }
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
    /// render thread. Constructors intentionally use DTO defaults for a real
    /// transport so startup never synchronously waits on network responses.
    pub fn start_backend_hydration(&mut self, cx: &mut Context<Self>) {
        if !self.home.rpc.is_sidecar() {
            return;
        }

        let home_rpc = self.home.rpc.clone();
        let teams_rpc = self.teams.rpc.clone();
        let themes_rpc = self.theme_packs.rpc.clone();
        let resources_rpc = self.resources.rpc.clone();
        let settings_rpc = self.settings_page.rpc.clone();
        cx.spawn(async move |this, cx| {
            let tasks_request = home_rpc.request_async(method::TASKS_GET_CONFIG, None);
            let tasks_response = cx
                .background_executor()
                .spawn(async move { tasks_request.recv().ok() })
                .await;
            let devices_request = home_rpc.request_async(method::DEVICE_LIST, None);
            let devices_response = cx
                .background_executor()
                .spawn(async move { devices_request.recv().ok() })
                .await;
            let execution_request = home_rpc.request_async(method::EXECUTION_GET_STATE, None);
            let execution_response = cx
                .background_executor()
                .spawn(async move { execution_request.recv().ok() })
                .await;

            let teams_request = teams_rpc.request_async(method::TEAM_LIST, None);
            let teams_response = cx
                .background_executor()
                .spawn(async move { teams_request.recv().ok() })
                .await;
            let sinners_request = teams_rpc.request_async(method::SINNER_LIST, None);
            let sinners_response = cx
                .background_executor()
                .spawn(async move { sinners_request.recv().ok() })
                .await;
            let themes_request = themes_rpc.request_async(method::THEME_PACK_LIST, None);
            let themes_response = cx
                .background_executor()
                .spawn(async move { themes_request.recv().ok() })
                .await;
            let resources_request = resources_rpc.request_async(method::RESOURCE_STATUS, None);
            let resources_response = cx
                .background_executor()
                .spawn(async move { resources_request.recv().ok() })
                .await;
            let hotkey_request = settings_rpc.request_async(method::HOTKEY_GET, None);
            let hotkey_response = cx
                .background_executor()
                .spawn(async move { hotkey_request.recv().ok() })
                .await;
            let system_request = settings_rpc.request_async(method::SYSTEM_SETTINGS_GET, None);
            let system_response = cx
                .background_executor()
                .spawn(async move { system_request.recv().ok() })
                .await;

            let _ = this.update(cx, |view, cx| {
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
            help_scroll: gpui::ScrollHandle::new(),
            home_log_scroll: gpui::ScrollHandle::new(),
            toast: None,
            toast_generation: 0,
            home_log_revision_seen: 0,
        };

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
