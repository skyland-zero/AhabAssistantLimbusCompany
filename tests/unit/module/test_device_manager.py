from __future__ import annotations

import json
import threading
import time

import pytest

from module.device_manager import DeviceError, DeviceInfo, DeviceManager, DeviceSession, DeviceTarget


def test_device_target_snapshot_round_trips_private_connection_fields() -> None:
    target = DeviceTarget(
        DeviceInfo("mumu:3", "MuMu 3", "127.0.0.1:16464"),
        "mumu",
        endpoint="127.0.0.1:16464",
        instance_number=3,
    )

    snapshot = target.to_snapshot()

    assert json.loads(json.dumps(snapshot)) == snapshot
    assert snapshot == {
        "id": "mumu:3",
        "name": "MuMu 3",
        "detail": "127.0.0.1:16464",
        "kind": "mumu",
        "endpoint": "127.0.0.1:16464",
        "instanceNumber": 3,
    }
    assert DeviceTarget.from_snapshot(snapshot) == target
    assert DeviceTarget.from_dict(snapshot) == target


def test_suspend_and_resume_lease_rejects_sidecar_writes_and_rebuilds_controller(monkeypatch) -> None:
    manager = DeviceManager()
    target = DeviceManager._make_mumu_target(0)
    old_controller = object()
    new_controller = object()
    manager._targets[target.info.id] = target
    manager._active = DeviceSession(target, old_controller)
    cleaned: list[DeviceSession] = []
    activated: list[DeviceSession] = []

    monkeypatch.setattr(manager, "_cleanup_session", lambda session: cleaned.append(session))
    monkeypatch.setattr(
        manager,
        "_open_target",
        lambda _target, **_kwargs: DeviceSession(target, new_controller),
    )
    monkeypatch.setattr(manager, "_activate_runtime", lambda session, **_kwargs: activated.append(session))

    lease = manager.suspend_for_execution("run-1", time.monotonic() + 2)

    assert lease["runId"] == "run-1"
    assert lease["generation"] == 1
    assert lease["state"] == "runner"
    assert lease["target"] == target.to_snapshot()
    assert cleaned == [DeviceSession(target, old_controller)]
    assert manager.lease_state == "runner"
    assert manager.active_session == DeviceSession(target, None)

    for write in (
        lambda: manager.connect(target.info.id),
        manager.disconnect,
        manager.reconnect_active,
        manager.bind_active_runtime,
    ):
        with pytest.raises(DeviceError, match="执行租约"):
            write()

    restored = manager.resume_after_execution("run-1", lease["generation"], deadline=time.monotonic() + 2)

    assert restored == {
        "status": "connected",
        "resumed": True,
        "deviceId": target.info.id,
        "runId": "run-1",
        "generation": 1,
    }
    assert manager.lease_state == "none"
    assert manager.execution_lease is None
    assert manager.active_session == DeviceSession(target, new_controller)
    assert activated == [DeviceSession(target, new_controller)]
    assert new_controller is not old_controller


def test_suspend_waits_for_inflight_manager_operation_and_stale_generation_is_rejected(monkeypatch) -> None:
    manager = DeviceManager()
    target = DeviceManager._make_mumu_target(1)
    manager._active = DeviceSession(target, object())
    manager._targets[target.info.id] = target
    monkeypatch.setattr(manager, "_cleanup_session", lambda _session: None)

    with manager._operation_condition:
        manager._device_operations = 1

    result: dict[str, object] = {}
    started = threading.Event()

    def suspend() -> None:
        started.set()
        result.update(manager.suspend_for_execution("run-2", time.monotonic() + 2))

    thread = threading.Thread(target=suspend, daemon=True)
    thread.start()
    assert started.wait(timeout=1)
    deadline = time.monotonic() + 1
    while manager.lease_state != "acquiring" and time.monotonic() < deadline:
        time.sleep(0.005)
    assert manager.lease_state == "acquiring"
    with manager._operation_condition:
        assert manager._device_operations == 1
        manager._device_operations = 0
        manager._operation_condition.notify_all()
    thread.join(timeout=1)

    assert not thread.is_alive()
    assert result["runId"] == "run-2"

    cancel = threading.Event()
    with manager._lock:
        manager._connecting = True
        manager._connect_generation = 9
        manager._connect_cancel = cancel
        manager._lease_state = "runner"
    assert not manager._connection_is_current_locked(9, cancel)
    assert not manager._connection_is_current_locked(8, cancel)


