from __future__ import annotations

from typing import Any

import module.device_manager as device_manager_module
from module.device_manager import DeviceInfo, DeviceManager, DeviceTarget
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


def test_device_ids_are_stable_and_do_not_contain_hwnd() -> None:
    target = DeviceManager._make_mumu_target(2)
    assert target.info.id == "mumu:2"
    assert target.endpoint == "127.0.0.1:16448"
    assert target.info.id == "mumu:2"


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

    listed = dispatcher.dispatch(
        {"jsonrpc": "2.0", "id": 1, "method": "device.list"}
    )
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
    response = dispatcher.dispatch(
        {"jsonrpc": "2.0", "id": 3, "method": "device.connect", "params": {}}
    )
    assert response["error"]["code"] == -32602


def test_dispatcher_rejects_unknown_methods() -> None:
    dispatcher = RpcDispatcher(StubDeviceManager(), version="test")
    response = dispatcher.dispatch(
        {"jsonrpc": "2.0", "id": 4, "method": "not.implemented"}
    )
    assert response["error"]["code"] == -32601
