use super::*;

use super::super::contract::{event, method};

#[test]
fn unknown_method_is_a_structured_error() {
    let response = MockClient::default().call("not.implemented", None);
    assert_eq!(response.error.unwrap().code, -32601);
}

#[test]
fn execution_has_deterministic_state_transitions() {
    let client = MockClient::default();
    assert_eq!(
        client
            .call(method::EXECUTION_GET_STATE, None)
            .result
            .unwrap()["state"],
        "idle"
    );
    assert!(!client.call(method::EXECUTION_START, None).is_error());
    assert!(!client.call(method::EXECUTION_PAUSE, None).is_error());
    assert!(!client.call(method::EXECUTION_RESUME, None).is_error());
    assert!(!client.call(method::EXECUTION_STOP, None).is_error());
    assert_eq!(
        client
            .call(method::EXECUTION_GET_STATE, None)
            .result
            .unwrap()["state"],
        "idle"
    );
    assert_eq!(client.take_events().len(), 5);
}

#[test]
fn execution_start_stays_idle_when_only_non_executable_settings_are_enabled() {
    for ahab_enabled in [false, true] {
        let client = MockClient::default();
        let mut config = TasksConfig::default();
        config.enabledTasks.daily_task = false;
        config.enabledTasks.get_reward = false;
        config.enabledTasks.buy_enkephalin = false;
        config.enabledTasks.mirror = false;
        config.enabledTasks.resonate_with_Ahab = ahab_enabled;

        assert!(
            !client
                .call(
                    method::TASKS_SET_CONFIG,
                    Some(serde_json::to_value(config).unwrap()),
                )
                .is_error()
        );
        assert!(!client.call(method::EXECUTION_START, None).is_error());
        assert_eq!(
            client
                .call(method::EXECUTION_GET_STATE, None)
                .result
                .unwrap()["state"],
            "idle"
        );
        assert!(client.take_events().is_empty());
    }
}

#[test]
fn device_starts_disconnected_until_explicit_connect() {
    let client = MockClient::default();
    assert_eq!(client.device_status(), ConnectionStatus::Disconnected);
    let devices = client.call(method::DEVICE_LIST, None).result.unwrap();
    assert_eq!(devices.as_array().unwrap().len(), 2);
    assert_eq!(devices[1]["id"], "mumu:0");
    assert_eq!(devices[1]["name"], "MuMu 模拟器");
    assert!(
        !client
            .call(method::DEVICE_CONNECT, Some(json!({ "id": "pc:limbus" })))
            .is_error()
    );
    let device_event = client.take_events().pop().unwrap();
    assert_eq!(device_event.event, event::DEVICE_STATUS);
    assert_eq!(device_event.payload["status"], "connected");
}

#[test]
fn cloned_clients_observe_one_backend_state() {
    let writer = MockClient::default();
    let reader = writer.shared();
    let team = json!({
        "id": "",
        "name": "shared",
        "sinners": [],
        "purpose": "general",
        "accessoryScheme": "burn",
        "enabled": true,
        "mirrorConfig": null
    });
    assert!(!writer.call(method::TEAM_SAVE, Some(team)).is_error());
    let teams = reader.call(method::TEAM_LIST, None).result.unwrap();
    assert_eq!(teams.as_array().unwrap().len(), 4);
}

#[test]
fn config_and_team_calls_round_trip() {
    let client = MockClient::default();
    let config = client.call(method::TASKS_GET_CONFIG, None).result.unwrap();
    assert_eq!(config["set_windows"]["set_win_size"], 1080);
    let team = json!({"id":"", "name":"test", "sinners":[], "purpose":"general", "accessoryScheme":"", "enabled":true, "mirrorConfig":null});
    assert!(!client.call(method::TEAM_SAVE, Some(team)).is_error());
    assert_eq!(
        client
            .call(method::TEAM_LIST, None)
            .result
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        4
    );
}
