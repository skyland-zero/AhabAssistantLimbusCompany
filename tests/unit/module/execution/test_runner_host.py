from __future__ import annotations

import importlib
import json
import logging
import os
from types import SimpleNamespace

import pytest

from module.execution.runner_host import RunnerTaskError, RunnerTaskHost
from module.execution.supervisor import ExecutionSpec


class _NoopBridge:
    def __init__(self) -> None:
        self.started = False
        self.closed = False

    def start(self) -> None:
        self.started = True

    def close(self) -> None:
        self.closed = True


def _run_host(host_factory):
    events: list[tuple[str, dict, bytes]] = []
    holder: dict[str, RunnerTaskHost] = {}

    def sink(message_type: str, fields: dict, payload: bytes) -> None:
        events.append((message_type, dict(fields), bytes(payload)))
        if message_type == "finished":
            holder["host"]._finish_ack.set()

    host = host_factory(sink)
    holder["host"] = host
    host.run()
    return host, events


def test_host_sets_spec_environment_binds_control_and_orders_final_events(monkeypatch) -> None:
    config_path = os.path.abspath("temporary-runner-config.yaml")
    resource_root = os.path.abspath("temporary-runner-resources")
    seen: dict[str, object] = {}

    def task_factory(context):
        seen["config_path"] = os.environ["AALC_CONFIG_PATH"]
        seen["resource_root"] = os.environ["AALC_RESOURCE_ROOT"]
        seen["run_id"] = os.environ["AALC_RUN_ID"]
        seen["runner_mode"] = os.environ["AALC_RUNNER_MODE"]
        seen["execution_runner"] = os.environ["AALC_EXECUTION_RUNNER"]
        seen["allow_emulator_launch"] = os.environ["AALC_ALLOW_EMULATOR_LAUNCH"]
        seen["runner_policy"] = json.loads(os.environ["AALC_RUNNER_POLICY"])
        seen["cancel_event"] = context.control.cancel_event
        context.defer_config_delta(
            "delta-1",
            base_revision=3,
            base_config_hash="sha256:test",
            changes={"hard_mirror_chance": 2},
        )
        context.defer_resource_released("adb.forward", "forward-1")
        context.after_completion(actions=["notify"], disposition="restore")
        return {"ok": True}

    monkeypatch.delenv("AALC_CONFIG_PATH", raising=False)
    monkeypatch.delenv("AALC_RESOURCE_ROOT", raising=False)
    monkeypatch.delenv("AALC_RUNNER_MODE", raising=False)
    monkeypatch.delenv("AALC_EXECUTION_RUNNER", raising=False)
    monkeypatch.delenv("AALC_ALLOW_EMULATOR_LAUNCH", raising=False)
    monkeypatch.delenv("AALC_RUNNER_POLICY", raising=False)
    spec = ExecutionSpec(
        run_id="run-host-1",
        task_id="mirror",
        config_path=config_path,
        resource_root=resource_root,
        platform_policy={"allowEmulatorLaunch": False},
    )
    bridge = _NoopBridge()
    host, events = _run_host(
        lambda sink: RunnerTaskHost(
            spec,
            task_factory=task_factory,
            mediator_bridge=bridge,
            event_sink=sink,
        )
    )

    assert seen == {
        "config_path": config_path,
        "resource_root": resource_root,
        "run_id": "run-host-1",
        "runner_mode": "1",
        "execution_runner": "1",
        "allow_emulator_launch": "0",
        "runner_policy": {
            "allowEmulatorLaunch": False,
            "runnerMode": True,
        },
        "cancel_event": host.control.cancel_event,
    }
    assert bridge.started is True
    assert bridge.closed is True
    assert [event[0] for event in events][-4:] == [
        "config.delta",
        "resource.released",
        "afterCompletion.requested",
        "finished",
    ]
    assert events[-1][1]["outcome"] == "completed"


def test_noop_explicitly_allows_missing_config_and_sets_runner_policy(monkeypatch) -> None:
    for name in (
        "AALC_CONFIG_PATH",
        "AALC_RESOURCE_ROOT",
        "AALC_RUNNER_MODE",
        "AALC_EXECUTION_RUNNER",
        "AALC_ALLOW_EMULATOR_LAUNCH",
        "AALC_RUNNER_POLICY",
    ):
        monkeypatch.delenv(name, raising=False)

    host = RunnerTaskHost(
        ExecutionSpec(run_id="run-host-noop", task_id="mirror", extra={"noOp": True}),
        mediator_bridge=_NoopBridge(),
        event_sink=lambda *_args: None,
    )
    host.initialize()
    try:
        assert os.environ["AALC_RUNNER_MODE"] == "1"
        assert os.environ["AALC_EXECUTION_RUNNER"] == "1"
        assert os.environ["AALC_ALLOW_EMULATOR_LAUNCH"] == "0"
        assert json.loads(os.environ["AALC_RUNNER_POLICY"]) == {
            "allowEmulatorLaunch": False,
            "runnerMode": True,
        }
        assert "AALC_CONFIG_PATH" not in os.environ
    finally:
        host.close()


