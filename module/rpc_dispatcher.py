"""JSON-RPC method dispatch for the GPUI sidecar.

Transport code lives in :mod:`module.websocket_server`; this module only
translates validated RPC calls into existing Python services.  Keeping the
boundary here also makes it possible to exercise the protocol without a
running WebSocket server.
"""

from __future__ import annotations

from typing import Any, Callable

from module.backend_application import BackendApplication
from module.device_manager import DeviceError, DeviceManager, get_device_manager
from module.logger import log


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
            "themePack.updateAll",
            "themePack.resetWeights",
            "resource.sync.start",
            "tool.start",
            "tool.stop",
            "tool.screenshot",
            "hotkey.set",
            "systemSettings.set",
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
            "hotkey.get": "hotkey_get",
            "hotkey.set": "hotkey_set",
            "systemSettings.get": "system_settings_get",
            "systemSettings.set": "system_settings_set",
            "device.list": "device.list",
            "device.connect": "device.connect",
            "device.disconnect": "device.disconnect",
        }
        route = route_names.get(method)
        if route is None:
            raise RpcDispatchError(-32601, f"Method not found: {method}")
        if method in {
            "app.ping",
            "app.version",
            "app.checkUpdate",
            "app.shutdown",
            "stats.getSummary",
            "tasks.getConfig",
            "execution.getState",
            "execution.stop",
            "execution.pause",
            "execution.resume",
            "team.list",
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
            "retryable": code in {-32000, -32030},
            "userMessage": message,
        }
        if isinstance(data, dict):
            details.update(data)
        elif data is not None:
            details["details"] = data
        error["data"] = details
        return {"jsonrpc": "2.0", "id": request_id, "error": error}


__all__ = ["RpcDispatchError", "RpcDispatcher"]
