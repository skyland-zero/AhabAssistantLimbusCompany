from __future__ import annotations

import queue
import threading
import time

import pytest

import module.execution.supervisor as supervisor_module
from module.execution.fake_runner import FakeRunnerFactory
from module.execution.ipc_protocol import Frame, make_event
from module.execution.platform import ProcessIdentity
from module.execution.supervisor import ExecutionSpec, RunnerState, RunnerSupervisor, SubprocessRunnerFactory, _Session


def wait_idle(supervisor: RunnerSupervisor):
    return supervisor.wait_for_idle(3)


def test_fake_runner_completes_once_and_acknowledges_finished() -> None:
    factory = FakeRunnerFactory("complete")
    supervisor = RunnerSupervisor(runner_factory=factory, hello_timeout=1, ready_timeout=1)
    accepted = supervisor.try_start({"taskId": "mirror"}, client_request_id="request-1")
    assert accepted.accepted is True
    final = wait_idle(supervisor)
    assert final.state is RunnerState.IDLE
    assert final.outcome == "completed"
    assert final.forced is False
    assert [command["type"] for command in factory.runners[0].commands] == ["attached", "start", "finishAck"]


def test_fake_runner_stop_is_idempotent_and_does_not_affect_next_run() -> None:
    factory = FakeRunnerFactory("hang")
    supervisor = RunnerSupervisor(runner_factory=factory, hello_timeout=1, ready_timeout=1, stop_grace=0.1, kill_wait=0.1)
    first = supervisor.start({"taskId": "mirror"})
    supervisor.wait_for_state(RunnerState.RUNNING, 2)
    stop = supervisor.request_stop(first.run_id)
    assert stop.accepted is True
    final = wait_idle(supervisor)
    assert final.outcome == "stopped"
    assert supervisor.request_stop(first.run_id).accepted is True

    second = supervisor.start({"taskId": "daily"})
    assert second.run_id != first.run_id
    supervisor.request_stop(second.run_id)
    assert wait_idle(supervisor).run_id == second.run_id


def test_bad_frame_is_failed_closed_and_old_run_cannot_stop_new_run() -> None:
    factory = FakeRunnerFactory("malformed")
    supervisor = RunnerSupervisor(runner_factory=factory, hello_timeout=1, ready_timeout=1, kill_wait=0.1)
    first = supervisor.start({"taskId": "mirror"})
    final = wait_idle(supervisor)
    assert final.outcome == "failed"
    assert final.forced is True
    assert final.error is not None and final.error["code"] == "RUNNER_PROTOCOL_ERROR"

    second = supervisor.start({"taskId": "daily"})
    stale = supervisor.request_stop(first.run_id)
    assert stale.accepted is False
    assert stale.error is not None and stale.error["code"] == "STALE_RUN"
    supervisor.request_stop(second.run_id)
    wait_idle(supervisor)


def test_duplicate_client_request_id_returns_same_run() -> None:
    factory = FakeRunnerFactory("hang")
    supervisor = RunnerSupervisor(runner_factory=factory, hello_timeout=1, ready_timeout=1)
    first = supervisor.try_start({"taskId": "mirror"}, client_request_id="same")
    duplicate = supervisor.try_start({"taskId": "mirror"}, client_request_id="same")
    assert duplicate.accepted is True
    assert duplicate.run_id == first.run_id
    supervisor.request_stop(first.run_id)
    wait_idle(supervisor)


def test_heartbeat_watchdog_forces_silent_runner() -> None:
    factory = FakeRunnerFactory("silent")
    supervisor = RunnerSupervisor(
        runner_factory=factory,
        hello_timeout=1,
        ready_timeout=1,
        heartbeat_interval=0.02,
        heartbeat_timeout=0.08,
        kill_wait=0.1,
    )
    supervisor.start({"taskId": "mirror"})
    final = wait_idle(supervisor)
    assert final.outcome == "crashed"
    assert final.forced is True
    assert final.error is not None and final.error["code"] == "RUNNER_UNRESPONSIVE"


