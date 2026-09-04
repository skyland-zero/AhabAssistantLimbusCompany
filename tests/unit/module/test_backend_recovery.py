from __future__ import annotations

from pathlib import Path
from typing import Any

import pytest

from module import resource_cleanup
from module.backend_application import recover_pending_cleanups
from module.execution.cleanup_ledger import CleanupLedger
from tests.unit.module.test_backend_application import (
    FakeLeaseDeviceManager,
    FakePreviewCapture,
    FakeRunnerSupervisor,
    make_application,
)


class _CleanupFake:
    def __init__(self, statuses: list[str]) -> None:
        self.statuses = list(statuses)
        self.calls: list[str] = []

    def execute(self, action: dict[str, Any], *, ledger: Any = None) -> dict[str, str]:
        del ledger
        self.calls.append(str(action.get("actionId")))
        status = self.statuses.pop(0) if self.statuses else "success"
        return {"status": status}


class _DefaultFactoryExecutor:
    def __init__(self, status: str) -> None:
        self.status = status
        self.calls: list[str] = []

    def execute(self, action: dict[str, Any], *, ledger: Any = None) -> dict[str, str]:
        del ledger
        self.calls.append(str(action.get("actionId")))
        return {"status": self.status}


def test_runner_start_seeds_private_ledger_and_adapter(tmp_path: Path) -> None:
    manager = FakeLeaseDeviceManager()
    supervisor = FakeRunnerSupervisor()
    app = make_application(
        FakePreviewCapture(),
        stats_path=tmp_path / "stats.json",
        device_manager=manager,
        runner_supervisor=supervisor,
        runner_enabled=True,
    )
    app._cleanup_ledger_root = tmp_path / "journals"
    app.config.values["mirror"] = True

    run_id = app.execution_start({"taskId": "mirror"})["runId"]
    path = tmp_path / "journals" / f"{run_id}.json"
    ledger = CleanupLedger.load(path)

    assert ledger.target_id == "pc:limbus"
    assert ledger.data["target"]["id"] == "pc:limbus"
    assert ledger.reservation["generation"] == 7
    assert app._runner_event_adapter is not None
    assert app._runner_event_adapter.ledger is app._cleanup_ledgers[run_id]
    app.close()


def test_runner_process_identity_is_persisted_and_reused_pid_is_not_terminated(tmp_path: Path) -> None:
    manager = FakeLeaseDeviceManager()
    supervisor = FakeRunnerSupervisor()
    app = make_application(
        FakePreviewCapture(),
        stats_path=tmp_path / "stats.json",
        device_manager=manager,
        runner_supervisor=supervisor,
        runner_enabled=True,
    )
    app._cleanup_ledger_root = tmp_path / "journals"
    app.config.values["mirror"] = True
    run_id = app.execution_start({"taskId": "mirror"})["runId"]

    supervisor._set(runnerPid=1234, runnerCreateTime=12.5)
    app.execution_get_state()
    ledger = CleanupLedger.load(tmp_path / "journals" / f"{run_id}.json")
    assert ledger.reservation["runnerPid"] == 1234
    assert ledger.reservation["runnerCreateTime"] == 12.5
    app.close()

    terminator_calls: list[int] = []
    cleanup = _CleanupFake([])
    results = recover_pending_cleanups(
        tmp_path / "journals",
        cleanup_executor=cleanup,
        process_identity_probe=lambda pid: {"alive": True, "createTime": 99.0},
        process_terminator=lambda pid, _create_time: terminator_calls.append(pid),
    )
    assert terminator_calls == []
    assert results and results[0]["complete"] is True


def test_normal_finalize_runs_injected_cleanup_and_removes_journal(tmp_path: Path) -> None:
    manager = FakeLeaseDeviceManager()
    supervisor = FakeRunnerSupervisor()
    cleanup = _CleanupFake(["success"])
    app = make_application(
        FakePreviewCapture(),
        stats_path=tmp_path / "stats.json",
        device_manager=manager,
        runner_supervisor=supervisor,
        runner_enabled=True,
    )
    app._cleanup_ledger_root = tmp_path / "journals"
    app._cleanup_executor = cleanup
    app.config.values["mirror"] = True
    run_id = app.execution_start({"taskId": "mirror"})["runId"]
    ledger = app._cleanup_ledgers[run_id]
    ledger.update_metadata({"target": {"id": "pc:limbus", "kind": "pc"}})
    ledger.record_resource_created("test.resource", "owned", metadata={})

    app._finalize_runner_execution(
        run_id,
        snapshot={"state": "idle", "runId": run_id, "outcome": "completed", "forced": False},
    )

    assert cleanup.calls == ["resource:test.resource:owned"]
    assert not (tmp_path / "journals" / f"{run_id}.json").exists()
    app.close()


def test_startup_recovery_marks_crashed_and_retains_disconnected_pending_journal(tmp_path: Path) -> None:
    path = tmp_path / "journals" / "run.json"
    ledger = CleanupLedger(path, run_id="run")
    ledger.update_metadata({"target": {"id": "adb:phone", "kind": "adb", "endpoint": "phone"}})
    ledger.record_resource_created("scrcpy_server", "server", metadata={"scid": "scoped"})
    cleanup = _CleanupFake(["deferred"])

    first = recover_pending_cleanups(tmp_path / "journals", cleanup_executor=cleanup)
    persisted = CleanupLedger.load(path)
    assert first[0]["pending"] is True
    assert first[0]["deviceDisposition"] == "disconnected"
    assert persisted.finished["outcome"] == "crashed"
    assert persisted.finished["forced"] is True

    cleanup.statuses.append("success")
    second = recover_pending_cleanups(tmp_path / "journals", cleanup_executor=cleanup)
    assert second[0]["complete"] is True
    assert not path.exists()


