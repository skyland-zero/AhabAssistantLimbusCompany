//! Layout migration after installing a release.
//!
//! Older releases used a root one-file backend and, later, a separate
//! `runner/` onedir tree.  The merged runtime only needs `backend/AALC
//! Backend.exe`; these helpers remove files the new release does not use while
//! keeping the current shared/separate layouts intact.

use std::fs;
use std::path::Path;

use crate::install::copy_file;
use crate::log::Logger;
use crate::release;

/// Refresh the runtime layout manifest even for an incremental update.
///
/// `release.json` tells the supervisor whether the sidecar or a standalone
/// executable owns the Runner role.  An incremental plan may not list it, but
/// leaving the old layout in place would make the next task launch the wrong
/// executable.
pub fn refresh_release_manifest(extract_root: &Path, install_root: &Path, logger: &mut Logger) {
    let source = extract_root.join("release.json");
    if !source.is_file() {
        return;
    }
    let target = install_root.join("release.json");
    match copy_file(&source, &target) {
        Ok(()) => logger.line("refreshed release.json"),
        Err(error) => logger.line(&format!("refresh release.json failed: {}", error.message)),
    }
}

/// Remove the pre-onedir root-level backend once the staged backend exists.
pub fn remove_legacy_sidecar(install_root: &Path, logger: &mut Logger) {
    let legacy = install_root.join("AALC Backend.exe");
    let staged = install_root.join("backend").join("AALC Backend.exe");
    if legacy.is_file() && staged.is_file() {
        match fs::remove_file(&legacy) {
            Ok(()) => logger.line("removed legacy root AALC Backend.exe"),
            Err(error) => logger.line(&format!("remove legacy backend failed: {error}")),
        }
    }
}

/// Remove the standalone Runner and separate runtime directory only when the
/// new release actually launches its Runner through the sidecar executable.
pub fn remove_legacy_runner_layout(install_root: &Path, extract_root: &Path, logger: &mut Logger) {
    let Some(info) = release::read_release_info(extract_root) else {
        return;
    };

    // The current shared release still launches backend/AALCRunner.exe, so
    // keep it.  A merged or separate release never uses it; remove the stale
    // copy.  Only a merged release makes runner/ stale as well.
    let backend_runner = info
        .runner_entry
        .replace('\\', "/")
        .eq_ignore_ascii_case("backend/AALCRunner.exe");
    if !backend_runner {
        let stale_runner = install_root.join("backend").join("AALCRunner.exe");
        if stale_runner.is_file() {
            match fs::remove_file(&stale_runner) {
                Ok(()) => logger.line("removed legacy backend/AALCRunner.exe"),
                Err(error) => logger.line(&format!("remove legacy runner failed: {error}")),
            }
        }
    }
    if info.shared_runtime {
        let legacy_runner_dir = install_root.join("runner");
        if legacy_runner_dir.is_dir() {
            match fs::remove_dir_all(&legacy_runner_dir) {
                Ok(()) => logger.line("removed legacy runner/ directory"),
                Err(error) => {
                    logger.line(&format!("remove legacy runner directory failed: {error}"))
                }
            }
        }
    }
}

