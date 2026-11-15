//! Command-line parsing for the native updater.

use std::ffi::OsString;

use crate::{Result, UpdaterError};

/// One invocation of the updater.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandLine {
    /// Apply an already-downloaded archive in the current process.
    Normal(String),
    /// Re-use the extraction performed by the previous updater process.
    Apply(String),
    Help,
    Version,
}

pub fn parse() -> Result<CommandLine> {
    parse_from(std::env::args_os().skip(1).collect())
}

pub fn parse_from(args: Vec<OsString>) -> Result<CommandLine> {
    let mut values = args.into_iter();
    let Some(first) = values.next() else {
        return Err(UpdaterError::usage(
            "缺少更新包文件名。用法：AALC Updater.exe <文件名>",
        ));
    };
    let first = first.to_string_lossy().to_string();
    match first.as_str() {
        "-h" | "--help" => Ok(CommandLine::Help),
        "-V" | "--version" => Ok(CommandLine::Version),
        "--apply-update" => {
            let Some(file_name) = values.next() else {
                return Err(UpdaterError::usage("--apply-update 缺少文件名"));
            };
            if values.next().is_some() {
                return Err(UpdaterError::usage("--apply-update 只接受一个文件名"));
            }
            let file_name = file_name.to_string_lossy().to_string();
            if file_name.trim().is_empty() {
                return Err(UpdaterError::usage("更新包文件名不能为空"));
            }
            Ok(CommandLine::Apply(file_name))
        }
        value if value.starts_with('-') => Err(UpdaterError::usage(format!(
            "未知参数：{value}。用法：AALC Updater.exe <文件名>"
        ))),
        value => {
            if values.next().is_some() {
                return Err(UpdaterError::usage("只接受一个更新包文件名"));
            }
            if value.trim().is_empty() {
                return Err(UpdaterError::usage("更新包文件名不能为空"));
            }
            Ok(CommandLine::Normal(value.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn strings(values: &[&str]) -> Vec<OsString> {
        values.iter().map(OsString::from).collect()
    }

    #[test]
    fn normal_archive_is_the_default() {
        assert_eq!(
            parse_from(strings(&["AALC.nsh_1.2.3.7z"])).unwrap(),
            CommandLine::Normal("AALC.nsh_1.2.3.7z".to_string())
        );
    }

    #[test]
    fn apply_update_takes_one_file() {
        assert_eq!(
            parse_from(strings(&["--apply-update", "AALC.nsh_1.2.3.7z"])).unwrap(),
            CommandLine::Apply("AALC.nsh_1.2.3.7z".to_string())
        );
    }

    #[test]
    fn missing_file_is_a_usage_error() {
        assert!(parse_from(strings(&[])).is_err());
        assert!(parse_from(strings(&["--apply-update"])).is_err());
        assert!(parse_from(strings(&["--unknown"])).is_err());
        assert!(parse_from(strings(&["one", "two"])).is_err());
    }

    #[test]
    fn help_and_version_are_recognised() {
        assert_eq!(parse_from(strings(&["--help"])).unwrap(), CommandLine::Help);
        assert_eq!(parse_from(strings(&["-V"])).unwrap(), CommandLine::Version);
    }
}
