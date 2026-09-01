from __future__ import annotations

import json
import threading
from types import SimpleNamespace
from typing import Any

import numpy as np

import module.device_manager as device_manager_module
from module.automation.input_handlers import AbstractInput
from module.device_manager import DeviceInfo, DeviceManager, DeviceSession, DeviceTarget
from module.rpc_dispatcher import RpcDispatcher


class StubDeviceManager:
    def __init__(self) -> None:
        self.busy_checker = lambda: False
        self.connected: str | None = None

    def set_busy_checker(self, checker):
        self.busy_checker = checker

    def list_devices(self) -> list[dict[str, Any]]:
        return [DeviceInfo("pc:limbus", "Limbus Company", "Windows").to_dict()]

    def connect(self, device_id: str) -> dict[str, Any]:
        if self.busy_checker():
            raise RuntimeError("busy")
        self.connected = device_id
        return {"deviceId": device_id, "status": "connected"}

    def disconnect(self) -> dict[str, Any]:
        self.connected = None
        return {"status": "disconnected"}


class RecordingController(AbstractInput):
    def __init__(self) -> None:
        super().__init__()
        self.screenshot_calls = 0
        self.disconnect_calls = 0
        self.adb_disconnect_calls = 0

    def screenshot(self):
        self.screenshot_calls += 1
        return np.zeros((2, 3, 3), dtype=np.uint8)

    def disconnect(self) -> None:
        self.disconnect_calls += 1

    def adb_disconnect(self) -> None:
        self.adb_disconnect_calls += 1


def test_device_ids_are_stable_and_do_not_contain_hwnd() -> None:
    target = DeviceManager._make_mumu_target(2)
    assert target.info.id == "mumu:2"
    assert target.endpoint == "127.0.0.1:16448"


def test_mumu_discovery_uses_existing_instances_from_manager(monkeypatch) -> None:
    manager = DeviceManager()
    payload = {
        "1": {
            "index": "1",
            "name": "MuMu安卓设备-1",
            "player_state": "start_finished",
            "adb_host_ip": "127.0.0.1",
            "adb_port": 16416,
        }
    }
    calls: list[list[str]] = []

    def run(command, **_kwargs):
        calls.append(command)
        return SimpleNamespace(returncode=0, stdout=json.dumps(payload), stderr="")

    monkeypatch.setattr(DeviceManager, "_find_mumu_manager", staticmethod(lambda: "MuMuManager.exe"))
    monkeypatch.setattr(device_manager_module.subprocess, "run", run)

    targets = manager._discover_mumu_targets()

    assert [target.info.id for target in targets] == ["mumu:1"]
    assert targets[0].info.name == "MuMu 模拟器 #1"
    assert targets[0].endpoint == "127.0.0.1:16416"
    assert calls == [["MuMuManager.exe", "info", "-v", "all"]]


def test_mumu_discovery_supports_multiple_and_stopped_instances(monkeypatch) -> None:
    manager = DeviceManager()
    payload = {
        "0": {
            "index": "0",
            "errcode": -200,
            "errmsg": "player index not found",
        },
        "2": {
            "index": "2",
            "player_state": "shutdown",
            "adb_host_ip": "127.0.0.1",
            "adb_port": 16448,
        },
        "3": {
            "index": "3",
            "player_state": "start_finished",
            "adb_host_ip": "127.0.0.1",
            "adb_port": 16480,
        },
    }

    monkeypatch.setattr(DeviceManager, "_find_mumu_manager", staticmethod(lambda: "MuMuManager.exe"))
    monkeypatch.setattr(
        device_manager_module.subprocess,
        "run",
        lambda *_args, **_kwargs: SimpleNamespace(returncode=0, stdout=json.dumps(payload), stderr=""),
    )

    targets = manager._discover_mumu_targets()

    assert [target.info.id for target in targets] == ["mumu:2", "mumu:3"]
    assert [target.endpoint for target in targets] == [
        "127.0.0.1:16448",
        "127.0.0.1:16480",
    ]


