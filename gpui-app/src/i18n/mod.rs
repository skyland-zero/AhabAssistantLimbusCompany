//! Small, static localization catalog shared by the native shell and pages.
//!
//! The persisted [`Language`](crate::model::Language) is the only language
//! setting.  Callers select a catalog at render time; the catalog contains no
//! mutable state and therefore cannot drift from `AppState.settings`.

pub mod en_us;
pub mod zh_cn;

use crate::model::Language;

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
    pub nav_settings: &'static str,
    pub titlebar_minimize: &'static str,
    pub titlebar_maximize: &'static str,
    pub titlebar_restore: &'static str,
    pub titlebar_close: &'static str,
    pub theme_light: &'static str,
    pub theme_dark: &'static str,
    pub theme_system: &'static str,
    pub contents: &'static str,
    pub home_title: &'static str,
    pub home_idle: &'static str,
    pub home_paused: &'static str,
    pub home_running: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub enum Key {
    AppName,
    TitlebarTitle,
    NavHome,
    NavTeams,
    NavThemes,
    NavToolbox,
    NavResources,
    NavHelp,
    NavSettings,
    TitlebarMinimize,
    TitlebarMaximize,
    TitlebarRestore,
    TitlebarClose,
    ThemeLight,
    ThemeDark,
    ThemeSystem,
    Contents,
    HomeTitle,
    HomeIdle,
    HomePaused,
    HomeRunning,
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
        Key::NavSettings => catalog.nav_settings,
        Key::TitlebarMinimize => catalog.titlebar_minimize,
        Key::TitlebarMaximize => catalog.titlebar_maximize,
        Key::TitlebarRestore => catalog.titlebar_restore,
        Key::TitlebarClose => catalog.titlebar_close,
        Key::ThemeLight => catalog.theme_light,
        Key::ThemeDark => catalog.theme_dark,
        Key::ThemeSystem => catalog.theme_system,
        Key::Contents => catalog.contents,
        Key::HomeTitle => catalog.home_title,
        Key::HomeIdle => catalog.home_idle,
        Key::HomePaused => catalog.home_paused,
        Key::HomeRunning => catalog.home_running,
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
}
