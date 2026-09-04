"""JSON-RPC method dispatch for the GPUI sidecar.

Transport code lives in :mod:`module.websocket_server`; this module only
translates validated RPC calls into existing Python services.  Keeping the
boundary here also makes it possible to exercise the protocol without a
running WebSocket server.
"""

from __future__ import annotations

import inspect
import re
from typing import Any, Callable

from module.backend_application import BackendApplication
from module.device_manager import DeviceError, DeviceManager, get_device_manager
from module.logger import log

SCHEMA_VERSION = 3


_EXECUTION_METHODS = frozenset(
    {
        "execution.start",
        "execution.pause",
        "execution.resume",
        "execution.stop",
        "execution.getState",
    }
)


_ERROR_CODE_NAMES = {
    -32600: "INVALID_REQUEST",
    -32601: "METHOD_NOT_FOUND",
    -32602: "INVALID_PARAMS",
    -32700: "PARSE_ERROR",
    -32000: "INTERNAL_ERROR",
    -32010: "EXECUTION_BUSY",
    -32011: "INVALID_EXECUTION_STATE",
    -32012: "INVALID_EXECUTION_STATE",
    -32013: "STALE_RUN",
    -32020: "DEVICE_ERROR",
    -32030: "QUEUE_FULL",
}


def _error_code_name(code: int, message: str) -> str:
    """Return a stable protocol code while retaining the JSON-RPC number."""

    if code in _ERROR_CODE_NAMES:
        return _ERROR_CODE_NAMES[code]
    # Backend adapters sometimes raise a structured-looking symbolic message
    # before they can construct RpcDispatchError.  Preserve that contract
    # name without making arbitrary user-facing text part of the code.
    if re.fullmatch(r"[A-Z][A-Z0-9_]{2,63}", message):
        return message
    return str(code)


class RpcDispatchError(Exception):
    """An error that can be returned as a structured JSON-RPC error."""

    def __init__(self, code: int, message: str, data: Any = None) -> None:
        super().__init__(message)
        self.code = code
        self.message = message
        self.data = data


