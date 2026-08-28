use gpui::Context;

use super::{AhabApp, Page, VisualState};
use crate::{
    app_inputs::{SettingsInputs, TeamInputs},
    ipc::MockClient,
    state::{
        AppState, HomeState, ResourcesState, SettingsPageState, TeamsState, ThemePacksState,
        ToolboxState,
    },
};

fn runtime_client() -> MockClient {
    let mode = std::env::var("AHAB_BACKEND")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let visual_run = std::env::var_os("AHAB_VISUAL_PAGE").is_some()
        || std::env::var_os("AHAB_VISUAL_STATE").is_some();
    let use_sidecar = match mode.as_str() {
        "mock" => false,
        "sidecar" | "real" => true,
        _ => !visual_run,
    };
    if use_sidecar {
        match MockClient::try_sidecar() {
            Ok(client) => return client,
            Err(error) => {
                eprintln!("Python sidecar unavailable, falling back to MockClient: {error}")
            }
        }
    }
    MockClient::default()
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
