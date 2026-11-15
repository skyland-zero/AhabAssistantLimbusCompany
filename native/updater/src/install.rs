//! Applying an extracted payload to the install directory.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::log::Logger;
use crate::paths;
use crate::{Result, UpdaterError};

const KNOWN_OPERATIONS: [&str; 5] = ["deleted_dir", "deleted", "added_dir", "added", "modified"];

#[derive(Debug, Clone)]
pub struct Entry {
    pub raw: String,
    pub target: PathBuf,
}

#[derive(Debug, Clone)]
pub struct CopyEntry {
    pub raw: String,
    pub source: PathBuf,
    pub target: PathBuf,
}

#[derive(Debug, Default, Clone)]
pub struct IncrementalPlan {
    pub deleted_dir: Vec<Entry>,
    pub deleted: Vec<Entry>,
    pub added_dir: Vec<Entry>,
    pub added: Vec<CopyEntry>,
    pub modified: Vec<CopyEntry>,
}

/// Load and fully validate `changes.json` before any process is terminated or
/// any file is touched.
pub fn load_incremental_plan(
    extract_root: &Path,
    temp: &Path,
    install_root: &Path,
) -> Result<Option<IncrementalPlan>> {
    let candidates = [extract_root.join("changes.json"), temp.join("changes.json")];
    let Some(path) = candidates.iter().find(|candidate| candidate.is_file()) else {
        return Ok(None);
    };

    let bytes = fs::read(path).map_err(|error| {
        UpdaterError::install(format!("无法读取更新清单 {}：{error}", path.display()))
    })?;
    let payload: Value = serde_json::from_slice(&bytes)
        .map_err(|error| UpdaterError::install(format!("更新清单不是有效 JSON：{error}")))?;
    let object = payload
        .as_object()
        .ok_or_else(|| UpdaterError::install("更新清单必须是对象"))?;

    let archive_root = extract_root
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_string);
    let mut plan = IncrementalPlan::default();

    for (operation, entries) in object {
        if !KNOWN_OPERATIONS.contains(&operation.as_str()) {
            return Err(UpdaterError::install(format!(
                "更新清单包含未知操作：{operation}"
            )));
        }
        let list = entries
            .as_array()
            .ok_or_else(|| UpdaterError::install(format!("更新清单操作 {operation} 必须是数组")))?;
        for entry in list {
            let raw = entry
                .as_str()
                .ok_or_else(|| UpdaterError::install("更新清单路径必须是字符串"))?;
            let relative = paths::normalize_relative_path(raw, archive_root.as_deref())?;
            let target = paths::resolve_within_root(install_root, &relative)?;
            match operation.as_str() {
                "deleted_dir" => plan.deleted_dir.push(Entry {
                    raw: raw.to_string(),
                    target,
                }),
                "deleted" => plan.deleted.push(Entry {
                    raw: raw.to_string(),
                    target,
                }),
                "added_dir" => plan.added_dir.push(Entry {
                    raw: raw.to_string(),
                    target,
                }),
                "added" => plan.added.push(CopyEntry {
                    raw: raw.to_string(),
                    source: paths::resolve_within_root(extract_root, &relative)?,
                    target,
                }),
                "modified" => plan.modified.push(CopyEntry {
                    raw: raw.to_string(),
                    source: paths::resolve_within_root(extract_root, &relative)?,
                    target,
                }),
                _ => unreachable!("operation was validated above"),
            }
        }
    }

    Ok(Some(plan))
}

pub fn apply_incremental(plan: &IncrementalPlan, logger: &mut Logger) -> Result<()> {
    for entry in &plan.deleted_dir {
        match fs::remove_dir_all(&entry.target) {
            Ok(()) => logger.line(&format!("deleted directory: {}", entry.raw)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => logger.line(&format!("delete directory failed ({}): {error}", entry.raw)),
        }
    }
    for entry in &plan.deleted {
        match fs::remove_file(&entry.target) {
            Ok(()) => logger.line(&format!("deleted file: {}", entry.raw)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => logger.line(&format!("delete file failed ({}): {error}", entry.raw)),
        }
    }
    for entry in &plan.added_dir {
        fs::create_dir_all(&entry.target).map_err(|error| {
            UpdaterError::install(format!("无法创建目录 {}：{error}", entry.target.display()))
        })?;
        logger.line(&format!("created directory: {}", entry.raw));
    }
    for entry in plan.added.iter().chain(plan.modified.iter()) {
        if !entry.source.is_file() {
            logger.line(&format!("source is missing, skipped: {}", entry.raw));
            continue;
        }
        if let Some(parent) = entry.target.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                UpdaterError::install(format!("无法创建目录 {}：{error}", parent.display()))
            })?;
        }
        copy_file(&entry.source, &entry.target)?;
        logger.line(&format!("updated file: {}", entry.raw));
    }
    Ok(())
}

/// Full cover: the historical updater removes `assets/images` first so the
/// bundled image set replaces any resource-sync leftovers, then copies every
/// extracted file over the install tree.
pub fn cover(extract_root: &Path, install_root: &Path, logger: &mut Logger) -> Result<()> {
    let images = install_root.join("assets").join("images");
    if images.is_dir() {
        match fs::remove_dir_all(&images) {
            Ok(()) => logger.line("removed assets/images before cover"),
            Err(error) => logger.line(&format!("remove assets/images failed: {error}")),
        }
    }
    copy_tree(extract_root, install_root, logger)?;
    Ok(())
}

