"""Runtime device selection for the GPUI sidecar.

The legacy application decides between a Windows game window and an Android
emulator through ``cfg.simulator``.  The GPUI client instead selects an opaque
stable device id (for example ``pc:limbus`` or ``mumu:0``).  This module is the
small compatibility layer between those two models.  It owns exactly one
active runtime session and keeps HWND/ADB objects out of the IPC contract.
"""

from __future__ import annotations

import os
import re
import threading
from dataclasses import dataclass
from typing import Any, Callable, Literal

from module.config import cfg
from module.logger import log

DeviceKind = Literal["pc", "mumu", "adb"]


class DeviceError(RuntimeError):
    """A user-correctable device discovery or connection error."""


@dataclass(frozen=True)
class DeviceInfo:
    """Public, serializable device information exposed to the UI."""

    id: str
    name: str
    detail: str | None = None

    def to_dict(self) -> dict[str, Any]:
        value: dict[str, Any] = {"id": self.id, "name": self.name}
        if self.detail is not None:
            value["detail"] = self.detail
        return value


@dataclass(frozen=True)
class DeviceTarget:
    """A discovered target plus private connection parameters."""

    info: DeviceInfo
    kind: DeviceKind
    hwnd: int | None = None
    endpoint: str | None = None
    instance_number: int | None = None


@dataclass
class DeviceSession:
    target: DeviceTarget
    controller: Any = None


StatusListener = Callable[[str, dict[str, Any]], None]


