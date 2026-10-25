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
import threading
import time
import uuid
from pathlib import Path


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Ahab Assistant GPUI sidecar")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=0)
    # The GPUI-owned launcher supplies the token through the child environment
    # so it does not appear in the process command line.  Manual/debug launches
    # may still pass --token explicitly.
    parser.add_argument("--token", default=None)
    parser.add_argument("--parent-pid", type=int, default=0)
    # Opt-in: only the GPUI-owned launch keeps a live stdin pipe.  External
    # launches and tests must not interpret an inherited closed stdin as a
    # parent-death signal.
    parser.add_argument("--watch-stdin", action="store_true")
    # Diagnostic entry points terminate without opening a listener or touching
    # the application state.  ``--diagnose`` additionally reports paths and
    # interpreter details.
    parser.add_argument("--health", action="store_true")
    parser.add_argument("--diagnose", action="store_true")
    return parser.parse_args()


async def _run(args: argparse.Namespace) -> None:
    started = time.monotonic()
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
    from module.rpc_contract import SCHEMA_VERSION

    sys.stdout.write(
        json.dumps(
            {
                "ready": True,
                "host": args.host,
                "port": port,
                "pid": os.getpid(),
                "bootId": server.boot_id,
                "schemaVersion": SCHEMA_VERSION,
                "version": _version(),
                "startupMs": round((time.monotonic() - started) * 1000),
            },
            separators=(",", ":"),
        )
        + "\n"
    )
    sys.stdout.flush()
    _redirect_stdout_after_ready()
    log.info("GPUI sidecar 已启动：%s:%s", args.host, port)

    if args.watch_stdin:
        _start_stdin_watch(stop_event)

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
    try:
        parent_create_time = psutil.Process(parent_pid).create_time()
    except psutil.Error:
        stop_event.set()
        return
    while not stop_event.is_set():
        try:
            process = psutil.Process(parent_pid)
            # PID existence alone is not identity: a reused PID must not keep
            # an orphaned sidecar alive.  Match the creation time captured at
            # startup, mirroring the runner/cleanup identity probes.
            if abs(process.create_time() - parent_create_time) > 0.001:
                stop_event.set()
                return
        except psutil.NoSuchProcess:
            stop_event.set()
            return
        except psutil.AccessDenied:
            # The parent is alive but temporarily uninspectable; retry.
            pass
        try:
            await asyncio.wait_for(stop_event.wait(), timeout=2.0)
        except asyncio.TimeoutError:
            continue


def _redirect_stdout_after_ready() -> None:
    """Keep the one-line readiness protocol from corrupting the pipe.

    The GPUI launcher stops draining stdout after the ready JSON.  Any later
    ``print`` from Python or a third-party library would otherwise hit a
    broken pipe.  Route text output to the diagnostic channel instead.
    """

    target = sys.stderr
    if target is None:
        try:
            target = open(os.devnull, "w", encoding="utf-8")  # noqa: SIM115
        except OSError:
            return
    try:
        sys.stdout = target
        sys.__stdout__ = target
    except Exception:
        pass


def _start_stdin_watch(stop_event: asyncio.Event) -> None:
    """Notify the sidecar when the GPUI-owned stdin pipe reaches EOF."""

    stream = getattr(sys.stdin, "buffer", sys.stdin)
    if stream is None:
        return
    try:
        loop = asyncio.get_running_loop()
        descriptor = stream.fileno()
    except (AttributeError, OSError, RuntimeError, ValueError):
        return

    def watch() -> None:
        # Read the raw descriptor instead of a BufferedReader: interpreter
        # shutdown waits for the buffer lock and aborts if a daemon thread is
        # still blocked inside ``readline``/``read`` at finalization time.
        try:
            while os.read(descriptor, 4096):
                pass
        except OSError:
            # A closed or broken stdin is indistinguishable from parent death
            # for an owned sidecar; let the normal shutdown path run.
            pass
        try:
            loop.call_soon_threadsafe(stop_event.set)
        except RuntimeError:
            pass

    try:
        threading.Thread(
            target=watch,
            name="AALCStdinWatch",
            daemon=True,
        ).start()
    except RuntimeError:
        pass


