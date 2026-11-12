//! Starting and stopping the optional Python sidecar.
//!
//! The sidecar prints one JSON ready line on stdout, then serves the loopback
//! WebSocket. The guard closes stdin first (the sidecar's parent-death signal)
//! before falling back to `kill`.

use std::collections::VecDeque;
use std::io::{BufRead, BufReader};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::time::{Duration, Instant};
use std::{env, process};

use serde_json::Value;

pub(super) struct SidecarGuard {
    pub(super) child: Mutex<Child>,
    /// PID reported by the ready line. On Windows a venv launcher can spawn
    /// the real interpreter as a child in source mode; this is the process
    /// that owns the WebSocket and must be terminated as a last resort.
    pub(super) reported_pid: u32,
}

impl SidecarGuard {
    /// Whether the launched process is still running. A failure to inspect the
    /// child is treated as alive: killing a possibly-live sidecar is worse than
    /// letting the caller retry the connection.
    pub(super) fn is_alive(&self) -> bool {
        let Ok(mut child) = self.child.lock() else {
            return true;
        };
        match child.try_wait() {
            Ok(Some(_)) => false,
            Ok(None) => true,
            Err(_) => true,
        }
    }

    /// Wait for the sidecar to exit without taking ownership of the child.
    pub(super) fn wait_timeout(&self, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        loop {
            let Ok(mut child) = self.child.lock() else {
                return true;
            };
            match child.try_wait() {
                Ok(Some(_)) => return true,
                Ok(None) => {}
                Err(_) => return true,
            }
            drop(child);
            if Instant::now() >= deadline {
                return false;
            }
            thread::sleep(Duration::from_millis(25));
        }
    }
}

impl Drop for SidecarGuard {
    fn drop(&mut self) {
        if let Ok(mut child) = self.child.lock() {
            // Closing the parent-side stdin pipe gives a still-running sidecar
            // the same parent-death signal it would get if the client crashed.
            let _ = child.stdin.take();
            let deadline = Instant::now() + Duration::from_secs(2);
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => return,
                    Ok(None) => {}
                    Err(_) => break,
                }
                if Instant::now() >= deadline {
                    break;
                }
                thread::sleep(Duration::from_millis(25));
            }
            let _ = child.kill();
            let _ = child.wait();
            drop(child);
            kill_process_tree(self.reported_pid);
        }
    }
}

fn kill_process_tree(pid: u32) {
    if pid == 0 {
        return;
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .creation_flags(CREATE_NO_WINDOW)
            .status();
    }
    #[cfg(not(windows))]
    {
        let _ = pid;
    }
}

#[derive(Clone, Default)]
struct StderrTail(Arc<Mutex<VecDeque<String>>>);

impl StderrTail {
    fn push(&self, line: String) {
        let Ok(mut lines) = self.0.lock() else {
            return;
        };
        if lines.len() >= 40 {
            lines.pop_front();
        }
        lines.push_back(line);
    }

    fn snapshot(&self) -> Vec<String> {
        self.0
            .lock()
            .map(|lines| lines.iter().cloned().collect())
            .unwrap_or_default()
    }
}

fn spawn_stderr_drain(stderr: process::ChildStderr, tail: StderrTail) {
    let _ = thread::Builder::new()
        .name("AhabSidecarStderr".to_owned())
        .spawn(move || {
            for line in BufReader::new(stderr).lines() {
                let Ok(line) = line else {
                    break;
                };
                tail.push(line);
            }
        });
}

fn with_stderr_tail(message: String, tail: &StderrTail) -> String {
    let lines = tail.snapshot();
    if lines.is_empty() {
        message
    } else {
        format!("{message}\nsidecar stderr:\n{}", lines.join("\n"))
    }
}