def test_mumu_discovery_does_not_fabricate_configured_instance_on_manager_error(monkeypatch) -> None:
    manager = DeviceManager()
    calls: list[list[str]] = []

    def run(command, **_kwargs):
        calls.append(command)
        return SimpleNamespace(
            returncode=1,
            stdout=json.dumps({"errcode": -200, "errmsg": "player index not found"}),
            stderr="",
        )

    monkeypatch.setattr(DeviceManager, "_find_mumu_manager", staticmethod(lambda: "MuMuManager.exe"))
    monkeypatch.setattr(device_manager_module.subprocess, "run", run)

    assert manager._discover_mumu_targets() == []
    assert calls == [
        ["MuMuManager.exe", "info", "-v", "all"],
        ["MuMuManager.exe", "info", "-v", "0"],
    ]


def test_connection_methods_identify_mumu_ipc_and_adb_paths() -> None:
    manager = DeviceManager()

    assert manager._connection_methods(DeviceManager._make_mumu_target(0)) == (
        "MuMu IPC（NemuIpc）+ ADB 辅助",
        "MuMu IPC（NemuIpc）",
        "MuMu IPC（NemuIpc）",
    )
    assert manager._connection_methods(DeviceManager._make_adb_target("127.0.0.1:5555")) == (
        "ADB（127.0.0.1:5555）",
        "ADB screencap",
        "ADB shell input + minitouch",
    )


def test_connection_methods_identify_windows_defaults(monkeypatch) -> None:
    values = {"background_click": True, "win_input_type": "background"}
    monkeypatch.setattr(
        device_manager_module.cfg,
        "get_value",
        lambda key, default=None: values.get(key, default),
    )
    target = DeviceTarget(DeviceInfo("pc:limbus", "Limbus Company"), "pc")

    assert DeviceManager._connection_methods(target) == (
        "Windows 游戏窗口",
        "默认窗口截图（PrintWindow）",
        "Windows 后台输入（pywin32）",
    )


def test_dispatcher_uses_canonical_device_methods() -> None:
    manager = StubDeviceManager()
    dispatcher = RpcDispatcher(manager, version="test")

    listed = dispatcher.dispatch({"jsonrpc": "2.0", "id": 1, "method": "device.list"})
    assert listed["result"][0]["id"] == "pc:limbus"

    connected = dispatcher.dispatch(
        {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "device.connect",
            "params": {"id": "pc:limbus"},
        }
    )
    assert connected["result"]["deviceId"] == "pc:limbus"
    assert manager.connected == "pc:limbus"


def test_dispatcher_rejects_invalid_device_params() -> None:
    dispatcher = RpcDispatcher(StubDeviceManager(), version="test")
    response = dispatcher.dispatch({"jsonrpc": "2.0", "id": 3, "method": "device.connect", "params": {}})
    assert response["error"]["code"] == -32602


def test_dispatcher_rejects_unknown_methods() -> None:
    dispatcher = RpcDispatcher(StubDeviceManager(), version="test")
    response = dispatcher.dispatch({"jsonrpc": "2.0", "id": 4, "method": "not.implemented"})
    assert response["error"]["code"] == -32601


def test_active_session_is_authoritative_for_input_and_screenshot(monkeypatch) -> None:
    from module.automation import auto
    from module.automation.input_handlers.simulator.mumu_control import MumuControl
    from module.automation.screenshot import ScreenShot

    manager = DeviceManager()
    selected = RecordingController()
    stale = RecordingController()
    session = DeviceSession(DeviceManager._make_mumu_target(2), selected)
    manager._active = session
    monkeypatch.setattr(device_manager_module, "_default_manager", manager)
    monkeypatch.setattr(MumuControl, "connection_device", stale)

    old_handler = auto.input_handler
    old_set_pause = auto.set_pause
    old_wait_pause = auto.wait_pause
    old_memory_protection = auto.memory_protection
    try:
        auto.init_input(session=session)
        assert auto.input_handler is selected

        image = ScreenShot.take_screenshot(gray=False)
        assert image is not None
        assert image.size == (3, 2)
        assert selected.screenshot_calls == 1
        assert stale.screenshot_calls == 0
    finally:
        auto.input_handler = old_handler
        auto.set_pause = old_set_pause
        auto.wait_pause = old_wait_pause
        auto.memory_protection = old_memory_protection


