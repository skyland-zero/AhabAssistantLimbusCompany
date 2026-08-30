use gpui::{ScrollHandle, Subscription};

use self::preview::PreviewControlState;

use crate::{
    app_inputs::{SettingsInputs, TeamInputs},
    i18n::{self, Key as I18nKey},
    pages::HomeViewRefs,
    shell,
    state::{
        AppState, HomeState, ResourcesState, SettingsPageState, TeamsState, ThemePacksState,
        ToolboxState,
    },
};

mod device;
mod interaction;
mod lifecycle;
mod preview;
mod render;
mod scheduling;
mod settings;
mod stats;
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BackendPhase {
    WaitingForFirstFrame,
    Starting,
    RetryWaiting,
    Ready,
    Failed,
    Disconnected,
    Restarting,
    Mock,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BackendStatus {
    pub(crate) phase: BackendPhase,
    pub(crate) retry_no: Option<u8>,
    pub(crate) last_error: Option<String>,
}

impl BackendStatus {
    pub(crate) fn waiting_for_first_frame() -> Self {
        Self {
            phase: BackendPhase::WaitingForFirstFrame,
            retry_no: None,
            last_error: None,
        }
    }

    pub(crate) fn mock() -> Self {
        Self {
            phase: BackendPhase::Mock,
            retry_no: None,
            last_error: None,
        }
    }

    pub(crate) fn can_manual_retry(&self) -> bool {
        matches!(
            self.phase,
            BackendPhase::Failed | BackendPhase::Disconnected
        )
    }

    pub(crate) fn is_ready(&self) -> bool {
        self.phase == BackendPhase::Ready
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BackendOperation {
    Idle,
    Connecting {
        attempt_id: u64,
        retry_no: u8,
        recovery: bool,
        terminal_phase: BackendPhase,
    },
    WaitingRetry {
        attempt_id: u64,
        retry_no: u8,
        recovery: bool,
        terminal_phase: BackendPhase,
    },
}

impl BackendOperation {
    pub(crate) fn is_idle(self) -> bool {
        matches!(self, Self::Idle)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct HomeInvalidation {
    pub(crate) root: bool,
    pub(crate) stats: bool,
    pub(crate) preview: bool,
    pub(crate) logs: bool,
}

impl HomeInvalidation {
    pub(crate) fn merge(&mut self, other: Self) {
        self.root |= other.root;
        self.stats |= other.stats;
        self.preview |= other.preview;
        self.logs |= other.logs;
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
    pub toast: Option<shell::Toast>,
    toast_generation: u64,
    pub(crate) home_views: Option<HomeViewRefs>,
    pub(crate) backend_status: BackendStatus,
    pub(crate) backend_operation: BackendOperation,
    pub(crate) backend_attempt_id: u64,
    pub(crate) backend_epoch: u64,
    pub(crate) stop_timeout_generation: u64,
    pub(crate) theme_persist_timer_generation: Option<u64>,
    pub(crate) preview_control: PreviewControlState,
    pub(crate) window_minimized: bool,
    pub(crate) window_subscriptions: Vec<Subscription>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_status_only_allows_manual_retry_after_failure() {
        let mut status = BackendStatus::waiting_for_first_frame();
        assert!(!status.can_manual_retry());

        status.phase = BackendPhase::RetryWaiting;
        assert!(!status.can_manual_retry());

        status.phase = BackendPhase::Failed;
        assert!(status.can_manual_retry());

        status.phase = BackendPhase::Disconnected;
        assert!(status.can_manual_retry());

        status.phase = BackendPhase::Ready;
        assert!(!status.can_manual_retry());
    }

    #[test]
    fn backend_operations_are_busy_until_the_attempt_finishes() {
        assert!(BackendOperation::Idle.is_idle());
        assert!(
            !BackendOperation::Connecting {
                attempt_id: 1,
                retry_no: 0,
                recovery: false,
                terminal_phase: BackendPhase::Failed,
            }
            .is_idle()
        );
        assert!(
            !BackendOperation::WaitingRetry {
                attempt_id: 1,
                retry_no: 1,
                recovery: false,
                terminal_phase: BackendPhase::Failed,
            }
            .is_idle()
        );
    }
}
