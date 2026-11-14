//! Small, static localization catalog shared by the native shell and pages.
//!
//! The persisted [`Language`](crate::model::Language) is the only language
//! setting. Callers select a catalog at render time; the catalog contains no
//! mutable state and therefore cannot drift from the saved settings.

mod bridge;
pub mod en_us;
pub mod messages;
pub mod zh_cn;

pub use bridge::I18n;
pub use messages::feedback;

use crate::model::Language;

/// A small render-time language pair for page-local labels.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Localized {
    pub zh: &'static str,
    pub en: &'static str,
}

impl Localized {
    pub const fn get(self, language: Language) -> &'static str {
        match language {
            Language::ZhCn => self.zh,
            Language::EnUs => self.en,
        }
    }
}

pub const fn paired(zh: &'static str, en: &'static str) -> Localized {
    Localized { zh, en }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Catalog {
    pub app_name: &'static str,
    pub titlebar_title: &'static str,
    pub nav_home: &'static str,
    pub nav_teams: &'static str,
    pub nav_themes: &'static str,
    pub nav_toolbox: &'static str,
    pub nav_resources: &'static str,
    pub nav_help: &'static str,
    pub nav_stats: &'static str,
    pub nav_settings: &'static str,
    pub titlebar_minimize: &'static str,
    pub titlebar_maximize: &'static str,
    pub titlebar_restore: &'static str,
    pub titlebar_close: &'static str,
    pub appearance_label: &'static str,
    pub theme_light: &'static str,
    pub theme_dark: &'static str,
    pub theme_system: &'static str,
    pub contents: &'static str,
    pub home_title: &'static str,
    pub home_idle: &'static str,
    pub home_paused: &'static str,
    pub home_running: &'static str,
    pub tray_show: &'static str,
    pub tray_hide: &'static str,
    pub tray_start: &'static str,
    pub tray_stop: &'static str,
    pub tray_pause: &'static str,
    pub tray_resume: &'static str,
    pub tray_quit: &'static str,
    pub preview_stopped: &'static str,
    pub preview_starting: &'static str,
    pub preview_error: &'static str,
}

/// A catalog key, as TSX names it (`navHome`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Key {
    AppName,
    TitlebarTitle,
    NavHome,
    NavTeams,
    NavThemes,
    NavToolbox,
    NavResources,
    NavHelp,
    NavStats,
    NavSettings,
    TitlebarMinimize,
    TitlebarMaximize,
    TitlebarRestore,
    TitlebarClose,
    AppearanceLabel,
    ThemeLight,
    ThemeDark,
    ThemeSystem,
    Contents,
    HomeTitle,
    HomeIdle,
    HomePaused,
    HomeRunning,
    TrayShow,
    TrayHide,
    TrayStart,
    TrayStop,
    TrayPause,
    TrayResume,
    TrayQuit,
    PreviewStopped,
    PreviewStarting,
    PreviewError,
}

impl Key {
    pub fn parse(value: &str) -> Option<Self> {
        Some(match value {
            "appName" => Self::AppName,
            "titlebarTitle" => Self::TitlebarTitle,
            "navHome" => Self::NavHome,
            "navTeams" => Self::NavTeams,
            "navThemes" => Self::NavThemes,
            "navToolbox" => Self::NavToolbox,
            "navResources" => Self::NavResources,
            "navHelp" => Self::NavHelp,
            "navStats" => Self::NavStats,
            "navSettings" => Self::NavSettings,
            "titlebarMinimize" => Self::TitlebarMinimize,
            "titlebarMaximize" => Self::TitlebarMaximize,
            "titlebarRestore" => Self::TitlebarRestore,
            "titlebarClose" => Self::TitlebarClose,
            "appearanceLabel" => Self::AppearanceLabel,
            "themeLight" => Self::ThemeLight,
            "themeDark" => Self::ThemeDark,
            "themeSystem" => Self::ThemeSystem,
            "contents" => Self::Contents,
            "homeTitle" => Self::HomeTitle,
            "homeIdle" => Self::HomeIdle,
            "homePaused" => Self::HomePaused,
            "homeRunning" => Self::HomeRunning,
            "trayShow" => Self::TrayShow,
            "trayHide" => Self::TrayHide,
            "trayStart" => Self::TrayStart,
            "trayStop" => Self::TrayStop,
            "trayPause" => Self::TrayPause,
            "trayResume" => Self::TrayResume,
            "trayQuit" => Self::TrayQuit,
            "previewStopped" => Self::PreviewStopped,
            "previewStarting" => Self::PreviewStarting,
            "previewError" => Self::PreviewError,
            _ => return None,
        })
    }
}

