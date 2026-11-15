//! Archive extraction and extracted-root detection.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{Result, UpdaterError};

/// Remove stale extraction directories from a previous update attempt.
pub fn reset_workspace(temp: &Path, archive: &Path) {
    let mut expected = vec![temp.join("AALC.nsh")];
    if let Some(stem) = archive.file_stem().and_then(|stem| stem.to_str()) {
        expected.push(temp.join(stem));
    }

    // A stale plan from the previous attempt must never be paired with a new
    // archive.
    let changes = temp.join("changes.json");
    if changes.is_file() {
        let _ = fs::remove_file(&changes);
    }

    let Ok(entries) = fs::read_dir(temp) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let is_expected = expected.iter().any(|candidate| candidate == &path);
        if is_expected || is_extract_root(&path) {
            let _ = fs::remove_dir_all(&path);
        }
    }
}

/// Extract `archive` into `temp` with the bundled 7-Zip executable.
pub fn extract(seven_zip: &Path, archive: &Path, temp: &Path) -> Result<()> {
    if !seven_zip.is_file() {
        return Err(UpdaterError::archive(format!(
            "缺少 7-Zip 运行时：{}",
            seven_zip.display()
        )));
    }

    let mut command = Command::new(seven_zip);
    command
        .arg("x")
        .arg(archive)
        .arg(format!("-o{}", temp.display()))
        .arg("-aoa")
        .arg("-y")
        .arg("-bd");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW keeps the child from flashing a console window.
        command.creation_flags(0x0800_0000);
    }

    let output = command
        .output()
        .map_err(|error| UpdaterError::archive(format!("无法启动 7-Zip 解压更新包：{error}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(UpdaterError::archive(format!(
            "7-Zip 解压失败（{}）：{}{}",
            output.status,
            stdout.trim(),
            stderr.trim()
        )));
    }
    Ok(())
}

/// Find the directory that contains the extracted release tree.
///
/// Releases are staged from `dist` with `7z a ... "AALC.nsh/*"`, so the usual
/// root is `AALC.nsh/`.  Older or test archives may use the archive stem, and
/// the single-directory fallback keeps those working instead of reproducing
/// the Python updater's "copy a directory that does not exist" failure.
pub fn detect_extract_root(temp: &Path, archive: &Path) -> Result<PathBuf> {
    if let Some(root) = existing_extract_root(temp, archive) {
        return Ok(root);
    }

    let mut discovered = Vec::new();
    if let Ok(entries) = fs::read_dir(temp) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && is_extract_root(&path) {
                discovered.push(path);
            }
        }
    }
    if discovered.len() == 1 {
        return Ok(discovered.remove(0));
    }

    Err(UpdaterError::archive(
        "无法定位更新包解压后的根目录（缺少 AALC.nsh.exe、backend 或 release.json）",
    ))
}

/// Return an existing extracted root without requiring a fresh extraction.
pub fn existing_extract_root(temp: &Path, archive: &Path) -> Option<PathBuf> {
    let mut candidates = vec![temp.join("AALC.nsh")];
    if let Some(stem) = archive.file_stem().and_then(|stem| stem.to_str()) {
        candidates.push(temp.join(stem));
    }
    candidates
        .into_iter()
        .find(|candidate| is_extract_root(candidate))
}

fn is_extract_root(path: &Path) -> bool {
    path.is_dir()
        && (path.join("AALC.nsh.exe").is_file()
            || path.join("backend").join("AALC Backend.exe").is_file()
            || path.join("release.json").is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "aalc-updater-archive-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|value| value.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn release_root_is_detected() {
        let temp = temp_dir();
        let root = temp.join("AALC.nsh");
        fs::create_dir_all(root.join("backend")).unwrap();
        fs::write(root.join("backend").join("AALC Backend.exe"), b"backend").unwrap();
        fs::write(root.join("release.json"), b"{}").unwrap();

        let archive = temp.join("AALC.nsh_1.2.3.7z");
        fs::write(&archive, b"archive").unwrap();

        assert_eq!(detect_extract_root(&temp, &archive).unwrap(), root);
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn archive_stem_root_is_detected() {
        let temp = temp_dir();
        let root = temp.join("AALC.nsh_1.2.3");
        fs::create_dir_all(root.join("backend")).unwrap();
        fs::write(root.join("backend").join("AALC Backend.exe"), b"backend").unwrap();

        let archive = temp.join("AALC.nsh_1.2.3.7z");
        fs::write(&archive, b"archive").unwrap();

        assert_eq!(detect_extract_root(&temp, &archive).unwrap(), root);
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn missing_root_is_an_error() {
        let temp = temp_dir();
        let archive = temp.join("AALC.nsh_1.2.3.7z");
        fs::write(&archive, b"archive").unwrap();
        assert!(detect_extract_root(&temp, &archive).is_err());
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn reset_workspace_removes_stale_roots() {
        let temp = temp_dir();
        let stale = temp.join("AALC.nsh");
        fs::create_dir_all(stale.join("backend")).unwrap();
        fs::write(stale.join("release.json"), b"{}").unwrap();
        let keep = temp.join("unrelated");
        fs::create_dir_all(&keep).unwrap();
        fs::write(temp.join("changes.json"), b"{}").unwrap();

        let archive = temp.join("AALC.nsh_1.2.3.7z");
        reset_workspace(&temp, &archive);

        assert!(!stale.exists());
        assert!(keep.exists());
        assert!(!temp.join("changes.json").exists());
        let _ = fs::remove_dir_all(&temp);
    }
}
