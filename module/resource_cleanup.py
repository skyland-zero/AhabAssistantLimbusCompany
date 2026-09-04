"""Run-scoped compensation actions for :class:`CleanupLedger` journals.

The sidecar owns a durable cleanup journal, while this module owns the small
set of *safe* actions that can be replayed after a Runner exits or is killed.
Everything is identified by the journal's ``runId`` and target identifiers;
there is deliberately no process-wide controller lookup and no ``adb
kill-server`` fallback.

The executor has injectable seams for recovery tests and platform-specific
hosts, while the default factory supplies conservative Win32/ADB backends.
Win32 uses only non-activating ``PostMessage``/window APIs.  ADB operations
select one explicit serial, verify the exact forward/socket identity, and use
argv commands with timeouts; there is deliberately no ``adb kill-server`` or
unscoped process cleanup fallback.
"""

from __future__ import annotations

import copy
import ctypes
import inspect
import platform as platform_module
import re
import socket as socket_module
import struct
import subprocess
import threading
import time
from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass
from typing import Any


class CleanupError(RuntimeError):
    """Base error for a rejected or failed compensation action."""


class CleanupIdentityError(CleanupError):
    """Raised when a journal/action does not belong to the requested run."""


class CleanupUnsupportedError(CleanupError):
    """Raised when no safe implementation exists for a requested action."""


class DeviceOfflineError(CleanupError):
    """Raised by an injected device backend when the target is offline."""


@dataclass(frozen=True)
class CleanupTimeouts:
    """Independent bounds for each compensation operation."""

    window_input: float = 2.0
    window_restore: float = 2.0
    android_input: float = 3.0
    adb_forward: float = 3.0
    remote: float = 5.0


@dataclass
class _CleanupWork:
    """One item handed to the executor's single controlled worker."""

    name: str
    action: Callable[[], Mapping[str, Any]]
    done: threading.Event
    started: float
    result: dict[str, Any] | None = None


def _copy(value: Any) -> Any:
    """Copy a value crossing the ledger/backend boundary."""

    return copy.deepcopy(value)


def _call_with_supported_kwargs(callback: Callable[..., Any], **kwargs: Any) -> Any:
    """Call a fake or production backend without swallowing callback errors."""

    try:
        signature = inspect.signature(callback)
    except (TypeError, ValueError):
        return callback(**kwargs)
    parameters = signature.parameters
    if any(parameter.kind is inspect.Parameter.VAR_KEYWORD for parameter in parameters.values()):
        return callback(**kwargs)
    accepted = {
        name: value
        for name, value in kwargs.items()
        if name in parameters
        and parameters[name].kind
        in {inspect.Parameter.POSITIONAL_OR_KEYWORD, inspect.Parameter.KEYWORD_ONLY}
    }
    return callback(**accepted)


def _offline_exception(error: BaseException) -> bool:
    if isinstance(error, DeviceOfflineError):
        return True
    if isinstance(error, (CleanupUnsupportedError, ImportError, ModuleNotFoundError)):
        # Optional Win32/ADB dependencies are expected to be absent on some
        # hosts.  Recovery must remain pending so the next startup can retry;
        # treating this as a completed failure would lose the obligation.
        return True
    if isinstance(error, (ConnectionError, TimeoutError)):
        return True
    text = str(error).casefold()
    return any(token in text for token in ("offline", "not found", "no device", "unavailable", "disconnected"))


def _status_from_action_results(actions: Sequence[Mapping[str, Any]]) -> str:
    statuses = {str(action.get("status")) for action in actions}
    if "failed" in statuses:
        return "failed"
    if "deferred" in statuses:
        return "deferred"
    return "success"


def _normalise_status(value: Any) -> str:
    if value in {"success", "done", "released", "removed", "requested", "skipped"}:
        return "success"
    if value in {"deferred", "pending"}:
        return "deferred"
    if value in {"failed", "error"}:
        return "failed"
    if value is False:
        return "failed"
    return "success"


def _safe_int(value: Any, *, minimum: int = 0, maximum: int = 2**31 - 1) -> int | None:
    if isinstance(value, bool):
        return None
    try:
        number = int(value)
    except (TypeError, ValueError):
        return None
    if number < minimum or number > maximum:
        return None
    return number


_RUN_ID_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}\Z")
_SERIAL_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}\Z")
_SESSION_RE = re.compile(r"[A-Za-z0-9][A-Za-z0-9_.-]{0,95}\Z")
_SCID_RE = re.compile(r"(?:0x)?[0-9A-Fa-f]{1,8}\Z")
_REMOTE_RE = re.compile(r"localabstract:[A-Za-z0-9][A-Za-z0-9_.-]{0,95}\Z")


def _validate_text(value: Any, name: str, pattern: re.Pattern[str]) -> str:
    if not isinstance(value, str) or pattern.fullmatch(value) is None:
        raise CleanupIdentityError(f"{name} contains unsupported characters or length")
    return value


def _validate_run_id(value: Any) -> str:
    return _validate_text(value, "runId", _RUN_ID_RE)


def _validate_serial(value: Any) -> str:
    return _validate_text(value, "device serial", _SERIAL_RE)


def _validate_session_name(value: Any, name: str = "socketName") -> str:
    if isinstance(value, str) and value.startswith("localabstract:"):
        value = value[len("localabstract:") :]
    return _validate_text(value, name, _SESSION_RE)


def _validate_scid(value: Any) -> str:
    if isinstance(value, bool):
        raise CleanupIdentityError("scid must be an unsigned 32-bit value")
    if isinstance(value, int):
        number = value
    elif isinstance(value, str) and _SCID_RE.fullmatch(value):
        number = int(value, 16 if value.lower().startswith("0x") else 16)
    else:
        raise CleanupIdentityError("scid must be hexadecimal")
    if not 0 <= number <= 0xFFFFFFFF:
        raise CleanupIdentityError("scid must fit in an unsigned 32-bit value")
    return f"0x{number:08x}"


def _validate_port(value: Any, name: str = "forward port") -> int:
    if isinstance(value, bool) or not isinstance(value, int) or not 1 <= value <= 65535:
        raise CleanupIdentityError(f"{name} must be an integer between 1 and 65535")
    return value


def _validate_remote(value: Any) -> str:
    if not isinstance(value, str) or _REMOTE_RE.fullmatch(value) is None:
        raise CleanupIdentityError("forward remote must be a localabstract socket")
    return value


def _session_socket(scid: str | None, socket_name: str | None) -> tuple[str | None, str | None]:
    normalized_scid = _validate_scid(scid) if scid is not None else None
    normalized_socket = _validate_session_name(socket_name) if socket_name is not None else None
    if normalized_scid is not None:
        derived = f"scrcpy_{int(normalized_scid, 16):08x}"
        if normalized_socket is not None and normalized_socket != derived:
            raise CleanupIdentityError("scid and socketName do not identify the same session")
        normalized_socket = derived
    return normalized_scid, normalized_socket


def _first(mapping: Mapping[str, Any], *keys: str) -> Any:
    for key in keys:
        if key in mapping:
            return mapping[key]
    return None


def _as_sequence(value: Any) -> list[Any]:
    if value is None:
        return []
    if isinstance(value, (str, bytes, bytearray)):
        return [value]
    if isinstance(value, Sequence):
        return list(value)
    return [value]


def _resource_records(data: Mapping[str, Any]) -> list[Mapping[str, Any]]:
    resources = data.get("resources", ())
    if not isinstance(resources, Sequence) or isinstance(resources, (str, bytes, bytearray)):
        return []
    return [resource for resource in resources if isinstance(resource, Mapping) and not resource.get("released", False)]


def _target(data: Mapping[str, Any]) -> Mapping[str, Any]:
    target = data.get("target")
    return target if isinstance(target, Mapping) else {}


def _target_kind(data: Mapping[str, Any]) -> str | None:
    target = _target(data)
    kind = _first(target, "kind", "deviceKind", "device_kind")
    if kind in {"pc", "mumu", "adb"}:
        return str(kind)
    if _first(data, "hwnd", "windowHandle", "window_handle") is not None or _first(target, "hwnd") is not None:
        return "pc"
    if _first(data, "deviceSerial", "serial", "device_serial") is not None or _first(target, "endpoint") is not None:
        return "adb"
    return None


def _window_handle(data: Mapping[str, Any]) -> int | None:
    target = _target(data)
    value = _first(data, "hwnd", "windowHandle", "window_handle")
    if value is None:
        value = _first(target, "hwnd")
    if value is None and isinstance(data.get("reservation"), Mapping):
        value = _first(data["reservation"], "hwnd", "windowHandle", "window_handle")
    return _safe_int(
        value,
        minimum=1,
    )


def _serial(data: Mapping[str, Any]) -> str | None:
    target = _target(data)
    value = _first(data, "deviceSerial", "serial", "device_serial")
    if value is None:
        value = _first(target, "endpoint", "serial", "deviceSerial")
    if value is None and isinstance(data.get("reservation"), Mapping):
        value = _first(data["reservation"], "deviceSerial", "serial", "device_serial", "endpoint")
    if value is None:
        return None
    return _validate_serial(value.strip()) if isinstance(value, str) else None


def _run_id(data: Mapping[str, Any]) -> str | None:
    value = _first(data, "runId", "run_id")
    if value is None:
        return None
    if not isinstance(value, str) or not value.strip():
        raise CleanupIdentityError("runId must be a non-empty string")
    return _validate_run_id(value.strip())