def _application_root() -> Path:
    if not getattr(sys, "frozen", False):
        return Path(__file__).resolve().parent
    executable_directory = Path(sys.executable).resolve().parent
    # The onedir bundle lives in ``<install>/backend`` while mutable state and
    # the external assets tree live in ``<install>``.  Walk up until the
    # release asset marker is found; a one-file build at the install root still
    # resolves on the first candidate.
    for candidate in (
        executable_directory,
        executable_directory.parent,
        executable_directory.parent.parent,
    ):
        if (candidate / "assets" / "config" / "version.txt").is_file():
            return candidate
    return executable_directory


def _configure_runtime_paths(root: Path) -> None:
    """Pin mutable sidecar state next to the installed application.

    PyInstaller's ``module.__file__`` points into ``_internal`` for an onedir
    bundle.  Without environment overrides, ``module.CONFIG_PATH`` and the
    theme-pack state would therefore be written inside the bundle (and, for
    the old one-file build, into a deleted extraction directory).  Explicit
    defaults also keep development and frozen runs on the same layout.
    """

    defaults = {
        "AALC_CONFIG_PATH": root / "config.yaml",
        "AALC_THEME_PACK_LIST_PATH": root / "theme_pack_list.yaml",
        "AALC_THEME_PACK_WEIGHT_PATH": root / "theme_pack_weight",
    }
    for name, path in defaults.items():
        os.environ.setdefault(name, str(path))


def _version() -> str:
    version_file = _application_root() / "assets" / "config" / "version.txt"
    try:
        return version_file.read_text(encoding="utf-8").strip()
    except OSError:
        return "unknown"


def _health_report(root: Path, *, diagnose: bool) -> dict[str, object]:
    from module.rpc_contract import CAPABILITIES, SCHEMA_VERSION

    report: dict[str, object] = {
        "ready": True,
        "version": _version(),
        "schemaVersion": SCHEMA_VERSION,
        "capabilities": list(CAPABILITIES),
        "bootId": uuid.uuid4().hex,
        "pid": os.getpid(),
        "frozen": bool(getattr(sys, "frozen", False)),
        "applicationRoot": str(root),
        "configPath": os.environ.get("AALC_CONFIG_PATH"),
        "themePackListPath": os.environ.get("AALC_THEME_PACK_LIST_PATH"),
        "themePackWeightPath": os.environ.get("AALC_THEME_PACK_WEIGHT_PATH"),
        "python": sys.version.split()[0],
        "executable": sys.executable,
    }
    if diagnose:
        report.update(
            {
                "cwd": os.getcwd(),
                "argv": list(sys.argv),
                "frozenExecutable": getattr(sys, "executable", None),
                "moduleSearchPaths": list(sys.path),
                "envKeys": sorted(
                    name
                    for name in os.environ
                    if name.startswith(("AALC_", "AHAB_", "PYTHON"))
                ),
            }
        )
    return report


def main() -> None:
    args = _parse_args()
    root = _application_root()
    _configure_runtime_paths(root)
    if args.health or args.diagnose:
        os.chdir(root)
        sys.stdout.write(
            json.dumps(
                _health_report(root, diagnose=args.diagnose),
                ensure_ascii=False,
                separators=(",", ":"),
            )
            + "\n"
        )
        sys.stdout.flush()
        return
    # Read and immediately remove the launcher-supplied token so Runner and
    # other child processes cannot inherit it.
    env_token = os.environ.pop("AHAB_BACKEND_TOKEN", None)
    token = args.token or env_token
    if not token:
        sys.stderr.write("sidecar token is required (--token or AHAB_BACKEND_TOKEN)\n")
        raise SystemExit(2)
    args.token = token
    os.chdir(root)
    try:
        asyncio.run(_run(args))
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
