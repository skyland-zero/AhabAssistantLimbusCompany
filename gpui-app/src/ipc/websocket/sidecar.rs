use super::*;

pub(super) struct SidecarGuard {
    pub(super) child: Mutex<Child>,
}

impl Drop for SidecarGuard {
    fn drop(&mut self) {
        if let Ok(mut child) = self.child.lock() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
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

pub(super) fn spawn_sidecar() -> Result<(Arc<SidecarGuard>, u16, String), String> {
    let token = format!(
        "{:x}-{:x}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|error| error.to_string())?
            .as_nanos()
    );
    let mut command = if let Ok(executable) = env::var("AHAB_BACKEND_EXE") {
        Command::new(executable)
    } else {
        let mut command = Command::new(find_python());
        command.arg("-u").arg(find_backend_script()?);
        command
    };
    command
        .arg("--host")
        .arg("127.0.0.1")
        .arg("--port")
        .arg("0")
        .arg("--token")
        .arg(&token)
        .arg("--parent-pid")
        .arg(std::process::id().to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = command
        .spawn()
        .map_err(|error| format!("启动 Python sidecar 失败：{error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "无法读取 Python sidecar 启动信息".to_owned())?;
    let ready = match read_ready_line(stdout) {
        Ok(ready) => ready,
        Err(error) => {
            terminate_child(&mut child);
            return Err(error);
        }
    };
    let Some(port) = ready.get("port").and_then(Value::as_u64) else {
        terminate_child(&mut child);
        return Err("sidecar 启动信息缺少端口".to_owned());
    };
    if port == 0 || port > u16::MAX as u64 {
        terminate_child(&mut child);
        return Err(format!("sidecar 返回了无效端口：{port}"));
    }
    if ready.get("ready") != Some(&Value::Bool(true)) {
        terminate_child(&mut child);
        return Err("sidecar 未报告 ready".to_owned());
    }
    Ok((
        Arc::new(SidecarGuard {
            child: Mutex::new(child),
        }),
        port as u16,
        token,
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
            return path;
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
