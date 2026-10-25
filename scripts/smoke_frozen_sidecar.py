"""Smoke-test a frozen onedir sidecar bundle.

The check is intentionally black-box: it launches the packaged executable,
verifies that the ready PID is the process GPUI would supervise, exercises a
real WebSocket request, and then asserts a graceful ``app.shutdown`` exit.
That is the property a one-file bundle could not guarantee.
"""

from __future__ import annotations

import argparse
import asyncio
import json
import os
import subprocess
import sys
import tempfile
import threading
import uuid
from pathlib import Path
from typing import Any

READY_TIMEOUT = 60.0
RECV_TIMEOUT = 30.0
SHUTDOWN_TIMEOUT = 60.0


def _read_ready(process: subprocess.Popen[str], timeout: float) -> dict[str, Any]:
    assert process.stdout is not None
    result: list[dict[str, Any] | None] = [None]

    def read() -> None:
        reader = process.stdout
        assert reader is not None
        while True:
            line = reader.readline()
            if not line:
                return
            try:
                value = json.loads(line)
            except json.JSONDecodeError:
                continue
            if value.get("ready") is True and isinstance(value.get("port"), int):
                result[0] = value
                return

    thread = threading.Thread(target=read, name="FrozenSidecarReady", daemon=True)
    thread.start()
    thread.join(timeout)
    if result[0] is None:
        raise RuntimeError("frozen sidecar did not report ready")
    return result[0]


async def _exercise(port: int, token: str) -> None:
    from websockets.asyncio.client import connect

    async with connect(f"ws://127.0.0.1:{port}") as client:
        await client.send(json.dumps({"type": "hello", "token": token}))
        hello = json.loads(await asyncio.wait_for(client.recv(), timeout=RECV_TIMEOUT))
        if hello.get("ok") is not True:
            raise RuntimeError(f"sidecar rejected hello: {hello!r}")

        methods = (
            (1, "app.ping"),
            (2, "app.version"),
            (3, "team.list"),
            (4, "tasks.getConfig"),
            (5, "app.diagnose"),
        )
        for request_id, method in methods:
            await client.send(
                json.dumps({"jsonrpc": "2.0", "id": request_id, "method": method})
            )
        responses = {}
        for _ in methods:
            response = json.loads(
                await asyncio.wait_for(client.recv(), timeout=RECV_TIMEOUT)
            )
            if "error" in response:
                raise RuntimeError(f"sidecar method failed: {response!r}")
            responses[response["id"]] = response["result"]
        if responses.get(1) != "pong":
            raise RuntimeError(f"unexpected ping response: {responses.get(1)!r}")
        if responses.get(2, {}).get("schemaVersion") != 3:
            raise RuntimeError(f"unexpected version response: {responses.get(2)!r}")
        if not isinstance(responses.get(3), list):
            raise RuntimeError(f"unexpected team.list response: {responses.get(3)!r}")
        if responses.get(4, {}).get("schemaVersion") != 3:
            raise RuntimeError(f"unexpected tasks.getConfig response: {responses.get(4)!r}")
        if responses.get(5, {}).get("bootId") != hello.get("bootId"):
            raise RuntimeError(f"unexpected diagnostics response: {responses.get(5)!r}")

        await client.send(json.dumps({"jsonrpc": "2.0", "id": 6, "method": "app.shutdown"}))
        shutdown = json.loads(await asyncio.wait_for(client.recv(), timeout=RECV_TIMEOUT))
        if shutdown.get("result") is not True:
            raise RuntimeError(f"unexpected shutdown response: {shutdown!r}")


def smoke(executable: Path, *, timeout: float = SHUTDOWN_TIMEOUT) -> dict[str, Any]:
    """Run the frozen sidecar and return its readiness metadata."""

    executable = Path(executable)
    if not executable.is_file():
        raise FileNotFoundError(f"frozen sidecar is missing: {executable}")

    token = f"smoke-{uuid.uuid4().hex}"
    with tempfile.TemporaryDirectory(prefix="aalc-sidecar-smoke-") as directory:
        root = Path(directory)
        environment = os.environ.copy()
        environment["AHAB_BACKEND_TOKEN"] = token
        environment["AALC_CONFIG_PATH"] = str(root / "config.yaml")
        environment["AALC_THEME_PACK_LIST_PATH"] = str(root / "theme_pack_list.yaml")
        environment["AALC_THEME_PACK_WEIGHT_PATH"] = str(root / "theme_pack_weight")
        process = subprocess.Popen(
            [str(executable), "--port", "0", "--watch-stdin"],
            cwd=executable.parent,
            env=environment,
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
        )
        try:
            ready = _read_ready(process, READY_TIMEOUT)
            if ready.get("pid") != process.pid:
                raise RuntimeError(
                    "frozen sidecar PID does not match the supervised process: "
                    f"{ready.get('pid')} != {process.pid}"
                )
            asyncio.run(_exercise(int(ready["port"]), token))
            process.wait(timeout=timeout)
            if process.returncode != 0:
                stderr = process.stderr.read() if process.stderr is not None else ""
                raise RuntimeError(
                    f"frozen sidecar exited with {process.returncode}\nstderr:\n{stderr}"
                )
            return {**ready, "supervisedPid": process.pid}
        finally:
            if process.poll() is None:
                process.kill()
                process.wait(timeout=5)


def main() -> int:
    parser = argparse.ArgumentParser(description="Smoke-test a frozen onedir sidecar")
    parser.add_argument(
        "executable",
        nargs="?",
        default=str(Path(__file__).resolve().parents[1] / "dist" / "AALCBackend" / "AALC Backend.exe"),
    )
    parser.add_argument("--timeout", type=float, default=SHUTDOWN_TIMEOUT)
    args = parser.parse_args()
    ready = smoke(Path(args.executable), timeout=args.timeout)
    sys.stdout.write(json.dumps(ready, ensure_ascii=False) + "\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
