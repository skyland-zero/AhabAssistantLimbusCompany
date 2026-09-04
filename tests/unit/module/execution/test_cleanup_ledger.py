from __future__ import annotations

import json
from pathlib import Path

import pytest

from module.execution.cleanup_ledger import CleanupActionState, CleanupLedger, LedgerError


def test_resource_events_are_idempotent_and_clean_journal_can_be_removed(tmp_path: Path) -> None:
    path = tmp_path / "run.json"
    ledger = CleanupLedger.create(path, run_id="run-1")
    assert ledger.record_resource_created("adb.forward", "123") is True
    assert ledger.record_resource_created("adb.forward", "123") is False
    assert ledger.record_resource_released("adb.forward", "123") is True
    assert ledger.record_resource_released("adb.forward", "123") is False

    # All derived actions are already done; recovery must not require a platform
    # executor just to transition the journal to complete.
    result = ledger.recover(None, delete_complete=False)
    assert result.complete is True
    assert ledger.status == CleanupLedger.STATUS_COMPLETE
    assert path.exists()
    ledger.recover(None)
    assert not path.exists()


def test_recovery_reopens_executing_action_and_preserves_partial_failure(tmp_path: Path) -> None:
    path = tmp_path / "run.json"
    ledger = CleanupLedger.create(path, run_id="run-2")
    ledger.reserve(scid=42, socket_name="aalc-run-2", adb_forward_port=2345, device_serial="emulator-5554")
    action_ids = {item["actionType"]: item["actionId"] for item in ledger.actions}
    ledger.mark_action_executing(action_ids["scrcpy.server"])

    def executor(action, *, ledger):
        del ledger
        if action["actionType"] == "adb.forward":
            return {"status": "failed", "detail": {"code": "OFFLINE"}}
        return {"status": "done"}

    reopened = CleanupLedger.load(path)
    result = reopened.recover(executor, delete_complete=False)
    assert result.complete is False
    assert reopened.action(action_ids["scrcpy.server"])["state"] == CleanupActionState.DONE.value
    assert reopened.action(action_ids["adb.forward"])["state"] == CleanupActionState.FAILED.value
    assert reopened.action(action_ids["scrcpy.server"])["attempts"] == 2
    assert reopened.status == CleanupLedger.STATUS_PENDING


def test_invalid_or_duplicate_journal_entries_fail_closed(tmp_path: Path) -> None:
    path = tmp_path / "corrupt.json"
    path.write_text(
        json.dumps(
            {
                "formatVersion": 1,
                "runId": "run-3",
                "status": "cleanup_pending",
                "resources": [
                    {"resourceType": "x", "resourceId": "1", "released": False},
                    {"resourceType": "x", "resourceId": "1", "released": False},
                ],
            }
        ),
        encoding="utf-8",
    )
    with pytest.raises(LedgerError):
        CleanupLedger.load(path)


def test_complete_journal_with_pending_obligation_is_rejected(tmp_path: Path) -> None:
    path = tmp_path / "incomplete.json"
    ledger = CleanupLedger.create(path, run_id="run-3b")
    ledger.record_action_pending("action-1", action_type="window.restore")
    value = ledger.data
    value["status"] = "complete"
    path.write_text(json.dumps(value), encoding="utf-8")
    with pytest.raises(LedgerError):
        CleanupLedger.load(path)


def test_persist_failure_does_not_corrupt_previous_atomic_snapshot(tmp_path: Path, monkeypatch) -> None:
    path = tmp_path / "run.json"
    ledger = CleanupLedger.create(path, run_id="run-4")
    original = path.read_bytes()

    def fail_replace(source, destination):
        del source, destination
        raise OSError("replace failed")

    monkeypatch.setattr("module.execution.cleanup_ledger.os.replace", fail_replace)
    with pytest.raises(LedgerError):
        ledger.record_resource_created("x", "1")
    assert path.read_bytes() == original
    assert CleanupLedger.load(path).resources == []
