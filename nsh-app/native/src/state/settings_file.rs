//! Loading and atomically replacing the app settings file.
//!
//! The client wrote a temp file, fsynced it, then used `MoveFileExW` with
//! replace-existing on Windows. This crate forbids `unsafe`, so Windows
//! removes an existing destination before the rename; the gap is limited to
//! one settings write and the file is only read at startup.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::model::settings::AppSettings;

pub const APP_SETTINGS_FILE: &str = "settings.json";

/// `%APPDATA%/<app>/settings.json` (or the platform's equivalent).
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

/// Reads `path`, or the defaults when it does not exist.
pub fn load(path: &Path) -> io::Result<AppSettings> {
    match fs::read_to_string(path) {
        Ok(contents) => AppSettings::from_json(&contents)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(AppSettings::default()),
        Err(error) => Err(error),
    }
}

pub fn save(path: &Path, settings: &AppSettings) -> io::Result<()> {
    let json = settings
        .to_json()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    atomic_write(path, json.as_bytes())
}

/// Writes `bytes` to `path` through a sibling temp file.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("settings");
    let temporary = parent.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));

    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.flush()?;
        file.sync_all()?;
        drop(file);
        replace_file(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(not(windows))]
fn replace_file(temporary: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(temporary, destination)
}

#[cfg(windows)]
fn replace_file(temporary: &Path, destination: &Path) -> io::Result<()> {
    if destination.exists() {
        fs::remove_file(destination)?;
    }
    fs::rename(temporary, destination)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn atomic_write_replaces_an_existing_file() {
        let path = std::env::temp_dir().join(format!(
            "ahab-settings-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        atomic_write(&path, b"one").unwrap();
        atomic_write(&path, b"two").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "two");
        let _ = fs::remove_file(path);
    }

    #[test]
    fn missing_file_loads_defaults_and_round_trips() {
        let path = std::env::temp_dir().join(format!(
            "ahab-settings-load-{}-{}.json",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        assert_eq!(load(&path).unwrap(), AppSettings::default());
        let settings = AppSettings {
            skinId: "limbus".into(),
            ..AppSettings::default()
        };
        save(&path, &settings).unwrap();
        assert_eq!(load(&path).unwrap(), settings);
        let _ = fs::remove_file(path);
    }
}
