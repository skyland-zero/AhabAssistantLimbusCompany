//! Path normalisation and containment checks for update payloads.

use std::path::{Component, Path, PathBuf};

use crate::{Result, UpdaterError};

/// Normalise a path from `changes.json` or a manifest.
///
/// The returned path is always relative, uses at most normal components and
/// has the archive root prefix removed when present.  Absolute paths, drive
/// letters, UNC paths, parent traversal and NUL bytes are rejected.
pub fn normalize_relative_path(raw: &str, archive_root: Option<&str>) -> Result<PathBuf> {
    if raw.is_empty() {
        return Err(UpdaterError::manifest("更新清单包含空路径"));
    }
    if raw.contains('\0') {
        return Err(UpdaterError::manifest("更新清单包含 NUL 字符"));
    }

    let portable = raw.replace('\\', "/");
    if portable.starts_with('/') {
        return Err(UpdaterError::manifest(format!(
            "更新清单包含绝对路径：{raw}"
        )));
    }
    let bytes = portable.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' {
        return Err(UpdaterError::manifest(format!(
            "更新清单包含盘符路径：{raw}"
        )));
    }

    let mut parts: Vec<&str> = Vec::new();
    for part in portable.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return Err(UpdaterError::manifest(format!(
                "更新清单包含父目录引用：{raw}"
            )));
        }
        if part.contains(':') {
            return Err(UpdaterError::manifest(format!(
                "更新清单包含非法路径组件：{raw}"
            )));
        }
        parts.push(part);
    }

    if let Some(root) = archive_root {
        if !root.is_empty()
            && parts
                .first()
                .is_some_and(|part| part.eq_ignore_ascii_case(root))
        {
            parts.remove(0);
        }
    }

    if parts.is_empty() {
        return Err(UpdaterError::manifest(format!(
            "更新清单不能指向根目录：{raw}"
        )));
    }

    Ok(parts.iter().collect())
}

/// Resolve `relative` under `root` and reject paths that escape it.
pub fn resolve_within_root(root: &Path, relative: &Path) -> Result<PathBuf> {
    if relative.is_absolute() {
        return Err(UpdaterError::manifest(format!(
            "路径必须是相对路径：{}",
            relative.display()
        )));
    }
    for component in relative.components() {
        match component {
            Component::Normal(_) => {}
            _ => {
                return Err(UpdaterError::manifest(format!(
                    "路径包含非法组件：{}",
                    relative.display()
                )));
            }
        }
    }

    let root = root.canonicalize().map_err(|error| {
        UpdaterError::install(format!("无法解析安装目录 {}：{error}", root.display()))
    })?;
    let candidate = root.join(relative);
    if !candidate.starts_with(&root) {
        return Err(UpdaterError::manifest(format!(
            "路径逃离可信根目录：{}",
            relative.display()
        )));
    }
    reject_reparse_points(&root, &candidate)?;
    Ok(candidate)
}

/// Reject symlinks, junctions and other reparse points between `root` and
/// `candidate`.  The install directory is user-writable, so a link planted
/// there must never redirect an update outside the application tree.
fn reject_reparse_points(root: &Path, candidate: &Path) -> Result<()> {
    let relative = candidate.strip_prefix(root).map_err(|_| {
        UpdaterError::manifest(format!("路径逃离可信根目录：{}", candidate.display()))
    })?;
    let mut current = root.to_path_buf();
    for component in relative.components() {
        current.push(component);
        if let Ok(metadata) = std::fs::symlink_metadata(&current) {
            if is_reparse_point(&metadata) {
                return Err(UpdaterError::manifest(format!(
                    "路径经过链接或重解析点：{}",
                    current.display()
                )));
            }
        }
    }
    Ok(())
}

#[cfg(windows)]
pub(crate) fn is_reparse_point(metadata: &std::fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;

    // FILE_ATTRIBUTE_REPARSE_POINT covers symlinks and junctions.
    metadata.file_attributes() & 0x0400 != 0
}

#[cfg(not(windows))]
pub(crate) fn is_reparse_point(metadata: &std::fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

/// Lower-case, backslash-normalised form used for process-path comparison.
pub fn normalize_for_compare(path: &Path) -> String {
    path.to_string_lossy()
        .replace('/', "\\")
        .trim_end_matches('\\')
        .to_lowercase()
}

/// Whether `path` is the install root itself or lives underneath it.
pub fn is_within_install_root(root: &Path, path: &Path) -> bool {
    let root = normalize_for_compare(root);
    let path = normalize_for_compare(path);
    path == root || path.starts_with(&format!("{root}\\"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths_are_normalised() {
        assert_eq!(
            normalize_relative_path("./assets/images/a.png", None).unwrap(),
            PathBuf::from("assets/images/a.png")
        );
        assert_eq!(
            normalize_relative_path("AALC.nsh/backend/a.exe", Some("AALC.nsh")).unwrap(),
            PathBuf::from("backend/a.exe")
        );
        assert_eq!(
            normalize_relative_path("assets\\images\\a.png", None).unwrap(),
            PathBuf::from("assets/images/a.png")
        );
    }

    #[test]
    fn escaping_paths_are_rejected() {
        for value in [
            "",
            "..",
            "../outside",
            "assets/../../outside",
            "/absolute",
            "C:/windows",
            "c:\\windows",
            "assets/\0bad",
        ] {
            assert!(
                normalize_relative_path(value, None).is_err(),
                "expected rejection for {value:?}"
            );
        }
    }

    #[test]
    fn archive_root_is_stripped_case_insensitively() {
        assert_eq!(
            normalize_relative_path("AALC.NSH/assets/a", Some("AALC.nsh")).unwrap(),
            PathBuf::from("assets/a")
        );
    }

    #[test]
    fn resolved_paths_stay_under_the_root() {
        let root = unique_temp_dir("paths-root");
        std::fs::create_dir_all(root.join("assets")).unwrap();
        assert_eq!(
            resolve_within_root(&root, Path::new("assets/a.png")).unwrap(),
            root.canonicalize().unwrap().join("assets/a.png")
        );
        assert!(resolve_within_root(&root, Path::new("../outside")).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    pub(crate) fn unique_temp_dir(label: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "aalc-updater-test-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|value| value.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }
}
