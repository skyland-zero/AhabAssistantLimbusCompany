"""Standalone headless Python sidecar entry point for the GPUI application.

This process owns automation services and local RPC only.  It never creates a
window or imports the legacy UI layer.
"""

from __future__ import annotations

import argparse
import asyncio
import json
import os
import signal
import sys
from pathlib import Path


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Ahab Assistant GPUI sidecar")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=0)
    parser.add_argument("--token", required=True)
    parser.add_argument("--parent-pid", type=int, default=0)
    return parser.parse_args()


async def _run(args: argparse.Namespace) -> None:
    from module.logger import log
    from module.logger.my_log import Logger

    Logger(headless=True)
    from module.backend_application import BackendApplication, recover_pending_cleanups
    from module.device_manager import get_device_manager
    from module.rpc_dispatcher import RpcDispatcher
    from module.websocket_server import WebSocketServer

    stop_event = asyncio.Event()

    def request_stop(_signal: int | None = None, _frame: object | None = None) -> None:
        try:
            asyncio.get_running_loop().call_soon_threadsafe(stop_event.set)
        except RuntimeError:
            # The interpreter is already shutting down.
            pass

    for signum in (signal.SIGINT, signal.SIGTERM):
        try:
            signal.signal(signum, request_stop)
        except (AttributeError, OSError, ValueError):
            pass

    manager = get_device_manager()
    # Startup recovery is a barrier: no WebSocket listener (and therefore no
    # new execution.start request) exists until every durable journal has been
    # inspected.  The recovery executor never performs unscoped process kills;
    # local PID cleanup requires an explicitly injected identity-safe handler.
    recovery_results = recover_pending_cleanups()
    for result in recovery_results:
        if result.get("pending") or result.get("status") == "failed":
            log.warning("CleanupLedger 启动恢复仍待处理：%s", result)
        else:
            log.info("CleanupLedger 启动恢复完成：%s", result.get("runId", "unknown"))
    application = BackendApplication(
        manager,
        version=_version(),
        shutdown=stop_event.set,
    )
    dispatcher = RpcDispatcher(application=application, version=_version())
    server = WebSocketServer(dispatcher, manager, token=args.token, application=application)
    port = await server.start(args.host, args.port)
    sys.stdout.write(
        json.dumps(
            {"ready": True, "host": args.host, "port": port, "pid": os.getpid()},
            separators=(",", ":"),
        )
        + "\n"
    )
    sys.stdout.flush()
    log.info("GPUI sidecar 已启动：%s:%s", args.host, port)

    parent_task: asyncio.Task[None] | None = None
    if args.parent_pid > 0:
        parent_task = asyncio.create_task(_watch_parent(args.parent_pid, stop_event))
    try:
        await stop_event.wait()
    finally:
        if parent_task is not None:
            parent_task.cancel()
        await server.stop()
        application.close()


async def _watch_parent(parent_pid: int, stop_event: asyncio.Event) -> None:
    try:
        import psutil
    except ImportError:
        return
    while not stop_event.is_set():
        if not psutil.pid_exists(parent_pid):
            stop_event.set()
            return
        try:
            await asyncio.wait_for(stop_event.wait(), timeout=2.0)
        except asyncio.TimeoutError:
            continue


def _application_root() -> Path:
    if getattr(sys, "frozen", False):
        return Path(sys.executable).resolve().parent
    return Path(__file__).resolve().parent


def _version() -> str:
    version_file = _application_root() / "assets" / "config" / "version.txt"
    try:
        return version_file.read_text(encoding="utf-8").strip()
    except OSError:
        return "unknown"


def main() -> None:
    args = _parse_args()
    os.chdir(_application_root())
    try:
        asyncio.run(_run(args))
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
