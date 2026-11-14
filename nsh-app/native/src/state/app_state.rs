//! The persisted app-shell settings: appearance, skin, accent and language.

use std::io;
use std::path::{Path, PathBuf};

use nsh_core::prelude::*;

use crate::model::{AppSettings, Language, ThemeMode};

use super::settings_file;

/// Non-rendering application state. File I/O stays here so pages never block a
/// frame on a settings write.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AppState {
    pub settings: AppSettings,
}

impl AppState {
    pub fn new(settings: AppSettings) -> Self {
        Self { settings }
    }

    pub fn load() -> Self {
        Self::load_from(&config_path())
    }

    pub fn load_from(path: &Path) -> Self {
        Self::new(settings_file::load(path).unwrap_or_default())
    }

    pub fn save(&self) -> io::Result<()> {
        self.save_to(&config_path())
    }

    pub fn save_to(&self, path: &Path) -> io::Result<()> {
        settings_file::save(path, &self.settings)
    }
}

/// `%APPDATA%/AhabAssistant/settings.json`, matching the GPUI client.
pub fn config_path() -> PathBuf {
    settings_file::config_path_for("AhabAssistant")
}

/// The app-shell settings TSX reads at startup and writes on change.
#[class]
pub struct AppStore {
    state: AppState,
    /// `None` in mock/test mode: defaults stay in memory so a developer's real
    /// `settings.json` cannot change the deterministic test surface.
    path: Option<PathBuf>,
}

#[class]
impl AppStore {
    /// Loads the settings file (defaults when it is missing or invalid).
    pub fn new() -> Self {
        if isolated_from_disk() {
            return Self {
                state: AppState::default(),
                path: None,
            };
        }
        let path = config_path();
        Self {
            state: AppState::load_from(&path),
            path: Some(path),
        }
    }

    /// The visual skin id (`default`, `glass`, `archive`, `limbus`, `mist`).
    pub fn skin(&self) -> String {
        self.state.settings.skinId.clone()
    }

    /// The accent id (`crimson`, `blue`, `amber`, `emerald`, `violet`,
    /// `limbus-brass`).
    pub fn accent(&self) -> String {
        self.state.settings.accentId.clone()
    }

    /// `light`, `dark` or `system`.
    pub fn appearance(&self) -> String {
        match self.state.settings.themeMode {
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
            ThemeMode::System => "system",
        }
        .into()
    }

    /// `zhCn` or `enUs`.
    pub fn language(&self) -> String {
        match self.state.settings.language {
            Language::ZhCn => "zhCn",
            Language::EnUs => "enUs",
        }
        .into()
    }

    /// The right rail's width in points.
    pub fn right_panel_width(&self) -> u32 {
        self.state.settings.rightPanelWidth
    }

    /// Whether the right rail is collapsed.
    pub fn right_panel_collapsed(&self) -> bool {
        self.state.settings.rightPanelCollapsed
    }

    /// Whether the sidebar is collapsed.
    pub fn sidebar_collapsed(&self) -> bool {
        self.state.settings.sidebarCollapsed
    }

    /// The colour theme's seed (`#RRGGBB`), or empty for the skin's own accent.
    pub fn theme_seed(&self) -> String {
        self.state.settings.themeSeed.clone().unwrap_or_default()
    }

    /// Saves the colour theme; an empty string clears it.
    pub fn set_theme_seed(&mut self, seed: String) -> Result<(), String> {
        let seed = seed.trim().to_owned();
        self.state.settings.themeSeed = if seed.is_empty() { None } else { Some(seed) };
        self.save()
    }

    /// Saves the skin and accent pair.
    pub fn set_skin(&mut self, skin: String, accent: String) -> Result<(), String> {
        self.state.settings.skinId = skin;
        self.state.settings.accentId = accent;
        self.save()
    }