def test_active_mumu_target_keeps_instance_and_endpoint(monkeypatch) -> None:
    import module.automation.input_handlers.simulator.mumu_control as mumu_module
    import module.automation.input_handlers.simulator.simulator_control as adb_module

    class FakeMumu:
        def __init__(self, *, instance_number: int) -> None:
            self.instance_number = instance_number

    class FakeAdb:
        def __init__(self, *, endpoint: str) -> None:
            self.endpoint = endpoint

    manager = DeviceManager()
    runtime_values: list[dict[str, Any]] = []
    monkeypatch.setattr(manager, "_set_runtime_config", lambda **values: runtime_values.append(values))
    monkeypatch.setattr(mumu_module, "MumuControl", FakeMumu)
    monkeypatch.setattr(adb_module, "SimulatorControl", FakeAdb)

    mumu_session = manager._open_target(DeviceManager._make_mumu_target(3))
    assert mumu_session.target.info.id == "mumu:3"
    assert mumu_session.target.endpoint == "127.0.0.1:16480"
    assert mumu_session.controller.instance_number == 3
    assert runtime_values[-1]["simulator_port"] == 16480

    adb_session = manager._open_target(DeviceManager._make_adb_target("127.0.0.1:5566"))
    assert adb_session.target.info.id == "adb:127.0.0.1:5566"
    assert adb_session.controller.endpoint == "127.0.0.1:5566"
    assert runtime_values[-1]["simulator_port"] == 5566


def test_reconnecting_same_selected_id_rebinds_existing_session(monkeypatch) -> None:
    manager = DeviceManager()
    controller = RecordingController()
    target = DeviceManager._make_mumu_target(1)
    session = DeviceSession(target, controller)
    manager._active = session
    manager._targets[target.info.id] = target
    rebound: list[DeviceSession] = []
    monkeypatch.setattr(manager, "_activate_runtime", rebound.append)

    result = manager.connect(target.info.id)

    assert result["alreadyConnected"] is True
    assert rebound == [session]
    assert manager.active_session is session


def test_running_mumu_uses_fast_ipc_attach(monkeypatch) -> None:
    import module.automation.input_handlers.simulator.mumu_control as mumu_module

    class FakeMumu:
        connection_device = None

        def __init__(self, *, instance_number: int, auto_start: bool = True) -> None:
            self.instance_number = instance_number
            self.auto_start = auto_start
            self.attach_calls = 0
            self.start_calls = 0
            self.disconnect_calls = 0
            self.adb_disconnect_calls = 0
            created.append(self)

        def can_attach_without_launch(self) -> bool:
            return True

        def attach_existing(self) -> None:
            self.attach_calls += 1

        def start(self) -> None:
            self.start_calls += 1

        def disconnect(self) -> None:
            self.disconnect_calls += 1

        def adb_disconnect(self) -> None:
            self.adb_disconnect_calls += 1

    manager = DeviceManager()
    target = DeviceManager._make_mumu_target(0)
    manager._targets[target.info.id] = target
    created: list[FakeMumu] = []
    activated: list[DeviceSession] = []

    monkeypatch.setattr(mumu_module, "MumuControl", FakeMumu)
    monkeypatch.setattr(manager, "_set_runtime_config", lambda **values: None)
    monkeypatch.setattr(manager, "_activate_runtime", activated.append)

    result = manager.connect(target.info.id)

    assert result == {"deviceId": "mumu:0", "status": "connected"}
    assert len(created) == 1
    assert created[0].auto_start is False
    assert created[0].attach_calls == 1
    assert created[0].start_calls == 0
    assert activated == [manager.active_session]

    manager.close()


