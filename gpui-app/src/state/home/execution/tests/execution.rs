use super::*;

#[test]
fn execution_task_changes_follow_status_and_stats_events() {
    let mut home = HomeState::default();
    let status_event = |seq, task| {
        EventEnvelope::new(
            crate::ipc::contract::event::EXECUTION_STATUS,
            json!({"state":"running","currentTaskId":task}),
        )
        .with_sequence(seq)
    };
    let stats_event = |seq, task| {
        let mut stats = ExecutionStatsPayload::default();
        stats.currentRun.state = ExecutionState::Running;
        stats.currentRun.currentTaskId = Some(task);
        EventEnvelope::new(
            crate::ipc::contract::event::EXECUTION_STATS,
            serde_json::to_value(stats).unwrap(),
        )
        .with_sequence(seq)
    };

    home.apply_events(vec![
        status_event(1, "daily_task"),
        stats_event(2, FixedTaskId::DailyTask),
        status_event(3, "get_reward"),
        stats_event(4, FixedTaskId::GetReward),
        status_event(5, "mirror"),
        stats_event(6, FixedTaskId::Mirror),
    ]);

    assert_eq!(home.execution.currentTaskId, Some(FixedTaskId::Mirror));
    assert_eq!(
        home.stats.currentRun.currentTaskId,
        Some(FixedTaskId::Mirror)
    );

    home.apply_events(vec![
        EventEnvelope::new(
            crate::ipc::contract::event::EXECUTION_STATUS,
            json!({"state":"idle","currentTaskId":null}),
        )
        .with_sequence(7),
        EventEnvelope::new(
            crate::ipc::contract::event::EXECUTION_STATS,
            serde_json::to_value(ExecutionStatsPayload::default()).unwrap(),
        )
        .with_sequence(8),
    ]);

    assert_eq!(home.execution.currentTaskId, None);
    assert_eq!(home.stats.currentRun.currentTaskId, None);
}

#[test]
fn execution_snapshots_reject_older_revisions_and_run_derived_events() {
    let mut home = HomeState::default();
    home.apply_events(vec![EventEnvelope::new(
        crate::ipc::contract::event::EXECUTION_STATUS,
        json!({
            "schemaVersion": 3,
            "state": "running",
            "stateRevision": 4,
            "runId": "run-new"
        }),
    )]);

    home.apply_events(vec![
        EventEnvelope::new(
            crate::ipc::contract::event::EXECUTION_STATUS,
            json!({
                "state": "paused",
                "stateRevision": 3,
                "runId": "run-new"
            }),
        ),
        EventEnvelope::new(
            crate::ipc::contract::event::EXECUTION_MIRROR_PROGRESS,
            json!({
                "current": 99,
                "total": 99,
                "isHard": false,
                "isInfinite": false,
                "runId": "run-old"
            }),
        ),
        EventEnvelope::new(
            crate::ipc::contract::event::EXECUTION_STATS,
            json!({
                "schemaVersion": 1,
                "currentRun": {
                    "runId": "run-old",
                    "state": "running",
                    "currentTaskId": null,
                    "startedAt": null,
                    "targets": {"exp": 0, "thread": 0, "mirror": 0},
                    "completed": {"exp": 0, "thread": 0, "mirror": 0},
                    "isMirrorInfinite": false,
                    "updatedAt": null
                },
                "today": {"exp": 0, "thread": 0, "mirror": 0},
                "week": {"exp": 0, "thread": 0, "mirror": 0},
                "updatedAt": 0
            }),
        ),
    ]);

    assert_eq!(home.execution.state, ExecutionState::Running);
    assert_eq!(home.execution.stateRevision, 4);
    assert!(home.mirror_progress.is_none());
    assert_eq!(home.stats.currentRun.runId, None);

    // A different run cannot replace an active one, even with a larger
    // revision.  It becomes eligible only after the current run reaches
    // its final idle snapshot.
    home.apply_events(vec![EventEnvelope::new(
        crate::ipc::contract::event::EXECUTION_STATUS,
        json!({
            "state": "starting",
            "stateRevision": 5,
            "runId": "run-next"
        }),
    )]);
    assert_eq!(home.execution.state, ExecutionState::Running);
    home.apply_events(vec![EventEnvelope::new(
        crate::ipc::contract::event::EXECUTION_STATUS,
        json!({
            "state": "idle",
            "stateRevision": 5,
            "runId": "run-new"
        }),
    )]);
    home.apply_events(vec![EventEnvelope::new(
        crate::ipc::contract::event::EXECUTION_STATUS,
        json!({
            "state": "starting",
            "stateRevision": 6,
            "runId": "run-next"
        }),
    )]);
    assert_eq!(home.execution.state, ExecutionState::Starting);
    assert_eq!(home.execution.runId.as_deref(), Some("run-next"));
}

