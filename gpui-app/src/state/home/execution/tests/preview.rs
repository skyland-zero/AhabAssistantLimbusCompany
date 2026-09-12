use super::*;

#[test]
fn preview_frames_follow_selected_device_and_clear_on_disconnect() {
    let mut home = HomeState::default();
    home.apply_events(vec![
        EventEnvelope::new(
            crate::ipc::contract::event::DEVICE_STATUS,
            json!({"deviceId":"pc:limbus","status":"connected"}),
        )
        .with_sequence(1),
        EventEnvelope::new(
            crate::ipc::contract::event::SCREENSHOT_FRAME,
            json!({
                "instanceId":"pc:limbus",
                "deviceId":"pc:limbus",
                "runId":null,
                "generation":1,
                "jpeg":[255,216,255,217],
                "width":720,
                "height":405
            }),
        )
        .with_sequence(2),
    ]);

    assert_eq!(home.preview_status, PreviewStatus::Running);
    assert_eq!(home.latest_screenshot.as_ref().unwrap().width, 720);

    home.apply_events(vec![
        EventEnvelope::new(
            crate::ipc::contract::event::DEVICE_STATUS,
            json!({"deviceId":null,"status":"disconnected"}),
        )
        .with_sequence(3),
    ]);

    assert_eq!(home.preview_status, PreviewStatus::Stopped);
    assert!(home.latest_screenshot.is_none());
}

#[test]
fn binary_preview_frames_keep_metadata_and_jpeg_bytes_separate() {
    let mut home = HomeState::default();
    home.apply_events(vec![
        EventEnvelope::new(
            crate::ipc::contract::event::DEVICE_STATUS,
            json!({"deviceId":"fixture","status":"connected"}),
        )
        .with_sequence(1),
        EventEnvelope::new(
            crate::ipc::contract::event::SCREENSHOT_FRAME,
            json!({
                "instanceId":"fixture",
                "deviceId":"fixture",
                "runId":null,
                "generation":1,
                "width":2,
                "height":1
            }),
        )
        .with_binary(vec![0xff, 0xd8, 0xff, 0xd9])
        .with_sequence(2),
    ]);

    let frame = home.latest_screenshot.expect("binary preview frame");
    assert_eq!(frame.instanceId, "fixture");
    assert_eq!(frame.jpeg, vec![0xff, 0xd8, 0xff, 0xd9]);
}

#[test]
fn preview_events_follow_run_generation_and_idle_recovery_boundaries() {
    let mut home = HomeState::default();
    home.apply_events(vec![
        EventEnvelope::new(
            crate::ipc::contract::event::DEVICE_STATUS,
            json!({"deviceId":"pc:limbus","status":"connected"}),
        ),
        EventEnvelope::new(
            crate::ipc::contract::event::EXECUTION_STATUS,
            json!({
                "state":"running",
                "stateRevision":1,
                "runId":"run-a",
                "deviceLease":"runner"
            }),
        ),
        EventEnvelope::new(
            crate::ipc::contract::event::SCREENSHOT_FRAME,
            json!({
                "instanceId":"pc:limbus",
                "deviceId":"pc:limbus",
                "runId":"run-a",
                "generation":4,
                "jpeg":[1],
                "width":1,
                "height":1
            }),
        ),
    ]);

    assert_eq!(home.latest_screenshot.as_ref().unwrap().generation, Some(4));
    assert_eq!(
        home.preview_identity.as_ref().unwrap().run_id.as_deref(),
        Some("run-a")
    );

    // An older generation and a different active run cannot replace the
    // current frame; status events use the same high-water mark.
    home.apply_events(vec![
        EventEnvelope::new(
            crate::ipc::contract::event::SCREENSHOT_FRAME,
            json!({
                "instanceId":"pc:limbus",
                "deviceId":"pc:limbus",
                "runId":"run-a",
                "generation":3,
                "jpeg":[2],
                "width":1,
                "height":1
            }),
        ),
        EventEnvelope::new(
            crate::ipc::contract::event::PREVIEW_STATUS,
            json!({
                "deviceId":"pc:limbus",
                "runId":"run-a",
                "generation":3,
                "status":"error",
                "error":"stale"
            }),
        ),
        EventEnvelope::new(
            crate::ipc::contract::event::SCREENSHOT_FRAME,
            json!({
                "instanceId":"pc:limbus",
                "deviceId":"pc:limbus",
                "runId":"run-b",
                "generation":1,
                "jpeg":[3],
                "width":1,
                "height":1
            }),
        ),
        EventEnvelope::new(
            crate::ipc::contract::event::SCREENSHOT_FRAME,
            json!({
                "instanceId":"pc:limbus",
                "deviceId":"pc:limbus",
                "runId":null,
                "generation":9,
                "jpeg":[4],
                "width":1,
                "height":1
            }),
        ),
    ]);
    assert_eq!(home.latest_screenshot.as_ref().unwrap().jpeg, vec![1]);
    assert_eq!(home.preview_status, PreviewStatus::Running);

    // Once the execution reaches idle, the completed Runner run is kept
    // only as a recent-run allowance while a fresh null-run sidecar
    // generation establishes a new baseline.
    home.apply_events(vec![EventEnvelope::new(
        crate::ipc::contract::event::EXECUTION_STATUS,
        json!({
            "state":"idle",
            "stateRevision":2,
            "runId":"run-a",
            "deviceLease":"none"
        }),
    )]);
    assert!(home.latest_screenshot.is_none());
    assert_eq!(home.preview_recent_run_id.as_deref(), Some("run-a"));

    home.apply_events(vec![
        EventEnvelope::new(
            crate::ipc::contract::event::PREVIEW_STATUS,
            json!({
                "deviceId":"pc:limbus",
                "runId":null,
                "generation":1,
                "status":"starting"
            }),
        ),
        EventEnvelope::new(
            crate::ipc::contract::event::SCREENSHOT_FRAME,
            json!({
                "instanceId":"pc:limbus",
                "deviceId":"pc:limbus",
                "runId":null,
                "generation":1,
                "jpeg":[5],
                "width":1,
                "height":1
            }),
        ),
        EventEnvelope::new(
            crate::ipc::contract::event::SCREENSHOT_FRAME,
            json!({
                "instanceId":"pc:limbus",
                "deviceId":"pc:limbus",
                "runId":"run-a",
                "generation":99,
                "jpeg":[6],
                "width":1,
                "height":1
            }),
        ),
    ]);
    assert_eq!(home.latest_screenshot.as_ref().unwrap().jpeg, vec![5]);
    assert_eq!(home.latest_screenshot.as_ref().unwrap().runId, None);
    assert_eq!(home.latest_screenshot.as_ref().unwrap().generation, Some(1));
}

