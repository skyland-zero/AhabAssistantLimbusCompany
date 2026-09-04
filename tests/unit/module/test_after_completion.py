from __future__ import annotations

from typing import Any

from module.after_completion import AfterCompletionCoordinator


class Journal:
    def __init__(self, records: list[dict[str, Any]] | None = None) -> None:
        self.records = list(records or [])

    def transition(self, action_id: str, state: str, **fields: Any) -> None:
        value = {"actionId": action_id, "state": state, **fields}
        self.records.append(value)


def _latest(journal: Journal) -> dict[str, dict[str, Any]]:
    latest: dict[str, dict[str, Any]] = {}
    for record in journal.records:
        latest[record["actionId"]] = record
    return latest


def test_completed_actions_are_ordered_and_typed_without_os_side_effects() -> None:
    calls: list[Any] = []
    journal = Journal()

    def handler(record: dict[str, Any]) -> dict[str, str]:
        calls.append(record["actionType"])
        return {"accepted": "yes"}

    def exit_event(event: str, payload: dict[str, Any]) -> None:
        calls.append((event, payload["actionId"]))

    result = AfterCompletionCoordinator(
        journal,
        handlers={"notification": handler, "toast": handler, "sound": handler, "power": lambda value: calls.append(value)},
        exit_request_handler=exit_event,
    ).execute(
        "run-1",
        outcome="completed",
        actions=["sound", "exit_aalc", "toast", "notification"],
        power_action="lock",
    )

    assert calls == [
        "notification",
        "toast",
        "sound",
        ("app.exitRequested", "run-1:exit_aalc"),
        "lock",
    ]
    assert [item["actionType"] for item in result["actions"]] == [
        "notification",
        "toast",
        "sound",
        "exit_aalc",
        "power",
    ]
    assert all(item["status"] == "done" for item in result["actions"])
    assert result["actions"][-1]["powerAction"] == "lock"
    assert result["actions"][3]["event"]["type"] == "app.exitRequested"
    assert _latest(journal)["run-1:power"]["state"] == "done"


def test_stopped_and_forced_runs_skip_actions_but_run_cleanup() -> None:
    calls: list[str] = []
    coordinator = AfterCompletionCoordinator(
        notification_handler=lambda record: calls.append("notification"),
        power_handler=lambda value: calls.append("power"),
        cleanup_handler=lambda record: calls.append("cleanup"),
    )

    stopped = coordinator.execute("run-stop", outcome="stopped", actions=["notification"], power_action="shutdown")
    forced = coordinator.execute("run-forced", outcome="completed", forced=True, actions=["notification"], power_action="shutdown")

    assert calls == ["cleanup", "cleanup"]
    assert stopped["eligible"] is False
    assert forced["eligible"] is False
    assert all(item["status"] == "skipped" for item in stopped["actions"])
    assert all(item["status"] == "skipped" for item in forced["actions"])
    assert stopped["actions"][0]["error"]["code"] == "OUTCOME_NOT_COMPLETED"
    assert forced["actions"][0]["error"]["code"] == "FORCED_RUN"


def test_done_action_is_idempotent_for_repeated_completion() -> None:
    calls: list[str] = []
    coordinator = AfterCompletionCoordinator(notification_handler=lambda record: calls.append(record["actionId"]))

    first = coordinator.execute("run-repeat", actions=["notification"])
    second = coordinator.execute("run-repeat", actions=["notification"])

    assert calls == ["run-repeat:notification"]
    assert first["actions"][0]["status"] == "done"
    assert second["actions"][0]["status"] == "done"
    assert second["actions"][0]["deduplicated"] is True