def _window_state(data: Mapping[str, Any]) -> dict[str, Any]:
    value = data.get("originalWindowState")
    if value is None and isinstance(data.get("reservation"), Mapping):
        value = data["reservation"].get("originalWindowState")
    state = dict(value) if isinstance(value, Mapping) else {}
    if "rect" not in state:
        rect = data.get("originalWindowRect")
        if rect is None and isinstance(data.get("reservation"), Mapping):
            rect = data["reservation"].get("originalWindowRect")
        if rect is not None:
            state["rect"] = _copy(rect)
    return state


def _metadata(resource: Mapping[str, Any]) -> Mapping[str, Any]:
    value = resource.get("metadata")
    return value if isinstance(value, Mapping) else {}


def _records_for_keys(data: Mapping[str, Any], keys: Sequence[str]) -> list[Any]:
    values: list[Any] = []
    for key in keys:
        if key in data:
            values.extend(_as_sequence(data[key]))
    for resource in _resource_records(data):
        metadata = _metadata(resource)
        resource_type = str(resource.get("resourceType", "")).casefold()
        if any(key in metadata for key in keys) or any(token in resource_type for token in keys):
            for key in keys:
                if key in metadata:
                    values.extend(_as_sequence(metadata[key]))
            if not any(key in metadata for key in keys):
                values.append(resource)
    return values


def _normalise_button(value: Any) -> str | None:
    if isinstance(value, int) and not isinstance(value, bool):
        return {1: "left", 2: "right", 3: "middle", 4: "x1", 5: "x2"}.get(value)
    if isinstance(value, str):
        value = value.casefold().strip().replace("button", "")
        return value if value in {"left", "right", "middle", "x1", "x2"} else None
    if isinstance(value, Mapping):
        return _normalise_button(_first(value, "button", "name", "id"))
    return None


def _normalise_key(value: Any) -> int | str | None:
    if isinstance(value, Mapping):
        value = _first(value, "key", "keyCode", "key_code", "vk", "virtualKey")
    if isinstance(value, int) and not isinstance(value, bool) and 0 <= value <= 255:
        return value
    if isinstance(value, str) and value.strip():
        text = value.strip()
        if text.isdigit() and 0 <= int(text) <= 255:
            return int(text)
        virtual_keys = {
            "back": 0x08,
            "tab": 0x09,
            "enter": 0x0D,
            "return": 0x0D,
            "shift": 0x10,
            "ctrl": 0x11,
            "control": 0x11,
            "alt": 0x12,
            "pause": 0x13,
            "capslock": 0x14,
            "esc": 0x1B,
            "escape": 0x1B,
            "space": 0x20,
            "pageup": 0x21,
            "pagedown": 0x22,
            "end": 0x23,
            "home": 0x24,
            "left": 0x25,
            "up": 0x26,
            "right": 0x27,
            "down": 0x28,
            "insert": 0x2D,
            "delete": 0x2E,
        }
        mapped = virtual_keys.get(text.casefold())
        if mapped is not None:
            return mapped
        return text
    return None


def _normalise_pointer(value: Any) -> int | None:
    if isinstance(value, Mapping):
        value = _first(value, "pointerId", "pointer_id", "id", "pointer")
    return _safe_int(value, minimum=0, maximum=255)


class Win32CleanupBackend:
    """Safe, non-activating Win32 implementation backed by ``ctypes``.

    The backend intentionally exposes only the operations needed by the
    cleanup ledger.  It never calls ``SetForegroundWindow``/``ShowWindow`` and
    every window operation is addressed by the journal's HWND.

    ``user32`` and ``ctypes_module`` are injectable for unit tests; production
    callers leave them unset and the Windows ``user32.dll`` is resolved lazily
    when cleanup is actually attempted.
    """

    _MOUSE_UP_MESSAGES = {
        "left": 0x0202,  # WM_LBUTTONUP
        "right": 0x0205,  # WM_RBUTTONUP
        "middle": 0x0208,  # WM_MBUTTONUP
        "x1": 0x020C,  # WM_XBUTTONUP
        "x2": 0x020C,
    }
    _WM_KEYUP = 0x0101
    _GWL_STYLE = -16
    _GWL_EXSTYLE = -20
    _HWND_TOPMOST = -1
    _HWND_NOTOPMOST = -2
    _SWP_NOSIZE = 0x0001
    _SWP_NOMOVE = 0x0002
    _SWP_NOZORDER = 0x0004
    _SWP_NOACTIVATE = 0x0010
    _SWP_FRAMECHANGED = 0x0020

    def __init__(self, *, user32: Any = None, ctypes_module: Any = None) -> None:
        ctypes_api = ctypes_module or ctypes
        self._ctypes = ctypes_api
        if user32 is None:
            try:
                user32 = ctypes_api.windll.user32
            except (AttributeError, ImportError, OSError) as error:  # pragma: no cover - host dependent
                raise CleanupUnsupportedError("Win32 ctypes backend is unavailable on this platform") from error
        self.user32 = user32

    def _function(self, name: str) -> Callable[..., Any]:
        function = getattr(self.user32, name, None)
        if not callable(function):
            raise CleanupUnsupportedError(f"Win32 user32 lacks {name}")
        return function

    @staticmethod
    def _window_handle(value: Any) -> int:
        if isinstance(value, bool) or not isinstance(value, int) or not 0 < value <= 2**64 - 1:
            raise CleanupError("HWND must be a positive integer")
        return value

    def is_window(self, *, hwnd: int, **_: Any) -> bool:
        return bool(self._function("IsWindow")(self._window_handle(hwnd)))

    def _post_message(self, hwnd: int, message: int, wparam: int, lparam: int = 0) -> None:
        post = getattr(self.user32, "PostMessageW", None) or getattr(self.user32, "PostMessageA", None)
        if not callable(post):
            raise CleanupUnsupportedError("Win32 user32 lacks PostMessage")
        result = post(hwnd, message, wparam, lparam)
        if not result:
            raise OSError(f"PostMessage failed for HWND {hwnd}")

    def release_mouse(self, *, hwnd: int, button: str, **_: Any) -> None:
        if button not in self._MOUSE_UP_MESSAGES:
            raise CleanupError(f"unsupported mouse button: {button}")
        hwnd = self._window_handle(hwnd)
        wparam = 0
        if button == "x1":
            wparam = 1 << 16
        elif button == "x2":
            wparam = 2 << 16
        self._post_message(hwnd, self._MOUSE_UP_MESSAGES[button], wparam)

    def release_key(self, *, hwnd: int, key: int | str, **_: Any) -> None:
        hwnd = self._window_handle(hwnd)
        if isinstance(key, str):
            if len(key) != 1:
                raise CleanupError(f"unsupported virtual key: {key}")
            key = ord(key.upper())
        if not isinstance(key, int) or not 0 <= key <= 255:
            raise CleanupError(f"unsupported virtual key: {key}")
        self._post_message(hwnd, self._WM_KEYUP, key)

    @staticmethod
    def _window_coordinate(value: Any, name: str) -> int:
        if isinstance(value, bool) or not isinstance(value, int) or not -(2**31) <= value <= 2**31 - 1:
            raise CleanupError(f"{name} must be a signed 32-bit integer")
        return value

    @staticmethod
    def _window_style(value: Any, name: str) -> int:
        if isinstance(value, bool) or not isinstance(value, int) or not 0 <= value <= 2**64 - 1:
            raise CleanupError(f"{name} must be an unsigned window style")
        return value

    def _clear_last_error(self) -> None:
        """Clear Win32's thread-local error before a zero-valued API call.

        ``SetWindowLongPtr`` returns the previous style, so zero is a valid
        success result.  Clearing the error first is required to distinguish
        that case from a real failure (which also returns zero).
        """

        clear = getattr(self._ctypes, "set_last_error", None)
        if callable(clear):
            clear(0)
            return
        # This hook is useful for small injected Win32 fakes.  Real ctypes
        # uses ``set_last_error`` above on Windows.
        clear = getattr(self.user32, "SetLastError", None)
        if callable(clear):
            clear(0)

    def _last_error(self) -> int | None:
        get = getattr(self._ctypes, "get_last_error", None)
        if callable(get):
            try:
                return int(get())
            except (TypeError, ValueError, OSError):
                return None
        get = getattr(self.user32, "GetLastError", None)
        if callable(get):
            try:
                return int(get())
            except (TypeError, ValueError, OSError):
                return None
        return None

    def _set_window_long(self, hwnd: int, index: int, value: int) -> None:
        setter = getattr(self.user32, "SetWindowLongPtrW", None) or getattr(self.user32, "SetWindowLongW", None)
        if not callable(setter):
            raise CleanupUnsupportedError("Win32 user32 lacks SetWindowLongPtr")
        self._clear_last_error()
        previous = setter(hwnd, index, value)
        try:
            previous_value = int(previous)
        except (TypeError, ValueError, OverflowError) as error:
            raise CleanupError("SetWindowLongPtr returned an invalid previous style") from error
        if previous_value == 0:
            last_error = self._last_error()
            if last_error not in (None, 0):
                raise OSError(
                    f"SetWindowLongPtr failed for HWND {hwnd}, index {index}: Win32 error {last_error}"
                )

    def restore_window(self, *, hwnd: int, state: Mapping[str, Any], **_: Any) -> None:
        if not self.is_window(hwnd=hwnd):
            raise DeviceOfflineError(f"window {hwnd} is no longer available")
        style = _first(state, "style", "windowStyle")
        ex_style = _first(state, "exStyle", "extendedStyle", "ex_style")
        style_changed = style is not None or ex_style is not None
        if style is not None:
            self._set_window_long(hwnd, self._GWL_STYLE, self._window_style(style, "style"))
        if ex_style is not None:
            self._set_window_long(hwnd, self._GWL_EXSTYLE, self._window_style(ex_style, "exStyle"))

        rect = _first(state, "rect", "windowRect", "originalWindowRect")
        topmost = state.get("topmost")
        if topmost is not None and not isinstance(topmost, bool):
            raise CleanupError("topmost must be a boolean")
        set_pos = self._function("SetWindowPos")
        if rect is not None:
            if not isinstance(rect, Sequence) or isinstance(rect, (str, bytes, bytearray)) or len(rect) != 4:
                raise CleanupError("original window rect must contain four coordinates")
            left, top, right, bottom = tuple(
                self._window_coordinate(value, name)
                for value, name in zip(rect, ("left", "top", "right", "bottom"), strict=True)
            )
            width = right - left
            height = bottom - top
            if width < 0 or height < 0:
                raise CleanupError("original window rect has negative size")
            if topmost is None:
                # Without a journaled z-order value, retain the current one.
                # HWND_TOP plus SWP_NOZORDER is intentionally non-mutating.
                insert_after = 0
                flags = self._SWP_NOACTIVATE | self._SWP_NOZORDER
            else:
                insert_after = self._HWND_TOPMOST if topmost else self._HWND_NOTOPMOST
                flags = self._SWP_NOACTIVATE
            if style_changed:
                flags |= self._SWP_FRAMECHANGED
            if not set_pos(hwnd, insert_after, left, top, width, height, flags):
                raise OSError(f"SetWindowPos failed for HWND {hwnd}")
        elif topmost is not None:
            insert_after = self._HWND_TOPMOST if topmost else self._HWND_NOTOPMOST
            if not set_pos(
                hwnd,
                insert_after,
                0,
                0,
                0,
                0,
                self._SWP_NOMOVE
                | self._SWP_NOSIZE
                | self._SWP_NOACTIVATE
                | (self._SWP_FRAMECHANGED if style_changed else 0),
            ):
                raise OSError(f"SetWindowPos failed for HWND {hwnd}")
        elif style_changed:
            if not set_pos(
                hwnd,
                0,
                0,
                0,
                0,
                0,
                self._SWP_NOMOVE
                | self._SWP_NOSIZE
                | self._SWP_NOZORDER
                | self._SWP_NOACTIVATE
                | self._SWP_FRAMECHANGED,
            ):
                raise OSError(f"SetWindowPos failed for HWND {hwnd}")


