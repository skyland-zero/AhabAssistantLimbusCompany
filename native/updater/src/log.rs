//! Lightweight file logging and user-visible error reporting.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Logger {
    file: Option<File>,
    path: Option<PathBuf>,
}

impl Logger {
    pub fn new(temp: &Path) -> Self {
        let path = temp.join("updater.log");
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .ok();
        Self {
            file,
            path: Some(path),
        }
    }

    pub fn line(&mut self, message: &str) {
        let Some(file) = self.file.as_mut() else {
            return;
        };
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        let _ = writeln!(file, "[{timestamp}] {message}");
        let _ = file.flush();
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
}

#[cfg(windows)]
pub fn message_box(title: &str, message: &str, error: bool) {
    use windows::core::PCWSTR;
    use windows::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, MB_ICONERROR, MB_ICONINFORMATION, MB_OK,
    };

    let title: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let message: Vec<u16> = message.encode_utf16().chain(std::iter::once(0)).collect();
    let style = if error {
        MB_OK | MB_ICONERROR
    } else {
        MB_OK | MB_ICONINFORMATION
    };
    unsafe {
        MessageBoxW(
            None,
            PCWSTR::from_raw(message.as_ptr()),
            PCWSTR::from_raw(title.as_ptr()),
            style,
        );
    }
}

#[cfg(not(windows))]
pub fn message_box(title: &str, message: &str, _error: bool) {
    let _ = writeln!(std::io::stderr(), "{title}: {message}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_logger_appends_lines() {
        let temp = std::env::temp_dir().join(format!(
            "aalc-updater-log-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|value| value.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&temp).unwrap();
        let mut logger = Logger::new(&temp);
        logger.line("first");
        logger.line("second");
        let text = std::fs::read_to_string(temp.join("updater.log")).unwrap();
        assert!(text.contains("first"));
        assert!(text.contains("second"));
        let _ = std::fs::remove_dir_all(&temp);
    }
}
