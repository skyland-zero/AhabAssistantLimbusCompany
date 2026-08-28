use serde::{Deserialize, Serialize};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

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
        // Defaults make older files forward compatible; this is the migration seam for
        // future versions, while unknown fields remain harmless to serde.
        let settings = serde_json::from_value::<AppSettings>(value)?;
        Ok(migrate(version, settings))
    }

    pub fn config_path() -> PathBuf {
        config_path_for("AhabAssistant")
    }
    pub fn load(path: &Path) -> io::Result<Self> {
        match fs::read_to_string(path) {
            Ok(contents) => Self::from_json(&contents)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error),
        }
    }
    pub fn save(&self, path: &Path) -> io::Result<()> {
        atomic_write(
            path,
            self.to_json()
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?
                .as_bytes(),
        )
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

pub fn config_path_for(app_name: &str) -> PathBuf {
    config_directory(app_name).join(APP_SETTINGS_FILE)
}

pub fn config_directory(app_name: &str) -> PathBuf {
    let base = std::env::var_os("APPDATA")
        .or_else(|| std::env::var_os("LOCALAPPDATA"))
        .or_else(|| std::env::var_os("XDG_CONFIG_HOME"))
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    base.join(app_name)
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut temporary = path.to_path_buf();
    let extension = temporary
        .extension()
        .and_then(|part| part.to_str())
        .unwrap_or("tmp");
    temporary.set_extension(format!("{extension}.tmp"));
    fs::write(&temporary, bytes)?;
    replace_file(&temporary, path)
}

#[cfg(not(windows))]
fn replace_file(temporary: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(temporary, destination)
}

#[cfg(windows)]
fn replace_file(temporary: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    let source: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
    let target: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    unsafe extern "system" {
        fn MoveFileExW(existing: *const u16, new: *const u16, flags: u32) -> i32;
    }
    const MOVEFILE_REPLACE_EXISTING: u32 = 0x1;
    const MOVEFILE_WRITE_THROUGH: u32 = 0x8;
    if unsafe {
        MoveFileExW(
            source.as_ptr(),
            target.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    } == 0
    {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_match_zustand_partialize() {
        let settings = AppSettings::default();
        assert!(!settings.sidebarCollapsed);
        assert_eq!(settings.rightPanelWidth, 280);
        assert!(!settings.rightPanelCollapsed);
        assert_eq!(settings.themeMode, ThemeMode::System);
        assert_eq!(settings.accentId, "crimson");
        assert_eq!(settings.language, Language::ZhCn);
        assert_eq!(settings.lastDeviceId, None);
    }
    #[test]
    fn versioned_json_round_trip_and_legacy_migration() {
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
    fn right_panel_width_is_bounded_for_version_zero_and_one() {
        for version in [0, 1] {
            for (width, expected) in [(100, MIN_RIGHT_PANEL_WIDTH), (900, MAX_RIGHT_PANEL_WIDTH)] {
                let json = format!(r#"{{"version":{version},"rightPanelWidth":{width}}}"#);
                let settings = AppSettings::from_json(&json).unwrap();
                assert_eq!(settings.rightPanelWidth, expected);
            }
        }
    }

    #[test]
    fn atomic_write_replaces_existing_file() {
        let path = std::env::temp_dir().join(format!("ahab-settings-{}.json", std::process::id()));
        atomic_write(&path, b"one").unwrap();
        atomic_write(&path, b"two").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "two");
        let _ = fs::remove_file(path);
    }
}