pub(super) fn random_token() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|error| format!("生成 sidecar token 失败：{error}"))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub(super) fn parse_loopback_address(url: &str) -> Result<SocketAddr, String> {
    let authority = url
        .strip_prefix("ws://")
        .ok_or_else(|| "sidecar URL 必须使用 ws://".to_owned())?
        .split('/')
        .next()
        .unwrap_or_default();
    let host_port = authority.split('?').next().unwrap_or_default().to_owned();
    let address: SocketAddr = host_port
        .parse()
        .map_err(|error| format!("无效的 sidecar 地址：{error}"))?;
    if !address.ip().is_loopback() {
        return Err("sidecar 只允许连接 loopback 地址".to_owned());
    }
    Ok(address)
}

pub(super) fn spawn_sidecar() -> Result<(Arc<SidecarGuard>, u16, String, Option<u64>), String> {
    let token = random_token()?;
    let (mut command, working_directory) = if let Ok(executable) = env::var("AHAB_BACKEND_EXE") {
        let path = PathBuf::from(executable);
        let directory = path.parent().map(Path::to_path_buf);
        (Command::new(path), directory)
    } else if let Some(executable) = find_backend_executable() {
        let directory = executable.parent().map(Path::to_path_buf);
        (Command::new(executable), directory)
    } else {
        let script = find_backend_script()?;
        let directory = script.parent().map(Path::to_path_buf);
        let mut command = Command::new(find_python(directory.as_deref()));
        command.arg("-u").arg(&script);
        (command, directory)
    };
    if let Some(directory) = working_directory {
        command.current_dir(directory);
    }
    command
        .arg("--host")
        .arg("127.0.0.1")
        .arg("--port")
        .arg("0")
        .env("AHAB_BACKEND_TOKEN", &token)
        .arg("--parent-pid")
        .arg(process::id().to_string())
        .arg("--watch-stdin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = command
        .spawn()
        .map_err(|error| format!("启动 Python sidecar 失败：{error}"))?;
    let stderr_tail = StderrTail::default();
    if let Some(stderr) = child.stderr.take() {
        spawn_stderr_drain(stderr, stderr_tail.clone());
    }
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "无法读取 Python sidecar 启动信息".to_owned())?;
    let ready = match read_ready_line(stdout) {
        Ok(ready) => ready,
        Err(error) => {
            terminate_child(&mut child);
            return Err(with_stderr_tail(error, &stderr_tail));
        }
    };
    let Some(port) = ready.get("port").and_then(Value::as_u64) else {
        terminate_child(&mut child);
        return Err(with_stderr_tail(
            "sidecar 启动信息缺少端口".to_owned(),
            &stderr_tail,
        ));
    };
    if port == 0 || port > u16::MAX as u64 {
        terminate_child(&mut child);
        return Err(with_stderr_tail(
            format!("sidecar 返回了无效端口：{port}"),
            &stderr_tail,
        ));
    }
    if ready.get("ready") != Some(&Value::Bool(true)) {
        terminate_child(&mut child);
        return Err(with_stderr_tail(
            "sidecar 未报告 ready".to_owned(),
            &stderr_tail,
        ));
    }
    let reported_pid = ready
        .get("pid")
        .and_then(Value::as_u64)
        .and_then(|pid| u32::try_from(pid).ok())
        .unwrap_or(0);
    Ok((
        Arc::new(SidecarGuard {
            child: Mutex::new(child),
            reported_pid,
        }),
        port as u16,
        token,
        ready.get("startupMs").and_then(Value::as_u64),
    ))
}

fn terminate_child(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn read_ready_line(stdout: impl std::io::Read + Send + 'static) -> Result<Value, String> {
    let (sender, receiver) = mpsc::channel();
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        let mut line = String::new();
        let mut last_lines = Vec::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) => {
                    let context = if last_lines.is_empty() {
                        "sidecar 进程在输出就绪前退出（stdout 为空，请检查 Python 依赖）".to_owned()
                    } else {
                        format!(
                            "sidecar 进程已退出，未检测到就绪 JSON。输出内容：\n{}",
                            last_lines.join("\n")
                        )
                    };
                    let _ = sender.send(Err(context));
                    break;
                }
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    if last_lines.len() < 10 {
                        last_lines.push(trimmed.to_owned());
                    }
                    if let Ok(value) = serde_json::from_str::<Value>(trimmed)
                        && (value.get("ready") == Some(&Value::Bool(true))
                            || value.get("port").is_some())
                    {
                        let _ = sender.send(Ok(value));
                        break;
                    }
                }
                Err(error) => {
                    let _ = sender.send(Err(format!("读取 sidecar 启动信息失败：{error}")));
                    break;
                }
            }
        }
    });
    receiver
        .recv_timeout(Duration::from_secs(20))
        .map_err(|error| format!("等待 sidecar 启动超时：{error}"))?
}