class AdbCleanupBackend:
    """Run-scoped ADB/scrcpy cleanup using ``adbutils`` and argv commands.

    No method accepts a shell command string supplied by a journal.  Device
    selection is always ``adb.device(serial)`` and a forward is removed only
    after its exact ``local``/``remote`` mapping is observed on that device.
    The optional command runner exists for the narrow remote-process lookup;
    it is always invoked with an argv list and an explicit timeout.
    """

    _TOUCH_MESSAGE = 2
    _TOUCH_CANCEL = 3
    _TOUCH_UP = 1
    _GENERIC_POINTER = 0xFFFFFFFFFFFFFFFE

    def __init__(
        self,
        adb_module: Any = None,
        *,
        socket_factory: Callable[..., Any] | None = None,
        command_runner: Callable[..., Any] | None = None,
        adb_path: str | None = None,
        command_timeout: float = 5.0,
    ) -> None:
        # Accept either ``adbutils`` itself or its already-imported ``adb``
        # singleton to keep the test and embedding API small.
        self.adb_module = adb_module
        self.socket_factory = socket_factory or socket_module.create_connection
        self.command_runner = command_runner or subprocess.run
        self.adb_path = adb_path
        self.command_timeout = max(0.1, float(command_timeout))

    def _adb(self) -> Any:
        if self.adb_module is not None:
            client = getattr(self.adb_module, "adb", self.adb_module)
            if callable(getattr(client, "device", None)):
                return client
            raise CleanupUnsupportedError("ADB client lacks device(serial)")
        try:
            from adbutils import adb
        except (ImportError, ModuleNotFoundError) as error:  # pragma: no cover - dependency optional
            raise CleanupUnsupportedError("adbutils is unavailable") from error
        return adb

    def _device(self, serial: str) -> Any:
        serial = _validate_serial(serial)
        try:
            device = self._adb().device(serial)
        except CleanupError:
            raise
        except (ImportError, ModuleNotFoundError) as error:  # pragma: no cover - optional dependency
            raise CleanupUnsupportedError("adbutils is unavailable") from error
        except Exception as error:
            raise DeviceOfflineError(f"ADB device {serial} is unavailable") from error
        actual_serial = getattr(device, "serial", None)
        if actual_serial is not None and str(actual_serial) != serial:
            raise CleanupIdentityError("ADB returned a device with a different serial")
        return device

    def is_online(self, *, serial: str, **_: Any) -> bool:
        device = self._device(serial)
        try:
            state = getattr(device, "state", None)
        except Exception as error:
            raise DeviceOfflineError(f"ADB device {serial} is unavailable") from error
        if state is not None and str(state).casefold() not in {"device", "online"}:
            return False
        return True

    @staticmethod
    def _forward_value(entry: Any, key: str) -> Any:
        value = getattr(entry, key, None)
        if value is None and isinstance(entry, Mapping):
            value = entry.get(key)
        return value

    def _forward_entries(self, device: Any) -> list[Any]:
        forward_list = getattr(device, "forward_list", None)
        if not callable(forward_list):
            raise CleanupUnsupportedError("ADB backend cannot verify forward ownership")
        try:
            return list(forward_list() or ())
        except Exception as error:
            raise DeviceOfflineError("cannot read ADB forward list") from error

    @classmethod
    def _matching_forward(
        cls,
        device: Any,
        *,
        serial: str,
        local_port: int,
        remote: str,
    ) -> Any | None:
        local = f"tcp:{_validate_port(local_port)}"
        for entry in cls._forward_entries_for_device(device):
            entry_serial = cls._forward_value(entry, "serial")
            entry_local = cls._forward_value(entry, "local")
            entry_remote = cls._forward_value(entry, "remote")
            # AdbDevice.forward_list() is already bound to ``serial`` and
            # older adbutils versions omit entry.serial.  An explicitly
            # different serial is never trusted.
            if entry_serial is not None and str(entry_serial) != serial:
                continue
            if str(entry_local) != local or str(entry_remote) != remote:
                continue
            return entry
        return None

    @classmethod
    def _forward_entries_for_device(cls, device: Any) -> list[Any]:
        forward_list = getattr(device, "forward_list", None)
        if not callable(forward_list):
            raise CleanupUnsupportedError("ADB backend cannot verify forward ownership")
        try:
            return list(forward_list() or ())
        except Exception as error:
            raise DeviceOfflineError("cannot read ADB forward list") from error

    @classmethod
    def _normalise_remote(
        cls,
        *,
        remote: str | None,
        scid: Any = None,
        socket_name: Any = None,
    ) -> tuple[str | None, str | None, str | None]:
        normalized_scid, normalized_socket = _session_socket(scid, socket_name)
        normalized_remote = None
        if remote is not None:
            normalized_remote = _validate_remote(remote)
        derived_remote = f"localabstract:{normalized_socket}" if normalized_socket is not None else None
        if normalized_remote is not None and derived_remote is not None and normalized_remote != derived_remote:
            raise CleanupIdentityError("forward remote does not match scid/socket")
        return normalized_remote or derived_remote, normalized_scid, normalized_socket

    @classmethod
    def _touch_message(
        cls,
        action: int,
        pointer_id: int,
        *,
        x: int = 0,
        y: int = 0,
        width: int = 1,
        height: int = 1,
    ) -> bytes:
        if action not in {cls._TOUCH_CANCEL, cls._TOUCH_UP}:
            raise CleanupError("unsupported scrcpy touch cleanup action")
        pointer_id = _safe_int(pointer_id, minimum=0, maximum=0xFFFFFFFFFFFFFFFF)
        if pointer_id is None:
            raise CleanupError("pointerId must be an unsigned integer")
        dimensions = (
            _safe_int(width, minimum=1, maximum=65535),
            _safe_int(height, minimum=1, maximum=65535),
        )
        coordinates = (
            _safe_int(x, minimum=-(2**31), maximum=2**31 - 1),
            _safe_int(y, minimum=-(2**31), maximum=2**31 - 1),
        )
        if None in dimensions or None in coordinates:
            raise CleanupError("scrcpy touch geometry is invalid")
        pressure = 0
        buttons = 0
        return struct.pack(
            ">BBQiiHHHii",
            cls._TOUCH_MESSAGE,
            action,
            pointer_id,
            coordinates[0],
            coordinates[1],
            dimensions[0],
            dimensions[1],
            pressure,
            buttons,
            buttons,
        )

    def _verified_control_socket(
        self,
        *,
        device: Any,
        serial: str,
        local_port: int,
        remote: str | None,
        scid: Any,
        socket_name: Any,
        timeout: float | None,
    ) -> Any:
        expected_remote, _, _ = self._normalise_remote(remote=remote, scid=scid, socket_name=socket_name)
        if expected_remote is None:
            raise CleanupUnsupportedError("scrcpy socket identity is missing")
        if self._matching_forward(device, serial=serial, local_port=local_port, remote=expected_remote) is None:
            # The forward may already have been removed by a prior retry.  A
            # caller treats this as an idempotent no-op and never connects to
            # a possibly reused local port.
            return None
        connect_timeout = self.command_timeout if timeout is None else max(0.1, float(timeout))
        try:
            return self.socket_factory(("127.0.0.1", _validate_port(local_port)), timeout=connect_timeout)
        except (ConnectionError, TimeoutError, OSError) as error:
            raise DeviceOfflineError("scrcpy control socket is unavailable") from error

    def cancel_touch(
        self,
        *,
        serial: str,
        pointer_id: int,
        run_id: str,
        local_port: int | None = None,
        forward_port: int | None = None,
        remote: str | None = None,
        scid: Any = None,
        socket_name: Any = None,
        x: int = 0,
        y: int = 0,
        width: int = 1,
        height: int = 1,
        timeout: float | None = None,
    ) -> dict[str, Any]:
        serial = _validate_serial(serial)
        _validate_run_id(run_id)
        port = local_port if local_port is not None else forward_port
        if port is None:
            raise CleanupUnsupportedError("scrcpy forward port is missing")
        port = _validate_port(port)
        device = self._device(serial)
        sock = self._verified_control_socket(
            device=device,
            serial=serial,
            local_port=port,
            remote=remote,
            scid=scid,
            socket_name=socket_name,
            timeout=timeout,
        )
        if sock is None:
            return {"status": "success", "detail": "forward_absent"}
        sendall = getattr(sock, "sendall", None)
        close = getattr(sock, "close", None)
        if not callable(sendall) or not callable(close):
            raise CleanupUnsupportedError("scrcpy control socket is invalid")
        try:
            # Send both packets: CANCEL clears a possibly multi-touch stream,
            # then UP is accepted by older servers that only implement the
            # ordinary pointer lifecycle.
            sendall(self._touch_message(self._TOUCH_CANCEL, pointer_id, x=x, y=y, width=width, height=height))
            sendall(self._touch_message(self._TOUCH_UP, pointer_id, x=x, y=y, width=width, height=height))
        except (ConnectionError, TimeoutError, OSError) as error:
            raise DeviceOfflineError("scrcpy control socket write failed") from error
        finally:
            try:
                close()
            except Exception:
                pass
        return {"status": "success", "detail": "touch_cancel_up"}

    def remove_forward(
        self,
        *,
        serial: str,
        local_port: int,
        remote: str | None = None,
        scid: Any = None,
        socket_name: Any = None,
        **_: Any,
    ) -> None:
        serial = _validate_serial(serial)
        local_port = _validate_port(local_port)
        expected_remote, _, _ = self._normalise_remote(remote=remote, scid=scid, socket_name=socket_name)
        if expected_remote is None:
            raise CleanupUnsupportedError("forward ownership identity is missing")
        device = self._device(serial)
        if self._matching_forward(device, serial=serial, local_port=local_port, remote=expected_remote) is None:
            return
        remove = getattr(device, "forward_remove", None)
        if not callable(remove):
            raise CleanupUnsupportedError("ADB backend cannot remove forward")
        local = f"tcp:{local_port}"
        try:
            remove(local, raise_non_found=False)
        except TypeError:
            remove(local)
        except (ConnectionError, TimeoutError, OSError) as error:
            raise DeviceOfflineError("ADB forward removal failed") from error

    def _command(self, argv: Sequence[str], *, timeout: float | None = None) -> Any:
        if not argv or any(not isinstance(arg, str) or not arg for arg in argv):
            raise CleanupIdentityError("ADB command contains an invalid argument")
        runner = self.command_runner
        effective_timeout = self.command_timeout if timeout is None else max(0.1, float(timeout))
        kwargs = {
            "capture_output": True,
            "check": False,
            "shell": False,
            "text": True,
            "timeout": effective_timeout,
        }
        try:
            signature = inspect.signature(runner)
        except (TypeError, ValueError):
            signature = None
        if signature is not None:
            parameters = signature.parameters
            if not any(parameter.kind is inspect.Parameter.VAR_KEYWORD for parameter in parameters.values()):
                kwargs = {key: value for key, value in kwargs.items() if key in parameters}
        try:
            return runner(list(argv), **kwargs)
        except subprocess.TimeoutExpired as error:
            raise DeviceOfflineError("ADB command timed out") from error
        except (ConnectionError, TimeoutError, OSError) as error:
            raise DeviceOfflineError("ADB command is unavailable") from error

    def _adb_executable(self) -> str:
        if self.adb_path is not None:
            if not isinstance(self.adb_path, str) or not self.adb_path.strip():
                raise CleanupUnsupportedError("ADB executable path is invalid")
            return self.adb_path
        try:
            path = getattr(self._adb(), "adb_path", None)
            if callable(path):
                value = path()
            else:
                # ``adbutils.adb`` exposes the helper as a module-level
                # function on supported versions.
                import adbutils

                value = adbutils.adb_path()
        except (ImportError, ModuleNotFoundError) as error:  # pragma: no cover - optional dependency
            raise CleanupUnsupportedError("adbutils is unavailable") from error
        except Exception as error:
            raise CleanupUnsupportedError("ADB executable path is unavailable") from error
        if not isinstance(value, str) or not value.strip():
            raise CleanupUnsupportedError("ADB executable path is unavailable")
        return value

    @staticmethod
    def _command_output(result: Any) -> tuple[int, str]:
        returncode = getattr(result, "returncode", 0)
        stdout = getattr(result, "stdout", "")
        if isinstance(stdout, bytes):
            stdout = stdout.decode(errors="replace")
        if not isinstance(stdout, str):
            stdout = str(stdout or "")
        if isinstance(returncode, bool) or not isinstance(returncode, int):
            returncode = 1
        return returncode, stdout

    @staticmethod
    def _matching_remote_process(lines: Sequence[str], *, scid: str, socket_name: str) -> list[int]:
        matches: list[int] = []
        for line in lines:
            # Match an actual scrcpy argument token.  A substring search would
            # let an unrelated command such as ``not_scid=...`` qualify for a
            # PID kill during cleanup.
            if "scrcpy" not in line.casefold():
                continue
            scid_match = re.search(r"(?:^|\s)scid=([^\s]+)", line)
            if scid_match is None or scid_match.group(1).casefold() != scid.casefold():
                continue
            # Scrcpy's current server command carries ``scid`` and derives
            # the abstract socket from it; older/future wrappers may also
            # print socket_name explicitly.  Accept the former only after the
            # validated scid/socket pair has been checked by ``_session_socket``
            # and reject an explicit conflicting socket marker.
            explicit_socket = re.search(r"(?:^|\s)(?:socket_name|socketName)=([^\s]+)", line)
            if explicit_socket is not None and explicit_socket.group(1) != socket_name:
                continue
            match = re.match(r"^\s*(\d+)\s+", line)
            if match is None:
                continue
            pid = _safe_int(match.group(1), minimum=2, maximum=2**31 - 1)
            if pid is not None:
                matches.append(pid)
        return sorted(set(matches))

    def remote_cleanup(
        self,
        *,
        reservation: Mapping[str, Any],
        run_id: str,
        timeout: float | None = None,
    ) -> dict[str, Any]:
        if not isinstance(reservation, Mapping):
            raise CleanupIdentityError("remote cleanup reservation must be an object")
        normalized_run_id = _validate_run_id(run_id)
        reserved_run_id = reservation.get("runId", reservation.get("run_id"))
        if reserved_run_id is not None:
            if not isinstance(reserved_run_id, str) or _validate_run_id(reserved_run_id) != normalized_run_id:
                raise CleanupIdentityError("remote cleanup reservation runId does not match runId")
        serial = reservation.get("serial") or reservation.get("deviceSerial")
        serial = _validate_serial(serial)
        if not self.is_online(serial=serial):
            raise DeviceOfflineError(f"ADB device {serial} is offline")
        scid, socket_name = _session_socket(
            reservation.get("scid"), reservation.get("socketName") or reservation.get("socket_name")
        )
        if scid is None or socket_name is None:
            raise CleanupUnsupportedError("remote cleanup requires scid and socketName")
        executable = self._adb_executable()
        ps_result = self._command(
            (executable, "-s", serial, "shell", "ps", "-A", "-o", "PID,ARGS"),
            timeout=timeout,
        )
        returncode, output = self._command_output(ps_result)
        if returncode != 0:
            raise DeviceOfflineError("cannot inspect scrcpy server processes")
        matches = self._matching_remote_process(output.splitlines(), scid=scid, socket_name=socket_name)
        if not matches:
            return {"status": "success", "detail": "remote_session_absent"}
        if len(matches) != 1:
            raise CleanupError("scrcpy remote session identity is ambiguous")
        pid = matches[0]
        kill_result = self._command(
            (executable, "-s", serial, "shell", "kill", "-TERM", str(pid)),
            timeout=timeout,
        )
        kill_returncode, _ = self._command_output(kill_result)
        if kill_returncode != 0:
            raise DeviceOfflineError("scrcpy remote session cleanup failed")
        return {"status": "success", "detail": "remote_session_terminated", "pid": pid}


