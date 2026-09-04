from __future__ import annotations

import os
import subprocess
import sys
import time
from pathlib import Path

import psutil
import pytest

from module.execution.platform import ProcessIdentity, WindowsJobAdapter, create_process_adapter


class Backend:
    def __init__(self, calls: list[str], *, fail_assign: bool = False) -> None:
        self.calls = calls
        self.fail_assign = fail_assign

    def create(self):
        self.calls.append("create")
        return "job"

    def assign(self, job, process):
        self.calls.append("assign")
        if self.fail_assign:
            raise OSError("nested job")

    def resume(self, process):
        self.calls.append("resume")

    def terminate(self, job, exit_code=1):
        self.calls.append("terminate")

    def close(self, job):
        self.calls.append("close")


def test_windows_adapter_is_fail_closed_before_resume(monkeypatch) -> None:
    calls: list[str] = []
    adapter = WindowsJobAdapter(backend=Backend(calls))
    monkeypatch.setattr("module.execution.platform.subprocess.Popen", lambda *args, **kwargs: FakePopen())
    process = adapter.launch(["runner.exe"])
    assert process.pid == 123
    assert calls[:3] == ["create", "assign", "resume"]


def test_windows_nested_job_failure_closes_job_without_resume(monkeypatch) -> None:
    calls: list[str] = []
    adapter = WindowsJobAdapter(backend=Backend(calls, fail_assign=True))
    monkeypatch.setattr("module.execution.platform.subprocess.Popen", lambda *args, **kwargs: FakePopen())
    try:
        adapter.launch(["runner.exe"])
    except Exception as exc:
        assert exc.code == "RUNNER_SUPERVISION_FAILED"
    assert calls == ["create", "assign", "close"]


class FakePopen:
    pid = 123

    def __init__(self) -> None:
        self._returncode = None

    def poll(self):
        return self._returncode

    def wait(self, timeout=None):
        self._returncode = 0
        return 0

    def terminate(self):
        self._returncode = -15

    def kill(self):
        self._returncode = -9


def test_platform_factory_selects_named_adapter() -> None:
    assert type(create_process_adapter("linux")).__name__ == "PosixProcessGroup"
    assert type(create_process_adapter("darwin")).__name__ == "MacOSProcessGroup"


def test_process_identity_checks_parent_pid_and_creation_time() -> None:
    expected = ProcessIdentity(10, 20.0, "run", parent_pid=3, parent_create_time=4.0)
    assert expected.matches(ProcessIdentity(10, 20.0005, "run", parent_pid=3, parent_create_time=4.0005))
    assert not expected.matches(ProcessIdentity(10, 20.0, "run", parent_pid=4, parent_create_time=4.0))
    assert not expected.matches(ProcessIdentity(10, 20.0, "run", parent_pid=3, parent_create_time=5.0))


@pytest.mark.skipif(os.name != "nt", reason="requires native Windows Job Objects")
def test_native_job_close_kills_runner_and_descendant(tmp_path: Path) -> None:
    marker = tmp_path / "descendant.pid"
    child_code = "import time; time.sleep(60)"
    runner_code = (
        "import pathlib, subprocess, sys, time; "
        "child = subprocess.Popen([sys.executable, '-c', sys.argv[2]]); "
        "pathlib.Path(sys.argv[1]).write_text(str(child.pid), encoding='ascii'); "
        "time.sleep(60)"
    )
    adapter = WindowsJobAdapter()
    managed = adapter.launch(
        [sys.executable, "-c", runner_code, str(marker), child_code],
        stdin=subprocess.DEVNULL,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
        env=os.environ.copy(),
    )
    try:
        deadline = time.monotonic() + 3.0
        while not marker.exists() and time.monotonic() < deadline:
            time.sleep(0.02)
        assert marker.exists()
        descendant_pid = int(marker.read_text(encoding="ascii"))
        assert psutil.pid_exists(managed.pid)
        assert psutil.pid_exists(descendant_pid)
        assert getattr(adapter.backend, "native", False) is True

        # Closing the last Job handle is the sidecar-crash containment path.
        managed.close()
        deadline = time.monotonic() + 3.0
        while (psutil.pid_exists(managed.pid) or psutil.pid_exists(descendant_pid)) and time.monotonic() < deadline:
            time.sleep(0.02)
        assert not psutil.pid_exists(managed.pid)
        assert not psutil.pid_exists(descendant_pid)
    finally:
        try:
            if psutil.pid_exists(managed.pid):
                adapter.kill(managed)
        except Exception:
            pass
        managed.close()
