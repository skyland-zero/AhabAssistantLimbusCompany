use gpui::Window;

use super::*;

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
    if let Ok(skin) = std::env::var("AHAB_VISUAL_SKIN")
        && !skin.trim().is_empty()
    {
        settings.skinId = skin;
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
            exit_requested: false,
            stop_timeout_generation: 0,
            stats_tick_generation: 0,
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

        app
    }

    pub(crate) fn apply_visual_state(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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
            VisualState::HomeMirrorDetails => {
                // Deferred: the sheet is pushed onto Root, which cannot be
                // mutated while this frame is being built.
                //
                // This capture shows the *empty* history. The records arrive from
                // the async bootstrap fetch, and a sheet snapshots its data when
                // it opens, so there is no point in the first frames where the
                // records exist and the sheet is still open. A poll-until-loaded
                // version was tried and was worse: when it lost the race it
                // photographed the plain home page, which reads as a pass while
                // proving nothing.
                cx.defer_in(window, move |view, window, cx| {
                    view.open_mirror_details(window, cx);
                });
            }
            VisualState::HomeDailyDetails => {
                // Deferred for the same reason as the mirror sheet: the sheet
                // layer is only drawn from what is on it after the frame being
                // built. Unlike that one, this capture shows real rows - the
                // body is an entity, so it repaints when the fetch lands.
                cx.defer_in(window, move |view, window, cx| {
                    view.open_stats_details(window, cx);
                });
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
                    self.create_team_inputs(window, cx);
                }
            }
            VisualState::TeamsShopEditor => {
                if let Some(team) = self.teams.teams.first().cloned() {
                    self.teams.open_edit(&team);
                    self.create_team_inputs(window, cx);
                    self.teams.set_editor_tab(crate::state::TeamEditorTab::Shop);
                }
            }
            VisualState::TeamsCombatEditor => {
                if let Some(team) = self.teams.teams.first().cloned() {
                    self.teams.open_edit(&team);
                    self.create_team_inputs(window, cx);
                    self.teams
                        .set_editor_tab(crate::state::TeamEditorTab::Combat);
                }
            }
            VisualState::TeamsStarlightEditor => {
                if let Some(team) = self.teams.teams.first().cloned() {
                    self.teams.open_edit(&team);
                    self.create_team_inputs(window, cx);
                    self.teams
                        .set_editor_tab(crate::state::TeamEditorTab::Starlight);
                }
            }
            VisualState::TeamsAdvancedEditor => {
                if let Some(team) = self.teams.teams.first().cloned() {
                    self.teams.open_edit(&team);
                    self.create_team_inputs(window, cx);
                    self.teams
                        .set_editor_tab(crate::state::TeamEditorTab::Advanced);
                }
            }
            VisualState::TeamsDelete => {
                // The confirmation is a Root dialog now, so setting
                // `delete_target` alone would leave the capture showing a page
                // with no dialog on it. The visual state has to open it too -
                // and it has to do so *after* this frame, because pushing to
                // Root's layer while the frame is being built does not land in
                // the frame the caller is about to present.
                if let Some(team) = self.teams.teams.first().cloned() {
                    cx.defer_in(window, move |view, window, cx| {
                        view.open_delete_confirmation(team, window, cx);
                    });
                }
            }
            VisualState::TeamsStatsClear => {
                // Needs the editor open on the stats tab, then the same deferred
                // open the delete state uses: pushing a Root dialog while the
                // frame is being built does not land in that frame.
                if let Some(team) = self.teams.teams.first().cloned() {
                    self.teams.open_edit(&team);
                    self.create_team_inputs(window, cx);
                    self.teams
                        .set_editor_tab(crate::state::TeamEditorTab::Starlight);
                    cx.defer_in(window, move |view, window, cx| {
                        view.open_clear_stats_confirmation(window, cx);
                    });
                }
            }
            VisualState::TeamsPresetOverwrite => {
                // The overwrite confirmation only appears when the preset
                // targets an *existing* team, so the picker has to be opened
                // against one - `open_preset_picker_for_slot` would take the
                // empty-slot path and apply the preset without asking.
                if let Some(team) = self.teams.teams.first().cloned() {
                    self.teams.open_preset_picker_for_team(&team);
                    if let Some(preset_id) = self.teams.presets.first().map(|p| p.presetId.clone())
                        && let Ok(false) = self.teams.select_preset(&preset_id)
                    {
                        cx.defer_in(window, move |view, window, cx| {
                            view.open_preset_overwrite_confirmation(window, cx);
                        });
                    }
                }
            }
            VisualState::TeamsPresetPicker => {
                // Deferred like the dialog states: and the sheet is opened on
                // Root, which cannot be mutated while this frame is being built.
                cx.defer_in(window, move |view, window, cx| {
                    view.open_team_preset_picker_for_slot(1, window, cx);
                });
            }
            VisualState::TeamsSelect => {
                if let Some(team) = self.teams.teams.first().cloned() {
                    self.teams.open_edit(&team);
                    self.create_team_inputs(window, cx);
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