def test_cold_mumu_start_is_accepted_and_finishes_in_background(monkeypatch) -> None:
    import module.automation.input_handlers.simulator.mumu_control as mumu_module

    started = threading.Event()
    release = threading.Event()

    class FakeMumu:
        connection_device = None

        def __init__(self, *, instance_number: int, auto_start: bool = True) -> None:
            self.instance_number = instance_number
            self.auto_start = auto_start
            self.start_calls = 0
            self.disconnect_calls = 0
            self.adb_disconnect_calls = 0

        def can_attach_without_launch(self) -> bool:
            return False

        def start(self) -> None:
            self.start_calls += 1
            started.set()
            release.wait(timeout=2)

        def disconnect(self) -> None:
            self.disconnect_calls += 1

        def adb_disconnect(self) -> None:
            self.adb_disconnect_calls += 1

    manager = DeviceManager()
    target = DeviceManager._make_mumu_target(1)
    manager._targets[target.info.id] = target
    connected = threading.Event()
    events: list[tuple[str, dict[str, Any]]] = []
    manager.add_status_listener(
        lambda event, payload: (
            events.append((event, payload)),
            connected.set() if payload.get("status") == "connected" else None,
        )
    )
    activated: list[DeviceSession] = []

    monkeypatch.setattr(mumu_module, "MumuControl", FakeMumu)
    monkeypatch.setattr(manager, "_set_runtime_config", lambda **values: None)
    monkeypatch.setattr(manager, "_activate_runtime", activated.append)

    result = manager.connect(target.info.id)

    assert result["deviceId"] == target.info.id
    assert result["status"] == "connecting"
    assert result["accepted"] is True
    assert isinstance(result["runId"], str) and result["runId"]
    assert started.wait(timeout=1)
    assert manager.active_session is None
    assert events == [
        (
            "device.status",
            {
                "deviceId": target.info.id,
                "status": "connecting",
                "runId": result["runId"],
            },
        )
    ]

    release.set()
    assert connected.wait(timeout=1)
    assert manager.active_session is not None
    assert activated == [manager.active_session]
    assert events[-1] == (
        "device.status",
        {
            "deviceId": target.info.id,
            "status": "connected",
            "runId": result["runId"],
        },
    )

    manager.close()


def test_cold_mumu_failure_reports_disconnected_asynchronously(monkeypatch) -> None:
    import module.automation.input_handlers.simulator.mumu_control as mumu_module

    failed = threading.Event()

    class FakeMumu:
        connection_device = None

        def __init__(self, *, instance_number: int, auto_start: bool = True) -> None:
            self.disconnect_calls = 0
            self.adb_disconnect_calls = 0

        def can_attach_without_launch(self) -> bool:
            return False

        def start(self) -> None:
            raise RuntimeError("simulated MuMu launch failure")

        def disconnect(self) -> None:
            self.disconnect_calls += 1

        def adb_disconnect(self) -> None:
            self.adb_disconnect_calls += 1

    manager = DeviceManager()
    target = DeviceManager._make_mumu_target(2)
    manager._targets[target.info.id] = target
    statuses: list[dict[str, Any]] = []
    notices: list[dict[str, Any]] = []
    manager.add_status_listener(
        lambda _event, payload: (
            statuses.append(payload),
            failed.set() if payload["status"] == "disconnected" else None,
        )
    )
    manager.add_notice_listener(lambda _event, payload: notices.append(payload))

    monkeypatch.setattr(mumu_module, "MumuControl", FakeMumu)
    monkeypatch.setattr(manager, "_set_runtime_config", lambda **values: None)
    monkeypatch.setattr(manager, "_activate_runtime", lambda _session: None)

    result = manager.connect(target.info.id)

    assert result["accepted"] is True
    assert failed.wait(timeout=1)
    assert manager.active_session is None
    assert statuses[0]["status"] == "connecting"
    assert statuses[-1]["status"] == "disconnected"
    assert any("simulated MuMu launch failure" in notice["message"] for notice in notices)

    manager.close()


