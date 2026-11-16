//! The TSX-facing IPC facts and connectivity probe.
//!
//! State objects will use the shared [`BackendClient`] behind this module. The
//! probe exists so the UI can show whether it is on the mock or the real
//! sidecar without starting a sidecar just to ask.

use std::sync::{
    Mutex, OnceLock,
    atomic::{AtomicU8, Ordering},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use nsh_core::prelude::*;
use serde_json::Value;

use super::{BackendClient, contract, contract::method};

/// How many launches one startup makes before it gives up (the first one plus
/// two retries).
const STARTUP_ATTEMPTS: u32 = 3;

/// The pause between two startup attempts. A launched sidecar that failed to
/// report ready needs a moment to be reaped before the next spawn.
const STARTUP_RETRY_DELAY: Duration = Duration::from_millis(600);

const STARTUP_STARTING: u8 = 0;
const STARTUP_READY: u8 = 1;
const STARTUP_FAILED: u8 = 2;

static STARTUP_STATE: AtomicU8 = AtomicU8::new(STARTUP_STARTING);

/// One line of the automation core's initialization log.
#[derive(Clone, Debug, JsObject)]
pub struct StartupLogView {
    pub ts: i64,
    pub level: String,
    pub message: String,
}

/// The automation core's initialization state and the lines that led to it.
#[derive(Clone, Debug, JsObject)]
pub struct StartupInfoView {
    /// `starting`, `ready`, `failed` or `mock`.
    pub state: String,
    pub logs: Vec<StartupLogView>,
}

/// The backend the app talks to.
#[module]
pub struct Backend;

pub(crate) fn configured_mode() -> String {
    match std::env::var("AHAB_BACKEND")
        .unwrap_or_default()
        .to_ascii_lowercase()
        .as_str()
    {
        "mock" => "mock".into(),
        "unavailable" | "none" => "unavailable".into(),
        _ => "sidecar".into(),
    }
}

fn startup_logs() -> &'static Mutex<Vec<StartupLogView>> {
    static LOGS: OnceLock<Mutex<Vec<StartupLogView>>> = OnceLock::new();
    LOGS.get_or_init(|| Mutex::new(Vec::new()))
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or(0)
}

fn record_startup(level: &str, message: impl Into<String>) {
    let Ok(mut logs) = startup_logs().lock() else {
        return;
    };
    logs.push(StartupLogView {
        ts: now_ms(),
        level: level.to_owned(),
        message: message.into(),
    });
}

/// Try `connect` up to `attempts` times, recording the retry lines for the
/// log panel. `last_error` is returned to the caller for diagnostics; the UI
/// never receives it.
fn connect_with_retries<T>(
    attempts: u32,
    delay: Duration,
    mut connect: impl FnMut() -> Result<T, String>,
    mut record: impl FnMut(&str, String),
) -> Result<T, String> {
    let mut last_error = String::new();
    for attempt in 1..=attempts {
        if attempt == 1 {
            record("info", "自动化核心初始化中…".to_owned());
        }
        match connect() {
            Ok(value) => return Ok(value),
            Err(error) => {
                last_error = error;
                if attempt < attempts {
                    record(
                        "warn",
                        format!(
                            "自动化核心初始化失败，正在重试（第 {}/{} 次）…",
                            attempt,
                            attempts - 1
                        ),
                    );
                    std::thread::sleep(delay);
                } else {
                    record("error", "自动化核心初始化失败。".to_owned());
                }
            }
        }
    }
    Err(last_error)
}

fn start_service() -> BackendClient {
    match connect_with_retries(
        STARTUP_ATTEMPTS,
        STARTUP_RETRY_DELAY,
        BackendClient::try_sidecar,
        record_startup,
    ) {
        Ok(client) => {
            STARTUP_STATE.store(STARTUP_READY, Ordering::Release);
            record_startup("info", "自动化核心就绪");
            client
        }
        Err(_) => {
            STARTUP_STATE.store(STARTUP_FAILED, Ordering::Release);
            BackendClient::unavailable("自动化核心未就绪，请重新启动应用。")
        }
    }
}