def test_hello_creation_time_mismatch_is_failed_closed(monkeypatch) -> None:
    factory = FakeRunnerFactory("complete", pid=9001)
    identities = iter((ProcessIdentity(9001, 10.0), ProcessIdentity(9001, 11.0)))
    monkeypatch.setattr(
        "module.execution.supervisor.process_identity",
        lambda process, *, run_id=None: next(identities),
    )
    supervisor = RunnerSupervisor(runner_factory=factory, hello_timeout=1, ready_timeout=1, kill_wait=0.1)
    supervisor.start({"taskId": "mirror"})
    final = wait_idle(supervisor)
    assert final.outcome == "failed"
    assert final.forced is True
    assert final.error is not None and final.error["code"] == "RUNNER_PROTOCOL_ERROR"


class _DelayedHelloRunner:
    """Small process double that keeps the bootstrap hello pending on demand."""

    pid = 9100

    def __init__(self, spec: ExecutionSpec) -> None:
        self.spec = spec
        self.commands: list[dict] = []
        self._events: queue.Queue[Frame | None] = queue.Queue()
        self._commands: queue.Queue[dict] = queue.Queue()
        self._hello_gate = threading.Event()
        self._closed = threading.Event()
        self._exited = threading.Event()
        self._exit_code: int | None = None
        self.created = threading.Event()
        self._thread = threading.Thread(target=self._run, daemon=True)

    def start(self) -> "_DelayedHelloRunner":
        self._thread.start()
        self.created.set()
        return self

    def release_hello(self) -> None:
        self._hello_gate.set()

    def _run(self) -> None:
        try:
            self._hello_gate.wait(2.0)
            if self._closed.is_set():
                return
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
            attached = self._commands.get(timeout=2.0)
            self.commands.append(attached)
            start = self._commands.get(timeout=2.0)
            self.commands.append(start)
            self._events.put(Frame(make_event("ready", self.spec.run_id, 1, device={})))
            self._events.put(Frame(make_event("status", self.spec.run_id, 2, status="running")))
            while not self._closed.is_set():
                try:
                    command = self._commands.get(timeout=0.05)
                except queue.Empty:
                    continue
                self.commands.append(command)
                if command.get("type") == "stop":
                    self._events.put(Frame(make_event("status", self.spec.run_id, 3, status="stopping")))
                    self._events.put(
                        Frame(make_event("finished", self.spec.run_id, 4, outcome="stopped", forced=False, deviceDisposition="restore"))
                    )
                    self._exit_code = 0
                    return
                if command.get("type") == "finishAck":
                    self._exit_code = 0
                    return
        except BaseException:
            self._exit_code = 2
        finally:
            self._exit_code = 0 if self._exit_code is None else self._exit_code
            self._exited.set()
            self._events.put(None)

    def send_command(self, header, payload=b"") -> None:
        del payload
        if self._closed.is_set():
            raise BrokenPipeError("runner is closed")
        self._commands.put(dict(header))

    def read_event(self):
        return self._events.get()

    def poll(self):
        return None if not self._exited.is_set() else self._exit_code

    def wait(self, timeout=None):
        if not self._exited.wait(timeout):
            raise TimeoutError("runner did not exit")
        return int(self._exit_code or 0)

    def terminate(self) -> None:
        self._closed.set()
        self._exit_code = -15
        self._exited.set()
        self._events.put(None)

    kill = terminate

    def close(self) -> None:
        self._closed.set()


class _DelayedHelloFactory:
    def __init__(self) -> None:
        self.runner: _DelayedHelloRunner | None = None
        self.created = threading.Event()

    def launch(self, spec: ExecutionSpec, *, expected_parent_pid: int) -> _DelayedHelloRunner:
        del expected_parent_pid
        self.runner = _DelayedHelloRunner(spec).start()
        self.created.set()
        return self.runner