class CleanupActionExecutor:
    """Replay only the run-scoped input/window/device cleanup in a journal."""

    _WORKER_IDLE_TIMEOUT = 0.25

    def __init__(
        self,
        *,
        run_id: str | None = None,
        win32_backend: Any = None,
        adb_backend: Any = None,
        remote_cleanup: Callable[..., Any] | None = None,
        platform_name: str | None = None,
        timeouts: CleanupTimeouts | Mapping[str, float] | None = None,
        clock: Callable[[], float] = time.monotonic,
    ) -> None:
        if run_id is not None:
            if not isinstance(run_id, str) or not run_id.strip():
                raise ValueError("run_id must be a non-empty string")
            self.run_id = _validate_run_id(run_id.strip())
        else:
            self.run_id = None
        self.win32_backend = win32_backend
        self.adb_backend = adb_backend
        self.remote_cleanup = remote_cleanup
        self.platform_name = platform_name or platform_module.system()
        self.clock = clock
        if timeouts is None:
            self.timeouts = CleanupTimeouts()
        elif isinstance(timeouts, CleanupTimeouts):
            self.timeouts = timeouts
        elif isinstance(timeouts, Mapping):
            defaults = CleanupTimeouts()
            self.timeouts = CleanupTimeouts(
                **{
                    field: max(0.0, float(timeouts.get(field, getattr(defaults, field))))
                    for field in ("window_input", "window_restore", "android_input", "adb_forward", "remote")
                }
            )
        else:
            raise TypeError("timeouts must be CleanupTimeouts or a mapping")
        self._lock = threading.RLock()
        self._worker_condition = threading.Condition(self._lock)
        self._bound_run_id: str | None = self.run_id
        self._completed_steps: dict[str, dict[str, Any]] = {}
        self._inflight: set[str] = set()
        self._worker_thread: threading.Thread | None = None
        self._worker_task: _CleanupWork | None = None
        self._worker_busy = False
        self._worker_stop_requested = False
        self._closed = False
        self._worker_count = 0
        self._timeout_count = 0
        self._deferred_count = 0
        self._last_result: dict[str, Any] | None = None
        self._last_error: str | None = None

    def __call__(self, source: Any, *, run_id: str | None = None, ledger: Any = None) -> dict[str, Any]:
        return self.execute(source, run_id=run_id, ledger=ledger)

    def execute(
        self,
        source: Any,
        *,
        run_id: str | None = None,
        ledger: Any = None,
    ) -> dict[str, Any]:
        """Execute a journal or one ``CleanupLedger`` action.

        ``CleanupLedger.recover(executor=...)`` invokes an executor once per
        action as ``execute(action, ledger=ledger)``.  The same public method
        also accepts a ledger/snapshot directly and executes all independent
        steps, which is convenient for a supervisor that batches recovery.
        """

        if ledger is not None:
            return self._execute_action(source, ledger, run_id=run_id)
        return self._execute_journal(source, run_id=run_id)

    def _execute_journal(self, ledger: Any, *, run_id: str | None = None) -> dict[str, Any]:
        """Execute every independent step and return structured outcomes."""

        data = self._snapshot(ledger)
        expected_run_id = self._bind_identity(data, run_id)
        reservation = data.get("reservation")
        if isinstance(reservation, Mapping):
            for key, value in reservation.items():
                data.setdefault(key, _copy(value))
        kind = _target_kind(data)
        steps: dict[str, dict[str, Any]] = {}
        if kind == "pc":
            steps["window_input_release"] = self._run_step(
                "window_input_release", lambda: self._release_window_input(data, expected_run_id), self.timeouts.window_input
            )
            steps["window_restore"] = self._run_step(
                "window_restore", lambda: self._restore_window(data, expected_run_id), self.timeouts.window_restore
            )
        elif kind in {"mumu", "adb"}:
            steps["android_input_release"] = self._run_step(
                "android_input_release", lambda: self._release_android_input(data, expected_run_id), self.timeouts.android_input
            )
            steps["adb_forward_remove"] = self._run_step(
                "adb_forward_remove", lambda: self._remove_forward(data, expected_run_id), self.timeouts.adb_forward
            )
            steps["remote_scrcpy_cleanup"] = self._run_step(
                "remote_scrcpy_cleanup", lambda: self._remote_cleanup(data, expected_run_id), self.timeouts.remote
            )
        else:
            steps["target_validation"] = {
                "status": "failed",
                "detail": "journal target kind is missing or unsupported",
            }

        overall = _status_from_action_results(list(steps.values()))
        result = {"runId": expected_run_id, "targetKind": kind, "status": overall, "steps": steps}
        if any(step.get("status") == "deferred" for step in steps.values()):
            result["deviceOffline"] = True
        return result

    def _bind_identity(self, data: Mapping[str, Any], requested_run_id: str | None = None) -> str:
        journal_run_id = _run_id(data)
        if requested_run_id is not None:
            if not isinstance(requested_run_id, str) or not requested_run_id.strip():
                raise CleanupIdentityError("requested runId must be a non-empty string")
            requested_run_id = _validate_run_id(requested_run_id.strip())
        expected_run_id = self.run_id or requested_run_id or journal_run_id
        if not expected_run_id:
            raise CleanupIdentityError("cleanup journal is missing runId")
        if requested_run_id is not None and requested_run_id != expected_run_id:
            raise CleanupIdentityError("requested runId does not match executor runId")
        if journal_run_id != expected_run_id:
            raise CleanupIdentityError("cleanup journal runId does not match requested runId")
        for resource in _resource_records(data):
            metadata = _metadata(resource)
            resource_run_id = _first(resource, "runId", "run_id") or _first(metadata, "runId", "run_id")
            if resource_run_id is not None:
                if not isinstance(resource_run_id, str) or _validate_run_id(resource_run_id) != expected_run_id:
                    raise CleanupIdentityError("cleanup resource belongs to another run")
        with self._lock:
            if self._bound_run_id is None:
                self._bound_run_id = expected_run_id
            elif self._bound_run_id != expected_run_id:
                raise CleanupIdentityError("executor cannot be reused for another run")
        return expected_run_id

    def _execute_action(self, action: Any, ledger: Any, *, run_id: str | None = None) -> dict[str, Any]:
        """Handle one action object emitted by ``CleanupLedger.recover``."""

        if not isinstance(action, Mapping):
            raise CleanupError("cleanup action must be an object")
        data = self._snapshot(ledger)
        expected_run_id = self._bind_identity(data, run_id)
        payload = action.get("payload")
        payload = dict(payload) if isinstance(payload, Mapping) else {}
        action_run_id = _first(action, "runId", "run_id") or _first(payload, "runId", "run_id")
        if action_run_id is not None:
            if not isinstance(action_run_id, str) or _validate_run_id(action_run_id) != expected_run_id:
                raise CleanupIdentityError("cleanup action runId does not match ledger runId")

        reservation = data.get("reservation")
        if isinstance(reservation, Mapping):
            for key, value in reservation.items():
                data.setdefault(key, _copy(value))
        for key, value in payload.items():
            data[key] = _copy(value)
        data["runId"] = expected_run_id
        action_id = str(action.get("actionId", action.get("id", "")))
        action_type = str(action.get("actionType", action.get("type", ""))).casefold()
        resource_type = str(payload.get("resourceType", "")).casefold()
        if action_type == "resource":
            action_type = resource_type
            metadata = payload.get("metadata")
            if isinstance(metadata, Mapping):
                data["resources"] = []
                data.update(_copy(dict(metadata)))

        if action_type in {"input.release", "input_release"}:
            obligations = payload.get("obligations", payload.get("inputState"))
            if isinstance(obligations, Mapping):
                data.update(_copy(dict(obligations)))
            kind = _target_kind(data)
            if kind == "pc":
                result = self._run_step(
                    action_id or f"{expected_run_id}:input-release", lambda: self._release_window_input(data, expected_run_id), self.timeouts.window_input
                )
            elif kind in {"mumu", "adb"}:
                result = self._run_step(
                    action_id or f"{expected_run_id}:input-release", lambda: self._release_android_input(data, expected_run_id), self.timeouts.android_input
                )
            else:
                result = {"status": "deferred", "detail": "input target kind is unavailable"}
        elif action_type in {"window.restore", "window_restore"}:
            data.setdefault("target", {"kind": "pc"})
            result = self._run_step(
                action_id or f"{expected_run_id}:window-restore", lambda: self._restore_window(data, expected_run_id), self.timeouts.window_restore
            )
        elif action_type in {"adb.forward", "adb_forward"} or "forward" in resource_type:
            data.setdefault("target", {"kind": "adb"})
            result = self._run_step(
                action_id or f"{expected_run_id}:adb-forward", lambda: self._remove_forward(data, expected_run_id), self.timeouts.adb_forward
            )
        elif action_type in {"scrcpy.server", "scrcpy_server"} or "server" in resource_type or "scrcpy" in resource_type:
            data.setdefault("target", {"kind": "adb"})
            result = self._run_step(
                action_id or f"{expected_run_id}:scrcpy-server", lambda: self._remote_cleanup(data, expected_run_id), self.timeouts.remote
            )
        else:
            # Runner process/job cleanup and unknown resource types belong to
            # the host that owns process handles; never pretend this module
            # performed an unsafe action it does not implement.
            result = {"status": "deferred", "detail": f"unsupported cleanup action: {action_type or resource_type}"}
        return result

    @staticmethod
    def _snapshot(ledger: Any) -> dict[str, Any]:
        if isinstance(ledger, Mapping):
            return _copy(dict(ledger))
        snapshot = getattr(ledger, "snapshot", None)
        if callable(snapshot):
            value = snapshot()
        else:
            value = getattr(ledger, "data", None)
        if not isinstance(value, Mapping):
            raise TypeError("ledger must be a mapping or expose snapshot()/data")
        return _copy(dict(value))

    def _run_step(self, name: str, action: Callable[[], Mapping[str, Any]], timeout: float) -> dict[str, Any]:
        with self._lock:
            cached = self._completed_steps.get(name)
            if cached is not None:
                result = _copy(cached)
                result["alreadyDone"] = True
                return result
            if self._closed:
                self._deferred_count += 1
                return {
                    "status": "deferred",
                    "detail": "cleanup executor is closed",
                    "alreadyDone": False,
                }
            # A timed-out callback may still be executing.  Do not enqueue a
            # second attempt for another step (or create another daemon
            # thread) while that one is alive.  This is the key per-run
            # boundedness invariant.
            if self._worker_busy or self._worker_task is not None or self._inflight:
                self._deferred_count += 1
                return {
                    "status": "deferred",
                    "detail": "previous cleanup action is still running",
                    "alreadyDone": False,
                }

            started = self.clock()
            work = _CleanupWork(name=name, action=action, done=threading.Event(), started=started)
            self._worker_task = work
            self._inflight.add(name)
            if not self._ensure_worker_locked():
                self._worker_task = None
                self._inflight.discard(name)
                self._deferred_count += 1
                return {
                    "status": "deferred",
                    "detail": "cleanup worker is unavailable",
                    "alreadyDone": False,
                }
            self._worker_condition.notify_all()

        wait_timeout = max(0.0, float(timeout))
        if not work.done.wait(wait_timeout):
            with self._lock:
                self._timeout_count += 1
                self._deferred_count += 1
            return {
                "status": "deferred",
                "detail": f"step exceeded {wait_timeout:.3f}s timeout; worker remains inflight",
                "alreadyDone": False,
                "elapsedMs": max(0.0, (self.clock() - started) * 1000.0),
            }

        with self._lock:
            result = _copy(work.result) if isinstance(work.result, Mapping) else None
        if result is None:
            result = {
                "status": "failed",
                "detail": "cleanup worker returned no result",
                "alreadyDone": False,
            }
        result.setdefault("alreadyDone", False)
        result.setdefault("elapsedMs", max(0.0, (self.clock() - started) * 1000.0))
        return result

    def _ensure_worker_locked(self) -> bool:
        """Start at most one worker for this run, with the lock held."""

        worker = self._worker_thread
        if worker is not None and worker.is_alive():
            return True
        if self._worker_stop_requested or self._closed:
            return False
        # A dead thread with a task still marked active must not be replaced:
        # replacing it could execute the same external action twice if the
        # original thread is merely between state updates.
        if self._worker_busy:
            return False
        worker = threading.Thread(
            target=self._worker_loop,
            name=f"AALCCleanup-{self.run_id or 'run'}",
            daemon=True,
        )
        self._worker_thread = worker
        self._worker_count += 1
        worker.start()
        return True

    def _worker_loop(self) -> None:
        """Run queued callbacks serially until ``close`` requests a drain."""

        while True:
            with self._worker_condition:
                while self._worker_task is None and not self._worker_stop_requested:
                    # Do not retain one idle daemon forever when a caller
                    # forgets to call close after a one-shot recovery.  A
                    # timed-out callback is never idle, so this cannot create
                    # a replacement while an unbounded backend is running.
                    if not self._worker_condition.wait(self._WORKER_IDLE_TIMEOUT):
                        self._worker_thread = None
                        self._worker_condition.notify_all()
                        return
                if self._worker_task is None and self._worker_stop_requested:
                    self._worker_thread = None
                    self._worker_condition.notify_all()
                    return
                work = self._worker_task
                self._worker_task = None
                self._worker_busy = True

            result: dict[str, Any]
            try:
                raw = work.action()
                if isinstance(raw, Mapping):
                    result = dict(raw)
                else:
                    result = {"status": "failed", "detail": "cleanup worker returned a non-object result"}
                result.setdefault("status", "success")
                result.setdefault("alreadyDone", False)
            except BaseException as error:  # isolate one cleanup step from the worker lifecycle
                result = {
                    "status": "deferred" if _offline_exception(error) else "failed",
                    "error": type(error).__name__,
                    "detail": str(error),
                    "alreadyDone": False,
                }

            result.setdefault("elapsedMs", max(0.0, (self.clock() - work.started) * 1000.0))
            with self._worker_condition:
                work.result = result
                if result.get("status") == "success":
                    self._completed_steps[work.name] = _copy(result)
                if result.get("status") in {"failed", "deferred"}:
                    detail = result.get("detail")
                    self._last_error = str(detail) if detail is not None else None
                self._last_result = _copy(result)
                self._worker_busy = False
                self._inflight.discard(work.name)
                work.done.set()
                self._worker_condition.notify_all()

    def drain(self, timeout: float = 5.0) -> dict[str, Any]:
        """Wait for the one in-flight callback without starting new work.

        A callback supplied by an embedding host may ignore its operation
        timeout.  ``drain`` is therefore itself bounded; when it expires the
        daemon worker is intentionally left alone and diagnostics report the
        outstanding action instead of spawning another retry thread.
        """

        wait_timeout = max(0.0, float(timeout))
        deadline = self.clock() + wait_timeout
        with self._worker_condition:
            while self._worker_busy or self._worker_task is not None:
                remaining = deadline - self.clock()
                if remaining <= 0:
                    break
                self._worker_condition.wait(remaining)
        value = self.diagnostics()
        value["drained"] = not value["inflight"] and not value["queued"]
        value["drainTimeout"] = wait_timeout if not value["drained"] else 0.0
        return value

    def close(self, timeout: float = 5.0) -> dict[str, Any]:
        """Stop accepting work and boundedly drain the controlled worker."""

        with self._worker_condition:
            self._closed = True
            self._worker_stop_requested = True
            self._worker_condition.notify_all()
            worker = self._worker_thread
        if worker is not None and worker is not threading.current_thread():
            worker.join(timeout=max(0.0, float(timeout)))
        value = self.diagnostics()
        value["closed"] = True
        value["drained"] = not value["inflight"] and not value["queued"]
        return value

    def diagnostics(self) -> dict[str, Any]:
        """Return bounded worker state suitable for logs/structured errors."""

        with self._lock:
            worker = self._worker_thread
            return {
                "bounded": True,
                "maxWorkers": 1,
                "maxInflight": 1,
                "workerCount": self._worker_count,
                "workerAlive": bool(worker is not None and worker.is_alive()),
                "workerName": worker.name if worker is not None else None,
                "inflight": bool(self._worker_busy or self._inflight),
                "inflightAction": next(iter(self._inflight), None),
                "queued": self._worker_task is not None,
                "timedOut": self._timeout_count,
                "deferred": self._deferred_count,
                "completedSteps": len(self._completed_steps),
                "lastResult": _copy(self._last_result),
                "lastError": self._last_error,
                "closed": self._closed,
            }

    diagnostic = diagnostics

    def _win32(self) -> Any:
        if self.platform_name.casefold() != "windows":
            return None
        if self.win32_backend is None:
            self.win32_backend = Win32CleanupBackend()
        return self.win32_backend

    def _release_window_input(self, data: Mapping[str, Any], run_id: str) -> dict[str, Any]:
        backend = self._win32()
        if backend is None:
            return {"status": "success", "skipped": True, "detail": "non_windows"}
        hwnd = _window_handle(data)
        if hwnd is None:
            return {"status": "failed", "detail": "journal HWND is missing or invalid"}
        window_probe = self._window_probe(backend, hwnd, run_id)
        if window_probe.get("status") != "success":
            return {"status": str(window_probe.get("status", "failed")), "detail": "window availability probe failed", "probe": window_probe}
        if window_probe.get("value", True) is False:
            return {"status": "deferred", "detail": "target window is unavailable"}

        buttons = _records_for_keys(
            data,
            (
                "pressedMouseButtons",
                "mouseButtons",
                "buttonsDown",
                "mouse_buttons",
                "mouseButton",
                "buttonDown",
                "button",
            ),
        )
        keys = _records_for_keys(
            data,
            ("pressedKeys", "keysDown", "keys", "keyCodes", "keyCode", "keyDown", "key"),
        )
        actions: list[dict[str, Any]] = []
        seen_buttons: set[str] = set()
        for value in buttons:
            button = _normalise_button(value)
            if button is None:
                actions.append({"status": "failed", "detail": f"invalid mouse button: {value!r}"})
                continue
            if button in seen_buttons:
                continue
            seen_buttons.add(button)
            method = getattr(backend, "release_mouse", None) or getattr(backend, "mouse_up", None)
            if not callable(method):
                actions.append({"status": "failed", "detail": "Win32 backend lacks release_mouse"})
                break
            actions.append(
                self._run_action(
                    f"{run_id}:window:mouse:{button}",
                    method,
                    hwnd=hwnd,
                    button=button,
                    run_id=run_id,
                )
            )
        seen_keys: set[int | str] = set()
        for value in keys:
            key = _normalise_key(value)
            if key is None:
                actions.append({"status": "failed", "detail": f"invalid virtual key: {value!r}"})
                continue
            if key in seen_keys:
                continue
            seen_keys.add(key)
            method = getattr(backend, "release_key", None) or getattr(backend, "key_up", None)
            if not callable(method):
                actions.append({"status": "failed", "detail": "Win32 backend lacks release_key"})
                break
            actions.append(self._run_action(f"{run_id}:window:key:{key}", method, hwnd=hwnd, key=key, run_id=run_id))
        return {
            "status": _status_from_action_results(actions) if actions else "success",
            "actions": actions,
            "detail": "released_recorded_inputs" if actions else "no_recorded_inputs",
        }

    def _window_probe(self, backend: Any, hwnd: int, run_id: str) -> dict[str, Any]:
        method = getattr(backend, "is_window", None)
        if not callable(method):
            return {"status": "success", "value": True}
        result = self._run_action(f"{run_id}:window:exists", method, hwnd=hwnd, run_id=run_id)
        # A negative existence probe is an offline/deferred condition, not a
        # failed cleanup operation.  Preserve the boolean so callers can
        # avoid posting input to a recycled HWND.
        if result.get("value") is False and result.get("status") == "failed":
            result["status"] = "success"
        return result

    def _restore_window(self, data: Mapping[str, Any], run_id: str) -> dict[str, Any]:
        backend = self._win32()
        if backend is None:
            return {"status": "success", "skipped": True, "detail": "non_windows"}
        hwnd = _window_handle(data)
        state = _window_state(data)
        if hwnd is None:
            return {"status": "failed", "detail": "journal HWND is missing or invalid"}
        if not state:
            return {"status": "success", "detail": "no_original_window_state"}
        window_probe = self._window_probe(backend, hwnd, run_id)
        if window_probe.get("status") != "success":
            return {"status": str(window_probe.get("status", "failed")), "detail": "window availability probe failed", "probe": window_probe}
        if window_probe.get("value", True) is False:
            return {"status": "deferred", "detail": "target window is unavailable"}
        method = getattr(backend, "restore_window", None)
        if not callable(method):
            return {"status": "failed", "detail": "Win32 backend lacks restore_window"}
        action = self._run_action(
            f"{run_id}:window:restore",
            method,
            hwnd=hwnd,
            state=_copy(state),
            rect=_first(state, "rect", "windowRect", "originalWindowRect"),
            style=_first(state, "style", "windowStyle"),
            ex_style=_first(state, "exStyle", "extendedStyle", "ex_style"),
            topmost=state.get("topmost"),
            run_id=run_id,
        )
        return {"status": action.get("status", "failed"), "actions": [action], "detail": "restored_window_state"}

    def _device_online(self, backend: Any, serial: str, run_id: str) -> dict[str, Any]:
        method = getattr(backend, "is_online", None)
        if not callable(method):
            return {"status": "success", "value": True}
        return self._run_action(f"{run_id}:adb:online", method, serial=serial, run_id=run_id)

    def _android_context(self, data: Mapping[str, Any], run_id: str) -> tuple[str | None, dict[str, Any]]:
        serial = _serial(data)
        nested_reservation = data.get("reservation")
        nested_reservation = nested_reservation if isinstance(nested_reservation, Mapping) else {}
        reservation = {
            "runId": run_id,
            "serial": serial,
            "scid": _first(data, "scid", "reservedScrcpyScid", "reserved_scid")
            or _first(nested_reservation, "scid", "reservedScrcpyScid", "reserved_scid"),
            "socketName": _first(data, "socketName", "reservedSocketName", "reserved_socket_name")
            or _first(nested_reservation, "socketName", "reservedSocketName", "reserved_socket_name"),
            "adbForwardPort": _first(
                data,
                "adbForwardPort",
                "reservedAdbForwardPort",
                "reserved_adb_forward_port",
            )
            or _first(
                nested_reservation,
                "adbForwardPort",
                "reservedAdbForwardPort",
                "reserved_adb_forward_port",
            ),
        }
        # A resource.created event can be the only durable record if the
        # initial reservation write raced process creation.  Accept only the
        # same run's resource metadata (validated by execute()) and fill the
        # missing scoped identifiers from it.
        for resource in _resource_records(data):
            metadata = _metadata(resource)
            for output_key, keys in {
                "scid": ("scid", "reservedScrcpyScid", "reserved_scid"),
                "socketName": ("socketName", "reservedSocketName", "reserved_socket_name"),
                "adbForwardPort": (
                    "adbForwardPort",
                    "reservedAdbForwardPort",
                    "reserved_adb_forward_port",
                    "forwardPort",
                    "localPort",
                    "local_port",
                ),
            }.items():
                if reservation[output_key] is None:
                    candidate = _first(metadata, *keys)
                    if candidate is not None:
                        reservation[output_key] = candidate
        reservation["runId"] = _validate_run_id(run_id)
        if reservation["serial"] is not None:
            reservation["serial"] = _validate_serial(reservation["serial"])
        reservation["scid"], reservation["socketName"] = _session_socket(
            reservation.get("scid"), reservation.get("socketName")
        )
        if reservation.get("adbForwardPort") is not None:
            reservation["adbForwardPort"] = _validate_port(reservation["adbForwardPort"])
        return serial, reservation

    def _release_android_input(self, data: Mapping[str, Any], run_id: str) -> dict[str, Any]:
        serial, reservation = self._android_context(data, run_id)
        if serial is None:
            return {"status": "failed", "detail": "journal device serial is missing"}
        backend = self.adb_backend or AdbCleanupBackend()
        online = self._device_online(backend, serial, run_id)
        if online.get("status") != "success" or online.get("value", True) is False:
            return {"status": "deferred", "detail": "device is offline", "probe": online}
        pointers = _records_for_keys(
            data,
            ("activeTouches", "touches", "pointers", "touchPointers", "pointerIds", "pointerId"),
        )
        normalised: list[int] = []
        for value in pointers:
            pointer = _normalise_pointer(value)
            if pointer is not None and pointer not in normalised:
                normalised.append(pointer)
        if not normalised:
            return {"status": "success", "detail": "no_recorded_touches"}
        method = getattr(backend, "cancel_touch", None) or getattr(backend, "send_touch_cancel", None)
        batch_method = getattr(backend, "cancel_touches", None)
        actions: list[dict[str, Any]] = []
        if callable(method):
            for pointer in normalised:
                actions.append(
                    self._run_action(
                        f"{run_id}:adb:touch:{pointer}",
                        method,
                        serial=serial,
                        pointer_id=pointer,
                        local_port=reservation.get("adbForwardPort"),
                        remote=self._forward_remote_from_data(data, reservation),
                        scid=reservation.get("scid"),
                        socket_name=reservation.get("socketName"),
                        run_id=run_id,
                    )
                )
        elif callable(batch_method):
            actions.append(
                self._run_action(
                    f"{run_id}:adb:touch:batch",
                    batch_method,
                    serial=serial,
                    pointer_ids=tuple(normalised),
                    local_port=reservation.get("adbForwardPort"),
                    remote=self._forward_remote_from_data(data, reservation),
                    scid=reservation.get("scid"),
                    socket_name=reservation.get("socketName"),
                    run_id=run_id,
                )
            )
        else:
            return {"status": "deferred", "detail": "run-scoped touch cleanup backend is not bound"}
        return {"status": _status_from_action_results(actions), "actions": actions, "detail": "cancelled_recorded_touches"}

    @staticmethod
    def _forward_remote_from_data(data: Mapping[str, Any], reservation: Mapping[str, Any]) -> str | None:
        for resource in _resource_records(data):
            if "forward" not in str(resource.get("resourceType", "")).casefold():
                continue
            remote = _first(_metadata(resource), "remote", "forwardRemote", "remoteSocket")
            if remote is not None:
                return remote
        socket_name = reservation.get("socketName")
        return f"localabstract:{socket_name}" if socket_name is not None else None

    def _remove_forward(self, data: Mapping[str, Any], run_id: str) -> dict[str, Any]:
        serial, reservation = self._android_context(data, run_id)
        if serial is None:
            return {"status": "failed", "detail": "journal device serial is missing"}
        port = _safe_int(reservation.get("adbForwardPort"), minimum=1, maximum=65535)
        remote = None
        for resource in _resource_records(data):
            resource_type = str(resource.get("resourceType", "")).casefold()
            if "forward" not in resource_type:
                continue
            metadata = _metadata(resource)
            candidate = _first(metadata, "adbForwardPort", "forwardPort", "localPort", "local_port", "port")
            if port is None:
                port = _safe_int(candidate, minimum=1, maximum=65535)
            remote = _first(metadata, "remote", "forwardRemote", "remoteSocket")
            resource_run_id = _first(resource, "runId", "run_id") or _first(metadata, "runId", "run_id")
            if resource_run_id is not None and str(resource_run_id) != run_id:
                return {"status": "failed", "detail": "forward resource belongs to another run"}
            break
        if port is None:
            return {"status": "success", "detail": "no_reserved_forward"}
        backend = self.adb_backend or AdbCleanupBackend()
        online = self._device_online(backend, serial, run_id)
        if online.get("status") != "success" or online.get("value", True) is False:
            return {"status": "deferred", "detail": "device is offline", "probe": online}
        method = getattr(backend, "remove_forward", None)
        if not callable(method):
            return {"status": "failed", "detail": "ADB backend lacks identity-checked remove_forward"}
        action = self._run_action(
            f"{run_id}:adb:forward:{port}",
            method,
            serial=serial,
            local_port=port,
            remote=remote,
            scid=reservation.get("scid"),
            socket_name=reservation.get("socketName"),
            run_id=run_id,
        )
        return {"status": action.get("status", "failed"), "actions": [action], "detail": "removed_owned_forward"}

    def _remote_cleanup(self, data: Mapping[str, Any], run_id: str) -> dict[str, Any]:
        serial, reservation = self._android_context(data, run_id)
        has_remote_resource = any(
            any(token in str(resource.get("resourceType", "")).casefold() for token in ("scrcpy", "remote", "server"))
            for resource in _resource_records(data)
        )
        if not has_remote_resource and not reservation.get("scid") and not reservation.get("socketName"):
            return {"status": "success", "detail": "no_reserved_remote_session"}
        if serial is None:
            return {"status": "failed", "detail": "journal device serial is missing"}
        backend = self.adb_backend or AdbCleanupBackend()
        online = self._device_online(backend, serial, run_id)
        if online.get("status") != "success" or online.get("value", True) is False:
            return {"status": "deferred", "detail": "device is offline", "probe": online}
        callback = self.remote_cleanup or getattr(backend, "remote_cleanup", None)
        if not callable(callback):
            return {"status": "deferred", "detail": "run-scoped remote cleanup backend is not bound"}
        callback_result = self._run_action(
            f"{run_id}:adb:remote",
            callback,
            reservation=_copy(reservation),
            run_id=run_id,
        )
        return {"status": callback_result.get("status", "failed"), "actions": [callback_result], "detail": "requested_scoped_remote_cleanup"}

    def _run_action(self, action_key: str, callback: Callable[..., Any], **kwargs: Any) -> dict[str, Any]:
        timeout = kwargs.pop("timeout", None)

        def action() -> Mapping[str, Any]:
            value = _call_with_supported_kwargs(callback, **kwargs)
            if isinstance(value, Mapping):
                result = dict(value)
                result["status"] = _normalise_status(result.get("status", "success"))
                return result
            return {"status": _normalise_status(value), "value": value}

        # Inner action calls use the operation timeout supplied by _run_step's
        # outer bound.  Keeping a separate key still prevents duplicate actions
        # when an individual backend call outlives that bound.
        return self._run_action_with_default_timeout(action_key, action, timeout)

    def _run_action_with_default_timeout(
        self,
        action_key: str,
        action: Callable[[], Mapping[str, Any]],
        timeout: float | None,
    ) -> dict[str, Any]:
        del action_key, timeout
        # All backend calls are already inside the executor's single worker
        # (``_run_step``).  A second thread here was the source of the old
        # timeout leak: the outer step could time out while this inner daemon
        # remained blocked, and retries could accumulate more inners.  Keep
        # the adapter synchronous so one worker owns the whole cleanup step.
        try:
            result = action()
        except BaseException as error:  # isolate one backend action from the worker lifecycle
            return {
                "status": "deferred" if _offline_exception(error) else "failed",
                "error": type(error).__name__,
                "detail": str(error),
                "alreadyDone": False,
            }
        if not isinstance(result, Mapping):
            return {"status": "failed", "detail": "cleanup action returned no result", "alreadyDone": False}
        return dict(result)


