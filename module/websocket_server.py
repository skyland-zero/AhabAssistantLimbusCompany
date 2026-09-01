"""Bounded local WebSocket transport for the GPUI sidecar."""

from __future__ import annotations

import asyncio
import json
import logging
import struct
from collections import deque
from concurrent.futures import ThreadPoolExecutor
from typing import Any

from websockets.asyncio.server import ServerConnection, serve
from websockets.exceptions import ConnectionClosed

from module.backend_application import SCHEMA_VERSION, BackendApplication
from module.device_manager import DeviceManager
from module.logger import log
from module.rpc_dispatcher import RpcDispatcher

MAX_INFLIGHT_REQUESTS = 64
MAX_OUTBOUND_EVENTS = 512
READ_WORKERS = 4
_COALESCIBLE_EVENTS = {
    "screenshot.frame",
    "preview.status",
    "execution.status",
    "execution.mirrorProgress",
    "execution.stats",
    "tool.status",
    "device.status",
    "resource.sync.progress",
}
_DROPPABLE_EVENTS = _COALESCIBLE_EVENTS | {"log.entry"}


class _BroadcastLogHandler(logging.Handler):
    """Forward AALC log records without retaining the legacy UI buffer."""

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
                {"ts": int(record.created * 1000), "level": level, "message": record.getMessage()},
            )
        except Exception:
            pass