def test_suspend_timeout_keeps_lease_fail_closed_until_old_cleanup_finishes(monkeypatch) -> None:
    manager = DeviceManager()
    target = DeviceManager._make_mumu_target(2)
    manager._active = DeviceSession(target, object())
    manager._targets[target.info.id] = target
    cleanup_started = threading.Event()
    release_cleanup = threading.Event()

    def cleanup(_session: DeviceSession) -> None:
        cleanup_started.set()
        release_cleanup.wait(timeout=2)

    monkeypatch.setattr(manager, "_cleanup_session", cleanup)

    with pytest.raises(DeviceError, match="截止时间"):
        manager.suspend_for_execution("run-3", time.monotonic() + 0.03)
    assert cleanup_started.wait(timeout=1)
    assert manager.lease_state == "restoring"
    assert manager.lease_recovery == "restart_backend"
    assert manager.lease_error == {
        "code": "DEVICE_LEASE_FAILED",
        "message": "设备控制器未能在截止时间前退出",
        "recovery": "restart_backend",
    }
    with pytest.raises(DeviceError, match="执行租约"):
        manager.connect(target.info.id)
    with pytest.raises(DeviceError, match="禁止创建新控制器"):
        manager.resume_after_execution("run-3")

    release_cleanup.set()
    cleanup_thread = manager._lease_cleanup_thread
    assert cleanup_thread is not None
    cleanup_thread.join(timeout=1)
    assert not cleanup_thread.is_alive()


def test_suspend_timeout_marks_lease_terminal_and_rejects_new_controller(monkeypatch) -> None:
    """A failed acquisition keeps the old session but cannot resume over it."""

    manager = DeviceManager()
    target = DeviceManager._make_mumu_target(4)
    old_controller = object()
    manager._active = DeviceSession(target, old_controller)
    manager._targets[target.info.id] = target
    stale_cancel = threading.Event()
    with manager._operation_condition:
        manager._device_operations = 1
        manager._connecting = True
        manager._connect_generation = 10
        manager._connect_cancel = stale_cancel

    with pytest.raises(DeviceError, match="安静下来"):
        manager.suspend_for_execution("run-operation-race", time.monotonic() + 0.03)
    assert manager.lease_state == "restoring"
    assert manager.lease_recovery == "restart_backend"
    assert manager.active_session == DeviceSession(target, old_controller)
    assert stale_cancel.is_set()
    assert not manager._connection_is_current_locked(10, stale_cancel)

    opened = threading.Event()
    replacement = object()
    monkeypatch.setattr(
        manager,
        "_open_target",
        lambda _target, **_kwargs: (opened.set() or DeviceSession(target, replacement)),
    )
    with pytest.raises(DeviceError, match="禁止创建新控制器"):
        manager.resume_after_execution("run-operation-race", deadline=time.monotonic() + 2)
    assert not opened.is_set()

    activated: list[DeviceSession] = []
    monkeypatch.setattr(manager, "_activate_runtime", lambda session, **_kwargs: activated.append(session))
    stale_session = DeviceSession(target, replacement)
    assert not manager._finish_connection(stale_session, target.info.id, "stale", 10, stale_cancel)
    assert activated == []
    assert manager.active_session == DeviceSession(target, old_controller)

    with manager._operation_condition:
        manager._device_operations = 0
        manager._operation_condition.notify_all()


def test_resume_claims_restoring_transition_against_concurrent_caller(monkeypatch) -> None:
    """Only one caller may open a replacement controller for a lease."""

    manager = DeviceManager()
    target = DeviceManager._make_mumu_target(5)
    manager._active = DeviceSession(target, object())
    manager._targets[target.info.id] = target
    monkeypatch.setattr(manager, "_cleanup_session", lambda _session: None)
    manager.suspend_for_execution("run-resume-race", time.monotonic() + 2)

    opened = threading.Event()
    release_open = threading.Event()
    replacement = object()

    def open_target(_target: DeviceTarget, **_kwargs: object) -> DeviceSession:
        opened.set()
        assert release_open.wait(timeout=2)
        return DeviceSession(target, replacement)

    monkeypatch.setattr(manager, "_open_target", open_target)
    monkeypatch.setattr(manager, "_activate_runtime", lambda *_args, **_kwargs: None)
    result: dict[str, object] = {}

    thread = threading.Thread(
        target=lambda: result.update(
            manager.resume_after_execution("run-resume-race", deadline=time.monotonic() + 2)
        ),
        daemon=True,
    )
    thread.start()
    assert opened.wait(timeout=1)
    with pytest.raises(DeviceError, match="正在恢复"):
        manager.resume_after_execution("run-resume-race")

    release_open.set()
    thread.join(timeout=1)
    assert not thread.is_alive()
    assert result["status"] == "connected"