def _validate_record_identifiers(data: Mapping[str, Any], run_id: str) -> None:
    """Validate only identifiers that can address an external resource.

    Ledger metadata may contain arbitrary diagnostic fields, so validation is
    deliberately limited to the known reservation/target/resource locations.
    Invalid values fail before any backend is selected; a malformed journal can
    therefore never be turned into an ADB or Win32 operation by accident.
    """

    _validate_run_id(run_id)
    kind = _target_kind(data)
    target = _target(data)
    reservation = data.get("reservation")
    reservation = reservation if isinstance(reservation, Mapping) else {}
    roots = (data, target, reservation)
    for root in roots:
        for key in ("deviceSerial", "serial", "device_serial", "endpoint"):
            if key in root and root[key] is not None:
                _validate_serial(root[key])
        for key in ("scid", "reservedScrcpyScid", "reserved_scid"):
            if key in root and root[key] is not None:
                _validate_scid(root[key])
        for key in ("socketName", "reservedSocketName", "reserved_socket_name"):
            if key in root and root[key] is not None:
                _validate_session_name(root[key])
        for key in ("adbForwardPort", "reservedAdbForwardPort", "reserved_adb_forward_port"):
            if key in root and root[key] is not None:
                _validate_port(root[key])
        for key in ("hwnd", "windowHandle", "window_handle"):
            if key in root and root[key] is not None:
                value = root[key]
                if isinstance(value, bool) or not isinstance(value, int) or value <= 0:
                    raise CleanupIdentityError("HWND must be a positive integer")

    for resource in _resource_records(data):
        metadata = _metadata(resource)
        resource_run_id = _first(resource, "runId", "run_id") or _first(metadata, "runId", "run_id")
        if resource_run_id is not None:
            if not isinstance(resource_run_id, str):
                raise CleanupIdentityError("cleanup resource runId must be a string")
            if _validate_run_id(resource_run_id) != run_id:
                raise CleanupIdentityError("cleanup resource belongs to another run")
        for key in ("deviceSerial", "serial", "device_serial"):
            if key in metadata and metadata[key] is not None:
                _validate_serial(metadata[key])
        for key in ("scid", "reservedScrcpyScid", "reserved_scid"):
            if key in metadata and metadata[key] is not None:
                _validate_scid(metadata[key])
        for key in ("socketName", "reservedSocketName", "reserved_socket_name"):
            if key in metadata and metadata[key] is not None:
                _validate_session_name(metadata[key])
        for key in ("adbForwardPort", "forwardPort", "localPort", "local_port", "port"):
            if key in metadata and metadata[key] is not None:
                _validate_port(metadata[key])
        for key in ("remote", "forwardRemote", "remoteSocket"):
            if key in metadata and metadata[key] is not None:
                _validate_remote(metadata[key])

    if kind in {"mumu", "adb"} and _serial(data) is None and not reservation.get("deviceSerial"):
        raise CleanupIdentityError("ADB cleanup record is missing device serial")


