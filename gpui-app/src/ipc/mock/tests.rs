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
    let events = client.take_events();
    assert_eq!(
        events
            .iter()
            .filter(|item| item.event == event::EXECUTION_STATUS)
            .count(),
        4
    );
    assert_eq!(
        events
            .iter()
            .filter(|item| item.event == event::EXECUTION_STATS)
            .count(),
        4
    );
}

#[test]
fn execution_start_reports_the_first_enabled_task() {
    let client = MockClient::default();

    assert!(!client.call(method::EXECUTION_START, None).is_error());

    let events = client.take_events();
    let status = events
        .iter()
        .find(|item| item.event == event::EXECUTION_STATUS)
        .expect("execution status event");
    let stats = events
        .iter()
        .find(|item| item.event == event::EXECUTION_STATS)
        .expect("execution stats event");

    assert_eq!(status.payload["currentTaskId"], "daily_task");
    assert_eq!(stats.payload["currentRun"]["currentTaskId"], "daily_task");
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
fn preview_control_is_idempotent_and_reports_connection_state() {
    let client = MockClient::default();
    let stopped = client.call(method::PREVIEW_SET_ENABLED, Some(json!({"enabled": false})));
    assert_eq!(
        stopped.result.unwrap(),
        json!({"enabled": false, "running": false})
    );

    client.call(method::DEVICE_CONNECT, Some(json!({"id": "pc:limbus"})));
    let running = client.call(method::PREVIEW_SET_ENABLED, Some(json!({"enabled": true})));
    assert_eq!(
        running.result.unwrap(),
        json!({"enabled": true, "running": true})
    );
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
    assert_eq!(config["daily_task"]["set_thread_count"], 3);
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

#[test]
fn team_stats_calls_round_trip_and_clear_only_history() {
    let client = MockClient::default();
    let stats = client.call(method::TEAM_STATS_GET, Some(json!({ "id": "team-1" })));
    assert!(!stats.is_error());
    assert_eq!(stats.result.as_ref().unwrap()["totalCount"], 6);
    assert_eq!(
        stats.result.as_ref().unwrap()["hard"]["averageSeconds"],
        120.5
    );

    let cleared = client.call(method::TEAM_STATS_CLEAR, Some(json!({ "id": "team-1" })));
    assert!(!cleared.is_error());
    assert_eq!(cleared.result.as_ref().unwrap()["totalCount"], 0);
    let after = client.call(method::TEAM_STATS_GET, Some(json!({ "id": "team-1" })));
    assert_eq!(after.result.unwrap()["hard"]["count"], 0);
}

#[test]
fn notification_test_accepts_unsaved_spt_without_persisting_it() {
    let client = MockClient::default();
    let response = client.call(
        method::NOTIFICATION_TEST,
        Some(json!({"spt": "SPT_unsaved-test-value"})),
    );
    assert!(!response.is_error());
    assert_eq!(response.result.unwrap()["accepted"], true);
    assert!(
        client
            .call(method::SYSTEM_SETTINGS_GET, None)
            .result
            .unwrap()["wxpusher_spt"]
            .as_str()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn notification_test_rejects_missing_or_invalid_spt_without_echoing_it() {
    let client = MockClient::default();
    let missing = client.call(method::NOTIFICATION_TEST, Some(json!({"spt": ""})));
    assert_eq!(missing.error.unwrap().message, "SPT 未配置");
    let invalid = client.call(method::NOTIFICATION_TEST, Some(json!({"spt": "secret"})));
    let error = invalid.error.unwrap();
    assert_eq!(error.message, "SPT 格式无效");
    assert!(!error.message.contains("secret"));
}
