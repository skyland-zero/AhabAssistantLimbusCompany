//! Native updater for AALC release archives.
//!
//! The updater is a self-contained Windows executable.  It verifies a detached
//! Ed25519 manifest, extracts the archive with the bundled 7-Zip runtime,
//! stops the application and its helpers, replaces the install tree and
//! restarts the frontend.  It intentionally has no Python dependency so the
//! release does not ship a second interpreter.

pub mod archive;
pub mod args;
pub mod install;
pub mod keys;
pub mod log;
pub mod manifest;
pub mod migration;
pub mod paths;
pub mod processes;
pub mod release;

use std::env;
use std::ffi::OsString;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use args::CommandLine;

pub const FRONTEND_NAME: &str = "AALC.nsh.exe";
pub const UPDATER_NAME: &str = "AALC Updater.exe";
pub const APPLY_UPDATER_NAME: &str = "AALC Updater.apply.exe";
/// The flag a merged runtime uses to start its Runner role.
pub const RUNNER_ROLE_FLAG: &str = "--execution-runner";

#[derive(Debug)]
pub struct UpdaterError {
    pub code: i32,
    pub message: String,
}

impl UpdaterError {
    pub fn new(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn usage(message: impl Into<String>) -> Self {
        Self::new(2, format!("参数错误：{}", message.into()))
    }

    pub fn signature(message: impl Into<String>) -> Self {
        Self::new(3, format!("更新包校验失败：{}", message.into()))
    }

    pub fn manifest(message: impl Into<String>) -> Self {
        Self::new(3, format!("更新清单无效：{}", message.into()))
    }

    pub fn archive(message: impl Into<String>) -> Self {
        Self::new(4, format!("解压失败：{}", message.into()))
    }

    pub fn install(message: impl Into<String>) -> Self {
        Self::new(5, format!("安装失败：{}", message.into()))
    }

    pub fn process(message: impl Into<String>) -> Self {
        Self::new(6, format!("终止进程失败：{}", message.into()))
    }

    pub fn restart(message: impl Into<String>) -> Self {
        Self::new(7, format!("重启失败：{}", message.into()))
    }
}

impl fmt::Display for UpdaterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for UpdaterError {}

impl From<std::io::Error> for UpdaterError {
    fn from(error: std::io::Error) -> Self {
        Self::new(1, format!("I/O 错误：{error}"))
    }
}

pub type Result<T> = std::result::Result<T, UpdaterError>;

pub fn run() -> Result<()> {
    match args::parse()? {
        CommandLine::Help => {
            print_help();
            Ok(())
        }
        CommandLine::Version => {
            let _ = writeln!(
                std::io::stdout(),
                "AALC Updater {}",
                env!("CARGO_PKG_VERSION")
            );
            Ok(())
        }
        CommandLine::Normal(file_name) => run_update(file_name, false),
        CommandLine::Apply(file_name) => run_update(file_name, true),
    }
}

fn run_update(file_name: String, apply_mode: bool) -> Result<()> {
    let (install_root, current_exe) = resolve_install_root()?;
    let temp = install_root.join("update_temp");
    fs::create_dir_all(&temp)?;
    let mut logger = log::Logger::new(&temp);
    logger.line(&format!(
        "AALC Updater {} starting (apply={apply_mode})",
        env!("CARGO_PKG_VERSION")
    ));

    // Always run from update_temp so the installed updater can be replaced
    // while this process is alive, matching the historical Python contract.
    let current_name = current_exe
        .file_name()
        .ok_or_else(|| UpdaterError::install("更新器文件名无效"))?;
    let staged = temp.join(current_name);
    if !same_path(&current_exe, &staged) {
        fs::copy(&current_exe, &staged).map_err(|error| {
            UpdaterError::install(format!("无法复制更新器到 update_temp：{error}"))
        })?;
        let args: Vec<OsString> = env::args_os().skip(1).collect();
        spawn_detached(&staged, &args, &install_root)?;
        logger.line("relaunched from update_temp");
        return Ok(());
    }

    let archive_name = Path::new(&file_name);
    if archive_name.file_name().and_then(|name| name.to_str()) != Some(file_name.as_str()) {
        return Err(UpdaterError::usage("更新包文件名必须是纯文件名"));
    }

    let archive_path = temp.join(&file_name);
    if !archive_path.is_file() {
        return Err(UpdaterError::install(format!(
            "更新包不存在：{}",
            archive_path.display()
        )));
    }
    let stem = archive_name
        .file_stem()
        .and_then(|stem| stem.to_str())
        .ok_or_else(|| UpdaterError::usage("更新包文件名缺少扩展名"))?;
    let manifest_path = temp.join(format!("{stem}.manifest.json"));
    let signature_path = temp.join(format!("{stem}.manifest.sig"));

    // Verification is deliberately before extraction and process termination.
    let public_keys = read_public_keys(&install_root);
    let verified = manifest::verify_files(
        &manifest_path,
        &signature_path,
        &archive_path,
        public_keys.as_deref(),
    )?;
    logger.line(&format!(
        "signature verified: {} ({})",
        verified.version, verified.key_id
    ));

    let extract_root = if apply_mode {
        match archive::existing_extract_root(&temp, &archive_path) {
            Some(root) => {
                logger.line(&format!("reusing extraction at {}", root.display()));
                root
            }
            None => extract_payload(&mut logger, &install_root, &temp, &archive_path)?,
        }
    } else {
        extract_payload(&mut logger, &install_root, &temp, &archive_path)?
    };

    let plan = install::load_incremental_plan(&extract_root, &temp, &install_root)?;
    if plan.is_some() {
        logger.line("incremental update plan loaded and validated");
    }

    if !apply_mode
        && handoff_updater(
            &extract_root,
            &install_root,
            &temp,
            &file_name,
            &current_exe,
            &mut logger,
        )?
    {
        return Ok(());
    }

    processes::terminate_install_processes(&install_root)?;
    logger.line("application processes stopped");

    if let Some(plan) = plan {
        install::apply_incremental(&plan, &mut logger)?;
        logger.line("incremental update applied");
    } else {
        install::cover(&extract_root, &install_root, &mut logger)?;
        logger.line("full cover applied");
    }
    migration::refresh_release_manifest(&extract_root, &install_root, &mut logger);

    migration::remove_legacy_sidecar(&install_root, &mut logger);
    migration::remove_legacy_runner_layout(&install_root, &extract_root, &mut logger);
    migration::cleanup(
        &temp,
        &extract_root,
        &archive_path,
        &manifest_path,
        &signature_path,
        &mut logger,
    );

    if env::var("AALC_UPDATER_NO_RESTART").ok().as_deref() == Some("1") {
        logger.line("restart skipped by AALC_UPDATER_NO_RESTART");
    } else {
        restart_frontend(&install_root)?;
        logger.line("frontend restarted");
    }

    if env::var("AALC_UPDATER_PAUSE").ok().as_deref() == Some("1") {
        pause_for_enter();
    }

    Ok(())
}

fn resolve_install_root() -> Result<(PathBuf, PathBuf)> {
    let current_exe = env::current_exe()
        .map_err(|error| UpdaterError::install(format!("无法定位更新器：{error}")))?;
    let current_exe = current_exe.canonicalize().unwrap_or(current_exe);
    let directory = current_exe
        .parent()
        .ok_or_else(|| UpdaterError::install("更新器路径无效"))?;

    let mut candidates = Vec::new();
    if directory
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("update_temp"))
    {
        if let Some(parent) = directory.parent() {
            candidates.push(parent.to_path_buf());
        }
    }
    candidates.push(directory.to_path_buf());
    if let Ok(cwd) = env::current_dir() {
        candidates.push(cwd);
    }