class WebSocketServer:
    """Serve JSON-RPC with bounded concurrency and ordered event delivery."""

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
        self._inflight_requests = 0
        self._outbound: deque[tuple[str, dict[str, Any], int | None]] = deque()
        self._outbound_ready: asyncio.Event | None = None
        self._sender_task: asyncio.Task[None] | None = None
        self._mutation_executor: ThreadPoolExecutor | None = None
        self._read_executor: ThreadPoolExecutor | None = None
        self._log_handler = _BroadcastLogHandler(self)
        self.application.add_event_listener(self._on_application_event)

    async def start(self, host: str = "127.0.0.1", port: int = 0) -> int:
        if self._server is not None:
            return self.port or 0
        self._loop = asyncio.get_running_loop()
        self._mutation_executor = ThreadPoolExecutor(max_workers=1, thread_name_prefix="AALCRpcMutation")
        self._read_executor = ThreadPoolExecutor(max_workers=READ_WORKERS, thread_name_prefix="AALCRpcRead")
        self._outbound_ready = asyncio.Event()
        self._sender_task = asyncio.create_task(self._sender_loop(), name="AALCEventSender")
        self.application.add_event_listener(self._on_application_event)
        log.addHandler(self._log_handler)
        try:
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
        except BaseException:
            # Executor threads, the sender task, and the event listener all
            # outlive ``serve`` setup unless they are explicitly unwound.
            # Keep a failed start retryable and avoid leaking a log handler.
            await self.stop()
            raise

    async def stop(self) -> None:
        server = self._server
        self._server = None
        if server is not None:
            server.close()
            await server.wait_closed()

        clients = tuple(self._clients)
        self._clients.clear()
        for client in clients:
            try:
                await client.close()
            except Exception:
                pass
        self._send_locks.clear()

        for task in tuple(self._request_tasks):
            task.cancel()
        if self._request_tasks:
            await asyncio.gather(*self._request_tasks, return_exceptions=True)
        self._request_tasks.clear()
        self._inflight_requests = 0

        if self._sender_task is not None:
            self._sender_task.cancel()
            await asyncio.gather(self._sender_task, return_exceptions=True)
            self._sender_task = None
        self._outbound.clear()

        mutation_executor, self._mutation_executor = self._mutation_executor, None
        read_executor, self._read_executor = self._read_executor, None
        if mutation_executor is not None:
            await asyncio.to_thread(mutation_executor.shutdown, wait=True, cancel_futures=True)
        if read_executor is not None:
            await asyncio.to_thread(read_executor.shutdown, wait=True, cancel_futures=True)
        log.removeHandler(self._log_handler)
        self.application.remove_event_listener(self._on_application_event)
        self._loop = None

    def publish(self, event: str, payload: dict[str, Any], sequence: int | None = None) -> None:
        """Queue an event from any worker thread without spawning a task."""

        loop = self._loop
        if loop is None or self._server is None:
            return
        loop.call_soon_threadsafe(self._enqueue_event, event, dict(payload), sequence)

    def _enqueue_event(self, event: str, payload: dict[str, Any], sequence: int | None) -> None:
        if event in _COALESCIBLE_EVENTS:
            key = payload.get("deviceId") or payload.get("toolId") or payload.get("runId")
            for index in range(len(self._outbound) - 1, -1, -1):
                queued_event, queued_payload, _ = self._outbound[index]
                queued_key = (
                    queued_payload.get("deviceId") or queued_payload.get("toolId") or queued_payload.get("runId")
                )
                if queued_event == event and queued_key == key:
                    del self._outbound[index]
                    break

        if len(self._outbound) >= MAX_OUTBOUND_EVENTS:
            for index, (queued_event, _, _) in enumerate(self._outbound):
                if queued_event in _DROPPABLE_EVENTS:
                    del self._outbound[index]
                    break
            else:
                for client in tuple(self._clients):
                    asyncio.create_task(client.close(code=1013, reason="event backpressure"))
                self._outbound.clear()

        self._outbound.append((event, payload, sequence))
        if self._outbound_ready is not None:
            self._outbound_ready.set()

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
                    await self._send(connection, self._error(None, -32600, "RPC 请求必须使用文本帧"))
                    continue
                try:
                    request = json.loads(raw)
                except TypeError, json.JSONDecodeError:
                    await self._send(connection, self._error(None, -32700, "无法解析 JSON"))
                    continue
                if not isinstance(request, dict):
                    await self._send(connection, self._error(None, -32600, "无效的 JSON-RPC 请求"))
                    continue
                if self._inflight_requests >= MAX_INFLIGHT_REQUESTS:
                    await self._send(connection, self._error(request.get("id"), -32030, "后端请求队列已满"))
                    continue
                self._inflight_requests += 1
                task = asyncio.create_task(self._handle_request(connection, request))
                self._request_tasks.add(task)
                task.add_done_callback(self._request_done)
        except ConnectionClosed:
            pass
        finally:
            self._clients.discard(connection)
            self._send_locks.pop(connection, None)

    def _request_done(self, task: asyncio.Task[Any]) -> None:
        self._inflight_requests = max(0, self._inflight_requests - 1)
        self._request_tasks.discard(task)

    async def _authenticate(self, connection: ServerConnection) -> bool:
        try:
            raw = await asyncio.wait_for(connection.recv(), timeout=5)
            if isinstance(raw, bytes):
                raise TypeError("binary hello")
            hello = json.loads(raw)
        except asyncio.TimeoutError, ConnectionClosed, TypeError, json.JSONDecodeError:
            await connection.close(code=1008, reason="authentication required")
            return False

        if not isinstance(hello, dict) or hello.get("type") != "hello" or hello.get("token") != self.token:
            await connection.close(code=1008, reason="invalid sidecar token")
            return False
        await connection.send(
            json.dumps({"type": "hello", "ok": True, "schemaVersion": SCHEMA_VERSION}, separators=(",", ":"))
        )
        return True

    async def _handle_request(self, connection: ServerConnection, request: dict[str, Any]) -> None:
        executor = (
            self._mutation_executor if self.dispatcher.is_mutating(request.get("method")) else self._read_executor
        )
        if executor is None:
            await self._send(connection, self._error(request.get("id"), -32000, "后端服务尚未启动"))
            return
        loop = asyncio.get_running_loop()
        try:
            response = await loop.run_in_executor(executor, self.dispatcher.dispatch, request)
        except asyncio.CancelledError:
            raise
        except Exception as error:
            log.exception("RPC transport worker failed")
            response = self._error(request.get("id"), -32000, f"后端内部错误：{error}")
        try:
            await self._send(connection, response)
        except ConnectionClosed:
            pass

    async def _sender_loop(self) -> None:
        while True:
            ready = self._outbound_ready
            if ready is None:
                return
            await ready.wait()
            while self._outbound:
                event, payload, sequence = self._outbound.popleft()
                try:
                    await self._broadcast_event(event, payload, sequence)
                except asyncio.CancelledError:
                    raise
                except Exception:
                    log.exception("sidecar 事件广播失败：%s", event)
            ready.clear()

    async def _broadcast_event(self, event: str, payload: dict[str, Any], sequence: int | None) -> None:
        message: str | bytes
        if event == "screenshot.frame" and isinstance(payload.get("jpeg"), bytes):
            jpeg = payload.pop("jpeg")
            header: dict[str, Any] = {"event": event, "payload": payload}
            if sequence is not None:
                header["seq"] = sequence
            metadata = json.dumps(header, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
            message = struct.pack(">I", len(metadata)) + metadata + jpeg
        else:
            envelope: dict[str, Any] = {"event": event, "payload": payload}
            if sequence is not None:
                envelope["seq"] = sequence
            message = json.dumps(envelope, ensure_ascii=False, separators=(",", ":"))

        clients = tuple(self._clients)
        if not clients:
            return
        results = await asyncio.gather(
            *(self._send_encoded(client, message) for client in clients), return_exceptions=True
        )
        for client, result in zip(clients, results):
            if isinstance(result, Exception):
                self._clients.discard(client)
                self._send_locks.pop(client, None)

    async def _send(self, connection: ServerConnection, message: dict[str, Any]) -> None:
        await self._send_encoded(connection, json.dumps(message, ensure_ascii=False, separators=(",", ":")))

    async def _send_encoded(self, connection: ServerConnection, message: str | bytes) -> None:
        lock = self._send_locks.setdefault(connection, asyncio.Lock())
        async with lock:
            await connection.send(message)

    @staticmethod
    def _error(request_id: Any, code: int, message: str) -> dict[str, Any]:
        return {
            "jsonrpc": "2.0",
            "id": request_id,
            "error": {
                "code": code,
                "message": message,
                "data": {"retryable": code in {-32000, -32001, -32030}, "userMessage": message},
            },
        }


__all__ = ["MAX_INFLIGHT_REQUESTS", "MAX_OUTBOUND_EVENTS", "READ_WORKERS", "WebSocketServer"]