#[test]
fn execution_status_does_not_overwrite_active_or_finalized_runs() {
    let mut home = HomeState::default();
    home.apply_execution_status(ExecutionStatusPayload {
        state: ExecutionState::Running,
        stateRevision: 4,
        runId: Some("run-current".into()),
        ..ExecutionStatusPayload::default()
    });
    assert!(!home.apply_execution_status(ExecutionStatusPayload {
        state: ExecutionState::Running,
        stateRevision: 5,
        runId: Some("run-old".into()),
        ..ExecutionStatusPayload::default()
    }));
    assert_eq!(home.execution.runId.as_deref(), Some("run-current"));

    assert!(home.apply_execution_status(ExecutionStatusPayload {
        state: ExecutionState::Idle,
        stateRevision: 6,
        runId: Some("run-current".into()),
        ..ExecutionStatusPayload::default()
    }));
    assert!(!home.apply_execution_status(ExecutionStatusPayload {
        state: ExecutionState::Running,
        stateRevision: 7,
        runId: Some("run-current".into()),
        ..ExecutionStatusPayload::default()
    }));
    assert_eq!(home.execution.state, ExecutionState::Idle);
}

#[test]
fn final_idle_stats_are_applied_after_the_idle_status_snapshot() {
    let mut home = HomeState::default();
    home.apply_execution_status(ExecutionStatusPayload {
        state: ExecutionState::Running,
        stateRevision: 1,
        runId: Some("run-finished".into()),
        ..ExecutionStatusPayload::default()
    });
    home.apply_execution_status(ExecutionStatusPayload {
        state: ExecutionState::Idle,
        stateRevision: 2,
        runId: Some("run-finished".into()),
        outcome: Some(crate::model::ExecutionOutcome::Completed),
        ..ExecutionStatusPayload::default()
    });

    let mut stats = ExecutionStatsPayload::default();
    stats.currentRun.runId = Some("run-finished".into());
    stats.currentRun.state = ExecutionState::Idle;
    stats.currentRun.completed.mirror = 3;
    home.apply_events(vec![EventEnvelope::new(
        crate::ipc::contract::event::EXECUTION_STATS,
        serde_json::to_value(stats).unwrap(),
    )]);

    assert_eq!(home.stats.currentRun.runId.as_deref(), Some("run-finished"));
    assert_eq!(home.stats.currentRun.completed.mirror, 3);
}

#[test]
fn app_exit_requests_accept_only_the_latest_run() {
    let mut home = HomeState::default();
    home.apply_execution_status(ExecutionStatusPayload {
        state: ExecutionState::Idle,
        stateRevision: 8,
        runId: Some("run-latest".into()),
        outcome: Some(crate::model::ExecutionOutcome::Completed),
        ..ExecutionStatusPayload::default()
    });

    assert!(home.accepts_exit_request(Some("run-latest")));
    assert!(!home.accepts_exit_request(Some("run-old")));
    // Legacy sidecars did not include a run association on lifecycle
    // events; keep that compatibility path explicit.
    assert!(home.accepts_exit_request(None));
}

#[test]
fn idle_state_is_busy_when_a_device_lease_is_still_present() {
    let mut home = HomeState::default();
    assert!(!home.is_busy());
    home.execution.deviceLease = crate::model::DeviceLeaseState::Restoring;
    assert!(home.is_busy());
}