def test_startup_recovery_reports_and_removes_already_complete_journal(tmp_path: Path) -> None:
    path = tmp_path / "journals" / "complete.json"
    ledger = CleanupLedger(path, run_id="complete")
    ledger.mark_complete()

    first = recover_pending_cleanups(tmp_path / "journals", cleanup_executor=_CleanupFake([]))
    assert first[0]["runId"] == "complete"
    assert first[0]["complete"] is True
    assert not path.exists()
    assert recover_pending_cleanups(tmp_path / "journals") == []


def test_default_cleanup_factory_is_bound_per_run_and_normal_finalize_deletes_journal(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    factory_calls: list[tuple[str, Any]] = []
    executors: list[_DefaultFactoryExecutor] = []

    def factory(run_id: str, record: Any) -> _DefaultFactoryExecutor:
        factory_calls.append((run_id, record))
        executor = _DefaultFactoryExecutor("success")
        executors.append(executor)
        return executor

    monkeypatch.setattr(resource_cleanup, "build_default_cleanup_executor", factory)
    app = make_application(
        FakePreviewCapture(),
        stats_path=tmp_path / "stats.json",
        device_manager=FakeLeaseDeviceManager(),
        runner_supervisor=FakeRunnerSupervisor(),
        runner_enabled=True,
    )
    app._cleanup_ledger_root = tmp_path / "journals"
    app.config.values["mirror"] = True
    run_id = app.execution_start({"taskId": "mirror"})["runId"]
    ledger = app._cleanup_ledgers[run_id]
    ledger.record_resource_created("test.resource", "owned", metadata={})

    app._finalize_runner_execution(
        run_id,
        snapshot={"state": "idle", "runId": run_id, "outcome": "completed", "forced": False},
    )

    assert len(factory_calls) == 1
    assert factory_calls[0][0] == run_id
    assert factory_calls[0][1] is ledger
    assert executors[0].calls == ["resource:test.resource:owned"]
    assert not (tmp_path / "journals" / f"{run_id}.json").exists()
    app.close()


@pytest.mark.parametrize(("outcome", "forced"), [("crashed", True), ("failed", True)])
def test_default_cleanup_factory_runs_for_forced_finalize_without_completion_actions(
    tmp_path: Path,
    monkeypatch: pytest.MonkeyPatch,
    outcome: str,
    forced: bool,
) -> None:
    factory_calls: list[str] = []

    def factory(run_id: str, record: Any) -> _DefaultFactoryExecutor:
        del record
        factory_calls.append(run_id)
        return _DefaultFactoryExecutor("success")

    monkeypatch.setattr(resource_cleanup, "build_default_cleanup_executor", factory)
    from tests.unit.module.test_backend_application import FakeAfterCompletionCoordinator

    coordinator = FakeAfterCompletionCoordinator()
    app = make_application(
        FakePreviewCapture(),
        stats_path=tmp_path / f"stats-{outcome}.json",
        device_manager=FakeLeaseDeviceManager(),
        runner_supervisor=FakeRunnerSupervisor(),
        runner_enabled=True,
        after_completion_coordinator=coordinator,
    )
    app._cleanup_ledger_root = tmp_path / f"journals-{outcome}"
    app.config.values["mirror"] = True
    run_id = app.execution_start({"taskId": "mirror"})["runId"]
    app._cleanup_ledgers[run_id].record_resource_created("test.resource", "owned", metadata={})

    app._finalize_runner_execution(
        run_id,
        snapshot={"state": "idle", "runId": run_id, "outcome": outcome, "forced": forced},
    )

    assert factory_calls == [run_id]
    assert coordinator.requests == []
    assert not (tmp_path / f"journals-{outcome}" / f"{run_id}.json").exists()
    app.close()


def test_default_cleanup_factory_deferred_recovery_is_pending_then_rebuilt_per_run(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    path = tmp_path / "journals" / "run.json"
    ledger = CleanupLedger(path, run_id="run")
    ledger.update_metadata({"target": {"id": "adb:phone", "kind": "adb", "endpoint": "phone"}})
    ledger.record_resource_created("scrcpy_server", "server", metadata={"scid": "0x1234"})
    statuses = ["deferred", "success"]
    factory_calls: list[tuple[str, Any]] = []

    def factory(run_id: str, record: Any) -> _DefaultFactoryExecutor:
        factory_calls.append((run_id, record))
        return _DefaultFactoryExecutor(statuses.pop(0))

    monkeypatch.setattr(resource_cleanup, "build_default_cleanup_executor", factory)

    first = recover_pending_cleanups(tmp_path / "journals")
    assert first[0]["pending"] is True
    assert first[0]["deviceDisposition"] == "disconnected"
    assert path.exists()
    assert CleanupLedger.load(path).finished["outcome"] == "crashed"

    second = recover_pending_cleanups(tmp_path / "journals")
    assert second[0]["complete"] is True
    assert not path.exists()
    assert [run_id for run_id, _record in factory_calls] == ["run", "run"]
    assert factory_calls[0][1] is not factory_calls[1][1]
