from __future__ import annotations

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
    monkeypatch.setattr(mumu_module, "sleep", lambda _: None)

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
    monkeypatch.setattr(mumu_module, "sleep", lambda _: None)

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
