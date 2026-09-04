"""Deterministic in-memory Runner used by contract tests and local smoke runs."""

from __future__ import annotations

import queue
import threading
import time
from collections.abc import Mapping
from typing import Any, Callable

from .ipc_protocol import Frame, Sequence, make_event
from .supervisor import ExecutionSpec


class FakeRunner:
    """A controllable RunnerProcess implementation.

    ``behavior`` may be ``"complete"``, ``"hang"``, ``"crash"`` or
    ``"malformed"``.  A callable receives the fake instance and can enqueue custom
    frames with :meth:`emit`.  It is intentionally pipe-like at the API boundary,
    so the same supervisor tests exercise real handshake and sequence logic.
    """

    def __init__(
        self,
        spec: ExecutionSpec,
        *,
        behavior: str | Callable[["FakeRunner"], Any] = "complete",
        pid: int = 9001,
        heartbeat_interval: float = 0.2,
    ) -> None:
        self.spec = spec
        self.pid = int(pid)
        self.behavior = behavior
        self.heartbeat_interval = max(0.02, float(heartbeat_interval))
        self.commands: list[dict[str, Any]] = []
        self._events: queue.Queue[Frame | None] = queue.Queue()
        self._commands: queue.Queue[Mapping[str, Any]] = queue.Queue()
        self._sequence = Sequence()
        self._closed = threading.Event()
        self._started = threading.Event()
        self._exit = threading.Event()
        self._exit_code: int | None = None
        self._thread = threading.Thread(target=self._run, name=f"fake-runner-{self.pid}", daemon=True)

    def start(self) -> "FakeRunner":
        self._thread.start()
        return self

    def emit(self, message_type: str, payload: bytes = b"", **fields: Any) -> int:
        sequence = self._sequence.next()
        header = make_event(
            message_type,
            self.spec.run_id,
            sequence,
            binaryLength=len(payload),
            **fields,
        )
        self._events.put(Frame(header, bytes(payload)))
        return sequence

    def emit_raw(self, header: Mapping[str, Any], payload: bytes = b"") -> None:
        self._events.put(Frame(dict(header), bytes(payload)))

    def _run(self) -> None:
        try:
            # hello is the sole event without seq.
            self._events.put(
                Frame(
                    {
                        "type": "hello",
                        "protocol": 1,
                        "runId": self.spec.run_id,
                        "pid": self.pid,
                        "binaryLength": 0,
                    }
                )
            )
            attached = self._next_command(timeout=10.0)
            if attached is None or attached.get("type") != "attached":
                self._exit_code = 2
                return
            start = self._next_command(timeout=10.0)
            if start is None or start.get("type") != "start":
                self._exit_code = 2
                return
            self._started.set()
            if self.behavior == "malformed":
                self.emit_raw({"type": "status", "protocol": 1, "runId": self.spec.run_id, "seq": 1, "binaryLength": 2}, b"x")
                self._exit_code = 2
                return
            self.emit("ready", device={})
            self.emit("status", status="running")
            if self.behavior == "crash":
                self._exit_code = 17
                return
            if callable(self.behavior):
                self.behavior(self)
            elif self.behavior == "complete":
                self.emit("task.started", taskId=self.spec.task_id, timestamp=time.time())
                self.emit("task.completed", taskId=self.spec.task_id, result={"ok": True})
                final_seq = self.emit("finished", outcome="completed", forced=False, deviceDisposition="restore")
                self._wait_for_ack(final_seq)
            elif self.behavior == "hang":
                self._hang()
            elif self.behavior == "silent":
                self._silent()
            else:
                raise ValueError(f"unknown fake behavior: {self.behavior}")
            self._exit_code = 0
        except BaseException:
            self._exit_code = 2
        finally:
            self._exit.set()
            self._events.put(None)

    def _next_command(self, timeout: float | None = None) -> Mapping[str, Any] | None:
        try:
            command = self._commands.get(timeout=timeout)
        except queue.Empty:
            return None
        self.commands.append(dict(command))
        return command

    def _wait_for_ack(self, final_seq: int) -> None:
        deadline = time.monotonic() + 1.0
        while time.monotonic() < deadline and not self._closed.is_set():
            command = self._next_command(timeout=min(0.05, deadline - time.monotonic()))
            if command is None:
                continue
            if command.get("type") == "finishAck" and int(command.get("finalSeq", -1)) >= final_seq:
                return
            if command.get("type") == "stop":
                return

    def _hang(self) -> None:
        next_heartbeat = time.monotonic()
        while not self._closed.is_set():
            timeout = max(0.0, next_heartbeat - time.monotonic())
            command = self._next_command(timeout=timeout)
            if command is not None:
                if command.get("type") == "stop":
                    self.emit("status", status="stopping")
                    self.emit("finished", outcome="stopped", forced=False, deviceDisposition="restore")
                    return
                if command.get("type") == "setPaused":
                    self.emit("status", status="paused" if command.get("paused") else "running")
            if time.monotonic() >= next_heartbeat:
                self.emit("heartbeat", monotonic=time.monotonic())
                next_heartbeat = time.monotonic() + self.heartbeat_interval

    def _silent(self) -> None:
        """Remain alive without heartbeats so watchdog tests are deterministic."""

        while not self._closed.is_set():
            command = self._next_command(timeout=0.05)
            if command is not None and command.get("type") == "stop":
                return

    def send_command(self, header: Mapping[str, Any], payload: bytes = b"") -> None:
        if self._closed.is_set():
            raise BrokenPipeError("fake Runner is closed")
        self._commands.put(dict(header))

    def read_event(self) -> Frame | None:
        value = self._events.get()
        return value

    def poll(self) -> int | None:
        return None if not self._exit.is_set() else self._exit_code

    def wait(self, timeout: float | None = None) -> int:
        if not self._exit.wait(timeout):
            raise TimeoutError("fake Runner did not exit")
        return int(self._exit_code if self._exit_code is not None else -1)

    def terminate(self) -> None:
        self._closed.set()
        self._exit_code = -15
        if not self._thread.is_alive():
            self._exit.set()

    def kill(self) -> None:
        self._closed.set()
        self._exit_code = -9
        if not self._thread.is_alive():
            self._exit.set()

    def close(self) -> None:
        self._closed.set()


class FakeRunnerFactory:
    """Factory that creates and starts one :class:`FakeRunner` per launch."""

    def __init__(self, behavior: str | Callable[[FakeRunner], Any] = "complete", *, pid: int = 9001) -> None:
        self.behavior = behavior
        self.pid = pid
        self.runners: list[FakeRunner] = []

    def launch(self, spec: ExecutionSpec, *, expected_parent_pid: int = 0) -> FakeRunner:
        del expected_parent_pid
        runner = FakeRunner(spec, behavior=self.behavior, pid=self.pid + len(self.runners)).start()
        self.runners.append(runner)
        return runner


__all__ = ["FakeRunner", "FakeRunnerFactory"]
