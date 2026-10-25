from __future__ import annotations

import asyncio
import json
import os
import subprocess
import sys
import uuid
from pathlib import Path

import pytest

pytest.importorskip("websockets")
from websockets.asyncio.client import connect

# ``device.list`` enumerates ADB devices and can legitimately take several
# seconds on a machine with a running emulator, so every receive gets a
# generous bound instead of a per-message 3s scheduler race.
_RECV_TIMEOUT = 20.0
_READY_TIMEOUT = 30.0
_SHUTDOWN_TIMEOUT = 30.0


def _sidecar_environment(tmp_path: Path) -> dict[str, str]:
    environment = os.environ.copy()
    # The sidecar derives config, stats and cleanup journals from
    # ``AALC_CONFIG_PATH``; pin them into the test's temporary directory so
    # this test can never rewrite the developer's real config.yaml, backup
    # directory, runtime_stats.json or .aalc-runs journals.
    environment["AALC_CONFIG_PATH"] = str(tmp_path / "config.yaml")
    environment["AALC_THEME_PACK_LIST_PATH"] = str(tmp_path / "theme_pack_list.yaml")
    environment["AALC_THEME_PACK_WEIGHT_PATH"] = str(tmp_path / "theme_pack_weight")
    return environment


def test_health_reports_paths_without_starting_a_server(tmp_path: Path) -> None:
    root = Path(__file__).resolve().parents[2]
    completed = subprocess.run(
        [sys.executable, "main_backend.py", "--health"],
        cwd=root,
        env=_sidecar_environment(tmp_path),
        capture_output=True,
        text=True,
        timeout=60,
        check=False,
    )
    assert completed.returncode == 0, completed.stderr
    report = json.loads(completed.stdout)
    assert report["ready"] is True
    assert report["schemaVersion"] == 3
    assert report["pid"] > 0
    assert report["configPath"] == str(tmp_path / "config.yaml")


def test_owned_sidecar_exits_when_stdin_closes(tmp_path: Path) -> None:
    asyncio.run(_exercise_stdin_eof(tmp_path))


async def _exercise_stdin_eof(tmp_path: Path) -> None:
    token = f"stdin-{uuid.uuid4().hex}"
    root = Path(__file__).resolve().parents[2]
    process = subprocess.Popen(
        [
            sys.executable,
            "main_backend.py",
            "--token",
            token,
            "--port",
            "0",
            "--watch-stdin",
        ],
        cwd=root,
        env=_sidecar_environment(tmp_path),
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        bufsize=1,
    )
    try:
        ready = await _read_ready(process)
        assert ready["port"] > 0
        assert process.stdin is not None
        process.stdin.close()
        await asyncio.to_thread(process.wait, 10)
        assert process.returncode == 0
    finally:
        if process.poll() is None:
            process.kill()
            await asyncio.to_thread(process.wait, 5)


def test_real_sidecar_serves_the_gpui_contract(tmp_path: Path) -> None:
    asyncio.run(_exercise_real_sidecar(tmp_path))


async def _exercise_real_sidecar(tmp_path: Path) -> None:
    token = f"integration-{uuid.uuid4().hex}"
    root = Path(__file__).resolve().parents[2]
    environment = _sidecar_environment(tmp_path)
    process = subprocess.Popen(
        [sys.executable, "main_backend.py", "--token", token, "--port", "0"],
        cwd=root,
        env=environment,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        bufsize=1,
    )
    try:
        ready = await _read_ready(process)
        async with connect(f"ws://127.0.0.1:{ready['port']}") as client:
            await client.send(json.dumps({"type": "hello", "token": token}))
            hello = json.loads(await asyncio.wait_for(client.recv(), timeout=_RECV_TIMEOUT))
            assert hello["ok"] is True
            assert hello["schemaVersion"] == 3
            assert hello["bootId"]
            assert "core.rpc.v3" in hello["capabilities"]
            # Source-mode venv launchers may exec the real interpreter as a
            # child; the frozen onedir smoke test asserts PID equality instead.
            assert hello["pid"] > 0

            methods = [
                "app.ping",
                "app.version",
                "stats.getSummary",
                "stats.getDailySummary",
                "tasks.getConfig",
                "execution.getState",
                "team.list",
                "sinner.list",
                "themePack.list",
                "resource.status",
                "hotkey.get",
                "systemSettings.get",
                "device.list",
                "app.diagnose",
            ]
            for request_id, method in enumerate(methods, start=1):
                await client.send(json.dumps({"jsonrpc": "2.0", "id": request_id, "method": method}))
            responses = [
                json.loads(await asyncio.wait_for(client.recv(), timeout=_RECV_TIMEOUT))
                for _ in methods
            ]
            assert all("error" not in response for response in responses), responses
            responses_by_id = {response["id"]: response["result"] for response in responses}
            assert responses_by_id[1] == "pong"
            assert responses_by_id[3]["schemaVersion"] == 1
            assert responses_by_id[4]["schemaVersion"] == 1
            assert responses_by_id[5]["schemaVersion"] == 3
            assert responses_by_id[9]["schemaVersion"] == 3
            diagnose = responses_by_id[14]
            assert diagnose["bootId"] == hello["bootId"]
            assert diagnose["pid"] == hello["pid"]
            assert diagnose["schemaVersion"] == 3
            assert diagnose["droppedEvents"] >= 0

            teams = responses_by_id[7]
            if teams:
                team_id = teams[0]["id"]
                await client.send(
                    json.dumps(
                        {
                            "jsonrpc": "2.0",
                            "id": 80,
                            "method": "team.stats.get",
                            "params": {"id": team_id},
                        }
                    )
                )
                team_stats = json.loads(await asyncio.wait_for(client.recv(), timeout=_RECV_TIMEOUT))
                assert "error" not in team_stats, team_stats
                assert team_stats["result"]["teamId"] == team_id

            await client.send(
                json.dumps(
                    {
                        "jsonrpc": "2.0",
                        "id": 90,
                        "method": "not.implemented",
                    }
                )
            )
            unknown = json.loads(await asyncio.wait_for(client.recv(), timeout=_RECV_TIMEOUT))
            assert unknown["error"]["code"] == -32601

            await client.send(
                json.dumps(
                    {
                        "jsonrpc": "2.0",
                        "id": 91,
                        "method": "device.connect",
                        "params": {},
                    }
                )
            )
            invalid = json.loads(await asyncio.wait_for(client.recv(), timeout=_RECV_TIMEOUT))
            assert invalid["error"]["code"] == -32602

            await client.send(json.dumps({"jsonrpc": "2.0", "id": 99, "method": "app.shutdown"}))
            shutdown = json.loads(await asyncio.wait_for(client.recv(), timeout=_RECV_TIMEOUT))
            assert shutdown["result"] is True
        await asyncio.to_thread(process.wait, _SHUTDOWN_TIMEOUT)
        assert process.returncode == 0
    finally:
        if process.poll() is None:
            process.terminate()
            await asyncio.to_thread(process.wait, 5)


async def _read_ready(process: subprocess.Popen[str]) -> dict[str, int]:
    assert process.stdout is not None
    for _ in range(40):
        line = await asyncio.wait_for(
            asyncio.to_thread(process.stdout.readline), timeout=_READY_TIMEOUT
        )
        if not line:
            break
        try:
            value = json.loads(line)
        except json.JSONDecodeError:
            continue
        if value.get("ready") is True and isinstance(value.get("port"), int):
            return value
    output = ""
    if process.stdout is not None:
        output = process.stdout.read()
    raise AssertionError(f"sidecar did not become ready: {output}")
