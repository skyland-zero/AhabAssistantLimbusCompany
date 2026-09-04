from __future__ import annotations

import subprocess
import threading
import time
from types import SimpleNamespace

import pytest

import module.automation.input_handlers.simulator.mumu_control as mumu_module


class FakeAdbDevice:
    def __init__(self, *, fail: bool = False) -> None:
        self.fail = fail
        self.start_calls = 0

    def app_start(self, package_name: str) -> None:
        self.start_calls += 1
        if self.fail:
            raise RuntimeError(f"device for {package_name} not found")


def _controller_without_connect() -> object:
    controller = object.__new__(mumu_module.MumuControl)
    # These unit tests exercise the legacy sidecar adapter directly.  Keep
    # that contract explicit instead of inheriting a Runner env left by an
    # earlier integration test in the same pytest process.
    controller._runner_policy = mumu_module.RunnerPolicy(runner_mode=False, allow_emulator_launch=True)
    controller.device = None
    controller.package_list = True
    controller.start_game_times = 0
    controller.game_package_name = "com.ProjectMoon.LimbusCompany"
    controller.get_mumu_adb_port = lambda: "127.0.0.1:16384"
    return controller


def test_start_game_rebuilds_stale_adb_device_before_retry(monkeypatch) -> None:
    stale = FakeAdbDevice(fail=True)
    fresh = FakeAdbDevice()
    controller = _controller_without_connect()
    controller.device = stale
    adb_connect_calls = 0

    def adb_connect() -> None:
        nonlocal adb_connect_calls
        adb_connect_calls += 1

    controller.adb_connect = adb_connect
    monkeypatch.setattr(mumu_module.adb, "device", lambda endpoint: fresh)
    monkeypatch.setattr(controller, "interruptible_sleep", lambda _seconds: True)

    controller.start_game()

    assert stale.start_calls == 1
    assert fresh.start_calls == 1
    assert adb_connect_calls == 1
    assert controller.device is fresh
    assert controller.start_game_times == 0


def test_start_game_has_bounded_retries(monkeypatch) -> None:
    devices = [FakeAdbDevice(fail=True) for _ in range(3)]
    created_devices = devices.copy()
    controller = _controller_without_connect()
    adb_connect_calls = 0

    def adb_connect() -> None:
        nonlocal adb_connect_calls
        adb_connect_calls += 1

    controller.adb_connect = adb_connect
    monkeypatch.setattr(mumu_module.adb, "device", lambda endpoint: devices.pop(0))
    monkeypatch.setattr(controller, "interruptible_sleep", lambda _seconds: True)

    with pytest.raises(RuntimeError, match="无法启动 Limbus Company"):
        controller.start_game()

    assert adb_connect_calls == 3
    assert all(device.start_calls == 1 for device in created_devices)


def test_adb_disconnect_is_lazy_before_first_adb_use() -> None:
    controller = _controller_without_connect()
    controller._adb_connected = False
    controller.get_mumu_adb_port = lambda: pytest.fail("lazy cleanup queried the ADB port")

    controller.adb_disconnect()


def test_start_attaches_to_a_running_instance_without_launch(monkeypatch) -> None:
    controller = object.__new__(mumu_module.MumuControl)
    attached = 0

    def attach_existing() -> None:
        nonlocal attached
        attached += 1

    monkeypatch.setattr(controller, "can_attach_without_launch", lambda: True)
    monkeypatch.setattr(controller, "attach_existing", attach_existing)

    controller.start()

    assert attached == 1


def test_get_launch_status_rejects_a_missing_instance(monkeypatch) -> None:
    controller = _manager_controller()
    monkeypatch.setattr(
        mumu_module.subprocess,
        "run",
        lambda *_args, **_kwargs: SimpleNamespace(
            stdout='{"errcode": -200, "errmsg": "player index not found"}',
            returncode=1,
        ),
    )

    with pytest.raises(mumu_module.NemuIpcError, match="实例 0 不存在或不可用"):
        controller.get_launch_status()


def test_start_does_not_retry_a_missing_instance(monkeypatch) -> None:
    controller = object.__new__(mumu_module.MumuControl)
    controller.multi_instance_number = 0
    calls = 0

    def missing_instance():
        nonlocal calls
        calls += 1
        raise mumu_module.NemuIpcError("实例 0 不存在或不可用")

    monkeypatch.setattr(controller, "can_attach_without_launch", missing_instance)

    with pytest.raises(mumu_module.NemuIpcError, match="实例 0 不存在或不可用"):
        controller.start()

    assert calls == 1


