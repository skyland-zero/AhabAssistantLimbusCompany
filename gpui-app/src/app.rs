use std::sync::Arc;

use gpui::{Image, RenderImage, ScrollHandle};

use crate::{
    app_inputs::{SettingsInputs, TeamInputs},
    i18n::{self, Key as I18nKey},
    shell,
    state::{
        AppState, HomeState, ResourcesState, SettingsPageState, TeamsState, ThemePacksState,
        ToolboxState,
    },
};

mod device;
mod interaction;
mod lifecycle;
mod render;
mod settings;
mod team_editor;

// Keep the existing page-facing imports stable while the palette lives with
// the reusable controls.
pub use crate::components::style::{
    ACCENT, BACKGROUND, BORDER, SURFACE, SURFACE_HOVER, TEXT, TEXT_MUTED,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Page {
    Home,
    Teams,
    ThemePacks,
    Toolbox,
    Resources,
    Help,
    Settings,
}

impl Page {
    #[allow(dead_code)]
    pub const ALL: [Page; 7] = [
        Self::Home,
        Self::Teams,
        Self::ThemePacks,
        Self::Toolbox,
        Self::Resources,
        Self::Help,
        Self::Settings,
    ];

    #[allow(dead_code)]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Home => "主控台",
            Self::Teams => "队伍管理",
            Self::ThemePacks => "主题包",
            Self::Toolbox => "工具箱",
            Self::Resources => "资源中心",
            Self::Help => "帮助",
            Self::Settings => "设置",
        }
    }

    pub fn label_for(self, language: crate::model::Language) -> &'static str {
        let key = match self {
            Self::Home => I18nKey::NavHome,
            Self::Teams => I18nKey::NavTeams,
            Self::ThemePacks => I18nKey::NavThemes,
            Self::Toolbox => I18nKey::NavToolbox,
            Self::Resources => I18nKey::NavResources,
            Self::Help => I18nKey::NavHelp,
            Self::Settings => I18nKey::NavSettings,
        };
        i18n::text(language, key)
    }

    #[allow(dead_code)]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Home => "CONSOLE",
            Self::Teams => "TEAMS",
            Self::ThemePacks => "THEME PACKS",
            Self::Toolbox => "TOOLBOX",
            Self::Resources => "RESOURCES",
            Self::Help => "HELP",
            Self::Settings => "SETTINGS",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Some(match value.to_ascii_lowercase().as_str() {
            "home" | "console" => Self::Home,
            "teams" => Self::Teams,
            "themes" | "theme-packs" => Self::ThemePacks,
            "toolbox" => Self::Toolbox,
            "resources" => Self::Resources,
            "help" => Self::Help,
            "settings" => Self::Settings,
            _ => return None,
        })
    }
}

/// Root UI state. Home owns its task, execution, device, and log state so the
/// page can evolve without scattering business state through render methods.
pub struct AhabApp {
    pub current_page: Page,
    pub state: AppState,
    pub home: HomeState,
    pub teams: TeamsState,
    pub theme_packs: ThemePacksState,
    pub toolbox: ToolboxState,
    pub resources: ResourcesState,
    pub settings_page: SettingsPageState,
    pub team_inputs: TeamInputs,
    pub settings_inputs: SettingsInputs,
    visual_state: Option<VisualState>,
    pub help_scroll: ScrollHandle,
    pub home_log_scroll: ScrollHandle,
    pub toast: Option<shell::Toast>,
    toast_generation: u64,
    home_log_revision_seen: u64,
    pub(crate) screenshot_image_source: Option<Arc<Image>>,
    pub(crate) screenshot_render_image: Option<Arc<RenderImage>>,
    pub(crate) screenshot_image_revision: u64,
    pub(crate) screenshot_pending_image_source: Option<Arc<Image>>,
    pub(crate) screenshot_pending_render_image: Option<Arc<RenderImage>>,
    pub(crate) screenshot_pending_image_revision: Option<u64>,
}

/// Deterministic transient states used by the visual-regression harness. These
/// never enter persisted settings or the production IPC contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VisualState {
    HomeExpanded,
    HomeSelect,
    HomeRunning,
    HomePaused,
    HomeAfterCompletion,
    TeamsEditor,
    TeamsDelete,
    TeamsSelect,
    SettingsHotkey,
    SettingsSelect,
    SettingsLatest,
    ToolboxRunning,
    ResourcesSyncing,
    HelpScrolled,
}

impl VisualState {
    fn parse(value: &str) -> Option<Self> {
        Some(match value.to_ascii_lowercase().as_str() {
            "home-expanded" => Self::HomeExpanded,
            "home-select" => Self::HomeSelect,
            "home-running" => Self::HomeRunning,
            "home-paused" => Self::HomePaused,
            "home-after-completion" => Self::HomeAfterCompletion,
            "teams-editor" => Self::TeamsEditor,
            "teams-delete" => Self::TeamsDelete,
            "teams-select" => Self::TeamsSelect,
            "settings-hotkey" => Self::SettingsHotkey,
            "settings-select" => Self::SettingsSelect,
            "settings-latest" => Self::SettingsLatest,
            "toolbox-running" => Self::ToolboxRunning,
            "resources-syncing" => Self::ResourcesSyncing,
            "help-scrolled" => Self::HelpScrolled,
            _ => return None,
        })
    }

    const fn page(self) -> Page {
        match self {
            Self::HomeExpanded
            | Self::HomeSelect
            | Self::HomeRunning
            | Self::HomePaused
            | Self::HomeAfterCompletion => Page::Home,
            Self::TeamsEditor | Self::TeamsDelete | Self::TeamsSelect => Page::Teams,
            Self::SettingsHotkey | Self::SettingsSelect | Self::SettingsLatest => Page::Settings,
            Self::ToolboxRunning => Page::Toolbox,
            Self::ResourcesSyncing => Page::Resources,
            Self::HelpScrolled => Page::Help,
        }
    }
}
