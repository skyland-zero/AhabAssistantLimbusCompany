use super::*;
use crate::ipc::BackendClient;

fn state() -> HomeState {
    let state = HomeState::new(BackendClient::mock());
    assert!(state.error.is_none(), "mock must load: {:?}", state.error);
    state
}

#[test]
fn load_reads_tasks_devices_and_stats() {
    let state = state();
    assert_eq!(state.tasks.set_windows.set_win_size, 1080);
    assert_eq!(state.tasks.daily_task.set_EXP_count, 1);
    assert_eq!(state.devices.len(), 2);
    assert_eq!(state.device_status, ConnectionStatus::Disconnected);
    assert_eq!(state.execution.state, ExecutionState::Idle);
}

#[test]
fn selection_round_trips_through_the_task_config() {
    let mut state = state();
    let selection = json!({
        "daily": {
            "enabled": true, "exp": "2", "thread": "4", "team": "7",
            "chain": "on", "chainCount": "6", "targetedExp": "on",
            "expSlash": "3", "threadSun": "9",
        },
        "rewards": {"enabled": false, "mode": "mail"},
        "enkephalin": {"enabled": true, "times": "3", "grandet": "on", "skip": "on"},
        "mirror": {
            "enabled": true, "runs": "5", "difficulty": "hard", "hardTarget": "15",
            "infinite": "on", "noWeekly": "on", "keyboard": "on",
        },
        "resonance": {"enabled": false},
    });
    state.configure(selection).unwrap();
    assert_eq!(state.tasks.daily_task.set_EXP_count, 2);
    assert_eq!(state.tasks.daily_task.daily_teams, 7);
    assert_eq!(state.tasks.daily_task.use_continuous_combat_select, 6);
    assert!(state.tasks.daily_task.targeted_teaming_EXP);
    assert_eq!(state.tasks.daily_task.EXP_day_1_2, 3);
    assert_eq!(state.tasks.daily_task.thread_day_7, 9);
    assert_eq!(state.tasks.daily_task.set_thread_count, 4);
    assert!(state.tasks.daily_task.use_continuous_combat);
    assert!(!state.tasks.enabledTasks.get_reward);
    assert_eq!(state.tasks.get_reward.set_get_prize, 2);
    assert_eq!(state.tasks.buy_enkephalin.set_lunacy_to_enkephalin, 3);
    assert!(state.tasks.buy_enkephalin.Dr_Grandet_mode);
    assert!(state.tasks.buy_enkephalin.skip_enkephalin);
    assert_eq!(state.tasks.mirror.set_mirror_count, 5);
    assert!(state.tasks.mirror.hard_mirror);
    assert_eq!(state.tasks.mirror.hard_mirror_target_floors, 15);
    assert!(state.tasks.mirror.infinite_dungeons);
    assert!(state.tasks.mirror.no_weekly_bonuses);
    assert!(state.tasks.mirror.mirror_keyboard_navigation);
    assert!(!state.tasks.enabledTasks.resonate_with_Ahab);
    assert!(!state.tasks.resonate_with_Ahab.enabled);

    let derived = state.selection();
    assert_eq!(derived["daily"]["exp"], "2");
    assert_eq!(derived["daily"]["chain"], "on");
    assert_eq!(derived["daily"]["team"], "7");
    assert_eq!(derived["daily"]["chainCount"], "6");
    assert_eq!(derived["daily"]["threadSun"], "9");
    assert_eq!(derived["rewards"]["mode"], "mail");
    assert_eq!(derived["enkephalin"]["skip"], "on");
    assert_eq!(derived["mirror"]["difficulty"], "hard");
    assert_eq!(derived["mirror"]["hardTarget"], "15");
    assert_eq!(derived["mirror"]["infinite"], "on");
    assert_eq!(derived["mirror"]["hatred"], "off");
    assert_eq!(derived["resonance"]["enabled"], false);
}

#[test]
fn start_submits_the_enabled_tasks_and_collects_the_log() {
    let mut state = state();
    assert!(state.start().unwrap());
    assert_eq!(state.execution.state, ExecutionState::Running);
    assert_eq!(state.feedback.as_deref(), Some("已提交启动请求"));
    assert!(
        state
            .logs
            .iter()
            .any(|entry| entry.message.contains("Mock 任务已开始"))
    );
    assert_eq!(state.window_summary()["resolution"], "1080P");
}

#[test]
fn a_queue_with_no_executable_task_does_not_start() {
    let mut state = state();
    let selection = json!({
        "daily": {"enabled": false},
        "rewards": {"enabled": false},
        "enkephalin": {"enabled": false},
        "mirror": {"enabled": false},
        "resonance": {"enabled": true, "voice": "on"},
    });
    state.configure(selection).unwrap();
    assert!(!state.start().unwrap());
    assert_eq!(state.execution.state, ExecutionState::Idle);
    assert_eq!(state.feedback.as_deref(), Some("没有选择可执行任务"));
}

