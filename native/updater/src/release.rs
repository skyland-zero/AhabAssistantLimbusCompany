//! Minimal `release.json` reader used for layout migration.

use std::fs;
use std::path::Path;

use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseInfo {
    /// The `runner.entry` recorded by the release.
    pub runner_entry: String,
    /// The release launches its Runner through the sidecar executable with a
    /// role flag, so the old standalone `AALCRunner.exe` is stale.
    pub merged_runner: bool,
    /// The release reuses one `_internal` runtime.  A standalone `runner/`
    /// bundle left by an older separate layout is stale.
    pub shared_runtime: bool,
}

/// Read the release manifest from an extracted payload.
///
/// A missing or malformed manifest is not an update error by itself: older
/// archives did not describe the runner layout, and keeping their files is
/// safer than guessing.  The caller treats `None` as "no migration".
pub fn read_release_info(extract_root: &Path) -> Option<ReleaseInfo> {
    let bytes = fs::read(extract_root.join("release.json")).ok()?;
    let payload: Value = serde_json::from_slice(&bytes).ok()?;
    let runner = payload.get("runner")?;
    let entry = runner
        .get("entry")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())?
        .to_string();
    let merged_runner = entry
        .replace('\\', "/")
        .eq_ignore_ascii_case("backend/AALC Backend.exe");
    let shared_runtime = payload
        .get("runtime")
        .and_then(Value::as_str)
        .is_some_and(|runtime| runtime.eq_ignore_ascii_case("shared"));

    Some(ReleaseInfo {
        runner_entry: entry,
        merged_runner,
        shared_runtime,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "aalc-updater-release-{}-{}",
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
    fn current_shared_runner_is_not_migrated() {
        let root = temp_dir();
        fs::write(
            root.join("release.json"),
            r#"{"runtime":"shared","runner":{"entry":"backend/AALCRunner.exe","args":[]}}"#,
        )
        .unwrap();
        assert_eq!(
            read_release_info(&root),
            Some(ReleaseInfo {
                runner_entry: "backend/AALCRunner.exe".to_string(),
                merged_runner: false,
                shared_runtime: true
            })
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn separate_runner_is_not_migrated() {
        let root = temp_dir();
        fs::write(
            root.join("release.json"),
            r#"{"runtime":"separate","runner":{"entry":"runner/AALCRunner.exe","args":[]}}"#,
        )
        .unwrap();
        assert_eq!(
            read_release_info(&root),
            Some(ReleaseInfo {
                runner_entry: "runner/AALCRunner.exe".to_string(),
                merged_runner: false,
                shared_runtime: false
            })
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn merged_runner_is_detected_from_entry() {
        let root = temp_dir();
        fs::write(
            root.join("release.json"),
            r#"{"runtime":"shared","runner":{"entry":"backend/AALC Backend.exe","args":["--execution-runner"]}}"#,
        )
        .unwrap();
        assert_eq!(
            read_release_info(&root),
            Some(ReleaseInfo {
                runner_entry: "backend/AALC Backend.exe".to_string(),
                merged_runner: true,
                shared_runtime: true
            })
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn malformed_or_missing_manifest_keeps_the_old_layout() {
        let root = temp_dir();
        assert_eq!(read_release_info(&root), None);
        fs::write(root.join("release.json"), b"not json").unwrap();
        assert_eq!(read_release_info(&root), None);
        fs::write(root.join("release.json"), r#"{"runner":{"args":[]}}"#).unwrap();
        assert_eq!(read_release_info(&root), None);
        let _ = fs::remove_dir_all(&root);
    }
}