def build_default_cleanup_executor(
    run_id: str,
    record: Any,
    *,
    platform_name: str | None = None,
    timeouts: CleanupTimeouts | Mapping[str, float] | None = None,
    adb_module: Any = None,
    user32: Any = None,
    ctypes_module: Any = None,
    socket_factory: Callable[..., Any] | None = None,
    command_runner: Callable[..., Any] | None = None,
    adb_path: str | None = None,
    remote_cleanup: Callable[..., Any] | None = None,
    clock: Callable[[], float] = time.monotonic,
) -> CleanupActionExecutor:
    """Build the production-safe executor for one cleanup record.

    ``record`` may be a :class:`CleanupLedger` or its snapshot mapping.  The
    record is validated immediately, but platform dependencies are resolved
    lazily by the returned executor.  Thus a missing ``adbutils``/Win32 API or
    an offline device produces a structured ``deferred`` step during recovery,
    leaving the durable obligation available for a later retry.

    Optional backend arguments are intentionally narrow test seams.  A host
    can inject a fake ``user32``, adb client, socket factory, command runner,
    or already run-scoped remote cleanup callback without changing cleanup
    semantics.
    """

    normalized_run_id = _validate_run_id(run_id.strip() if isinstance(run_id, str) else run_id)
    executor = CleanupActionExecutor(
        run_id=normalized_run_id,
        adb_backend=AdbCleanupBackend(
            adb_module,
            socket_factory=socket_factory,
            command_runner=command_runner,
            adb_path=adb_path,
        ),
        remote_cleanup=remote_cleanup,
        platform_name=platform_name,
        timeouts=timeouts,
        clock=clock,
    )
    snapshot = executor._snapshot(record)
    executor._bind_identity(snapshot, normalized_run_id)
    _validate_record_identifiers(snapshot, normalized_run_id)
    if user32 is not None or ctypes_module is not None:
        executor.win32_backend = Win32CleanupBackend(user32=user32, ctypes_module=ctypes_module)
    return executor


