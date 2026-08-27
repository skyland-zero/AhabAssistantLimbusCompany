use std::{io, path::Path};

use crate::model::AppSettings;

/// Non-rendering application state. Persistence is intentionally kept outside
/// GPUI render methods so loading and atomic writes never block a frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AppState {
    pub settings: AppSettings,
}

impl AppState {
    pub fn new(settings: AppSettings) -> Self {
        Self { settings }
    }

    pub fn load() -> Self {
        Self::load_from(&AppSettings::config_path())
    }

    pub fn load_from(path: &Path) -> Self {
        Self::new(AppSettings::load(path).unwrap_or_default())
    }

    pub fn save(&self) -> io::Result<()> {
        self.save_to(&AppSettings::config_path())
    }

    pub fn save_to(&self, path: &Path) -> io::Result<()> {
        self.settings.save(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_or_invalid_settings_fall_back_to_defaults() {
        let missing = std::env::temp_dir().join(format!(
            "ahab-app-state-missing-{}-{}.json",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        let _ = std::fs::remove_file(&missing);
        let missing_state = AppState::load_from(&missing);
        assert_eq!(missing_state.settings, AppSettings::default());

        let invalid = missing.with_file_name("ahab-app-state-invalid.json");
        std::fs::write(&invalid, "not json").unwrap();
        let invalid_state = AppState::load_from(&invalid);
        assert_eq!(invalid_state.settings, AppSettings::default());
        let _ = std::fs::remove_file(invalid);
    }

    #[test]
    fn right_panel_collapsed_round_trips_without_a_window() {
        let path = std::env::temp_dir().join(format!(
            "ahab-app-state-roundtrip-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);

        let mut state = AppState::default();
        state.settings.rightPanelCollapsed = true;
        state.save_to(&path).unwrap();

        let loaded = AppState::load_from(&path);
        assert!(loaded.settings.rightPanelCollapsed);
        assert_eq!(loaded.settings.rightPanelWidth, 280);
        let _ = std::fs::remove_file(path);
    }
}
