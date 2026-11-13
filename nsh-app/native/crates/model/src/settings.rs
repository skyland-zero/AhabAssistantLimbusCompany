//! Settings payloads: the persisted appearance/preferences model.
//!
//! File I/O stays out of `model/`; `state/settings_file.rs` owns it. This file
//! keeps the versioned JSON shape and its migration, which `cargo test` can
//! check without touching the user's profile.

use serde::{Deserialize, Serialize};

pub const APP_SETTINGS_FILE: &str = "settings.json";
pub const MIN_RIGHT_PANEL_WIDTH: u32 = 280;
pub const MAX_RIGHT_PANEL_WIDTH: u32 = 800;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    Light,
    #[default]
    Dark,
    System,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum Language {
    #[default]
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "en-US")]
    EnUs,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default)]
#[allow(non_snake_case)]
pub struct AppSettings {
    pub sidebarCollapsed: bool,
    pub rightPanelWidth: u32,
    pub rightPanelCollapsed: bool,
    pub themeMode: ThemeMode,
    pub accentId: String,
    /// Visual skin id (`"default"`, `"limbus"`, ...). Unknown values stay in the
    /// file and the theme layer falls back to the default skin.
    pub skinId: String,
    pub language: Language,
    pub lastDeviceId: Option<String>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            sidebarCollapsed: false,
            rightPanelWidth: 280,
            rightPanelCollapsed: false,
            themeMode: ThemeMode::System,
            accentId: "crimson".into(),
            skinId: "default".into(),
            language: Language::ZhCn,
            lastDeviceId: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
struct VersionedSettings {
    version: u32,
    #[serde(flatten)]
    settings: AppSettings,
}

impl AppSettings {
    pub const CURRENT_VERSION: u32 = 1;

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&VersionedSettings {
            version: Self::CURRENT_VERSION,
            settings: self.clone(),
        })
    }

    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        let value: serde_json::Value = serde_json::from_str(json)?;
        let version = value
            .get("version")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as u32;
        // Defaults make older files forward compatible; this is the migration
        // seam for future versions, while unknown fields remain harmless.
        let settings = serde_json::from_value::<AppSettings>(value)?;
        Ok(migrate(version, settings))
    }
}

fn migrate(version: u32, mut settings: AppSettings) -> AppSettings {
    if version == 0 && settings.rightPanelWidth == 0 {
        settings.rightPanelWidth = MIN_RIGHT_PANEL_WIDTH;
    }
    settings.rightPanelWidth = settings
        .rightPanelWidth
        .clamp(MIN_RIGHT_PANEL_WIDTH, MAX_RIGHT_PANEL_WIDTH);
    settings
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_client() {
        let settings = AppSettings::default();
        assert!(!settings.sidebarCollapsed);
        assert_eq!(settings.rightPanelWidth, 280);
        assert!(!settings.rightPanelCollapsed);
        assert_eq!(settings.themeMode, ThemeMode::System);
        assert_eq!(settings.accentId, "crimson");
        assert_eq!(settings.skinId, "default");
        assert_eq!(settings.language, Language::ZhCn);
        assert_eq!(settings.lastDeviceId, None);
    }

    #[test]
    fn versioned_json_round_trips_and_migrates_legacy_files() {
        let settings = AppSettings::default();
        let json = settings.to_json().unwrap();
        assert!(json.contains("\"version\": 1"));
        assert_eq!(AppSettings::from_json(&json).unwrap(), settings);
        let old = r#"{"sidebarCollapsed":true,"accentId":"blue"}"#;
        let migrated = AppSettings::from_json(old).unwrap();
        assert!(migrated.sidebarCollapsed);
        assert_eq!(migrated.rightPanelWidth, 280);
        assert_eq!(migrated.language, Language::ZhCn);
    }

    #[test]
    fn new_skin_and_accent_ids_round_trip_without_a_schema_migration() {
        let settings = AppSettings {
            skinId: "limbus".into(),
            accentId: "limbus-brass".into(),
            ..AppSettings::default()
        };
        let json = settings.to_json().unwrap();
        assert_eq!(AppSettings::from_json(&json).unwrap(), settings);
        let legacy = AppSettings::from_json(r#"{"accentId":"blue"}"#).unwrap();
        assert_eq!(legacy.skinId, "default");
    }

    #[test]
    fn right_panel_width_is_bounded_for_version_zero_and_one() {
        for version in [0, 1] {
            for (width, expected) in [(100, MIN_RIGHT_PANEL_WIDTH), (900, MAX_RIGHT_PANEL_WIDTH)] {
                let json = format!(r#"{{"version":{version},"rightPanelWidth":{width}}}"#);
                let settings = AppSettings::from_json(&json).unwrap();
                assert_eq!(settings.rightPanelWidth, expected);
            }
        }
    }
}