#[test]
fn pause_resume_and_stop_follow_the_execution_state() {
    let mut state = state();
    state.start().unwrap();
    state.pause_or_resume().unwrap();
    assert_eq!(state.execution.state, ExecutionState::Paused);
    state.pause_or_resume().unwrap();
    assert_eq!(state.execution.state, ExecutionState::Running);
    state.stop().unwrap();
    assert_eq!(state.execution.state, ExecutionState::Idle);
}

#[test]
fn device_refresh_and_connect_update_the_console() {
    let mut state = state();
    state.refresh_devices().unwrap();
    assert_eq!(state.devices.len(), 2);
    assert_eq!(state.feedback.as_deref(), Some("已刷新设备列表"));

    state.connect_device("mumu:0").unwrap();
    assert_eq!(state.device_status, ConnectionStatus::Connected);
    assert_eq!(state.selected_device.as_deref(), Some("mumu:0"));
    assert_eq!(state.feedback.as_deref(), Some("设备已连接"));
}

#[test]
fn window_settings_are_saved_through_the_task_config() {
    let mut state = state();
    state.set_window("size", &json!(1440)).unwrap();
    assert_eq!(state.window_summary()["resolution"], "1440P");
    state.set_window("asyncInput", &json!(true)).unwrap();
    assert_eq!(state.window_summary()["asyncInput"], true);
    state.set_window("position", &json!("left_top")).unwrap();
    assert_eq!(state.window_summary()["position"], "left_top");
    state
        .set_window("screenshotInterval", &json!(0.2))
        .unwrap();
    assert!((state.window_summary()["screenshotInterval"].as_f64().unwrap() - 0.2).abs() < 1e-3);
    state.set_window("mouseInterval", &json!("0.1")).unwrap();
    assert!((state.window_summary()["mouseInterval"].as_f64().unwrap() - 0.1).abs() < 1e-3);
    state.set_window("reduceMiscontact", &json!(false)).unwrap();
    assert_eq!(state.window_summary()["reduceMiscontact"], false);
    state.set_window("size", &json!(1080)).unwrap();
    assert_eq!(state.window_summary()["resolution"], "1080P");
}

#[test]
fn a_window_size_arrives_as_text_from_the_card() {
    let mut state = state();
    state.set_window("size", &json!("720")).unwrap();
    assert_eq!(state.window_summary()["resolution"], "720P");
    state.set_window("size", &json!("1440")).unwrap();
    assert_eq!(state.window_summary()["resolution"], "1440P");
}

#[test]
fn a_window_setting_off_its_range_is_rejected() {
    let mut state = state();
    assert!(state.set_window("screenshotInterval", &json!(30)).is_err());
    assert!(state.set_window("position", &json!(42)).is_ok());
    assert_eq!(state.window_summary()["position"], "free");
}

#[test]
fn after_completion_actions_round_trip_and_reject_unknown_keys() {
    let mut state = state();
    state
        .set_after_completion(
            vec!["exit_game".into(), "exit_emulator".into()],
            "lock".into(),
            true,
        )
        .unwrap();
    let view = state.after_completion();
    assert_eq!(view["actions"][0], "exit_game");
    assert_eq!(view["actions"][1], "exit_emulator");
    assert_eq!(view["power"], "lock");
    assert_eq!(view["keep"], true);
    assert_eq!(
        state.tasks.afterCompletion.powerAction,
        crate::model::AfterPowerAction::Lock
    );

    state
        .set_after_completion(Vec::new(), "none".into(), false)
        .unwrap();
    assert!(!state.tasks.afterCompletion.keepAfterCompletion);
    assert!(state.tasks.afterCompletion.actions.is_empty());

    assert!(
        state
            .set_after_completion(vec!["exit_everything".into()], "none".into(), false)
            .is_err()
    );
    assert!(
        state
            .set_after_completion(Vec::new(), "reboot".into(), false)
            .is_err()
    );
}

#[test]
fn statistics_payload_and_daily_summary_are_available_for_details() {
    let mut state = state();
    let payload = state.stats_payload();
    assert!(payload["today"].is_object());
    let history = payload["mirrorHistory"].as_array().expect("mock history");
    assert_eq!(history.len(), 6);
    assert!(history[0]["battleSeconds"].as_f64().unwrap() > 0.0);
    assert!(history[0]["team"]["name"].is_string());

    // The page's poll drains the queue and returns the same payload.
    assert_eq!(state.poll_stats()["mirrorHistory"].as_array().unwrap().len(), 6);

    let daily = state.load_daily_stats().unwrap();
    assert_eq!(daily.schemaVersion, 1);
    assert_eq!(daily.days.len(), 4);
    assert_eq!(daily.days[3].mirror, 2);
}

#[test]
fn logs_are_bounded_and_clearable() {
    let mut state = state();
    for index in 0..350 {
        state.push_log("info", &format!("line {index}"));
    }
    assert_eq!(state.logs.len(), MAX_LOGS);
    state.clear_logs();
    assert!(state.logs.is_empty());
    assert_eq!(state.feedback.as_deref(), Some("日志已清空"));
}
