from __future__ import annotations

import threading
import time
from typing import Any
from unittest.mock import Mock

import pytest

import module.device_manager as device_manager_module
from module.device_manager import (
    DeviceError,
    DeviceInfo,
    DeviceManager,
    DeviceSession,
    DeviceTarget,
    RunnerDeviceRuntime,
    create_runner_runtime,
    get_device_manager,
    get_runner_runtime,
    get_runner_session,
)


def _target_snapshot(kind: str, **values: Any) -> dict[str, Any]:
    target = DeviceTarget(
        DeviceInfo(values.pop("id", f"{kind}:test"), values.pop("name", "Runner target")),
        kind,  # type: ignore[arg-type]
        **values,
    )
    return target.to_snapshot()


@pytest.fixture(autouse=True)
def _clear_runner_runtime() -> None:
    previous = get_runner_runtime()
    if previous is not None:
        previous.close()
    yield
    current = get_runner_runtime()
    if current is not None:
        current.close()
    device_manager_module._runner_runtime = None


def test_runner_mumu_factory_attaches_existing_without_start(monkeypatch) -> None:
    import module.automation.input_handlers.simulator.mumu_control as mumu_module

    calls: list[dict[str, Any]] = []

    class FakeMumu:
        connection_device = None

        def __init__(self, **kwargs: Any) -> None:
            calls.append({"construct": kwargs})

        def can_attach_without_launch(self) -> bool:
            calls.append({"can_attach": True})
            return True

        def attach_existing(self) -> None:
            calls.append({"attach": True})

        def disconnect(self) -> None:
            calls.append({"disconnect": True})

        def adb_disconnect(self) -> None:
            calls.append({"adb_disconnect": True})

    monkeypatch.setattr(mumu_module, "MumuControl", FakeMumu)
    monkeypatch.setattr(DeviceManager, "_activate_runtime", lambda *_args, **_kwargs: None)

    runtime = create_runner_runtime(
        _target_snapshot("mumu", id="mumu:3", instance_number=3, endpoint="127.0.0.1:16480"),
        "run-mumu",
    )

    assert runtime.target.kind == "mumu"
    assert runtime.manager.active_session is runtime.session
    assert calls[0]["construct"]["auto_start"] is False
    assert calls[0]["construct"]["runner_policy"].runner_mode is True
    assert calls[0]["construct"]["runner_policy"].allow_emulator_launch is False
    assert {"attach": True} in calls
    assert not any("start" in call for call in calls)
    runtime.close(device_disposition="restore")


def test_runner_adb_factory_passes_run_scoped_scrcpy_reservation(monkeypatch) -> None:
    import module.automation.input_handlers.simulator.scrcpy_control as scrcpy_module

    created: list[dict[str, Any]] = []

    class FakeScrcpy:
        connection_device = None

        def __init__(self, **kwargs: Any) -> None:
            created.append(kwargs)
            self.session_reservation = {
                "runId": kwargs["run_id"],
                "scid": kwargs["scid"],
                "socketName": kwargs["socket_name"],
                "forwardPort": kwargs["reserved_forward_port"],
                "serial": kwargs["endpoint"],
            }
            self.cleanup_calls = 0

        def cleanup_session(self) -> None:
            self.cleanup_calls += 1

    monkeypatch.setattr(scrcpy_module, "ScrcpyControl", FakeScrcpy)
    monkeypatch.setattr(DeviceManager, "_activate_runtime", lambda *_args, **_kwargs: None)

    remote_cleanup = Mock()
    runtime = create_runner_runtime(
        _target_snapshot("adb", id="adb:127.0.0.1:5555", endpoint="127.0.0.1:5555"),
        "run-adb",
        {
            "reservedScrcpyScid": "0x1234abcd",
            "reservedSocketName": "scrcpy_1234abcd",
            "reservedAdbForwardPort": 40123,
            "serial": "127.0.0.1:5555",
        },
        remote_cleanup=remote_cleanup,
    )

    assert created[0]["run_id"] == "run-adb"
    assert created[0]["scid"] == "0x1234abcd"
    assert created[0]["socket_name"] == "scrcpy_1234abcd"
    assert created[0]["reserved_forward_port"] == 40123
    assert created[0]["runner_policy"].runner_mode is True
    assert get_runner_session() is runtime.session
    assert runtime.reservation["scid"] == "0x1234abcd"
    runtime.close(device_disposition="game_closed")
    assert runtime.close(device_disposition="game_closed")["alreadyClosed"] is True