#[test]
fn new_run_stats_arriving_before_status_are_not_dropped() {
    let mut home = HomeState::default();
    // A previous run finished: idle with a non-zero revision.
    home.apply_execution_status(ExecutionStatusPayload {
        state: ExecutionState::Idle,
        stateRevision: 4,
        runId: Some("run-old".into()),
        outcome: Some(crate::model::ExecutionOutcome::Completed),
        ..ExecutionStatusPayload::default()
    });

    let stats = ExecutionStatsPayload {
        updatedAt: 10,
        currentRun: crate::model::CurrentRunStats {
            runId: Some("run-new".into()),
            state: ExecutionState::Running,
            ..crate::model::CurrentRunStats::default()
        },
        ..ExecutionStatsPayload::default()
    };
    home.apply_events(vec![EventEnvelope::new(
        crate::ipc::contract::event::EXECUTION_STATS,
        serde_json::to_value(stats).unwrap(),
    )]);

    // The stats payload is admitted while idle; the following status
    // event moves the authoritative run forward.
    assert_eq!(home.stats.currentRun.runId.as_deref(), Some("run-new"));
    home.apply_execution_status(ExecutionStatusPayload {
        state: ExecutionState::Running,
        stateRevision: 5,
        runId: Some("run-new".into()),
        ..ExecutionStatusPayload::default()
    });
    assert_eq!(home.execution.runId.as_deref(), Some("run-new"));
}

#[test]
fn stale_stats_never_regress_a_newer_summary() {
    let mut home = HomeState::default();
    let newer = ExecutionStatsPayload {
        updatedAt: 20,
        today: crate::model::StatCounts {
            mirror: 5,
            ..crate::model::StatCounts::default()
        },
        ..ExecutionStatsPayload::default()
    };
    home.apply_events(vec![EventEnvelope::new(
        crate::ipc::contract::event::EXECUTION_STATS,
        serde_json::to_value(newer).unwrap(),
    )]);
    let older = ExecutionStatsPayload {
        updatedAt: 19,
        today: crate::model::StatCounts {
            mirror: 1,
            ..crate::model::StatCounts::default()
        },
        ..ExecutionStatsPayload::default()
    };
    home.apply_events(vec![EventEnvelope::new(
        crate::ipc::contract::event::EXECUTION_STATS,
        serde_json::to_value(older).unwrap(),
    )]);
    assert_eq!(home.stats.today.mirror, 5);
}

#[test]
fn run_scoped_messages_survive_idle_but_not_a_different_active_run() {
    let mut home = HomeState::default();
    home.apply_execution_status(ExecutionStatusPayload {
        state: ExecutionState::Idle,
        stateRevision: 3,
        runId: Some("run-a".into()),
        ..ExecutionStatusPayload::default()
    });
    // A completion notice for the finished run still reaches the log.
    home.apply_events(vec![EventEnvelope::new(
        crate::ipc::contract::event::LOG_ENTRY,
        json!({"ts": 1, "level": "info", "message": "done", "runId": "run-a"}),
    )]);
    assert!(home.logs.iter().any(|entry| entry.message == "done"));

    // While another run is active, only that run's messages are accepted.
    home.apply_execution_status(ExecutionStatusPayload {
        state: ExecutionState::Running,
        stateRevision: 4,
        runId: Some("run-b".into()),
        ..ExecutionStatusPayload::default()
    });
    home.apply_events(vec![EventEnvelope::new(
        crate::ipc::contract::event::LOG_ENTRY,
        json!({"ts": 2, "level": "info", "message": "late-a", "runId": "run-a"}),
    )]);
    home.apply_events(vec![EventEnvelope::new(
        crate::ipc::contract::event::LOG_ENTRY,
        json!({"ts": 3, "level": "info", "message": "current-b", "runId": "run-b"}),
    )]);
    assert!(!home.logs.iter().any(|entry| entry.message == "late-a"));
    assert!(home.logs.iter().any(|entry| entry.message == "current-b"));
}

#[test]
fn idle_transition_prunes_historical_run_identity_state() {
    let mut home = HomeState::default();
    home.execution_event_sequences.insert("run-old".into(), 12);
    home.preview_generation_floor
        .insert(("pc:limbus".into(), Some("run-old".into())), 9);
    home.apply_execution_status(ExecutionStatusPayload {
        state: ExecutionState::Idle,
        stateRevision: 2,
        runId: Some("run-current".into()),
        ..ExecutionStatusPayload::default()
    });
    assert!(!home.execution_event_sequences.contains_key("run-old"));
    assert!(
        !home
            .preview_generation_floor
            .contains_key(&("pc:limbus".to_string(), Some("run-old".to_string())))
    );
}