    for candidate in candidates {
        if candidate.join(FRONTEND_NAME).is_file()
            || candidate.join("backend").join("AALC Backend.exe").is_file()
        {
            let root = candidate.canonicalize().unwrap_or(candidate);
            return Ok((root, current_exe));
        }
    }

    Err(UpdaterError::usage(
        "无法定位安装目录（缺少 AALC.nsh.exe 或 backend/AALC Backend.exe）",
    ))
}

fn read_public_keys(install_root: &Path) -> Option<String> {
    fs::read_to_string(install_root.join("update_public_keys.json")).ok()
}

fn extract_payload(
    logger: &mut log::Logger,
    install_root: &Path,
    temp: &Path,
    archive_path: &Path,
) -> Result<PathBuf> {
    let seven_zip = install_root.join("assets").join("binary").join("7za.exe");
    archive::reset_workspace(temp, archive_path);
    logger.line("extracting archive");
    archive::extract(&seven_zip, archive_path, temp)?;
    let root = archive::detect_extract_root(temp, archive_path)?;
    logger.line(&format!("extracted to {}", root.display()));
    Ok(root)
}

fn handoff_updater(
    extract_root: &Path,
    install_root: &Path,
    temp: &Path,
    file_name: &str,
    current_exe: &Path,
    logger: &mut log::Logger,
) -> Result<bool> {
    let candidate = extract_root.join(UPDATER_NAME);
    if !candidate.is_file() || same_path(&candidate, current_exe) {
        return Ok(false);
    }

    let staged = temp.join(APPLY_UPDATER_NAME);
    fs::copy(&candidate, &staged)
        .map_err(|error| UpdaterError::install(format!("无法复制新版本更新器：{error}")))?;
    spawn_detached(
        &staged,
        &[OsString::from("--apply-update"), OsString::from(file_name)],
        install_root,
    )?;
    logger.line("handed off to bundled updater");
    Ok(true)
}