class DeviceManager:
    """Discover and manage the single device selected by the GPUI client.

    Discovery is deliberately side-effect-light.  In particular, listing a
    MuMu target never constructs ``MumuControl`` and therefore never launches
    an emulator.  Expensive work happens only in ``connect``.
    """

    _RUNTIME_CONFIG_KEYS = (
        "simulator",
        "simulator_type",
        "simulator_port",
        "mumu_instance_number",
    )

    def __init__(self) -> None:
        self._lock = threading.RLock()
        self._targets: dict[str, DeviceTarget] = {}
        self._active: DeviceSession | None = None
        self._connecting = False
        self._status_listeners: list[StatusListener] = []
        self._notice_listeners: list[StatusListener] = []
        self._busy_checker: Callable[[], bool] = lambda: False
        self._runtime_snapshot = {
            key: cfg.get_value(key) for key in self._RUNTIME_CONFIG_KEYS
        }

    @property
    def active_id(self) -> str | None:
        with self._lock:
            return self._active.target.info.id if self._active else None

    @property
    def active_session(self) -> DeviceSession | None:
        with self._lock:
            return self._active

    def require_active_session(self) -> DeviceSession:
        """Return the selected runtime session or raise a user-facing error.

        The GPUI sidecar must never silently fall back to the persisted legacy
        simulator configuration. Legacy callers that do not use the device
        manager can keep their compatibility path, but sidecar execution
        entry points should use this guard.
        """
        with self._lock:
            session = self._active
        if session is None:
            raise DeviceError("未连接设备，请先选择并连接设备")
        if session.target.kind in ("mumu", "adb") and session.controller is None:
            raise DeviceError("当前设备会话缺少模拟器控制器")
        if session.target.kind == "pc" and not self._valid_hwnd(session.target.hwnd):
            raise DeviceError("当前设备窗口已失效，请重新连接设备")
        return session

    def require_active_controller(self) -> Any:
        """Return the controller belonging to the selected simulator session."""
        session = self.require_active_session()
        if session.target.kind not in ("mumu", "adb"):
            raise DeviceError("当前选中设备不是模拟器")
        return session.controller

    def bind_active_runtime(self) -> DeviceSession:
        """Rebind automation to the currently selected runtime session."""
        session = self.require_active_session()
        self._activate_runtime(session)
        return session

    def set_busy_checker(self, checker: Callable[[], bool]) -> None:
        self._busy_checker = checker

    def add_status_listener(self, listener: StatusListener) -> None:
        with self._lock:
            if listener not in self._status_listeners:
                self._status_listeners.append(listener)

    def remove_status_listener(self, listener: StatusListener) -> None:
        with self._lock:
            if listener in self._status_listeners:
                self._status_listeners.remove(listener)

    def add_notice_listener(self, listener: StatusListener) -> None:
        with self._lock:
            if listener not in self._notice_listeners:
                self._notice_listeners.append(listener)

    def remove_notice_listener(self, listener: StatusListener) -> None:
        with self._lock:
            if listener in self._notice_listeners:
                self._notice_listeners.remove(listener)

    def list_devices(self) -> list[dict[str, Any]]:
        """Return currently discoverable targets without opening them."""
        targets: dict[str, DeviceTarget] = {}

        pc_target = self._discover_pc_window()
        if pc_target is not None:
            targets[pc_target.info.id] = pc_target

        for target in self._discover_mumu_targets():
            targets[target.info.id] = target

        for target in self._discover_adb_targets():
            # A configured MuMu endpoint should be represented by the stable
            # MuMu id rather than by a second generic ADB row.
            if target.endpoint and any(
                known.endpoint == target.endpoint and known.kind == "mumu"
                for known in targets.values()
            ):
                continue
            targets[target.info.id] = target

        with self._lock:
            self._targets = targets
            active_id = self._active.target.info.id if self._active else None

        result = [target.info.to_dict() for target in targets.values()]
        if active_id and active_id not in targets:
            # Keep the selected entry visible while a transient rescan is in
            # progress.  The next connect/refresh will resolve it again.
            with self._lock:
                active = self._active
            if active is not None:
                result.insert(0, active.target.info.to_dict())
        return result

    def connect(self, device_id: str) -> dict[str, Any]:
        """Connect the selected target and rebind the legacy automation layer."""
        if not device_id:
            raise DeviceError("设备 ID 不能为空")
        if self._busy_checker():
            raise DeviceError("任务运行期间不能切换设备")

        with self._lock:
            if self._connecting:
                raise DeviceError("已有设备连接任务正在进行")
            if self._active and self._active.target.info.id == device_id:
                active_session = self._active
            else:
                active_session = None
            if active_session is None:
                self._connecting = True

        if active_session is not None:
            # Rebind the compatibility layer even when the selected id is
            # unchanged. This repairs old callers that cleared a class-level
            # pointer without creating another controller.
            try:
                self._activate_runtime(active_session)
            except Exception as error:
                log.exception("重新绑定设备运行时失败：%s", device_id)
                raise DeviceError(f"重新绑定设备失败：{error}") from error
            return {
                "deviceId": device_id,
                "status": "connected",
                "alreadyConnected": True,
            }

        self._emit_status(device_id, "connecting")
        session: DeviceSession | None = None
        try:
            target = self._resolve_target(device_id)
            self._disconnect_active(restore_config=False)
            session = self._open_target(target)
            self._activate_runtime(session)
            with self._lock:
                self._active = session
            self._emit_status(device_id, "connected")
            self._emit_connection_details(session)
            return {"deviceId": device_id, "status": "connected"}
        except Exception as error:
            log.exception("连接设备失败：%s", device_id)
            if session is not None:
                try:
                    self._cleanup_session(session)
                except Exception:
                    log.exception("清理失败的设备连接时出错")
            try:
                self._disconnect_active(restore_config=False)
            except Exception:
                log.exception("清理失败的设备连接时出错")
            self._restore_runtime_config()
            self._emit_status(device_id, "disconnected")
            self._emit_notice("error", f"连接设备失败：{error}")
            if isinstance(error, DeviceError):
                raise
            raise DeviceError(f"连接设备失败：{error}") from error
        finally:
            with self._lock:
                self._connecting = False

    def disconnect(self) -> dict[str, Any]:
        """Release the active target without closing a game or emulator."""
        return self._release_active(check_busy=True, notice=True)

    def release_after_task(self) -> dict[str, Any]:
        """Release a session after an explicit task-owned shutdown action.

        ``disconnect`` is intentionally blocked while execution is marked
        busy. A task that explicitly closes its emulator still needs a
        manager-owned cleanup after the action, otherwise the manager keeps a
        dead session and the preview thread keeps retrying it.
        """
        return self._release_active(check_busy=False, notice=False)

    def _release_active(self, *, check_busy: bool, notice: bool) -> dict[str, Any]:
        if check_busy and self._busy_checker():
            raise DeviceError("任务运行期间不能断开设备")
        with self._lock:
            active_id = self._active.target.info.id if self._active else None

        # Stop consumers before tearing down the controller. BackendApplication
        # maps this event to PreviewCapture.stop(), preventing a completed
        # task's preview loop from racing the connection cleanup.
        self._emit_status(None, "disconnected")
        self._disconnect_active(restore_config=True)
        if notice and active_id:
            self._emit_notice("info", "设备已断开连接")
        return {"status": "disconnected"}

    def close(self) -> None:
        """Best-effort shutdown hook for the sidecar process."""
        try:
            self._disconnect_active(restore_config=True)
        except Exception:
            log.exception("关闭设备管理器时清理连接失败")

    def _resolve_target(self, device_id: str) -> DeviceTarget:
        with self._lock:
            target = self._targets.get(device_id)
        if target is not None:
            if target.kind == "pc" and not self._valid_hwnd(target.hwnd):
                target = self._discover_pc_window()
                if target is not None:
                    with self._lock:
                        self._targets[device_id] = target
            if target is not None:
                return target

        if device_id == "pc:limbus":
            target = self._discover_pc_window()
        elif device_id.startswith("mumu:"):
            instance_text = device_id.removeprefix("mumu:")
            try:
                instance = int(instance_text)
            except ValueError as error:
                raise DeviceError(f"无效的 MuMu 设备 ID：{device_id}") from error
            target = self._make_mumu_target(instance)
        elif device_id.startswith("adb:"):
            endpoint = device_id.removeprefix("adb:")
            target = self._make_adb_target(endpoint)
        else:
            target = None

        if target is None:
            raise DeviceError(f"未找到设备：{device_id}")
        with self._lock:
            self._targets[device_id] = target
        return target

    def _open_target(self, target: DeviceTarget) -> DeviceSession:
        if target.kind == "pc":
            if not self._valid_hwnd(target.hwnd):
                raise DeviceError("未找到有效的 Limbus Company 游戏窗口")
            return DeviceSession(target)

        if target.kind == "mumu":
            if target.instance_number is None:
                raise DeviceError("MuMu 实例编号缺失")
            self._set_target_runtime_config(target)
            from module.automation.input_handlers.simulator.mumu_control import MumuControl

            return DeviceSession(target, MumuControl(instance_number=target.instance_number))

        if target.endpoint is None:
            raise DeviceError("ADB 设备地址缺失")
        self._set_target_runtime_config(target)
        from module.automation.input_handlers.simulator.simulator_control import SimulatorControl

        return DeviceSession(target, SimulatorControl(endpoint=target.endpoint))

    def _activate_runtime(self, session: DeviceSession) -> None:
        # Legacy modules still read these values. Keep them synchronized with
        # the selected session whenever a task or an RPC rebinds the runtime.
        self._set_target_runtime_config(session.target)
        if session.target.kind == "pc":
            from module.game_and_screen import screen

            screen.handle.bind(session.target.hwnd or 0)
        from module.automation import auto

        # ``auto`` is created during module import, before the GPUI selection
        # exists.  Rebuild only its input binding after activating the target.
        auto.init_input(session=session)

    def _set_target_runtime_config(self, target: DeviceTarget) -> None:
        if target.kind == "pc":
            self._set_runtime_config(simulator=False)
            return

        if target.kind == "mumu":
            if target.instance_number is None:
                raise DeviceError("MuMu 实例编号缺失")
            self._set_runtime_config(
                simulator=True,
                simulator_type=0,
                simulator_port=self._mumu_port(target.instance_number),
                mumu_instance_number=target.instance_number,
            )
            return

        if target.endpoint is None:
            raise DeviceError("ADB 设备地址缺失")
        if ":" in target.endpoint:
            port = self._port_from_endpoint(target.endpoint)
        else:
            try:
                port = int(cfg.get_value("simulator_port", 0) or 0)
            except (TypeError, ValueError):
                port = 0
        self._set_runtime_config(
            simulator=True,
            simulator_type=10,
            simulator_port=port,
            mumu_instance_number=-1,
        )

    def _disconnect_active(self, *, restore_config: bool) -> None:
        with self._lock:
            session = self._active
            self._active = None
        if session is None:
            return

        try:
            self._cleanup_session(session)
        except Exception as error:
            log.warning("释放设备连接失败：%s", error)
        finally:
            if restore_config:
                self._restore_runtime_config()

    @staticmethod
    def _cleanup_session(session: DeviceSession) -> None:
        if session.target.kind == "mumu":
            controller = session.controller
            if controller is not None:
                try:
                    controller.disconnect()
                except Exception as error:
                    log.debug("断开 MuMu NemuIpc 失败：%s", error)
                try:
                    controller.adb_disconnect()
                except Exception as error:
                    log.debug("断开 MuMu ADB 失败：%s", error)
            from module.automation.input_handlers.simulator.mumu_control import MumuControl

            if MumuControl.connection_device is controller:
                MumuControl.connection_device = None
        elif session.target.kind == "adb":
            controller = session.controller
            if controller is not None:
                try:
                    if getattr(controller, "simulator_control", None) is not None:
                        controller.simulator_control.stop()
                except Exception as error:
                    log.debug("停止 minitouch 失败：%s", error)
                try:
                    controller.adb_disconnect()
                except Exception as error:
                    log.debug("断开 ADB 失败：%s", error)
            from module.automation.input_handlers.simulator.simulator_control import SimulatorControl

            if SimulatorControl.connection_device is controller:
                SimulatorControl.connection_device = None
        else:
            from module.game_and_screen import screen

            screen.handle.clear()

    def _set_runtime_config(self, **values: Any) -> None:
        for key, value in values.items():
            if key in self._RUNTIME_CONFIG_KEYS:
                cfg.unsaved_set_value(key, value, stacklevel=3)

    def _restore_runtime_config(self) -> None:
        self._set_runtime_config(**self._runtime_snapshot)

    def _emit_status(self, device_id: str | None, status: str) -> None:
        payload = {"deviceId": device_id, "status": status}
        self._emit(self._status_listeners, "device.status", payload)

    def _emit_notice(self, level: str, message: str) -> None:
        self._emit(self._notice_listeners, "app.notice", {"level": level, "message": message})

    def _emit_connection_details(self, session: DeviceSession) -> None:
        connection, screenshot, input_method = self._connection_methods(session.target)
        self._emit_notice("info", f"已连接设备：{session.target.info.name}")
        self._emit_notice("info", f"连接方式：{connection}")
        self._emit_notice("info", f"截图方式：{screenshot}；输入方式：{input_method}")

    @staticmethod
    def _connection_methods(target: DeviceTarget) -> tuple[str, str, str]:
        if target.kind == "mumu":
            return (
                "MuMu IPC（NemuIpc）+ ADB 辅助",
                "MuMu IPC（NemuIpc）",
                "MuMu IPC（NemuIpc）",
            )

        if target.kind == "adb":
            endpoint = target.endpoint or "未知设备"
            return (
                f"ADB（{endpoint}）",
                "ADB screencap",
                "ADB shell input + minitouch",
            )

        screenshot = (
            "默认窗口截图（PrintWindow）"
            if bool(cfg.get_value("background_click", True))
            else "默认窗口截图（GDI，失败回退 pyautogui）"
        )
        input_method = {
            "background": "Windows 后台输入（pywin32）",
            "foreground": "Windows 前台输入（pyautogui）",
            "window_move": "Windows 窗口移动输入",
        }.get(str(cfg.get_value("win_input_type", "background")), "Windows 后台输入（pywin32）")
        return "Windows 游戏窗口", screenshot, input_method

    @staticmethod
    def _emit(listeners: list[StatusListener], event: str, payload: dict[str, Any]) -> None:
        for listener in list(listeners):
            try:
                listener(event, payload)
            except Exception:
                log.exception("设备事件监听器执行失败：%s", event)

    @staticmethod
    def _valid_hwnd(hwnd: int | None) -> bool:
        if not hwnd:
            return False
        try:
            import win32gui

            return bool(win32gui.IsWindow(hwnd))
        except Exception:
            return False

    def _discover_pc_window(self) -> DeviceTarget | None:
        try:
            import win32gui
            import win32process
        except ImportError:
            return None

        wanted_title = str(cfg.get_value("game_title_name", "LimbusCompany"))
        matches: list[DeviceTarget] = []

        def callback(hwnd: int, _extra: object) -> bool:
            try:
                if not win32gui.IsWindowVisible(hwnd):
                    return True
                title = win32gui.GetWindowText(hwnd)
                class_name = win32gui.GetClassName(hwnd)
                if class_name != "UnityWndClass" or title != wanted_title:
                    return True
                _, pid = win32process.GetWindowThreadProcessId(hwnd)
                _, _, width, height = win32gui.GetClientRect(hwnd)
                detail = f"Windows · {width}×{height} · PID {pid}"
                matches.append(
                    DeviceTarget(
                        DeviceInfo("pc:limbus", "Limbus Company", detail),
                        "pc",
                        hwnd=hwnd,
                    )
                )
            except Exception:
                return True
            return True

        try:
            win32gui.EnumWindows(callback, None)
        except Exception:
            return None
        if len(matches) > 1:
            log.warning("发现多个 Limbus Company 窗口，使用第一个匹配项")
        return matches[0] if matches else None

    def _discover_mumu_targets(self) -> list[DeviceTarget]:
        if not self._find_mumu_manager():
            return []
        instance = self._configured_mumu_instance()
        return [self._make_mumu_target(instance)]

    def _discover_adb_targets(self) -> list[DeviceTarget]:
        try:
            from adbutils import adb

            devices = adb.device_list()
        except Exception as error:
            log.debug("扫描 ADB 设备失败：%s", error)
            return []

        result: list[DeviceTarget] = []
        for device in devices:
            serial = str(getattr(device, "serial", ""))
            if not serial:
                continue
            state = str(getattr(device, "state", "device"))
            result.append(
                self._make_adb_target(
                    serial,
                    detail=f"ADB · {serial} · {state}",
                )
            )
        return result

    @staticmethod
    def _make_mumu_target(instance: int) -> DeviceTarget:
        endpoint = f"127.0.0.1:{DeviceManager._mumu_port(instance)}"
        return DeviceTarget(
            DeviceInfo(
                f"mumu:{instance}",
                f"MuMu 模拟器 #{instance}",
                f"实例 {instance} · ADB {DeviceManager._mumu_port(instance)}",
            ),
            "mumu",
            endpoint=endpoint,
            instance_number=instance,
        )

    @staticmethod
    def _make_adb_target(endpoint: str, *, detail: str | None = None) -> DeviceTarget:
        return DeviceTarget(
            DeviceInfo(
                f"adb:{endpoint}",
                f"ADB 设备 {endpoint}",
                detail or f"ADB · {endpoint}",
            ),
            "adb",
            endpoint=endpoint,
        )

    @staticmethod
    def _mumu_port(instance: int) -> int:
        return 16384 + instance * 32

    @staticmethod
    def _port_from_endpoint(endpoint: str) -> int:
        match = re.search(r":(\d+)$", endpoint)
        if not match:
            raise DeviceError(f"ADB 地址必须包含端口：{endpoint}")
        port = int(match.group(1))
        if not 1 <= port <= 65535:
            raise DeviceError(f"ADB 端口无效：{port}")
        return port

    def _configured_mumu_instance(self) -> int:
        value = cfg.get_value("mumu_instance_number", -1)
        try:
            value = int(value)
        except (TypeError, ValueError):
            value = -1
        if value >= 0:
            return value
        port = cfg.get_value("simulator_port", 16384)
        try:
            port = int(port)
        except (TypeError, ValueError):
            port = 16384
        if port >= 16384 and (port - 16384) % 32 == 0:
            return (port - 16384) // 32
        return 0

    @staticmethod
    def _find_mumu_manager() -> str | None:
        if os.name != "nt":
            return None
        try:
            import winreg

            names = (
                "MuMuPlayer-12.0",
                "MuMuPlayer",
                "MuMuPlayerGlobal-12.0",
                "MuMuPlayerGlobal",
                "YXArkNights-12.0",
            )
            for name in names:
                key_path = rf"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{name}"
                try:
                    with winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE, key_path) as key:
                        display_icon = str(winreg.QueryValueEx(key, "DisplayIcon")[0]).strip('"')
                    install_path = os.path.dirname(display_icon)
                    manager = os.path.join(install_path, "MuMuManager.exe")
                    if os.path.isfile(manager):
                        return manager
                    shell_manager = os.path.join(os.path.dirname(install_path), "shell", "MuMuManager.exe")
                    if os.path.isfile(shell_manager):
                        return shell_manager
                except (FileNotFoundError, OSError):
                    continue
        except (ImportError, OSError):
            return None
        return None


_default_manager: DeviceManager | None = None


def get_device_manager() -> DeviceManager:
    """Return the process-wide manager shared by RPC and task execution."""
    global _default_manager
    if _default_manager is None:
        _default_manager = DeviceManager()
    return _default_manager


def is_simulator_runtime() -> bool:
    """Return whether the selected runtime is an Android emulator.

    The legacy configuration remains the fallback for command-line callers,
    but a sidecar-selected session always wins over its persisted values.
    """
    session = get_device_manager().active_session
    if session is not None:
        return session.target.kind in ("mumu", "adb")
    return bool(cfg.get_value("simulator", False))


__all__ = [
    "DeviceError",
    "DeviceInfo",
    "DeviceManager",
    "DeviceSession",
    "DeviceTarget",
    "get_device_manager",
    "is_simulator_runtime",
]
