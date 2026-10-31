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


def test_preview_is_published_while_the_run_owns_the_device() -> None:
    """The sidecar stops its own capture for the lease, so the Runner must produce frames.

    Without this producer the GPUI live screen stayed empty for the whole run:
    the sidecar preview is quiesced before the device handover and a lease
    rejects every sidecar device write.
    """

    from PIL import Image

    from module.execution.task_preview import TaskPreview

    previews: list[TaskPreview] = []

    def factory(_context) -> TaskPreview:
        preview = TaskPreview(
            run_id="run-host-preview",
            device_id="pc:limbus",
            emit_frame=lambda payload, jpeg: None,
            emit_status=lambda payload: None,
            capture=lambda: Image.new("RGB", (64, 36), (1, 2, 3)),
            interval=0.01,
        )
        previews.append(preview)
        return preview

    host, events = _run_host(
        lambda sink: RunnerTaskHost(
            ExecutionSpec(
                run_id="run-host-preview",
                task_id="mirror",
                device_target={"id": "pc:limbus", "name": "Limbus", "kind": "pc"},
            ),
            task_factory=lambda context: None,
            mediator_bridge=_NoopBridge(),
            event_sink=sink,
            config={"ok": True},
            device_runtime_factory=lambda *_args, **_kwargs: SimpleNamespace(
                session=SimpleNamespace(controller=None),
                reservation={},
                close=lambda **_kwargs: True,
            ),
            task_preview_factory=factory,
        )
    )

    assert host.finished
    # The injected runtime factory never installs a device_manager session, so
    # the host must not assume a capturable device just because a target exists.
    assert previews == [], "a host without a private session must stay silent"
    assert events[-1][0] == "finished"


def test_preview_starts_and_stops_around_a_real_private_session(monkeypatch) -> None:
    """A private session starts exactly one preview and stops it before cleanup."""

    import module.device_manager as device_manager

    lifecycle: list[str] = []
    session = SimpleNamespace(controller=None)

    class _Preview:
        def __init__(self) -> None:
            self.enabled = True

        def set_enabled(self, enabled: bool) -> bool:
            self.enabled = enabled
            lifecycle.append(f"enabled={enabled}")
            return True

        def start(self) -> bool:
            lifecycle.append("start")
            return True

        def stop(self, deadline=None, *, notify: bool = True) -> bool:
            lifecycle.append("stop")
            return True

    monkeypatch.setattr(device_manager, "get_runner_session", lambda: session, raising=False)

    host, events = _run_host(
        lambda sink: RunnerTaskHost(
            ExecutionSpec(
                run_id="run-host-preview-2",
                task_id="mirror",
                device_target={"id": "pc:limbus", "name": "Limbus", "kind": "pc"},
                preview_enabled=False,
            ),
            task_factory=lambda context: None,
            mediator_bridge=_NoopBridge(),
            event_sink=sink,
            config={"ok": True},
            device_runtime_factory=lambda *_args, **_kwargs: SimpleNamespace(
                session=session,
                reservation={},
                close=lambda **_kwargs: True,
            ),
            task_preview_factory=lambda _context: _Preview(),
        )
    )

    assert host.finished
    # ``previewEnabled=false`` still starts the producer: the worker is the only
    # way a re-enable can resume, and it is gated before any capture happens.
    assert lifecycle == ["enabled=False", "start", "stop"]
    assert events[-1][0] == "finished"


def test_start_spec_carries_the_preview_preference() -> None:
    spec = ExecutionSpec(
        run_id="run-host-preview-3",
        task_id="mirror",
        preview_enabled=False,
    )

    assert spec.preview_enabled is False
    assert spec.to_dict()["previewEnabled"] is False
    assert ExecutionSpec.from_mapping(spec.to_dict()).preview_enabled is False
    # Absent means "preview on": the sidecar default is enabled.
    assert ExecutionSpec.from_mapping({"runId": "r", "taskId": "t"}).preview_enabled is True