/// The Python that runs `main_backend.py` in development.
///
/// Search the script's directory (the repo root) and the working directory for
/// a `.venv`, so `nsh run nsh-app` from any subdirectory still finds it. A
/// packaged app uses `AHAB_BACKEND_EXE` or the staged backend beside the exe.
fn find_python(script_directory: Option<&Path>) -> PathBuf {
    if let Some(python) = env::var_os("AHAB_PYTHON") {
        return PathBuf::from(python);
    }
    let mut roots: Vec<PathBuf> = script_directory
        .map(Path::to_path_buf)
        .into_iter()
        .collect();
    if let Ok(current) = env::current_dir() {
        roots.push(current);
    }
    if let Some(exe) = env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
    {
        roots.push(exe);
    }
    for root in roots {
        for ancestor in root.ancestors().take(5) {
            let candidate = venv_python(ancestor);
            if candidate.is_file() {
                return candidate.canonicalize().unwrap_or(candidate);
            }
        }
    }
    PathBuf::from("python")
}

#[cfg(windows)]
fn venv_python(root: &Path) -> PathBuf {
    root.join(".venv").join("Scripts").join("python.exe")
}

#[cfg(not(windows))]
fn venv_python(root: &Path) -> PathBuf {
    root.join(".venv").join("bin").join("python")
}

fn find_backend_script() -> Result<PathBuf, String> {
    if let Some(script) = env::var_os("AHAB_BACKEND_SCRIPT") {
        let path = PathBuf::from(script);
        if path.is_file() {
            return Ok(path);
        }
    }
    if let Some(path) = search_ancestors("main_backend.py") {
        return Ok(path);
    }
    Err("找不到 main_backend.py，请设置 AHAB_BACKEND_SCRIPT".to_owned())
}

fn find_backend_executable() -> Option<PathBuf> {
    let roots = [
        env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf)),
        env::current_dir().ok(),
    ];
    roots
        .into_iter()
        .flatten()
        .flat_map(|root| backend_executable_candidates(&root))
        .find(|path| path.is_file())
}

fn backend_executable_candidates(root: &Path) -> [PathBuf; 2] {
    // Release builds stage the onedir backend under `backend/`. Keep the
    // root-level lookup last so a leftover one-file executable from an older
    // release cannot shadow the staged bundle during an incremental update.
    [
        root.join("backend").join("AALC Backend.exe"),
        root.join("AALC Backend.exe"),
    ]
}

/// Look for `name` in the working directory and up to four ancestors (repo
/// root when the app runs from `nsh-app/` or `nsh-app/native/`).
fn search_ancestors(name: &str) -> Option<PathBuf> {
    let current = env::current_dir().ok()?;
    for ancestor in current.ancestors().take(5) {
        let candidate = ancestor.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_address_parses_only_local_ws_urls() {
        let address = parse_loopback_address("ws://127.0.0.1:1234/path").expect("loopback");
        assert_eq!(address.port(), 1234);
        assert!(parse_loopback_address("wss://127.0.0.1:1234").is_err());
        assert!(parse_loopback_address("ws://example.com:1234").is_err());
        assert!(parse_loopback_address("ws://127.0.0.1").is_err());
    }

    #[test]
    fn token_is_32_random_bytes_in_hex() {
        let token = random_token().expect("random token");
        assert_eq!(token.len(), 64);
        assert!(token.chars().all(|character| character.is_ascii_hexdigit()));
        assert_ne!(token, random_token().expect("second token"));
    }
}