def test_start_reports_a_timeout_instead_of_recursing(monkeypatch) -> None:
    controller = _manager_controller(instance=2)
    load_calls = 0

    monkeypatch.setattr(controller, "can_attach_without_launch", lambda: False)
    monkeypatch.setattr(controller, "get_app_keptlive", lambda: False)
    monkeypatch.setattr(controller, "get_launch_status", lambda: "not_launched")
    monkeypatch.setattr(mumu_module, "run_as_user", lambda _command: None)
    monkeypatch.setattr(controller, "interruptible_sleep", lambda _seconds: True)
    monkeypatch.setattr(
        mumu_module.cfg,
        "get_value",
        lambda key, default=None: 1 if key == "start_emulator_timeout" else default,
    )

    def load_dll() -> None:
        nonlocal load_calls
        load_calls += 1

    monkeypatch.setattr(controller, "load_dll", load_dll)

    with pytest.raises(mumu_module.NemuIpcError, match="实例 2 启动超时"):
        controller.start()

    assert load_calls == 0


def _manager_controller(*, path: str = "MuMuManager.exe", instance: object = 0) -> object:
    controller = object.__new__(mumu_module.MumuControl)
    controller._runner_policy = mumu_module.RunnerPolicy(runner_mode=False, allow_emulator_launch=True)
    controller.exe_path = path
    controller.multi_instance_number = instance
    return controller


def test_manager_commands_use_validated_argv_and_a_bounded_timeout(monkeypatch) -> None:
    calls: list[tuple[list[str], dict[str, object]]] = []

    def run(argv, **kwargs):
        calls.append((argv, kwargs))
        return SimpleNamespace(stdout='{"player_state":"start_finished"}', returncode=0)

    monkeypatch.setattr(mumu_module.subprocess, "run", run)
    controller = _manager_controller()

    assert controller.get_launch_status() == "start_finished"

    argv, kwargs = calls[0]
    assert argv == ["MuMuManager.exe", "info", "-v", "0"]
    assert kwargs["shell"] is False
    assert kwargs["timeout"] == controller._MUMU_MANAGER_TIMEOUT


def test_manager_command_rejects_path_and_instance_injection_before_spawn(monkeypatch) -> None:
    def spawn(*_args, **_kwargs):
        pytest.fail("invalid manager input must not spawn")

    monkeypatch.setattr(mumu_module.subprocess, "run", spawn)

    for controller in (
        _manager_controller(path="MuMuManager.exe;whoami"),
        _manager_controller(instance="0;whoami"),
    ):
        with pytest.raises(ValueError):
            controller.get_launch_status()


def test_manager_timeout_is_propagated_without_shell_fallback(monkeypatch) -> None:
    calls: list[tuple[list[str], dict[str, object]]] = []

    def run(argv, **kwargs):
        calls.append((argv, kwargs))
        raise subprocess.TimeoutExpired(argv, kwargs["timeout"])

    monkeypatch.setattr(mumu_module.subprocess, "run", run)
    controller = _manager_controller()

    with pytest.raises(subprocess.TimeoutExpired):
        controller.get_launch_status()

    assert calls[0][1]["shell"] is False
    assert calls[0][1]["timeout"] == controller._MUMU_MANAGER_TIMEOUT


def test_runner_attach_failure_is_fail_closed(monkeypatch) -> None:
    controller = _manager_controller(path="C:\\MuMu\\MuMuManager.exe")
    controller._runner_policy = mumu_module.RunnerPolicy(runner_mode=True)
    monkeypatch.setattr(controller, "is_running", lambda: True)
    monkeypatch.setattr(
        controller,
        "get_app_keptlive",
        lambda: (_ for _ in ()).throw(subprocess.TimeoutExpired(["MuMuManager.exe"], 10)),
    )

    with pytest.raises(mumu_module.RunnerDevicePolicyError, match="verify an existing MuMu"):
        controller.can_attach_without_launch()


def test_stop_request_wakes_interruptible_wait() -> None:
    controller = _manager_controller()
    result: list[bool] = []

    worker = threading.Thread(target=lambda: result.append(controller.interruptible_sleep(30)))
    worker.start()
    time.sleep(0.02)
    controller.request_stop()
    worker.join(timeout=1)

    assert not worker.is_alive()
    assert result == [False]


def test_mumu_click_releases_touch_when_wait_is_cancelled(monkeypatch) -> None:
    controller = _manager_controller()
    events: list[str] = []
    controller.down = lambda *_args: events.append("down")
    controller.up = lambda: events.append("up")
    monkeypatch.setattr(controller, "interruptible_sleep", lambda _seconds: False)

    controller.click(10, 20)

    assert events == ["down", "up"]