pub fn cleanup(
    temp: &Path,
    extract_root: &Path,
    archive: &Path,
    manifest: &Path,
    signature: &Path,
    logger: &mut Logger,
) {
    for (path, label) in [
        (archive, "archive"),
        (manifest, "manifest"),
        (signature, "signature"),
    ] {
        if path.exists() {
            match fs::remove_file(path) {
                Ok(()) => logger.line(&format!("removed {label}")),
                Err(error) => logger.line(&format!("remove {label} failed: {error}")),
            }
        }
    }
    if extract_root != temp && extract_root.is_dir() {
        match fs::remove_dir_all(extract_root) {
            Ok(()) => logger.line("removed extracted payload"),
            Err(error) => logger.line(&format!("remove extracted payload failed: {error}")),
        }
    }
    let temp_changes = temp.join("changes.json");
    if temp_changes.exists() {
        let _ = fs::remove_file(temp_changes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_dir(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "aalc-updater-migration-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|value| value.as_nanos())
                .unwrap_or(0)
        ));
        fs::create_dir_all(&path).unwrap();
        path
    }

    fn logger(temp: &Path) -> Logger {
        Logger::new(temp)
    }

    #[test]
    fn refresh_release_manifest_updates_the_installed_layout() {
        let temp = temp_dir("release-refresh");
        let install = temp.join("install");
        let extract = temp.join("AALC.nsh");
        fs::create_dir_all(&install).unwrap();
        fs::create_dir_all(&extract).unwrap();
        fs::write(
            install.join("release.json"),
            r#"{"runner":{"entry":"backend/AALCRunner.exe"}}"#,
        )
        .unwrap();
        fs::write(
            extract.join("release.json"),
            r#"{"runner":{"entry":"backend/AALC Backend.exe","args":["--execution-runner"]}}"#,
        )
        .unwrap();

        let mut log = logger(&temp);
        refresh_release_manifest(&extract, &install, &mut log);

        let installed = fs::read_to_string(install.join("release.json")).unwrap();
        assert!(installed.contains("AALC Backend.exe"));
        assert!(installed.contains("--execution-runner"));
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn legacy_runner_migration_respects_the_release_layout() {
        let temp = temp_dir("runner");
        let install = temp.join("install");
        let extract = temp.join("AALC.nsh");
        fs::create_dir_all(install.join("backend")).unwrap();
        fs::create_dir_all(install.join("runner")).unwrap();
        fs::create_dir_all(&extract).unwrap();
        fs::write(install.join("backend").join("AALC Backend.exe"), b"backend").unwrap();
        fs::write(install.join("backend").join("AALCRunner.exe"), b"runner").unwrap();
        fs::write(install.join("runner").join("AALCRunner.exe"), b"runner").unwrap();

        fs::write(
            extract.join("release.json"),
            r#"{"runtime":"shared","runner":{"entry":"backend/AALCRunner.exe"}}"#,
        )
        .unwrap();
        let mut log = logger(&temp);
        remove_legacy_runner_layout(&install, &extract, &mut log);
        assert!(install.join("backend").join("AALCRunner.exe").exists());
        assert!(!install.join("runner").exists());

        fs::create_dir_all(install.join("runner")).unwrap();
        fs::write(install.join("runner").join("AALCRunner.exe"), b"runner").unwrap();
        fs::write(install.join("backend").join("AALCRunner.exe"), b"runner").unwrap();
        fs::write(
            extract.join("release.json"),
            r#"{"runtime":"separate","runner":{"entry":"runner/AALCRunner.exe"}}"#,
        )
        .unwrap();
        remove_legacy_runner_layout(&install, &extract, &mut log);
        assert!(!install.join("backend").join("AALCRunner.exe").exists());
        assert!(install.join("runner").exists());

        fs::write(
            extract.join("release.json"),
            r#"{"runtime":"shared","runner":{"entry":"backend/AALC Backend.exe","args":["--execution-runner"]}}"#,
        )
        .unwrap();
        remove_legacy_runner_layout(&install, &extract, &mut log);
        assert!(!install.join("backend").join("AALCRunner.exe").exists());
        assert!(!install.join("runner").exists());
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn legacy_root_backend_is_removed_when_staged() {
        let temp = temp_dir("legacy");
        let install = temp.join("install");
        fs::create_dir_all(install.join("backend")).unwrap();
        fs::write(install.join("AALC Backend.exe"), b"legacy").unwrap();
        fs::write(install.join("backend").join("AALC Backend.exe"), b"staged").unwrap();

        let mut log = logger(&temp);
        remove_legacy_sidecar(&install, &mut log);

        assert!(!install.join("AALC Backend.exe").exists());
        assert!(install.join("backend").join("AALC Backend.exe").exists());
        let _ = fs::remove_dir_all(&temp);
    }
}
