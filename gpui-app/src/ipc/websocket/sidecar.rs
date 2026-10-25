use std::time::Instant;

use super::*;

pub(super) struct SidecarGuard {
    pub(super) child: Mutex<Child>,
    /// PID reported by the ready line.  On Windows a venv launcher can spawn
    /// the real interpreter as a child in source mode; this is the process
    /// that owns the WebSocket and must be terminated as a last resort.
    pub(super) reported_pid: u32,
}

impl SidecarGuard {
    /// Whether the launched process is still running.  A failure to inspect the
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
            // the same parent-death signal it would get if GPUI crashed.  Give
            // it a short bounded chance to finish `application.close()` before
            // falling back to TerminateProcess.
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

fn spawn_stderr_drain(stderr: std::process::ChildStderr, tail: StderrTail) {
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
        let mut command = Command::new(find_python());
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
        .arg(std::process::id().to_string())
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

fn find_python() -> PathBuf {
    if let Some(python) = env::var_os("AHAB_PYTHON") {
        return PathBuf::from(python);
    }
    let venv_candidates = [
        #[cfg(windows)]
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(".venv")
            .join("Scripts")
            .join("python.exe"),
        #[cfg(not(windows))]
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(".venv")
            .join("bin")
            .join("python"),
        #[cfg(windows)]
        PathBuf::from(".venv").join("Scripts").join("python.exe"),
        #[cfg(not(windows))]
        PathBuf::from(".venv").join("bin").join("python"),
    ];
    for path in venv_candidates {
        if path.is_file() {
            // Normalize the `..` in the development path.  Python compares
            // `sys.prefix` against the real venv path and emits a confusing
            // warning when the launcher hands it an unnormalized spelling.
            return path.canonicalize().unwrap_or(path);
        }
    }
    PathBuf::from("python")
}

fn find_backend_script() -> Result<PathBuf, String> {
    let candidates = [
        env::var_os("AHAB_BACKEND_SCRIPT").map(PathBuf::from),
        Some(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("main_backend.py"),
        ),
        env::current_dir()
            .ok()
            .map(|directory| directory.join("main_backend.py")),
    ];
    candidates
        .into_iter()
        .flatten()
        .map(|path| if path.is_file() { Ok(path) } else { Err(path) })
        .find_map(Result::ok)
        .ok_or_else(|| "找不到 main_backend.py，请设置 AHAB_BACKEND_SCRIPT".to_owned())
}

pub(super) fn backend_executable_candidates(root: &Path) -> [PathBuf; 2] {
    // Release builds stage the onedir backend under ``backend/``.  Keep the
    // root-level lookup last so a leftover one-file executable from an older
    // release cannot shadow the staged bundle during an incremental update.
    [
        root.join("backend").join("AALC Backend.exe"),
        root.join("AALC Backend.exe"),
    ]
}

fn find_backend_executable() -> Option<PathBuf> {
    let roots = [
        std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(Path::to_path_buf)),
        std::env::current_dir().ok(),
    ];
    roots
        .into_iter()
        .flatten()
        .flat_map(|root| backend_executable_candidates(&root))
        .find(|path| path.is_file())
}