fn copy_tree(source: &Path, destination: &Path, logger: &mut Logger) -> Result<()> {
    fs::create_dir_all(destination).map_err(|error| {
        UpdaterError::install(format!("无法创建目录 {}：{error}", destination.display()))
    })?;
    let entries = fs::read_dir(source).map_err(|error| {
        UpdaterError::install(format!("无法读取目录 {}：{error}", source.display()))
    })?;
    for entry in entries {
        let entry =
            entry.map_err(|error| UpdaterError::install(format!("读取目录项失败：{error}")))?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        let metadata = fs::symlink_metadata(&source_path).map_err(|error| {
            UpdaterError::install(format!("无法读取 {}：{error}", source_path.display()))
        })?;
        if paths::is_reparse_point(&metadata) {
            return Err(UpdaterError::install(format!(
                "更新包包含链接或重解析点：{}",
                source_path.display()
            )));
        }
        if metadata.is_dir() {
            copy_tree(&source_path, &destination_path, logger)?;
        } else if metadata.is_file() {
            copy_file(&source_path, &destination_path)?;
        } else {
            logger.line(&format!(
                "skipped unsupported file type: {}",
                source_path.display()
            ));
        }
    }
    Ok(())
}

#[allow(clippy::permissions_set_readonly_false)]
pub(crate) fn copy_file(source: &Path, destination: &Path) -> Result<()> {
    if let Ok(metadata) = fs::metadata(destination) {
        if metadata.permissions().readonly() {
            let mut permissions = metadata.permissions();
            permissions.set_readonly(false);
            let _ = fs::set_permissions(destination, permissions);
        }
    }
    fs::copy(source, destination).map_err(|error| {
        UpdaterError::install(format!("无法覆盖 {}：{error}", destination.display()))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "aalc-updater-install-{label}-{}-{}",
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
    fn incremental_plan_applies_deletes_and_copies() {
        let temp = temp_dir("incremental");
        let install = temp.join("install");
        let extract = temp.join("AALC.nsh");
        fs::create_dir_all(install.join("assets")).unwrap();
        fs::create_dir_all(extract.join("assets")).unwrap();
        fs::write(install.join("old.txt"), b"old").unwrap();
        fs::write(install.join("assets").join("old.png"), b"old").unwrap();
        fs::write(extract.join("new.txt"), b"new").unwrap();
        fs::write(extract.join("assets").join("new.png"), b"new").unwrap();
        fs::write(
            extract.join("changes.json"),
            serde_json::json!({
                "deleted": ["old.txt"],
                "deleted_dir": ["assets"],
                "added_dir": ["assets"],
                "added": ["assets/new.png"],
                "modified": ["new.txt"]
            })
            .to_string(),
        )
        .unwrap();

        let plan = load_incremental_plan(&extract, &temp, &install)
            .unwrap()
            .expect("plan");
        let mut log = logger(&temp);
        apply_incremental(&plan, &mut log).unwrap();

        assert!(!install.join("old.txt").exists());
        assert!(!install.join("assets").join("old.png").exists());
        assert_eq!(
            fs::read(install.join("assets").join("new.png")).unwrap(),
            b"new"
        );
        assert_eq!(fs::read(install.join("new.txt")).unwrap(), b"new");
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn incremental_plan_rejects_escaping_paths() {
        let temp = temp_dir("escape");
        let install = temp.join("install");
        let extract = temp.join("AALC.nsh");
        fs::create_dir_all(&install).unwrap();
        fs::create_dir_all(&extract).unwrap();
        fs::write(
            extract.join("changes.json"),
            r#"{"deleted":["../outside"]}"#,
        )
        .unwrap();

        assert!(load_incremental_plan(&extract, &temp, &install).is_err());
        let _ = fs::remove_dir_all(&temp);
    }

    #[test]
    fn cover_replaces_images_and_keeps_user_files() {
        let temp = temp_dir("cover");
        let install = temp.join("install");
        let extract = temp.join("AALC.nsh");
        fs::create_dir_all(install.join("assets").join("images")).unwrap();
        fs::create_dir_all(extract.join("assets").join("images")).unwrap();
        fs::write(install.join("config.yaml"), b"user").unwrap();
        fs::write(
            install.join("assets").join("images").join("old.png"),
            b"old",
        )
        .unwrap();
        fs::write(
            extract.join("assets").join("images").join("new.png"),
            b"new",
        )
        .unwrap();
        fs::write(extract.join("README.md"), b"readme").unwrap();
        fs::write(extract.join("release.json"), b"{}").unwrap();

        let mut log = logger(&temp);
        cover(&extract, &install, &mut log).unwrap();

        assert!(!install
            .join("assets")
            .join("images")
            .join("old.png")
            .exists());
        assert_eq!(
            fs::read(install.join("assets").join("images").join("new.png")).unwrap(),
            b"new"
        );
        assert_eq!(fs::read(install.join("config.yaml")).unwrap(), b"user");
        assert_eq!(fs::read(install.join("README.md")).unwrap(), b"readme");
        let _ = fs::remove_dir_all(&temp);
    }
}
