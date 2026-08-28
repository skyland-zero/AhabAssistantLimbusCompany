use super::*;

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
}