def test_restart_reconciles_safe_notifications_but_never_replays_dangerous_actions() -> None:
    journal = Journal(
        [
            {"actionId": "run-crash:notification", "runId": "run-crash", "actionType": "notification", "state": "executing"},
            {"actionId": "run-crash:exit_aalc", "runId": "run-crash", "actionType": "exit_aalc", "state": "executing"},
            {"actionId": "run-crash:power", "runId": "run-crash", "actionType": "power", "powerAction": "shutdown", "state": "executing"},
        ]
    )
    calls: list[str] = []
    coordinator = AfterCompletionCoordinator(
        journal,
        notification_handler=lambda record: calls.append("notification"),
        exit_request_handler=lambda record: calls.append("exit_aalc"),
        power_handler=lambda value: calls.append("power"),
    )

    latest = _latest(journal)
    assert latest["run-crash:notification"]["state"] == "pending"
    assert latest["run-crash:exit_aalc"]["state"] == "unknown"
    assert latest["run-crash:power"]["state"] == "unknown"

    result = coordinator.execute(
        "run-crash",
        actions=["notification", "exit_aalc"],
        power_action="shutdown",
    )

    assert calls == ["notification"]
    by_type = {item["actionType"]: item for item in result["actions"]}
    assert by_type["notification"]["status"] == "done"
    assert by_type["exit_aalc"]["status"] == "unknown"
    assert by_type["power"]["status"] == "unknown"
    assert by_type["exit_aalc"]["recovered"] is True


def test_action_failure_does_not_block_later_actions_or_cleanup() -> None:
    calls: list[str] = []

    def broken(record: dict[str, Any]) -> None:
        calls.append("toast")
        raise RuntimeError("toast unavailable")

    coordinator = AfterCompletionCoordinator(
        toast_handler=broken,
        sound_handler=lambda record: calls.append("sound"),
        cleanup_handler=lambda record: calls.append("cleanup"),
    )
    result = coordinator.execute("run-failure", actions=["toast", "sound"])

    assert calls == ["toast", "sound", "cleanup"]
    by_type = {item["actionType"]: item for item in result["actions"]}
    assert by_type["toast"]["status"] == "failed"
    assert by_type["toast"]["error"]["code"] == "ACTION_FAILED"
    assert by_type["sound"]["status"] == "done"
    assert result["cleanup"]["status"] == "done"


def test_device_owned_actions_and_invalid_power_are_rejected_by_allowlist() -> None:
    calls: list[str] = []
    coordinator = AfterCompletionCoordinator(
        handlers={"exit_aalc": lambda record: calls.append("exit"), "power": lambda value: calls.append("power")}
    )

    result = coordinator.execute(
        "run-unsafe",
        actions=["exit_game", "exit_emulator", "exit_aalc"],
        power_action="reboot",
    )

    by_type = {item["actionType"]: item for item in result["actions"]}
    assert calls == ["exit"]
    assert by_type["exit_game"]["status"] == "skipped"
    assert by_type["exit_emulator"]["status"] == "skipped"
    assert by_type["exit_game"]["error"]["code"] == "ACTION_NOT_ALLOWED"
    assert by_type["power"]["status"] == "skipped"
    assert by_type["power"]["error"]["code"] == "INVALID_POWER_ACTION"


def test_request_mapping_accepts_after_completion_wrapper_and_snake_case_aliases() -> None:
    calls: list[str] = []
    coordinator = AfterCompletionCoordinator(notification_handler=lambda record: calls.append(record["actionType"]))

    result = coordinator.execute(
        {
            "run_id": "run-request",
            "outcome": "completed",
            "afterCompletion": {"actions": ["notify"]},
        }
    )

    assert calls == ["notification"]
    assert result["request"] is True
    assert result["actions"][0]["actionId"] == "run-request:notification"


def test_two_argument_journal_callback_receives_every_state_transition() -> None:
    states: list[tuple[str, str]] = []
    coordinator = AfterCompletionCoordinator(
        journal=lambda action_id, state: states.append((action_id, state)),
        notification_handler=lambda record: None,
    )

    coordinator.execute("run-journal", actions=["notification"])

    assert states == [
        ("run-journal:notification", "pending"),
        ("run-journal:notification", "executing"),
        ("run-journal:notification", "done"),
    ]