fn spawn_detached(exe: &Path, args: &[OsString], cwd: &Path) -> Result<()> {
    let mut command = Command::new(exe);
    command.args(args).current_dir(cwd);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // DETACHED_PROCESS: the updater must survive the application it kills.
        command.creation_flags(0x0000_0008);
    }
    command
        .spawn()
        .map_err(|error| UpdaterError::install(format!("无法启动 {}：{error}", exe.display())))?;
    Ok(())
}

fn restart_frontend(install_root: &Path) -> Result<()> {
    let executable = install_root.join(FRONTEND_NAME);
    if !executable.is_file() {
        return Err(UpdaterError::restart(format!(
            "找不到前端程序：{}",
            executable.display()
        )));
    }
    let mut command = Command::new(&executable);
    command.current_dir(install_root);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0000_0008);
    }
    command
        .spawn()
        .map_err(|error| UpdaterError::restart(format!("无法启动前端程序：{error}")))?;
    Ok(())
}

fn pause_for_enter() {
    let _ = writeln!(std::io::stdout(), "按回车键退出...");
    let mut line = String::new();
    let _ = std::io::stdin().read_line(&mut line);
}

fn same_path(left: &Path, right: &Path) -> bool {
    let left = left.canonicalize().unwrap_or_else(|_| left.to_path_buf());
    let right = right.canonicalize().unwrap_or_else(|_| right.to_path_buf());
    if cfg!(windows) {
        let left = left.to_string_lossy();
        let right = right.to_string_lossy();
        left.eq_ignore_ascii_case(&right)
    } else {
        left == right
    }
}

fn print_help() {
    let _ = writeln!(
        std::io::stdout(),
        "AALC Updater {}\n\n用法：\n  AALC Updater.exe <更新包文件名>\n  AALC Updater.exe --apply-update <更新包文件名>\n\n环境变量：\n  AALC_UPDATE_REQUIRE_SIGNATURE  设为 0/false/no/off 可关闭强制签名\n  AALC_UPDATE_PUBLIC_KEYS        公钥映射 JSON（安装目录缺少 update_public_keys.json 时使用）\n  AALC_UPDATER_NO_RESTART        设为 1 时更新完成后不重启前端\n  AALC_UPDATER_PAUSE             设为 1 时退出前等待回车",
        env!("CARGO_PKG_VERSION")
    );
}

/// Show a fatal error to the user.  The log lives beside the downloaded
/// archive so a detached launch still leaves diagnostics.
pub fn report_fatal(error: &UpdaterError) {
    let message = format!("{}\n\n日志：update_temp/updater.log", error.message);
    let _ = writeln!(std::io::stderr(), "AALC Updater: {message}");
    append_fatal_log(&error.message);
    if env::var("AALC_UPDATER_NO_MESSAGE_BOX").ok().as_deref() != Some("1") {
        log::message_box("AALC Updater", &message, true);
    }
}

fn append_fatal_log(message: &str) {
    let mut candidates = Vec::new();
    if let Ok(cwd) = env::current_dir() {
        candidates.push(cwd.join("update_temp").join("updater.log"));
    }
    if let Ok(executable) = env::current_exe() {
        let executable = executable.canonicalize().unwrap_or(executable);
        if let Some(directory) = executable.parent() {
            candidates.push(directory.join("update_temp").join("updater.log"));
            if let Some(root) = directory.parent() {
                candidates.push(root.join("update_temp").join("updater.log"));
            }
        }
    }
    for path in candidates {
        let Some(parent) = path.parent() else {
            continue;
        };
        if !parent.is_dir() {
            continue;
        }
        let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(&path) else {
            continue;
        };
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0);
        let _ = writeln!(file, "[{timestamp}] FATAL {message}");
        let _ = file.flush();
        return;
    }
}