pub fn catalog(language: Language) -> &'static Catalog {
    match language {
        Language::ZhCn => &zh_cn::CATALOG,
        Language::EnUs => &en_us::CATALOG,
    }
}

pub fn text(language: Language, key: Key) -> &'static str {
    let catalog = catalog(language);
    match key {
        Key::AppName => catalog.app_name,
        Key::TitlebarTitle => catalog.titlebar_title,
        Key::NavHome => catalog.nav_home,
        Key::NavTeams => catalog.nav_teams,
        Key::NavThemes => catalog.nav_themes,
        Key::NavToolbox => catalog.nav_toolbox,
        Key::NavResources => catalog.nav_resources,
        Key::NavHelp => catalog.nav_help,
        Key::NavStats => catalog.nav_stats,
        Key::NavSettings => catalog.nav_settings,
        Key::TitlebarMinimize => catalog.titlebar_minimize,
        Key::TitlebarMaximize => catalog.titlebar_maximize,
        Key::TitlebarRestore => catalog.titlebar_restore,
        Key::TitlebarClose => catalog.titlebar_close,
        Key::AppearanceLabel => catalog.appearance_label,
        Key::ThemeLight => catalog.theme_light,
        Key::ThemeDark => catalog.theme_dark,
        Key::ThemeSystem => catalog.theme_system,
        Key::Contents => catalog.contents,
        Key::HomeTitle => catalog.home_title,
        Key::HomeIdle => catalog.home_idle,
        Key::HomePaused => catalog.home_paused,
        Key::HomeRunning => catalog.home_running,
        Key::TrayShow => catalog.tray_show,
        Key::TrayHide => catalog.tray_hide,
        Key::TrayStart => catalog.tray_start,
        Key::TrayStop => catalog.tray_stop,
        Key::TrayPause => catalog.tray_pause,
        Key::TrayResume => catalog.tray_resume,
        Key::TrayQuit => catalog.tray_quit,
        Key::PreviewStopped => catalog.preview_stopped,
        Key::PreviewStarting => catalog.preview_starting,
        Key::PreviewError => catalog.preview_error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogs_have_the_same_keys_in_both_languages() {
        assert_ne!(
            text(Language::ZhCn, Key::NavHome),
            text(Language::EnUs, Key::NavHome)
        );
        assert_eq!(
            catalog(Language::ZhCn).app_name,
            "Ahab Assistant · Limbus Company"
        );
        assert_eq!(
            catalog(Language::EnUs).app_name,
            "Ahab Assistant · Limbus Company"
        );
    }

    #[test]
    fn every_key_round_trips_through_its_tsx_name() {
        for key in [
            Key::AppName,
            Key::TitlebarTitle,
            Key::NavHome,
            Key::NavTeams,
            Key::NavThemes,
            Key::NavToolbox,
            Key::NavResources,
            Key::NavHelp,
            Key::NavStats,
            Key::NavSettings,
            Key::TitlebarMinimize,
            Key::TitlebarMaximize,
            Key::TitlebarRestore,
            Key::TitlebarClose,
            Key::AppearanceLabel,
            Key::ThemeLight,
            Key::ThemeDark,
            Key::ThemeSystem,
            Key::Contents,
            Key::HomeTitle,
            Key::HomeIdle,
            Key::HomePaused,
            Key::HomeRunning,
            Key::TrayShow,
            Key::TrayHide,
            Key::TrayStart,
            Key::TrayStop,
            Key::TrayPause,
            Key::TrayResume,
            Key::TrayQuit,
            Key::PreviewStopped,
            Key::PreviewStarting,
            Key::PreviewError,
        ] {
            let name = format!("{key:?}");
            let mut chars = name.chars();
            let tsx_name = match chars.next() {
                Some(first) => first.to_lowercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            };
            assert_eq!(Key::parse(&tsx_name), Some(key), "{tsx_name}");
        }
    }
}