def test_stop_before_hello_completes_handshake_then_stops() -> None:
    factory = _DelayedHelloFactory()
    supervisor = RunnerSupervisor(
        runner_factory=factory,
        hello_timeout=1,
        ready_timeout=1,
        stop_grace=0.5,
        kill_wait=0.1,
    )
    accepted = supervisor.start({"taskId": "mirror"})
    assert factory.created.wait(1.0)
    stopped = supervisor.request_stop(accepted.run_id)
    assert stopped.accepted is True
    runner = factory.runner
    assert runner is not None
    # No command is legal before hello.  Releasing hello lets the launch thread
    # send attached/start and then the deferred stop in command-sequence order.
    assert runner.commands == []
    runner.release_hello()
    final = supervisor.wait_for_idle(3.0)
    assert final.outcome == "stopped"
    assert [command["type"] for command in runner.commands] == ["attached", "start", "stop"]


def test_stop_ack_enqueue_does_not_wait_for_a_blocked_command_pipe() -> None:
    factory = FakeRunnerFactory("hang")
    supervisor = RunnerSupervisor(
        runner_factory=factory,
        hello_timeout=1,
        ready_timeout=1,
        stop_grace=0.05,
        kill_wait=0.05,
    )
    accepted = supervisor.start({"taskId": "mirror"})
    assert supervisor.wait_for_state(RunnerState.RUNNING, 2).state is RunnerState.RUNNING
    runner = factory.runners[0]
    original_send = runner.send_command
    blocked = threading.Event()

    def blocked_send(header, payload=b""):
        if header.get("type") == "stop":
            blocked.wait(5.0)
            return
        original_send(header, payload)

    runner.send_command = blocked_send
    started = time.monotonic()
    assert supervisor.request_stop(accepted.run_id).accepted is True
    assert time.monotonic() - started < 0.2
    final = supervisor.wait_for_idle(2)
    blocked.set()
    assert final.outcome == "stopped"


def test_finished_ack_is_sent_after_durable_event_callback(monkeypatch) -> None:
    spec = ExecutionSpec(run_id="ack-order", task_id="mirror")
    supervisor = RunnerSupervisor(runner_factory=FakeRunnerFactory("complete"))
    session = _Session(spec)
    order: list[str] = []
    ack_sent = threading.Event()

    def callback(message, payload):
        del payload
        assert message["type"] == "finished"
        order.append("callback")

    supervisor.event_callback = callback

    def send_command(current, message_type, **fields):
        del current, fields
        order.append(message_type)
        ack_sent.set()
        return 1

    monkeypatch.setattr(supervisor, "_send_command", send_command)
    header = make_event("finished", spec.run_id, 1, outcome="completed", forced=False, deviceDisposition="restore")
    assert supervisor._handle_event(session, header, b"") is True
    assert ack_sent.wait(1.0)
    assert order == ["callback", "finishAck"]


def test_factory_type_error_from_body_is_not_retried() -> None:
    calls: list[int] = []

    def launch(spec: ExecutionSpec, *, expected_parent_pid: int):
        del spec, expected_parent_pid
        calls.append(1)
        raise TypeError("factory body failure")

    supervisor = RunnerSupervisor(runner_factory=launch)
    session = _Session(ExecutionSpec(run_id="factory-error", task_id="mirror"))
    with pytest.raises(TypeError, match="factory body failure"):
        supervisor._launch_factory(session)
    assert calls == [1]


def test_frozen_factory_locates_runner_beside_application(monkeypatch, tmp_path) -> None:
    executable = tmp_path / "AALC Backend.exe"
    executable.write_bytes(b"backend")
    runner = tmp_path / "runner" / "AALCRunner.exe"
    runner.parent.mkdir()
    runner.write_bytes(b"runner")
    monkeypatch.setattr(supervisor_module.sys, "executable", str(executable))
    monkeypatch.setattr(supervisor_module.sys, "frozen", True, raising=False)
    monkeypatch.delenv("AHAB_RUNNER_EXE", raising=False)
    factory = SubprocessRunnerFactory()
    argv, is_frozen = factory.locate()
    assert argv == [str(runner)]
    assert is_frozen is True