    /// Saves the light/dark/system appearance.
    pub fn set_appearance(&mut self, mode: String) -> Result<(), String> {
        self.state.settings.themeMode = match mode.as_str() {
            "light" => ThemeMode::Light,
            "dark" => ThemeMode::Dark,
            _ => ThemeMode::System,
        };
        self.save()
    }

    /// Saves the interface language.
    pub fn set_language(&mut self, language: String) -> Result<(), String> {
        self.state.settings.language = match language.as_str() {
            "enUs" => Language::EnUs,
            _ => Language::ZhCn,
        };
        self.save()
    }

    /// Saves the right rail's width, held inside the model's bounds.
    pub fn set_right_panel_width(&mut self, width: u32) -> Result<(), String> {
        self.state.settings.rightPanelWidth = width.clamp(
            crate::model::settings::MIN_RIGHT_PANEL_WIDTH,
            crate::model::settings::MAX_RIGHT_PANEL_WIDTH,
        );
        self.save()
    }

    /// Saves the right rail's collapsed state.
    pub fn set_right_panel_collapsed(&mut self, collapsed: bool) -> Result<(), String> {
        self.state.settings.rightPanelCollapsed = collapsed;
        self.save()
    }

    /// Saves the sidebar's collapsed state.
    pub fn set_sidebar_collapsed(&mut self, collapsed: bool) -> Result<(), String> {
        self.state.settings.sidebarCollapsed = collapsed;
        self.save()
    }

    fn save(&self) -> Result<(), String> {
        match self.path.as_ref() {
            Some(path) => self.state.save_to(path).map_err(|error| error.to_string()),
            None => Ok(()),
        }
    }
}

impl Default for AppStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Whether the settings store must stay in memory instead of using the user's
/// real `settings.json`.
///
/// The app-shell settings are a *user* preference, so a test run must not read
/// them (a developer whose interface is English would fail every copy
/// assertion) and must not write them (the language and appearance tests switch
/// both, which would silently change the developer's own client). Two cases:
///
/// - `AHAB_BACKEND=mock`: the deterministic test surface (see `README.md`).
/// - `AHAB_SETTINGS_IN_MEMORY=1`: the headless `nsh test` runner, where the
///   mock is not the default. `nsh` starts the test host with it, so plain
///   `nsh test` is isolated too.
fn isolated_from_disk() -> bool {
    let flag = |name: &str| {
        std::env::var(name)
            .map(|value| value.eq_ignore_ascii_case("mock") || value.trim() == "1")
            .unwrap_or(false)
    };
    flag("AHAB_BACKEND") || flag("AHAB_SETTINGS_IN_MEMORY")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "ahab-app-state-{name}-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn missing_or_invalid_settings_fall_back_to_defaults() {
        let missing = temp_path("missing");
        let _ = std::fs::remove_file(&missing);
        assert_eq!(
            AppState::load_from(&missing).settings,
            AppSettings::default()
        );

        let invalid = temp_path("invalid");
        std::fs::write(&invalid, "not json").unwrap();
        assert_eq!(
            AppState::load_from(&invalid).settings,
            AppSettings::default()
        );
        let _ = std::fs::remove_file(invalid);
    }

    #[test]
    fn appearance_and_language_round_trip() {
        let path = temp_path("roundtrip");
        let mut state = AppState::default();
        state.settings.rightPanelCollapsed = true;
        state.settings.skinId = "limbus".into();
        state.settings.accentId = "limbus-brass".into();
        state.settings.themeMode = crate::model::ThemeMode::Light;
        state.settings.language = crate::model::Language::EnUs;
        state.save_to(&path).unwrap();

        let loaded = AppState::load_from(&path);
        assert!(loaded.settings.rightPanelCollapsed);
        assert_eq!(loaded.settings.skinId, "limbus");
        assert_eq!(loaded.settings.accentId, "limbus-brass");
        assert_eq!(loaded.settings.themeMode, crate::model::ThemeMode::Light);
        assert_eq!(loaded.settings.language, crate::model::Language::EnUs);
        let _ = std::fs::remove_file(path);
    }
}