# Explicit names for hosts/tests that want to document the implementation
# choice; ``Win32CleanupBackend`` remains the compatibility name.
CtypesWin32CleanupBackend = Win32CleanupBackend
WindowsCleanupBackend = Win32CleanupBackend

# Short alias for hosts that prefer an explicit resource-oriented name.
ResourceCleanupExecutor = CleanupActionExecutor


def execute_cleanup(
    source: Any,
    *,
    run_id: str | None = None,
    executor: CleanupActionExecutor | None = None,
    ledger: Any = None,
    **executor_kwargs: Any,
) -> dict[str, Any]:
    """One-shot callable for direct or per-action CleanupLedger recovery."""

    active_executor = executor or CleanupActionExecutor(run_id=run_id, **executor_kwargs)
    if ledger is not None:
        return active_executor.execute(source, run_id=run_id, ledger=ledger)
    return active_executor.execute(source, run_id=run_id)


__all__ = [
    "AdbCleanupBackend",
    "CleanupActionExecutor",
    "CleanupError",
    "CleanupIdentityError",
    "CleanupTimeouts",
    "CleanupUnsupportedError",
    "CtypesWin32CleanupBackend",
    "DeviceOfflineError",
    "ResourceCleanupExecutor",
    "WindowsCleanupBackend",
    "Win32CleanupBackend",
    "build_default_cleanup_executor",
    "execute_cleanup",
]
