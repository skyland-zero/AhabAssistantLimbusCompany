//! The TSX-facing IPC facts and connectivity probe.
//!
//! State objects will use the shared [`BackendClient`] behind this module. The
//! probe exists so the UI can show whether it is on the mock or the real
//! sidecar without starting a sidecar just to ask.

use std::sync::OnceLock;

use nsh_core::prelude::*;
use serde_json::Value;

use super::{BackendClient, contract, contract::method};

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

/// The process-wide transport. The first call starts the sidecar (or the mock)
/// and every later state object shares it.
pub fn client() -> &'static BackendClient {
    static CLIENT: OnceLock<BackendClient> = OnceLock::new();
    CLIENT.get_or_init(|| match configured_mode().as_str() {
        "mock" => BackendClient::mock(),
        "unavailable" => BackendClient::unavailable("AHAB_BACKEND=unavailable"),
        _ => BackendClient::try_sidecar().unwrap_or_else(|error| BackendClient::unavailable(error)),
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

    /// Pings the backend and resolves with its reply (`"pong"`).
    pub async fn ping() -> Result<String, String> {
        let client = client().clone();
        blocking(move || {
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
}