def test_production_without_session_fails_closed_before_global_import(monkeypatch, tmp_path) -> None:
    for name in (
        "AALC_CONFIG_PATH",
        "AALC_RESOURCE_ROOT",
        "AALC_RUNNER_MODE",
        "AALC_EXECUTION_RUNNER",
        "AALC_ALLOW_EMULATOR_LAUNCH",
        "AALC_RUNNER_POLICY",
    ):
        monkeypatch.delenv(name, raising=False)

    imported: list[str] = []
    original_import = importlib.import_module

    def guarded_import(name: str, package: str | None = None):
        imported.append(name)
        return original_import(name, package)

    monkeypatch.setattr(importlib, "import_module", guarded_import)
    host = RunnerTaskHost(
        ExecutionSpec(
            run_id="run-host-no-session",
            task_id="mirror",
            config_path=str(tmp_path / "runner.yaml"),
            device_target={"id": "pc:missing", "name": "missing", "kind": "pc"},
        ),
        mediator_bridge=_NoopBridge(),
        event_sink=lambda *_args: None,
        device_runtime_factory=lambda *_args, **_kwargs: SimpleNamespace(reservation={}),
    )

    with pytest.raises(RunnerTaskError, match="private device session"):
        host.initialize()

    assert imported == []
    assert os.environ["AALC_RUNNER_MODE"] == "1"
    assert os.environ["AALC_EXECUTION_RUNNER"] == "1"
    assert os.environ["AALC_ALLOW_EMULATOR_LAUNCH"] == "0"
    host.close()


def test_production_without_config_path_fails_without_shared_cfg_import(monkeypatch) -> None:
    for name in (
        "AALC_CONFIG_PATH",
        "AALC_RESOURCE_ROOT",
        "AALC_RUNNER_MODE",
        "AALC_EXECUTION_RUNNER",
        "AALC_ALLOW_EMULATOR_LAUNCH",
        "AALC_RUNNER_POLICY",
    ):
        monkeypatch.delenv(name, raising=False)

    imported: list[str] = []
    original_import = importlib.import_module

    def guarded_import(name: str, package: str | None = None):
        imported.append(name)
        return original_import(name, package)

    monkeypatch.setattr(importlib, "import_module", guarded_import)
    host = RunnerTaskHost(
        ExecutionSpec(run_id="run-host-no-config", task_id="mirror"),
        mediator_bridge=_NoopBridge(),
        event_sink=lambda *_args: None,
    )

    with pytest.raises(RunnerTaskError, match="configPath"):
        host.initialize()

    assert imported == []
    assert os.environ["AALC_RUNNER_MODE"] == "1"
    assert os.environ["AALC_EXECUTION_RUNNER"] == "1"
    host.close()


def test_default_task_factory_is_lazy_and_uses_shared_cancel_event(monkeypatch) -> None:
    calls: list[tuple[str, str | None]] = []
    original_import = importlib.import_module

    class FakeTask:
        def __init__(self) -> None:
            self.cancel_event = None

        def run(self) -> dict[str, bool]:
            assert self.cancel_event is not None
            return {"sharedCancel": True}

    def fake_import(name: str, package: str | None = None):
        calls.append((name, os.environ.get("AALC_CONFIG_PATH")))
        if name == "tasks.base.script_task_scheme":
            return SimpleNamespace(my_script_task=FakeTask)
        return original_import(name, package)

    monkeypatch.setattr(importlib, "import_module", fake_import)
    spec = ExecutionSpec(
        run_id="run-host-2",
        task_id="mirror",
        config_path=os.path.abspath("runner.yaml"),
        device_target={"id": "pc:test", "name": "test", "kind": "pc"},
    )
    bridge = _NoopBridge()
    host, events = _run_host(
        lambda sink: RunnerTaskHost(
            spec,
            mediator_bridge=bridge,
            event_sink=sink,
            config={"ok": True},
            device_runtime_factory=lambda *_args, **_kwargs: SimpleNamespace(
                session=SimpleNamespace(controller=None),
                reservation={},
                close=lambda **_kwargs: True,
            ),
        )
    )

    assert any(name == "tasks.base.script_task_scheme" for name, _ in calls)
    task_import = next(path for name, path in calls if name == "tasks.base.script_task_scheme")
    assert task_import == os.path.abspath("runner.yaml")
    assert events[-1][0] == "finished"
    assert events[-1][1]["outcome"] == "completed"


def test_ipc_logging_handler_routes_records_to_event_sink() -> None:
    logger = logging.getLogger("runner-host-test")
    logger.setLevel(logging.INFO)
    host, events = _run_host(
        lambda sink: RunnerTaskHost(
            ExecutionSpec(run_id="run-host-3", task_id="mirror"),
            task_factory=lambda context: logger.warning("hello from task"),
            mediator_bridge=_NoopBridge(),
            event_sink=sink,
            config={"ok": True},
        )
    )

    del host
    log_events = [fields for message_type, fields, _ in events if message_type == "log.entry"]
    assert log_events and log_events[0]["message"] == "hello from task"