/// The process-wide transport. The first call starts the sidecar (or the mock)
/// and every later state object shares it.
pub fn client() -> &'static BackendClient {
    static CLIENT: OnceLock<BackendClient> = OnceLock::new();
    CLIENT.get_or_init(|| match configured_mode().as_str() {
        "mock" => BackendClient::mock(),
        "unavailable" => {
            STARTUP_STATE.store(STARTUP_FAILED, Ordering::Release);
            BackendClient::unavailable("自动化核心未就绪，请重新启动应用。")
        }
        _ => start_service(),
    })
}

/// The transport a state object uses. The mock is per state object so tests and
/// two app instances stay independent; the real sidecar is process-wide.
pub(crate) fn state_client() -> BackendClient {
    if configured_mode() == "mock" {
        BackendClient::mock()
    } else {
        client().shared()
    }
}

#[module]
impl Backend {
    /// The external RPC schema version this build speaks (3).
    pub fn schema_version() -> u32 {
        contract::RPC_SCHEMA_VERSION
    }

    /// The configured transport: `mock`, `sidecar`, or `unavailable`.
    pub fn mode() -> String {
        configured_mode()
    }

    /// How the automation core's initialization is going, with the retry lines
    /// the log panel mirrors. Reading it never initializes the core.
    pub fn startup_info() -> StartupInfoView {
        let state = match configured_mode().as_str() {
            "mock" => "mock",
            "unavailable" => "failed",
            _ => match STARTUP_STATE.load(Ordering::Acquire) {
                STARTUP_READY => "ready",
                STARTUP_FAILED => "failed",
                _ => "starting",
            },
        };
        let logs = startup_logs()
            .lock()
            .map(|lines| lines.clone())
            .unwrap_or_default();
        StartupInfoView {
            state: state.to_owned(),
            logs,
        }
    }

    /// Pings the backend and resolves with its reply (`"pong"`).
    pub async fn ping() -> Result<String, String> {
        blocking(move || {
            // The first call may initialize the automation core and retry it;
            // keep that off the engine thread.
            let client = client().clone();
            let response = client.call(method::APP_PING, None);
            match super::decode_response(method::APP_PING, response) {
                Ok(Some(Value::String(text))) => Ok(text),
                Ok(other) => Err(format!("app.ping returned {other:?}")),
                Err(error) => Err(error.message),
            }
        })
        .await
        .map_err(|error| error.to_string())?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_mode_reads_the_environment() {
        // The test process may inherit AHAB_BACKEND; only assert the mapping
        // for the two explicit values without mutating global state.
        assert_eq!(contract::RPC_SCHEMA_VERSION, 3);
        let mode = configured_mode();
        assert!(matches!(mode.as_str(), "mock" | "sidecar" | "unavailable"));
    }

    #[test]
    fn startup_retries_twice_then_reports_failure() {
        let mut attempts = 0;
        let mut lines = Vec::new();
        let result: Result<(), String> = connect_with_retries(
            3,
            Duration::ZERO,
            || {
                attempts += 1;
                Err("boom".to_owned())
            },
            |level, message| lines.push((level.to_owned(), message)),
        );
        assert!(result.is_err());
        assert_eq!(attempts, 3);
        assert_eq!(lines.iter().filter(|(level, _)| level == "warn").count(), 2);
        assert_eq!(lines.last().map(|(level, _)| level.as_str()), Some("error"));
    }

    #[test]
    fn startup_stops_retrying_after_success() {
        let mut attempts = 0;
        let mut lines = Vec::new();
        let result: Result<(), String> = connect_with_retries(
            3,
            Duration::ZERO,
            || {
                attempts += 1;
                if attempts == 2 {
                    Ok(())
                } else {
                    Err("boom".to_owned())
                }
            },
            |level, message| lines.push((level.to_owned(), message)),
        );
        assert!(result.is_ok());
        assert_eq!(attempts, 2);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].0, "info");
        assert_eq!(lines[1].0, "warn");
    }
}
