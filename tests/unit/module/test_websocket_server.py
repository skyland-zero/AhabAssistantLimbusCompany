from __future__ import annotations

import asyncio
import json
import threading

import pytest

pytest.importorskip("websockets")
from websockets.asyncio.client import connect

from module.rpc_dispatcher import RpcDispatcher
from module.websocket_server import WebSocketServer
from tests.unit.module.test_backend_application import make_application


class _LaneDeviceManager:
    def set_busy_checker(self, checker) -> None:
        self.busy_checker = checker


class _LaneApplication:
    def __init__(self) -> None:
        self.device_manager = _LaneDeviceManager()
        self.mutation_started = threading.Event()
        self.release_mutation = threading.Event()
        self.listeners = []

    def add_event_listener(self, listener) -> None:
        if listener not in self.listeners:
            self.listeners.append(listener)

    def remove_event_listener(self, listener) -> None:
        if listener in self.listeners:
            self.listeners.remove(listener)

    def set_busy_checker(self, checker) -> None:
        self.device_manager.set_busy_checker(checker)

    def is_busy(self) -> bool:
        return False

    def tasks_set_config(self, params):
        self.mutation_started.set()
        self.release_mutation.wait(timeout=5)
        return True

    def execution_stop(self, params=None):
        return {"accepted": True, "runId": params.get("runId") if params else None, "state": "stopping"}


def test_websocket_authentication_requests_and_ordered_events() -> None:
    asyncio.run(_exercise_server())


async def _exercise_server() -> None:
    app = make_application()
    dispatcher = RpcDispatcher(application=app, version="test")
    server = WebSocketServer(
        dispatcher,
        app.device_manager,
        token="secret",
        application=app,
    )
    port = await server.start()
    try:
        async with connect(f"ws://127.0.0.1:{port}") as client:
            await client.send(json.dumps({"type": "hello", "token": "wrong"}))
            try:
                await client.recv()
            except Exception as error:
                assert "1008" in str(error) or "authentication" in str(error).lower()

        async with connect(f"ws://127.0.0.1:{port}") as client:
            await client.send(json.dumps({"type": "hello", "token": "secret"}))
            hello = json.loads(await client.recv())
            assert hello == {"type": "hello", "ok": True, "schemaVersion": 3}

            await client.send(
                json.dumps(
                    {
                        "jsonrpc": "2.0",
                        "id": 7,
                        "method": "app.ping",
                    }
                )
            )
            response = json.loads(await client.recv())
            assert response["id"] == 7
            assert response["result"] == "pong"

            app.emit("app.notice", {"level": "info", "message": "ordered"})
            event = json.loads(await asyncio.wait_for(client.recv(), timeout=1))
            assert event["event"] == "app.notice"
            assert event["seq"] == 1

            # RunnerEventAdapter keeps ``execution.log`` for legacy Python
            # listeners, while the GPUI wire contract consumes ``log.entry``.
            app.emit("execution.log", {"ts": 1, "level": "info", "message": "runner"})
            event = json.loads(await asyncio.wait_for(client.recv(), timeout=1))
            assert event == {
                "event": "log.entry",
                "payload": {"ts": 1, "level": "info", "message": "runner"},
                "seq": 2,
            }
    finally:
        await server.stop()
        app.close()


def test_execution_stop_uses_high_priority_lane_and_server_stop_closes_executors() -> None:
    asyncio.run(_exercise_execution_lane())


async def _exercise_execution_lane() -> None:
    app = _LaneApplication()
    dispatcher = RpcDispatcher(application=app)
    server = WebSocketServer(dispatcher, app.device_manager, token="secret", application=app)
    port = await server.start()
    try:
        async with connect(f"ws://127.0.0.1:{port}") as client:
            await client.send(json.dumps({"type": "hello", "token": "secret"}))
            assert json.loads(await client.recv())["schemaVersion"] == 3

            await client.send(
                json.dumps(
                    {
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": "tasks.setConfig",
                        "params": {},
                    }
                )
            )
            await asyncio.to_thread(app.mutation_started.wait, 2)
            assert app.mutation_started.is_set()

            await client.send(
                json.dumps(
                    {
                        "jsonrpc": "2.0",
                        "id": 2,
                        "method": "execution.stop",
                        "params": {"runId": "run-1"},
                    }
                )
            )
            stop_response = json.loads(await asyncio.wait_for(client.recv(), timeout=2))
            assert stop_response["id"] == 2
            assert stop_response["result"]["state"] == "stopping"

            app.release_mutation.set()
            mutation_response = json.loads(await asyncio.wait_for(client.recv(), timeout=2))
            assert mutation_response["id"] == 1
    finally:
        app.release_mutation.set()
        await server.stop()

    assert server._execution_executor is None
    assert server._mutation_executor is None
    assert server._read_executor is None
    assert app.listeners == []