#[test]
fn preview_events_missing_identity_metadata_fail_safe() {
    let mut home = HomeState::default();
    home.apply_events(vec![
        EventEnvelope::new(
            crate::ipc::contract::event::DEVICE_STATUS,
            json!({"deviceId":"pc:limbus","status":"connected"}),
        ),
        EventEnvelope::new(
            crate::ipc::contract::event::EXECUTION_STATUS,
            json!({"state":"running","stateRevision":1,"runId":"run-a"}),
        ),
        EventEnvelope::new(
            crate::ipc::contract::event::SCREENSHOT_FRAME,
            json!({
                "instanceId":"pc:limbus",
                "deviceId":"pc:limbus",
                "runId":"run-a",
                "generation":2,
                "jpeg":[7],
                "width":1,
                "height":1
            }),
        ),
    ]);
    let frame_before = home.latest_screenshot.clone();

    // Each event below is a plausible handshake-time partial payload, but
    // none is allowed to clear or replace the established frame.
    home.apply_events(vec![
        EventEnvelope::new(
            crate::ipc::contract::event::SCREENSHOT_FRAME,
            json!({
                "instanceId":"pc:limbus",
                "runId":"run-a",
                "generation":3,
                "jpeg":[8],
                "width":1,
                "height":1
            }),
        ),
        EventEnvelope::new(
            crate::ipc::contract::event::SCREENSHOT_FRAME,
            json!({
                "instanceId":"pc:limbus",
                "deviceId":"pc:limbus",
                "generation":3,
                "jpeg":[9],
                "width":1,
                "height":1
            }),
        ),
        EventEnvelope::new(
            crate::ipc::contract::event::PREVIEW_STATUS,
            json!({"deviceId":"pc:limbus","runId":"run-a","status":"error"}),
        ),
    ]);
    assert_eq!(home.latest_screenshot, frame_before);
    assert_eq!(home.preview_status, PreviewStatus::Running);
}

#[test]
fn local_backend_logs_use_the_same_bounded_queue() {
    let mut home = HomeState::default();
    home.append_local_log(LogLevel::Error, "backend failed");

    assert_eq!(home.logs.back().unwrap().message, "backend failed");
    assert_eq!(home.logs.back().unwrap().level, LogLevel::Error);

    for index in 0..301 {
        home.append_local_log(LogLevel::Info, format!("log {index}"));
    }

    assert_eq!(home.logs.len(), 300);
    assert_eq!(home.logs.front().unwrap().message, "log 1");
}
