from __future__ import annotations

import asyncio
import json

import pytest

pytest.importorskip("websockets")
from websockets.asyncio.client import connect

from module.rpc_dispatcher import RpcDispatcher
from module.websocket_server import WebSocketServer
from tests.unit.module.test_backend_application import make_application


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
            assert hello == {"type": "hello", "ok": True, "schemaVersion": 1}

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
    finally:
        await server.stop()
        app.close()