def test_runner_pc_factory_uses_snapshot_without_discovery(monkeypatch) -> None:
    monkeypatch.setattr(DeviceManager, "_valid_hwnd", staticmethod(lambda _hwnd: True))
    monkeypatch.setattr(DeviceManager, "_activate_runtime", lambda *_args, **_kwargs: None)
    for name in ("_discover_pc_window", "_discover_mumu_targets", "_discover_adb_targets"):
        monkeypatch.setattr(DeviceManager, name, Mock(side_effect=AssertionError(f"unexpected discovery: {name}")))

    runtime = create_runner_runtime(
        _target_snapshot("pc", id="pc:limbus", hwnd=1234),
        "run-pc",
    )

    assert runtime.target.hwnd == 1234
    assert runtime.controller is None
    assert runtime.manager.active_session == DeviceSession(runtime.target, None)
    assert get_device_manager() is runtime.manager
    runtime.close(device_disposition="restore")


def test_runner_factory_allows_only_one_private_runtime(monkeypatch) -> None:
    monkeypatch.setattr(DeviceManager, "_valid_hwnd", staticmethod(lambda _hwnd: True))
    monkeypatch.setattr(DeviceManager, "_activate_runtime", lambda *_args, **_kwargs: None)
    first = create_runner_runtime(_target_snapshot("pc", id="pc:one", hwnd=1234), "run-one")

    with pytest.raises(DeviceError, match="已有 Runner 运行时"):
        create_runner_runtime(_target_snapshot("pc", id="pc:two", hwnd=5678), "run-two")

    assert get_runner_runtime() is first
    first.close()


def test_runner_runtime_close_is_bounded_and_reuses_pending_cleanup(monkeypatch) -> None:
    manager = DeviceManager()
    target = DeviceTarget(DeviceInfo("pc:bounded", "Runner window"), "pc", hwnd=1)
    runtime = RunnerDeviceRuntime(manager, "run-bounded", DeviceSession(target))
    entered = threading.Event()
    release = threading.Event()
    cleanup_calls = 0

    def cleanup(_session: DeviceSession) -> None:
        nonlocal cleanup_calls
        cleanup_calls += 1
        entered.set()
        release.wait(timeout=2)

    monkeypatch.setattr(manager, "_cleanup_session", cleanup)

    with pytest.raises(DeviceError, match="截止时间"):
        runtime.close(deadline=time.monotonic() + 0.03)
    assert entered.wait(timeout=1)
    assert runtime.closed is False

    release.set()
    result = runtime.close(deadline=time.monotonic() + 1.0)

    assert result["cleanup"] is True
    assert runtime.closed is True
    assert cleanup_calls == 1


def test_runner_screenshot_never_uses_class_level_stale_controller(monkeypatch) -> None:
    from module.automation.input_handlers.simulator.mumu_control import MumuControl
    from module.automation.screenshot import ScreenShot

    class StaleController:
        screenshot_calls = 0

        def screenshot(self):
            self.screenshot_calls += 1
            raise AssertionError("stale controller was used")

    stale = StaleController()
    monkeypatch.setattr(MumuControl, "connection_device", stale)
    monkeypatch.setenv("AALC_RUNNER_MODE", "1")
    monkeypatch.setattr(device_manager_module, "_runner_runtime", None)

    with pytest.raises(ConnectionError, match="私有设备运行时"):
        ScreenShot.take_screenshot()
    with pytest.raises(ConnectionError, match="私有 MuMu session"):
        ScreenShot.mumu_screenshot()
    assert stale.screenshot_calls == 0


def test_runner_adb_screenshot_never_uses_class_level_stale_controller(monkeypatch) -> None:
    from module.automation.input_handlers.simulator.scrcpy_control import ScrcpyControl
    from module.automation.screenshot import ScreenShot

    stale = Mock()
    monkeypatch.setattr(ScrcpyControl, "connection_device", stale)
    monkeypatch.setenv("AALC_RUNNER_MODE", "1")
    monkeypatch.setattr(device_manager_module, "_runner_runtime", None)

    with pytest.raises(ConnectionError, match="私有 ADB session"):
        ScreenShot.adb_screenshot()
    stale.screenshot.assert_not_called()


def test_runner_automation_init_rejects_legacy_fallback(monkeypatch) -> None:
    from module.automation import auto
    from module.automation.input_handlers.simulator.mumu_control import MumuControl

    stale = Mock()
    monkeypatch.setattr(MumuControl, "connection_device", stale)
    monkeypatch.setenv("AALC_RUNNER_MODE", "true")
    monkeypatch.setattr(device_manager_module, "_runner_runtime", None)

    old_handler = auto.input_handler
    old_set_pause = auto.set_pause
    old_wait_pause = auto.wait_pause
    old_memory_protection = auto.memory_protection
    try:
        with pytest.raises(DeviceError, match="私有设备 session"):
            auto.init_input()
    finally:
        auto.input_handler = old_handler
        auto.set_pause = old_set_pause
        auto.wait_pause = old_wait_pause
        auto.memory_protection = old_memory_protection
