use super::*;
use crate::ipc::contract::method;
use std::time::Duration;

#[test]
fn sidecar_tokens_are_random_hex_values() {
    let first = sidecar::random_token().expect("token");
    let second = sidecar::random_token().expect("token");
    assert_eq!(first.len(), 64);
    assert!(first.chars().all(|character| character.is_ascii_hexdigit()));
    assert_ne!(first, second);
}

#[test]
fn onedir_backend_is_preferred_over_a_legacy_onefile_executable() {
    let root = Path::new("C:/AALC");
    let candidates = sidecar::backend_executable_candidates(root);
    assert_eq!(candidates[0], root.join("backend").join("AALC Backend.exe"));
    assert_eq!(candidates[1], root.join("AALC Backend.exe"));
}

#[test]
fn stable_device_sidecar_ids_are_not_hardware_handles() {
    assert!(sidecar::parse_loopback_address("ws://127.0.0.1:1234").is_ok());
    assert!(sidecar::parse_loopback_address("ws://192.168.0.1:1234").is_err());
    assert!(sidecar::parse_loopback_address("http://127.0.0.1:1234").is_err());
}

#[test]
fn failed_client_returns_structured_error() {
    let client = WebSocketClient::failed("test unavailable");
    let response = client.request("device.list", None);
    assert_eq!(response.error.unwrap().code, -32000);
}

#[test]
fn sidecar_round_trip_can_be_run_explicitly() {
    if std::env::var_os("AHAB_RUN_SIDECAR_INTEGRATION").is_none() {
        return;
    }
    let client = WebSocketClient::try_new().expect("sidecar should start");
    let response = client.request("device.list", None);
    assert!(response.error.is_none(), "sidecar response: {response:?}");
    assert!(response.result.unwrap().is_array());

    // A reconnect must keep the same sidecar process alive and establish a
    // fresh session; only the graceful shutdown below may end the process.
    let reconnected = client.reconnect().expect("sidecar should reconnect");
    assert!(reconnected.has_live_sidecar());
    let pong = reconnected.request_with_timeout(method::APP_PING, None, Duration::from_secs(5));
    assert_eq!(pong.result, Some(serde_json::json!("pong")));

    let shutdown =
        reconnected.request_with_timeout(method::APP_SHUTDOWN, None, Duration::from_secs(5));
    assert!(shutdown.error.is_none(), "shutdown response: {shutdown:?}");
    assert!(reconnected.wait_for_exit(Duration::from_secs(5)));
}
