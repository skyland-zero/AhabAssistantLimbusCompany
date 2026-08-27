"""JSON-RPC method dispatch for the GPUI sidecar.

Transport code lives in :mod:`module.websocket_server`; this module only
translates validated RPC calls into existing Python services.  Keeping the
boundary here also makes it possible to exercise the protocol without a
running WebSocket server.
"""

from __future__ import annotations

from typing import Any, Callable

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
    ) -> None:
        self.device_manager = device_manager or get_device_manager()
        self.version = version
        self._shutdown = shutdown
        self._busy_checker: Callable[[], bool] = lambda: False
        self.device_manager.set_busy_checker(lambda: self._busy_checker())

    def set_busy_checker(self, checker: Callable[[], bool]) -> None:
        self._busy_checker = checker

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
        except DeviceError as error:
            return self._error(request_id, -32020, str(error))
        except Exception as error:
            log.exception("RPC 方法执行失败：%s", method)
            return self._error(request_id, -32000, f"后端内部错误：{error}")

    def _call(self, method: str, params: Any) -> Any:
        if method == "app.ping":
            return "pong"
        if method == "app.version":
            return {"ui": "gpui", "backend": self.version}
        if method == "app.shutdown":
            if self._shutdown is not None:
                self._shutdown()
            return True

        if method == "device.list":
            return self.device_manager.list_devices()
        if method == "device.connect":
            values = self._object_params(params, "device.connect")
            device_id = values.get("id")
            if not isinstance(device_id, str) or not device_id:
                raise RpcDispatchError(-32602, "device.connect requires a non-empty string id")
            return self.device_manager.connect(device_id)
        if method == "device.disconnect":
            return self.device_manager.disconnect()

        if method == "execution.getState":
            # Execution control is added in the next adapter layer.  Returning
            # an explicit idle state keeps the sidecar handshake useful while
            # the GPUI device path is being migrated.
            return {"state": "idle", "currentTaskId": None}

        raise RpcDispatchError(-32601, f"Method not found: {method}")

    @staticmethod
    def _object_params(params: Any, method: str) -> dict[str, Any]:
        if not isinstance(params, dict):
            raise RpcDispatchError(-32602, f"{method} requires an object params value")
        return params

    @staticmethod
    def _error(request_id: Any, code: int, message: str, data: Any = None) -> dict[str, Any]:
        error: dict[str, Any] = {"code": code, "message": message}
        if data is not None:
            error["data"] = data
        return {"jsonrpc": "2.0", "id": request_id, "error": error}


__all__ = ["RpcDispatchError", "RpcDispatcher"]
