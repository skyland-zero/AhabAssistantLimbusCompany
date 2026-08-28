"""Local WebSocket transport for the GPUI sidecar."""

from __future__ import annotations

import asyncio
import json
import logging
from typing import Any

from websockets.asyncio.server import ServerConnection, serve
from websockets.exceptions import ConnectionClosed

from module.backend_application import SCHEMA_VERSION, BackendApplication
from module.device_manager import DeviceManager
from module.logger import log
from module.rpc_dispatcher import RpcDispatcher


class _BroadcastLogHandler(logging.Handler):
    """Forward AALC log records to all connected GPUI clients."""

    def __init__(self, owner: "WebSocketServer") -> None:
        super().__init__(level=logging.INFO)
        self.owner = owner

    def emit(self, record: logging.LogRecord) -> None:
        try:
            level = {
                logging.DEBUG: "debug",
                logging.INFO: "info",
                logging.WARNING: "warn",
                logging.ERROR: "error",
                logging.CRITICAL: "error",
            }.get(record.levelno, "info")
            self.owner.application.emit(
                "log.entry",
                {
                    "ts": int(record.created * 1000),
                    "level": level,
                    "message": record.getMessage(),
                },
            )
        except Exception:
            # Logging must never break the business worker that emitted it.
            pass


class WebSocketServer:
    """Serve JSON-RPC requests on loopback and broadcast backend events."""

    def __init__(
        self,
        dispatcher: RpcDispatcher,
        device_manager: DeviceManager,
        *,
        token: str,
        application: BackendApplication | None = None,
    ) -> None:
        self.dispatcher = dispatcher
        self.device_manager = device_manager
        self.application = application or dispatcher.application
        self.token = token
        self.port: int | None = None
        self._loop: asyncio.AbstractEventLoop | None = None
        self._server: Any = None
        self._clients: set[ServerConnection] = set()
        self._send_locks: dict[ServerConnection, asyncio.Lock] = {}
        self._request_tasks: set[asyncio.Task[Any]] = set()
        self._log_handler = _BroadcastLogHandler(self)

        self.application.add_event_listener(self._on_application_event)

    async def start(self, host: str = "127.0.0.1", port: int = 0) -> int:
        if self._server is not None:
            return self.port or 0
        self._loop = asyncio.get_running_loop()
        log.addHandler(self._log_handler)
        self._server = await serve(
            self._handler,
            host,
            port,
            max_size=8 * 1024 * 1024,
            ping_interval=20,
            ping_timeout=20,
        )
        sockets = getattr(self._server, "sockets", None) or []
        if not sockets:
            raise RuntimeError("WebSocket server did not create a listening socket")
        self.port = int(sockets[0].getsockname()[1])
        return self.port

    async def stop(self) -> None:
        server = self._server
        self._server = None
        if server is not None:
            server.close()
            await server.wait_closed()

        clients = tuple(self._clients)
        self._clients.clear()
        self._send_locks.clear()
        for client in clients:
            try:
                await client.close()
            except Exception:
                pass

        for task in tuple(self._request_tasks):
            if not task.done():
                task.cancel()
        self._request_tasks.clear()
        log.removeHandler(self._log_handler)
        self.application.remove_event_listener(self._on_application_event)
        self._loop = None

    def publish(self, event: str, payload: dict[str, Any], sequence: int | None = None) -> None:
        """Schedule an event broadcast; safe to call from worker threads."""
        loop = self._loop
        if loop is None or self._server is None:
            return

        def schedule() -> None:
            task = asyncio.create_task(self._broadcast(event, payload, sequence))
            self._request_tasks.add(task)
            task.add_done_callback(self._request_tasks.discard)

        loop.call_soon_threadsafe(schedule)

    def _on_application_event(self, event: str, payload: dict[str, Any], sequence: int) -> None:
        self.publish(event, payload, sequence)

    async def _handler(self, connection: ServerConnection) -> None:
        if not await self._authenticate(connection):
            return
        self._clients.add(connection)
        self._send_locks[connection] = asyncio.Lock()
        try:
            async for raw in connection:
                if isinstance(raw, bytes):
                    raw = raw.decode("utf-8")
                try:
                    request = json.loads(raw)
                except (TypeError, json.JSONDecodeError):
                    await self._send(
                        connection,
                        self._error(None, -32700, "无法解析 JSON"),
                    )
                    continue
                if not isinstance(request, dict):
                    await self._send(
                        connection,
                        self._error(None, -32600, "无效的 JSON-RPC 请求"),
                    )
                    continue
                task = asyncio.create_task(self._handle_request(connection, request))
                self._request_tasks.add(task)
                task.add_done_callback(self._request_tasks.discard)
        except ConnectionClosed:
            pass
        finally:
            self._clients.discard(connection)
            self._send_locks.pop(connection, None)

    async def _authenticate(self, connection: ServerConnection) -> bool:
        try:
            raw = await asyncio.wait_for(connection.recv(), timeout=5)
            if isinstance(raw, bytes):
                raw = raw.decode("utf-8")
            hello = json.loads(raw)
        except (asyncio.TimeoutError, ConnectionClosed, TypeError, json.JSONDecodeError):
            try:
                await connection.close(code=1008, reason="authentication required")
            except Exception:
                pass
            return False

        if not isinstance(hello, dict) or hello.get("type") != "hello" or hello.get("token") != self.token:
            try:
                await connection.close(code=1008, reason="invalid sidecar token")
            except Exception:
                pass
            return False
        await connection.send(
            json.dumps(
                {"type": "hello", "ok": True, "schemaVersion": SCHEMA_VERSION},
                separators=(",", ":"),
            )
        )
        return True

    async def _handle_request(
        self,
        connection: ServerConnection,
        request: dict[str, Any],
    ) -> None:
        response = await asyncio.to_thread(self.dispatcher.dispatch, request)
        try:
            await self._send(connection, response)
        except ConnectionClosed:
            pass

    async def _broadcast(self, event: str, payload: dict[str, Any], sequence: int | None = None) -> None:
        message = {"event": event, "payload": payload}
        if sequence is not None:
            message["seq"] = sequence
        clients = tuple(self._clients)
        if not clients:
            return
        results = await asyncio.gather(
            *(self._send(client, message) for client in clients),
            return_exceptions=True,
        )
        for client, result in zip(clients, results):
            if isinstance(result, Exception):
                self._clients.discard(client)
                self._send_locks.pop(client, None)

    async def _send(self, connection: ServerConnection, message: dict[str, Any]) -> None:
        lock = self._send_locks.setdefault(connection, asyncio.Lock())
        encoded = json.dumps(message, ensure_ascii=False, separators=(",", ":"))
        async with lock:
            await connection.send(encoded)

    @staticmethod
    def _error(request_id: Any, code: int, message: str) -> dict[str, Any]:
        return {
            "jsonrpc": "2.0",
            "id": request_id,
            "error": {"code": code, "message": message},
        }


__all__ = ["WebSocketServer"]