def test_disconnect_cancels_pending_mumu_without_late_connected_event(monkeypatch) -> None:
    import module.automation.input_handlers.simulator.mumu_control as mumu_module

    started = threading.Event()
    release = threading.Event()
    cancellation_started = threading.Event()
    created: list[Any] = []

    class FakeMumu:
        connection_device = None

        def __init__(self, *, instance_number: int, auto_start: bool = True) -> None:
            self.disconnect_calls = 0
            self.adb_disconnect_calls = 0
            created.append(self)

        def can_attach_without_launch(self) -> bool:
            return False

        def start(self) -> None:
            started.set()
            release.wait(timeout=2)

        def disconnect(self) -> None:
            self.disconnect_calls += 1

        def adb_disconnect(self) -> None:
            self.adb_disconnect_calls += 1

    manager = DeviceManager()
    target = DeviceManager._make_mumu_target(3)
    manager._targets[target.info.id] = target
    statuses: list[str] = []
    manager.add_status_listener(lambda _event, payload: statuses.append(payload["status"]))
    monkeypatch.setattr(mumu_module, "MumuControl", FakeMumu)
    monkeypatch.setattr(manager, "_set_runtime_config", lambda **values: None)
    monkeypatch.setattr(manager, "_activate_runtime", lambda _session: None)

    original_cancel = manager._cancel_pending_connection

    def cancel_pending() -> threading.Thread | None:
        thread = original_cancel()
        cancellation_started.set()
        return thread

    monkeypatch.setattr(manager, "_cancel_pending_connection", cancel_pending)
    result = manager.connect(target.info.id)
    assert result["accepted"] is True
    assert started.wait(timeout=1)

    disconnect_result: dict[str, Any] = {}
    disconnect_thread = threading.Thread(
        target=lambda: disconnect_result.update(manager.disconnect()),
        daemon=True,
    )
    disconnect_thread.start()
    assert cancellation_started.wait(timeout=1)
    release.set()
    disconnect_thread.join(timeout=1)

    assert not disconnect_thread.is_alive()
    assert disconnect_result == {"status": "disconnected"}
    assert manager.active_session is None
    assert statuses == ["connecting", "disconnected"]
    assert created[0].disconnect_calls == 1
    assert created[0].adb_disconnect_calls == 1

    manager.close()


def test_selected_device_kind_overrides_legacy_simulator_mode(monkeypatch) -> None:
    from module.device_manager import is_simulator_runtime

    manager = DeviceManager()
    monkeypatch.setattr(device_manager_module, "_default_manager", manager)
    manager._active = DeviceSession(
        DeviceTarget(DeviceInfo("pc:limbus", "Limbus Company"), "pc"),
    )
    assert is_simulator_runtime() is False

    manager._active = DeviceSession(DeviceManager._make_mumu_target(1), RecordingController())
    assert is_simulator_runtime() is True


def test_release_after_task_cleans_session_and_publishes_disconnect(monkeypatch) -> None:
    from module.automation.input_handlers.simulator.mumu_control import MumuControl

    manager = DeviceManager()
    controller = RecordingController()
    target = DeviceManager._make_mumu_target(0)
    manager._active = DeviceSession(target, controller)
    events: list[tuple[str, dict[str, Any]]] = []
    manager.add_status_listener(lambda event, payload: events.append((event, payload)))
    monkeypatch.setattr(MumuControl, "connection_device", controller)

    result = manager.release_after_task()

    assert result == {"status": "disconnected"}
    assert manager.active_session is None
    assert controller.disconnect_calls == 1
    assert controller.adb_disconnect_calls == 1
    assert events == [("device.status", {"deviceId": None, "status": "disconnected"})]
    assert MumuControl.connection_device is None
