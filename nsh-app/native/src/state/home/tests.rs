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
        "daily": {"enabled": true, "exp": "2", "thread": "4", "chain": "on"},
        "rewards": {"enabled": false, "mode": "weekly"},
        "enkephalin": {"enabled": true, "times": "3", "grandet": "on"},
        "mirror": {"enabled": true, "runs": "5", "difficulty": "hard"},
        "resonance": {"enabled": false, "voice": "off"},
    });
    state.configure(selection).unwrap();
    assert_eq!(state.tasks.daily_task.set_EXP_count, 2);
    assert_eq!(state.tasks.daily_task.set_thread_count, 4);
    assert!(state.tasks.daily_task.use_continuous_combat);
    assert!(!state.tasks.enabledTasks.get_reward);
    assert_eq!(state.tasks.get_reward.set_get_prize, 2);
    assert_eq!(state.tasks.buy_enkephalin.set_lunacy_to_enkephalin, 3);
    assert!(state.tasks.buy_enkephalin.Dr_Grandet_mode);
    assert_eq!(state.tasks.mirror.set_mirror_count, 5);
    assert!(state.tasks.mirror.hard_mirror);
    assert!(!state.tasks.enabledTasks.resonate_with_Ahab);
    assert!(!state.tasks.resonate_with_Ahab.enabled);

    let derived = state.selection();
    assert_eq!(derived["daily"]["exp"], "2");
    assert_eq!(derived["rewards"]["mode"], "weekly");
    assert_eq!(derived["mirror"]["difficulty"], "hard");
    assert_eq!(derived["resonance"]["voice"], "off");
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
    state.set_window_size(1440).unwrap();
    assert_eq!(state.window_summary()["resolution"], "1440P");
    state.toggle_async_input().unwrap();
    assert_eq!(state.window_summary()["asyncInput"], true);
    state.set_window_size(1080).unwrap();
    assert_eq!(state.window_summary()["resolution"], "1080P");
}

#[test]
fn statistics_payload_and_daily_summary_are_available_for_details() {
    let mut state = state();
    let payload = state.stats_payload();
    assert!(payload["today"].is_object());
    assert!(payload["mirrorHistory"].is_array());

    let daily = state.load_daily_stats().unwrap();
    assert_eq!(daily.schemaVersion, 1);
    assert!(daily.days.is_empty());
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