class RpcDispatcher:
    """Dispatch the sidecar's transport-independent JSON-RPC methods."""

    def __init__(
        self,
        device_manager: DeviceManager | None = None,
        *,
        version: str = "unknown",
        shutdown: Callable[[], None] | None = None,
        application: BackendApplication | None = None,
    ) -> None:
        # ``device_manager`` remains accepted for the small device-manager
        # compatibility tests and for callers upgrading from the first
        # sidecar prototype.  Production code passes the complete application
        # context instead.
        self.application = application or BackendApplication(
            device_manager or get_device_manager(),
            version=version,
            shutdown=shutdown,
        )
        self.device_manager = self.application.device_manager
        self.version = version
        self._shutdown = shutdown
        self._busy_checker: Callable[[], bool] = self.application.is_busy
        self.application.set_busy_checker(self._busy_checker)

    def set_busy_checker(self, checker: Callable[[], bool]) -> None:
        self._busy_checker = checker
        self.application.set_busy_checker(checker)

    @staticmethod
    def is_mutating(method: Any) -> bool:
        return method in {
            "app.shutdown",
            "tasks.setConfig",
            "execution.start",
            "execution.stop",
            "execution.pause",
            "execution.resume",
            "team.save",
            "team.delete",
            "team.stats.clear",
            "themePack.updateAll",
            "themePack.resetWeights",
            "resource.sync.start",
            "tool.start",
            "tool.stop",
            "tool.screenshot",
            "tool.resolution.set",
            "tool.resolution.reset",
            "hotkey.set",
            "systemSettings.set",
            "notification.test",
            "preview.setEnabled",
            "device.connect",
            "device.disconnect",
        }

    def dispatch(self, request: dict[str, Any]) -> dict[str, Any]:
        """Return a JSON-RPC response for one request object."""
        request_id = request.get("id")
        if request.get("jsonrpc") != "2.0" or not isinstance(request.get("method"), str):
            return self._error(request_id, -32600, "无效的 JSON-RPC 请求")

        method = request["method"]
        try:
            result = self._call(method, request.get("params"))
            return {"jsonrpc": "2.0", "id": request_id, "result": result}
        except RpcDispatchError as error:
            return self._error(request_id, error.code, error.message, error.data)
        except (TypeError, ValueError) as error:
            return self._error(request_id, -32602, str(error))
        except DeviceError as error:
            return self._error(request_id, -32020, str(error))
        except Exception as error:
            log.exception("RPC 方法执行失败：%s", method)
            return self._error(request_id, -32000, f"后端内部错误：{error}")

    def _call(self, method: str, params: Any) -> Any:
        route_names = {
            "app.ping": "app_ping",
            "app.version": "app_version",
            "app.checkUpdate": "app_check_update",
            "app.shutdown": "app_shutdown",
            "stats.getSummary": "stats_get_summary",
            "stats.getDailySummary": "stats_get_daily_summary",
            "tasks.getConfig": "tasks_get_config",
            "tasks.setConfig": "tasks_set_config",
            "execution.getState": "execution_get_state",
            "execution.start": "execution_start",
            "execution.stop": "execution_stop",
            "execution.pause": "execution_pause",
            "execution.resume": "execution_resume",
            "team.list": "team_list",
            "team.stats.get": "team_stats_get",
            "team.stats.clear": "team_stats_clear",
            "team.preset.list": "team_preset_list",
            "team.save": "team_save",
            "team.delete": "team_delete",
            "sinner.list": "sinner_list",
            "themePack.list": "theme_pack_list",
            "themePack.updateAll": "theme_pack_update_all",
            "themePack.resetWeights": "theme_pack_reset_weights",
            "resource.status": "resource_status",
            "resource.checkUpdate": "resource_check_update",
            "resource.sync.start": "resource_sync_start",
            "tool.start": "tool_start",
            "tool.stop": "tool_stop",
            "tool.screenshot": "tool_screenshot",
            "tool.resolution.set": "tool_resolution_set",
            "tool.resolution.reset": "tool_resolution_reset",
            "hotkey.get": "hotkey_get",
            "hotkey.set": "hotkey_set",
            "systemSettings.get": "system_settings_get",
            "systemSettings.set": "system_settings_set",
            "notification.test": "notification_test",
            "preview.setEnabled": "preview_set_enabled",
            "device.list": "device.list",
            "device.connect": "device.connect",
            "device.disconnect": "device.disconnect",
        }
        route = route_names.get(method)
        if route is None:
            raise RpcDispatchError(-32601, f"Method not found: {method}")
        if method in _EXECUTION_METHODS:
            return self._call_execution(method, route, params)
        if method in {
            "app.ping",
            "app.version",
            "app.checkUpdate",
            "app.shutdown",
            "stats.getSummary",
            "tasks.getConfig",
            "team.list",
            "team.preset.list",
            "sinner.list",
            "themePack.list",
            "themePack.resetWeights",
            "resource.status",
            "resource.checkUpdate",
            "tool.screenshot",
            "hotkey.get",
            "systemSettings.get",
            "device.list",
            "device.disconnect",
        }:
            if params is not None:
                raise RpcDispatchError(-32602, f"{method} does not accept params")
            return self._call_without_params(route)
        if route == "device.connect":
            return self._device_connect(params)
        if route == "device.list":
            return self.application.device_manager.list_devices()
        if route == "device.disconnect":
            return self.application.device_manager.disconnect()
        handler = getattr(self.application, route)
        return handler(params)

    def _call_execution(self, method: str, route: str, params: Any) -> Any:
        """Invoke an execution method with an explicit legacy-signature adapter.

        The schema-3 dispatcher forwards the request object so the application
        can validate ``runId``/``clientRequestId``.  Older application builds
        still expose no-argument pause/resume/stop methods; inspect their
        signature first and call those methods without parameters.  This keeps
        compatibility without catching a ``TypeError`` raised *inside* a
        method, which would otherwise hide real backend failures.
        """

        if params is not None and not isinstance(params, dict):
            raise RpcDispatchError(-32602, f"{method} requires an object params value")

        handler = getattr(self.application, route)
        if params is None or self._accepts_positional_params(handler):
            return handler(params) if params is not None else handler()
        return handler()

    @staticmethod
    def _accepts_positional_params(handler: Callable[..., Any]) -> bool:
        """Whether ``handler(params)`` is valid, without invoking ``handler``."""

        try:
            signature = inspect.signature(handler)
        except (TypeError, ValueError):
            # Some extension/builtin callables have no introspectable
            # signature.  Calling with params is the only way to preserve the
            # new contract; any resulting TypeError is allowed to propagate to
            # the normal dispatcher error boundary.
            return True
        try:
            signature.bind({})
        except TypeError:
            return False
        return True

    def _call_without_params(self, route: str) -> Any:
        if route == "device.list":
            return self.application.device_manager.list_devices()
        if route == "device.disconnect":
            return self.application.device_manager.disconnect()
        if route == "device.connect":
            raise RpcDispatchError(-32602, "device.connect requires an object params value")
        return getattr(self.application, route)()

    def _device_connect(self, params: Any) -> Any:
        values = self._object_params(params, "device.connect")
        device_id = values.get("id")
        if not isinstance(device_id, str) or not device_id:
            raise RpcDispatchError(-32602, "device.connect requires a non-empty string id")
        return self.application.device_manager.connect(device_id)

    @staticmethod
    def _object_params(params: Any, method: str) -> dict[str, Any]:
        if not isinstance(params, dict):
            raise RpcDispatchError(-32602, f"{method} requires an object params value")
        return params

    @staticmethod
    def _error(request_id: Any, code: int, message: str, data: Any = None) -> dict[str, Any]:
        error: dict[str, Any] = {"code": code, "message": message}
        details: dict[str, Any] = {
            "code": _error_code_name(code, message),
            "retryable": code in {-32000, -32030},
            "userMessage": message,
        }
        if isinstance(data, dict):
            details.update(data)
        elif data is not None:
            details["details"] = data
        error["data"] = details
        return {"jsonrpc": "2.0", "id": request_id, "error": error}


__all__ = ["SCHEMA_VERSION", "RpcDispatchError", "RpcDispatcher"]
