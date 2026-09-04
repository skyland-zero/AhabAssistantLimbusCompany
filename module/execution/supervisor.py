"""Execution state machine and Runner supervision.

``RunnerSupervisor`` owns one short-lived Runner process at a time.  The public
control methods only validate/update a small state snapshot and enqueue a command;
pipe I/O, handshake waits, watchdog timing and process reaping happen on private
threads.  A process factory and callbacks are injectable, making the contract
testable without launching Python or creating a platform Job Object.
"""

from __future__ import annotations

import inspect
import os
import sys
import threading
import time
import uuid
from collections import OrderedDict
from collections.abc import Callable, Mapping
from dataclasses import dataclass, field
from enum import StrEnum
from pathlib import Path
from typing import Any, Protocol

from .command_queue import CommandQueue, CommandQueueError
from .ipc_protocol import (
    Frame,
    FrameCodec,
    FrameIOError,
    FrameWriter,
    ProtocolError,
    validate_message,
)
from .ipc_protocol import (
    Sequence as WireSequence,
)
from .platform import (
    ManagedProcess,
    ProcessAdapter,
    ProcessIdentity,
    SupervisionError,
    create_process_adapter,
    process_identity,
)

SCHEMA_VERSION = 3
STDERR_TAIL_LIMIT = 256 * 1024


class RunnerState(StrEnum):
    IDLE = "idle"
    STARTING = "starting"
    RUNNING = "running"
    PAUSED = "paused"
    STOPPING = "stopping"
    RESTORING = "restoring"


class DeviceLeaseState(StrEnum):
    NONE = "none"
    ACQUIRING = "acquiring"
    RUNNER = "runner"
    RESTORING = "restoring"


class SupervisorError(RuntimeError):
    """A control request cannot be accepted in the current snapshot."""

    def __init__(self, code: str, message: str, *, snapshot: "ExecutionSnapshot | None" = None) -> None:
        super().__init__(message)
        self.code = code
        self.message = message
        self.snapshot = snapshot

    def as_dict(self) -> dict[str, Any]:
        return {"code": self.code, "message": self.message}


class RunnerProtocolFailure(SupervisorError):
    def __init__(self, message: str, *, snapshot: "ExecutionSnapshot | None" = None) -> None:
        super().__init__("RUNNER_PROTOCOL_ERROR", message, snapshot=snapshot)


class RunnerHandshakeTimeout(SupervisorError):
    def __init__(self, message: str, *, snapshot: "ExecutionSnapshot | None" = None) -> None:
        super().__init__("RUNNER_HANDSHAKE_TIMEOUT", message, snapshot=snapshot)


@dataclass(frozen=True, slots=True)
class ExecutionSpec:
    """Serializable snapshot passed to a one-shot Runner."""

    run_id: str
    task_id: str
    task_config: Mapping[str, Any] = field(default_factory=dict)
    runtime_config: Mapping[str, Any] = field(default_factory=dict)
    config_path: str | None = None
    config_revision: int = 0
    resource_root: str | None = None
    platform_policy: Mapping[str, Any] = field(default_factory=dict)
    allow_emulator_launch: bool = False
    device_target: Mapping[str, Any] = field(default_factory=dict)
    cleanup_reservation: Mapping[str, Any] = field(default_factory=dict)
    extra: Mapping[str, Any] = field(default_factory=dict)

    def __post_init__(self) -> None:
        if not isinstance(self.run_id, str) or not self.run_id:
            raise ValueError("run_id must be a non-empty string")
        if not isinstance(self.task_id, str) or not self.task_id:
            raise ValueError("task_id must be a non-empty string")
        if isinstance(self.config_revision, bool) or not isinstance(self.config_revision, int) or self.config_revision < 0:
            raise ValueError("config_revision must be a non-negative integer")
        if not isinstance(self.allow_emulator_launch, bool):
            raise ValueError("allow_emulator_launch must be bool")

    @classmethod
    def from_mapping(cls, value: Mapping[str, Any], *, run_id: str | None = None) -> "ExecutionSpec":
        if not isinstance(value, Mapping):
            raise TypeError("ExecutionSpec must be a mapping")
        data = dict(value)
        actual_run_id = str(data.pop("runId", data.pop("run_id", run_id or "")))
        task_id = str(data.pop("taskId", data.pop("task_id", "")))
        known = {
            "task_config": data.pop("taskConfig", data.pop("task_config", {})),
            "runtime_config": data.pop("runtimeConfig", data.pop("runtime_config", {})),
            "config_path": data.pop("configPath", data.pop("config_path", None)),
            "config_revision": data.pop("configRevision", data.pop("config_revision", 0)),
            "resource_root": data.pop("resourceRoot", data.pop("resource_root", None)),
            "platform_policy": data.pop("platformPolicy", data.pop("platform_policy", {})),
            "allow_emulator_launch": data.pop(
                "allowEmulatorLaunch",
                data.pop("allow_emulator_launch", False),
            ),
            "device_target": data.pop("deviceTarget", data.pop("device_target", {})),
            "cleanup_reservation": data.pop("cleanupReservation", data.pop("cleanup_reservation", {})),
            "extra": data,
        }
        return cls(run_id=actual_run_id, task_id=task_id, **known)

    def to_dict(self) -> dict[str, Any]:
        value: dict[str, Any] = {
            "runId": self.run_id,
            "taskId": self.task_id,
            "taskConfig": dict(self.task_config),
            "runtimeConfig": dict(self.runtime_config),
            "configPath": self.config_path,
            "configRevision": self.config_revision,
            "resourceRoot": self.resource_root,
            "platformPolicy": dict(self.platform_policy),
            "allowEmulatorLaunch": self.allow_emulator_launch,
            "deviceTarget": dict(self.device_target),
            "cleanupReservation": dict(self.cleanup_reservation),
        }
        value.update(dict(self.extra))
        return value

    as_dict = to_dict
    to_mapping = to_dict

    @property
    def runId(self) -> str:  # noqa: N802 - wire-schema compatibility
        return self.run_id

    @property
    def taskId(self) -> str:  # noqa: N802 - wire-schema compatibility
        return self.task_id

    @property
    def configPath(self) -> str | None:  # noqa: N802 - wire-schema compatibility
        return self.config_path

    @property
    def resourceRoot(self) -> str | None:  # noqa: N802 - wire-schema compatibility
        return self.resource_root


@dataclass(frozen=True, slots=True)
class ExecutionSnapshot:
    schema_version: int = SCHEMA_VERSION
    state: RunnerState = RunnerState.IDLE
    state_revision: int = 0
    current_task_id: str | None = None
    run_id: str | None = None
    runner_pid: int | None = None
    device_lease: DeviceLeaseState = DeviceLeaseState.NONE
    outcome: str | None = None
    forced: bool = False
    requested_by: str | None = None
    error: dict[str, Any] | None = None
    device_restore: str = "not_needed"
    ready_at: float | None = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "schemaVersion": self.schema_version,
            "state": self.state.value,
            "stateRevision": self.state_revision,
            "currentTaskId": self.current_task_id,
            "runId": self.run_id,
            "runnerPid": self.runner_pid,
            "deviceLease": self.device_lease.value,
            "outcome": self.outcome,
            "forced": self.forced,
            "requestedBy": self.requested_by,
            "error": None if self.error is None else dict(self.error),
            "deviceRestore": self.device_restore,
        }

    as_dict = to_dict

    def __getitem__(self, key: str) -> Any:
        return self.to_dict()[key]

    def get(self, key: str, default: Any = None) -> Any:
        return self.to_dict().get(key, default)


@dataclass(frozen=True, slots=True)
class CommandResult:
    accepted: bool
    snapshot: ExecutionSnapshot
    error: dict[str, Any] | None = None

    def to_dict(self) -> dict[str, Any]:
        value = self.snapshot.to_dict()
        value["accepted"] = self.accepted
        if self.error is not None:
            value["error"] = dict(self.error)
        return value

    def __getattr__(self, name: str) -> Any:
        # Keep lightweight handlers ergonomic: ``result.run_id`` works while the
        # explicit ``snapshot`` field remains available for JSON serialization.
        return getattr(self.snapshot, name)


class RunnerProcess(Protocol):
    """Process plus the two protocol streams consumed by the supervisor."""

    pid: int

    def send_command(self, header: Mapping[str, Any], payload: bytes = b"") -> None: ...

    def read_event(self) -> Frame | Mapping[str, Any] | None: ...

    def poll(self) -> int | None: ...

    def wait(self, timeout: float | None = None) -> int: ...

    def terminate(self) -> None: ...

    def kill(self) -> None: ...

    def close(self) -> None: ...


class RunnerFactory(Protocol):
    def launch(self, spec: ExecutionSpec, *, expected_parent_pid: int) -> RunnerProcess: ...


class _PipeRunnerProcess:
    """RunnerProcess wrapper used by :class:`SubprocessRunnerFactory`."""

    def __init__(
        self,
        process: ManagedProcess,
        command_stream: Any,
        event_stream: Any,
        stderr_stream: Any,
        *,
        run_id: str,
    ) -> None:
        self._process = process
        self._command_stream = command_stream
        self._event_stream = event_stream
        self._stderr_stream = stderr_stream
        self._writer = FrameWriter(command_stream, run_id=run_id)
        self._codec = FrameCodec()
        self._stderr_tail = bytearray()
        self._stderr_lock = threading.Lock()
        self.pid = int(process.pid)
        self._stderr_thread = threading.Thread(target=self._drain_stderr, name=f"runner-stderr-{self.pid}", daemon=True)
        self._stderr_thread.start()

    @property
    def stderr_tail(self) -> bytes:
        with self._stderr_lock:
            return bytes(self._stderr_tail)

    def _drain_stderr(self) -> None:
        stream = self._stderr_stream
        if stream is None:
            return
        try:
            while True:
                chunk = stream.read(8192)
                if not chunk:
                    return
                with self._stderr_lock:
                    self._stderr_tail.extend(bytes(chunk))
                    del self._stderr_tail[:-STDERR_TAIL_LIMIT]
        except (OSError, ValueError):
            return

    def send_command(self, header: Mapping[str, Any], payload: bytes = b"") -> None:
        self._codec.write(self._command_stream, header, payload, lock=self._writer._lock)

    def read_event(self) -> Frame | None:
        return self._codec.read(self._event_stream)

    def poll(self) -> int | None:
        return self._process.poll()

    def wait(self, timeout: float | None = None) -> int:
        return self._process.wait(timeout)

    def terminate(self) -> None:
        self._process.terminate()

    def kill(self) -> None:
        self._process.kill()

    def close(self) -> None:
        self._writer.close()
        for stream in (self._command_stream, self._event_stream, self._stderr_stream):
            try:
                if stream is not None:
                    stream.close()
            except (OSError, ValueError):
                pass
        close = getattr(self._process, "close", None)
        if callable(close):
            close()


class SubprocessRunnerFactory:
    """Locate and launch ``runner_bootstrap.py`` or the frozen Runner binary."""

    def __init__(
        self,
        *,
        app_root: str | os.PathLike[str] | None = None,
        adapter: ProcessAdapter | None = None,
        executable: str | os.PathLike[str] | None = None,
        python_executable: str | None = None,
    ) -> None:
        if app_root is None:
            # In a frozen sidecar ``__file__`` points into PyInstaller's private
            # extraction/runtime directory.  The staged Runner lives beside the
            # executable (``<app>/runner/AALCRunner.exe``), so derive the default
            # root from ``sys.executable`` in that mode.  Development imports keep
            # using the repository root.
            if getattr(sys, "frozen", False):
                app_root = Path(sys.executable).resolve().parent
            else:
                app_root = Path(__file__).resolve().parents[2]
        self.app_root = Path(app_root).resolve()
        self.adapter = adapter or create_process_adapter()
        self.executable = Path(executable).resolve() if executable is not None else None
        self._pythonpath: str | None = None
        if python_executable is not None:
            self.python_executable = python_executable
        elif os.name == "nt" and getattr(sys, "_base_executable", sys.executable) != sys.executable:
            # Some Python 3.14 virtual environments expose a launcher shim as
            # ``sys.executable``.  It creates a second interpreter process, which
            # violates the hello-PID/direct-child contract.  Use the base binary
            # directly and add the venv's site-packages explicitly.
            self.python_executable = str(getattr(sys, "_base_executable"))
            venv_site = Path(sys.executable).resolve().parents[1] / "Lib" / "site-packages"
            if venv_site.exists():
                self._pythonpath = str(venv_site)
        else:
            self.python_executable = sys.executable

    def locate(self) -> tuple[list[str], bool]:
        configured = os.environ.get("AHAB_RUNNER_EXE")
        if configured:
            configured_path = Path(configured)
            if not configured_path.is_absolute():
                raise SupervisorError("RUNNER_NOT_FOUND", "AHAB_RUNNER_EXE must be an absolute path")
            if not configured_path.exists():
                raise SupervisorError("RUNNER_NOT_FOUND", f"Runner does not exist: {configured_path}")
            return [str(configured_path)], True
        if self.executable is not None:
            if not self.executable.exists():
                raise SupervisorError("RUNNER_NOT_FOUND", f"Runner does not exist: {self.executable}")
            return [str(self.executable)], True
        frozen = getattr(sys, "frozen", False)
        if frozen:
            executable_name = "AALCRunner.exe" if os.name == "nt" else "AALCRunner"
            # ``app_root`` is normally the executable directory.  Retaining a
            # second candidate based on ``sys.executable`` also makes an
            # explicitly supplied private runtime root work for one-file/onedir
            # compatibility launchers without ever falling back to a PATH lookup.
            roots = [self.app_root]
            executable_root = Path(sys.executable).resolve().parent
            if executable_root not in roots:
                roots.append(executable_root)
            candidates = [root / "runner" / executable_name for root in roots]
            bundled = next((candidate for candidate in candidates if candidate.is_file()), candidates[0])
            if not bundled.is_file():
                raise SupervisorError("RUNNER_NOT_FOUND", f"Bundled Runner does not exist: {bundled}")
            return [str(bundled)], True
        bootstrap = self.app_root / "runner_bootstrap.py"
        if not bootstrap.exists():
            raise SupervisorError("RUNNER_NOT_FOUND", f"Runner bootstrap does not exist: {bootstrap}")
        return [self.python_executable, str(bootstrap)], False

    def launch(self, spec: ExecutionSpec, *, expected_parent_pid: int) -> RunnerProcess:
        command_argv, _ = self.locate()
        child_command, parent_command = os.pipe()
        parent_event, child_event = os.pipe()
        parent_stderr, child_stderr = os.pipe()
        # The child receives these as ordinary binary streams.  On POSIX explicit
        # fd flags make the contract obvious; Windows inherits std handles.
        argv = [
            *command_argv,
            "--run-id",
            spec.run_id,
            "--protocol",
            "1",
            "--expected-parent-pid",
            str(expected_parent_pid),
        ]
        child_fds = (child_command, child_event, child_stderr)
        try:
            if os.name == "nt":
                # Windows Job launchers inherit stdin/stdout/stderr handles and
                # still perform suspended-create → assign → resume.
                launch_env = os.environ.copy()
                if self._pythonpath:
                    existing = launch_env.get("PYTHONPATH")
                    launch_env["PYTHONPATH"] = (
                        self._pythonpath if not existing else self._pythonpath + os.pathsep + existing
                    )
                managed = self.adapter.launch(
                    argv,
                    cwd=self.app_root,
                    env=launch_env,
                    stdin=child_command,
                    stdout=child_event,
                    stderr=child_stderr,
                )
            else:
                argv += [
                    "--command-fd",
                    str(child_command),
                    "--event-fd",
                    str(child_event),
                    "--stderr-fd",
                    str(child_stderr),
                ]
                managed = self.adapter.launch(
                    argv,
                    cwd=self.app_root,
                    env=os.environ.copy(),
                    stdin=child_command,
                    stdout=child_event,
                    stderr=child_stderr,
                    pass_fds=child_fds,
                )
        except Exception:
            for fd in (*child_fds, parent_command, parent_event, parent_stderr):
                try:
                    os.close(fd)
                except OSError:
                    pass
            raise
        finally:
            for fd in child_fds:
                try:
                    os.close(fd)
                except OSError:
                    pass
        streams: list[Any] = []
        try:
            command_stream = os.fdopen(parent_command, "wb", buffering=0)
            streams.append(command_stream)
            event_stream = os.fdopen(parent_event, "rb", buffering=0)
            streams.append(event_stream)
            stderr_stream = os.fdopen(parent_stderr, "rb", buffering=0)
            streams.append(stderr_stream)
            return _PipeRunnerProcess(managed, command_stream, event_stream, stderr_stream, run_id=spec.run_id)
        except Exception:
            for stream in streams:
                try:
                    stream.close()
                except (OSError, ValueError):
                    pass
            # A process that was successfully created but could not be wrapped
            # must not survive outside its containment boundary.  ``close`` is
            # the native Job kill-on-close path; the fallback handles injected
            # process doubles.
            try:
                close = getattr(managed, "close", None)
                if callable(close):
                    close()
                else:
                    managed.kill()
            except Exception:
                pass
            raise


@dataclass(slots=True)
class _Session:
    spec: ExecutionSpec
    process: RunnerProcess | None = None
    process_identity: ProcessIdentity | None = None
    command_sequence: WireSequence = field(default_factory=WireSequence)
    last_event_seq: int = 0
    hello_seen: bool = False
    attached_sent: bool = False
    start_sent: bool = False
    stop_sent: bool = False
    ready_seen: bool = False
    finished_seen: bool = False
    finish_ack_sent: bool = False
    finalized: bool = False
    forced: bool = False
    stop_requested: bool = False
    requested_by: str | None = None
    exit_code: int | None = None
    last_heartbeat: float = field(default_factory=time.monotonic)
    hello_event: threading.Event = field(default_factory=threading.Event)
    ready_event: threading.Event = field(default_factory=threading.Event)
    finished_event: threading.Event = field(default_factory=threading.Event)
    process_exit: threading.Event = field(default_factory=threading.Event)
    events_eof: threading.Event = field(default_factory=threading.Event)
    command_queue: CommandQueue | None = None
    reader_error: BaseException | None = None
    protocol_failed: bool = False
    unresponsive: bool = False
    lock: threading.RLock = field(default_factory=threading.RLock)
    command_lock: threading.Lock = field(default_factory=threading.Lock)


class RunnerSupervisor:
    """Single-run actor coordinating Runner IPC and process containment."""

    def __init__(
        self,
        *,
        runner_factory: RunnerFactory | Callable[..., RunnerProcess] | None = None,
        process_adapter: ProcessAdapter | None = None,
        hello_timeout: float = 3.0,
        ready_timeout: float = 10.0,
        stop_grace: float = 3.0,
        kill_wait: float = 1.0,
        finish_ack_timeout: float = 1.0,
        heartbeat_interval: float = 1.0,
        heartbeat_timeout: float = 5.0,
        command_queue_size: int = 64,
        restore_callback: Callable[..., Any] | None = None,
        cleanup_callback: Callable[..., Any] | None = None,
        event_callback: Callable[[Mapping[str, Any], bytes], Any] | None = None,
        max_client_requests: int = 128,
    ) -> None:
        self.runner_factory = runner_factory or SubprocessRunnerFactory(adapter=process_adapter)
        if process_adapter is not None:
            self.process_adapter = process_adapter
        else:
            self.process_adapter = getattr(self.runner_factory, "adapter", None)
        self.hello_timeout = max(0.0, float(hello_timeout))
        self.ready_timeout = max(0.0, float(ready_timeout))
        self.stop_grace = max(0.0, float(stop_grace))
        self.kill_wait = max(0.0, float(kill_wait))
        self.finish_ack_timeout = max(0.05, float(finish_ack_timeout))
        self.heartbeat_interval = max(0.05, float(heartbeat_interval))
        self.heartbeat_timeout = max(self.heartbeat_interval, float(heartbeat_timeout))
        if isinstance(command_queue_size, bool) or not isinstance(command_queue_size, int) or command_queue_size <= 0:
            raise ValueError("command_queue_size must be a positive integer")
        self.command_queue_size = command_queue_size
        self.restore_callback = restore_callback
        self.cleanup_callback = cleanup_callback
        self.event_callback = event_callback
        self.max_client_requests = max(1, int(max_client_requests))
        self._condition = threading.Condition(threading.RLock())
        self._snapshot = ExecutionSnapshot()
        self._session: _Session | None = None
        self._client_requests: OrderedDict[str, ExecutionSnapshot] = OrderedDict()
        self._closed = False

    @property
    def snapshot(self) -> ExecutionSnapshot:
        with self._condition:
            return self._snapshot

    def get_state(self) -> ExecutionSnapshot:
        return self.snapshot

    def _replace_snapshot_locked(self, **changes: Any) -> ExecutionSnapshot:
        current = self._snapshot
        if "state" in changes and not isinstance(changes["state"], RunnerState):
            changes["state"] = RunnerState(changes["state"])
        if "device_lease" in changes and not isinstance(changes["device_lease"], DeviceLeaseState):
            changes["device_lease"] = DeviceLeaseState(changes["device_lease"])
        if changes.get("state") != current.state or any(
            key in changes and changes[key] != getattr(current, key) for key in changes
        ):
            changes.setdefault("state_revision", current.state_revision + 1)
        self._snapshot = ExecutionSnapshot(**{**current.__dict__, **changes}) if hasattr(current, "__dict__") else ExecutionSnapshot(
            schema_version=current.schema_version,
            state=changes.get("state", current.state),
            state_revision=changes.get("state_revision", current.state_revision),
            current_task_id=changes.get("current_task_id", current.current_task_id),
            run_id=changes.get("run_id", current.run_id),
            runner_pid=changes.get("runner_pid", current.runner_pid),
            device_lease=changes.get("device_lease", current.device_lease),
            outcome=changes.get("outcome", current.outcome),
            forced=changes.get("forced", current.forced),
            requested_by=changes.get("requested_by", current.requested_by),
            error=changes.get("error", current.error),
            device_restore=changes.get("device_restore", current.device_restore),
            ready_at=changes.get("ready_at", current.ready_at),
        )
        self._condition.notify_all()
        return self._snapshot

    def _remember_client_request_locked(self, request_id: str, snapshot: ExecutionSnapshot) -> None:
        self._client_requests[request_id] = snapshot
        self._client_requests.move_to_end(request_id)
        while len(self._client_requests) > self.max_client_requests:
            self._client_requests.popitem(last=False)

    def _make_spec(self, spec: ExecutionSpec | Mapping[str, Any], run_id: str) -> ExecutionSpec:
        if isinstance(spec, ExecutionSpec):
            if spec.run_id != run_id:
                value = spec.to_dict()
                value["runId"] = run_id
                return ExecutionSpec.from_mapping(value, run_id=run_id)
            return spec
        return ExecutionSpec.from_mapping(spec, run_id=run_id)

    def start(
        self,
        spec: ExecutionSpec | Mapping[str, Any],
        *,
        client_request_id: str | None = None,
        run_id: str | None = None,
    ) -> ExecutionSnapshot:
        result = self.try_start(spec, client_request_id=client_request_id, run_id=run_id)
        if not result.accepted:
            error = SupervisorError(
                result.error["code"] if result.error else "EXECUTION_BUSY",
                result.error["message"] if result.error else "execution is busy",
                snapshot=result.snapshot,
            )
            raise error
        return result.snapshot

    def try_start(
        self,
        spec: ExecutionSpec | Mapping[str, Any],
        *,
        client_request_id: str | None = None,
        run_id: str | None = None,
    ) -> CommandResult:
        request_key = str(client_request_id) if client_request_id is not None else None
        with self._condition:
            if request_key is not None and request_key in self._client_requests:
                return CommandResult(True, self._client_requests[request_key])
            if self._closed:
                error = {"code": "EXECUTION_BUSY", "message": "supervisor is closed"}
                return CommandResult(False, self._snapshot, error)
            if self._snapshot.state is not RunnerState.IDLE or self._snapshot.device_lease is not DeviceLeaseState.NONE:
                error = {"code": "EXECUTION_BUSY", "message": "another execution or device lease is active"}
                return CommandResult(False, self._snapshot, error)
            if run_id is not None:
                assigned_run_id = str(run_id)
            elif isinstance(spec, ExecutionSpec):
                assigned_run_id = spec.run_id
            elif isinstance(spec, Mapping) and spec.get("runId"):
                assigned_run_id = str(spec["runId"])
            else:
                assigned_run_id = str(uuid.uuid4())
            try:
                execution_spec = self._make_spec(spec, assigned_run_id)
            except (TypeError, ValueError) as exc:
                error = {"code": "INVALID_EXECUTION_SPEC", "message": str(exc)}
                return CommandResult(False, self._snapshot, error)
            session = _Session(execution_spec)
            self._session = session
            accepted_snapshot = self._replace_snapshot_locked(
                state=RunnerState.STARTING,
                current_task_id=execution_spec.task_id,
                run_id=execution_spec.run_id,
                runner_pid=None,
                device_lease=DeviceLeaseState.ACQUIRING,
                outcome=None,
                forced=False,
                requested_by=None,
                error=None,
                device_restore="not_needed",
                ready_at=None,
            )
            if request_key is not None:
                self._remember_client_request_locked(request_key, accepted_snapshot)
            thread = threading.Thread(target=self._launch_session, args=(session,), name=f"runner-launch-{execution_spec.run_id}", daemon=True)
            thread.start()
            return CommandResult(True, accepted_snapshot)

    # Convenient aliases for RPC adapters.
    reserve_start = try_start
    request_start = try_start

    def _launch_factory(self, session: _Session) -> RunnerProcess:
        factory = self.runner_factory
        expected_parent_pid = os.getpid()
        launch = getattr(factory, "launch", None)
        if callable(launch):
            return self._invoke_launch(launch, session.spec, expected_parent_pid)
        if callable(factory):
            return self._invoke_launch(factory, session.spec, expected_parent_pid)
        raise SupervisionError("RUNNER_SUPERVISION_FAILED", "runner factory is not callable")

    @staticmethod
    def _invoke_launch(
        launch: Callable[..., RunnerProcess],
        spec: ExecutionSpec,
        expected_parent_pid: int,
    ) -> RunnerProcess:
        """Call a factory once using a signature-compatible argument shape.

        Retrying a callable after catching ``TypeError`` is unsafe: a factory can
        have already allocated a process and then raise a genuine ``TypeError``
        in its body.  Inspecting the signature keeps compatibility with the
        small ``launch(spec)`` test doubles without duplicating launch side
        effects.
        """

        candidates = (
            ((spec,), {"expected_parent_pid": expected_parent_pid}),
            ((spec, expected_parent_pid), {}),
            ((spec,), {}),
        )
        try:
            signature = inspect.signature(launch)
        except (TypeError, ValueError):
            # Opaque builtins are rare here.  Make one best-effort call; do not
            # issue a second call after a body-level TypeError.
            return launch(spec, expected_parent_pid=expected_parent_pid)
        for args, kwargs in candidates:
            try:
                signature.bind(*args, **kwargs)
            except TypeError:
                continue
            return launch(*args, **kwargs)
        raise TypeError("runner factory has an unsupported launch signature")

    def _launch_session(self, session: _Session) -> None:
        try:
            if session.stop_requested:
                self._begin_restoring(session, outcome="stopped")
                return
            process = self._launch_factory(session)
            with session.lock:
                session.process = process
                session.process_identity = process_identity(process, run_id=session.spec.run_id)
                session.command_queue = self._new_command_queue(session, process)
            with self._condition:
                if self._session is not session or session.finalized:
                    self._force_process(session)
                    return
                self._replace_snapshot_locked(
                    runner_pid=int(process.pid),
                    device_lease=DeviceLeaseState.RUNNER,
                )
            reader = threading.Thread(target=self._read_events, args=(session,), name=f"runner-events-{session.spec.run_id}", daemon=True)
            watcher = threading.Thread(target=self._watch_process, args=(session,), name=f"runner-watch-{session.spec.run_id}", daemon=True)
            reader.start()
            watcher.start()
            watchdog = threading.Thread(target=self._watchdog_loop, args=(session,), name=f"runner-watchdog-{session.spec.run_id}", daemon=True)
            watchdog.start()
            if not session.hello_event.wait(self.hello_timeout):
                self._fail_start(session, RunnerHandshakeTimeout("Runner hello timed out"))
                return
            # Bootstrap only accepts ``attached`` followed by ``start``.  A stop
            # can arrive during hello or initialization, so complete this tiny
            # handshake first and queue stop immediately afterwards.  Sending a
            # bare stop here makes the Runner reject its command protocol and
            # turns an ordinary early stop into a startup failure.
            self._send_command(session, "attached")
            with session.lock:
                session.attached_sent = True
            self._send_command(session, "start", spec=session.spec.to_dict())
            with session.lock:
                session.start_sent = True
            self._send_stop_if_ready(session)
            if not session.ready_event.wait(self.ready_timeout):
                self._fail_start(session, RunnerHandshakeTimeout("Runner ready timed out"))
                return
            self._send_stop_if_ready(session)
        except RunnerHandshakeTimeout as exc:
            self._fail_start(session, exc)
        except CommandQueueError as exc:
            self._fail_start(session, SupervisorError(exc.code, exc.message))
        except TimeoutError as exc:
            self._fail_start(session, SupervisorError("IPC_BACKPRESSURE", str(exc)))
        except (SupervisionError, OSError, ProtocolError, FrameIOError) as exc:
            self._fail_start(session, SupervisorError("RUNNER_SUPERVISION_FAILED", str(exc)))
        except Exception as exc:  # pragma: no cover - defensive boundary
            self._fail_start(session, SupervisorError("RUNNER_SUPERVISION_FAILED", f"{exc}"))

    def _new_command_queue(self, session: _Session, process: RunnerProcess) -> CommandQueue:
        return CommandQueue(
            # Resolve the method at write time.  Besides keeping the queue
            # independent from a concrete process wrapper, this lets a process
            # adapter revoke/replace its stream writer during shutdown and keeps
            # injected blocked-pipe doubles observable by the supervisor.
            lambda header, payload=b"": process.send_command(header, payload),
            run_id=session.spec.run_id,
            sequence=session.command_sequence,
            maxsize=self.command_queue_size,
            name=f"runner-command-writer-{session.spec.run_id}",
            on_error=lambda error: self._on_command_queue_error(session, error),
        )

    def _command_queue(self, session: _Session) -> CommandQueue:
        with session.lock:
            queue = session.command_queue
            process = session.process
            if queue is None and process is not None:
                queue = self._new_command_queue(session, process)
                session.command_queue = queue
            if queue is None:
                raise FrameIOError("Runner process is not available")
            return queue

    def _enqueue_command(self, session: _Session, message_type: str, **fields: Any):
        """Admit a command without waiting for the pipe writer."""

        return self._command_queue(session).enqueue(message_type, fields=fields)

    def _send_command(self, session: _Session, message_type: str, **fields: Any) -> int:
        """Compatibility helper for the bootstrap handshake.

        Handshake callers wait for their ticket on a private launch thread. RPC
        callers use :meth:`_enqueue_command` so a blocked pipe never blocks the
        request handler or the stop-grace timer.
        """

        ticket = self._enqueue_command(session, message_type, **fields)
        return ticket.wait(max(0.05, self.finish_ack_timeout))

    def _on_command_queue_error(self, session: _Session, error: CommandQueueError) -> None:
        """Fail closed when an admitted command cannot reach the Runner.

        This callback is invoked by the queue writer for asynchronous commands
        (for example ``setPaused``).  Admission failures are routed here by the
        synchronous callers as well, so the public snapshot always contains the
        same structured IPC error and process termination does not depend on a
        future heartbeat.
        """

        with self._condition:
            if self._session is not session or session.finalized:
                return
            session.protocol_failed = True
            session.stop_requested = True
            session.requested_by = session.requested_by or "watchdog"
            current_error = self._snapshot.error or error.as_dict()
            self._replace_snapshot_locked(
                state=RunnerState.STOPPING,
                requested_by=session.requested_by,
                error=current_error,
                outcome="failed",
            )
        self._force_process(session)

    def _send_stop_if_ready(self, session: _Session) -> bool:
        """Send the one idempotent stop command once start is on the wire."""

        with session.lock:
            if session.finalized or not session.stop_requested or not session.start_sent or session.stop_sent:
                return False
            requested_by = session.requested_by or "user"
            session.stop_sent = True
        try:
            # Admission is intentionally non-blocking.  The stop-grace timer
            # must remain independent of a writer blocked in the OS pipe.
            self._enqueue_command(session, "stop", requestedBy=requested_by)
        except CommandQueueError as exc:
            self._on_command_queue_error(session, exc)
        except Exception as exc:
            # The watcher/force path owns process termination when the pipe has
            # already gone away.  Keeping ``stop_sent`` true prevents retries
            # from racing a closing process.
            self._on_command_queue_error(session, CommandQueueError("IPC_WRITE_FAILED", str(exc)))
        return True

    def _read_events(self, session: _Session) -> None:
        try:
            # Continue after the process handle reports exit: the peer may have
            # already serialized final frames and the sidecar must drain them
            # before finalization.
            while True:
                process = session.process
                if process is None:
                    return
                value = process.read_event()
                if value is None:
                    return
                if isinstance(value, Frame):
                    header, payload = value.header, value.payload
                elif isinstance(value, Mapping):
                    header, payload = dict(value), b""
                else:
                    raise ProtocolError("Runner returned an invalid event object")
                self._handle_event(session, header, payload)
        except (ProtocolError, FrameIOError, OSError) as exc:
            session.reader_error = exc
            self._handle_protocol_failure(session, str(exc))
        except Exception as exc:  # pragma: no cover - defensive boundary
            session.reader_error = exc
            self._handle_protocol_failure(session, f"event reader failed: {exc}")
        finally:
            session.events_eof.set()

    def _watchdog_loop(self, session: _Session) -> None:
        """Force an alive Runner that has stopped heartbeating."""

        while not session.process_exit.wait(self.heartbeat_interval):
            with session.lock:
                if session.finalized or session.finished_seen:
                    return
                if not session.hello_seen or not session.ready_seen:
                    continue
                process = session.process
                last_heartbeat = session.last_heartbeat
            if process is None or process.poll() is not None:
                return
            if time.monotonic() - last_heartbeat < self.heartbeat_timeout:
                continue
            with self._condition:
                if self._session is not session or session.finalized:
                    return
                session.stop_requested = True
                session.requested_by = "watchdog"
                session.unresponsive = True
                self._replace_snapshot_locked(
                    state=RunnerState.STOPPING,
                    requested_by="watchdog",
                    error={
                        "code": "RUNNER_UNRESPONSIVE",
                        "message": "Runner heartbeat timed out",
                    },
                )
            self._force_process(session)
            return

    def _watch_process(self, session: _Session) -> None:
        process = session.process
        if process is None:
            return
        try:
            exit_code = process.wait()
        except Exception as exc:  # pragma: no cover - process implementation detail
            session.reader_error = exc
            exit_code = -1
        session.exit_code = int(exit_code)
        session.process_exit.set()
        # Process exit does not imply that the pipe reader consumed the final
        # serialized frame.  Wait briefly for EOF before deriving the outcome.
        session.events_eof.wait(max(0.1, self.kill_wait))
        self._on_process_exit(session)

    def _handle_event(self, session: _Session, header: Mapping[str, Any], payload: bytes) -> bool:
        message = validate_message(header, direction="event", expected_run_id=session.spec.run_id)
        message_type = message["type"]
        if message_type == "hello":
            with session.lock:
                if session.hello_seen:
                    raise ProtocolError("duplicate hello")
                process = session.process
                pid = message.get("pid")
                if process is not None:
                    try:
                        hello_pid = int(pid)
                    except (TypeError, ValueError) as exc:
                        raise ProtocolError("hello.pid is invalid") from exc
                    if hello_pid != int(process.pid):
                        raise ProtocolError("hello.pid does not match launched process")
                    expected_identity = session.process_identity
                    if expected_identity is not None:
                        hello_identity = process_identity(process, run_id=session.spec.run_id)
                        if not expected_identity.matches(hello_identity):
                            raise ProtocolError("hello process identity does not match launched process")
                session.hello_seen = True
                session.last_heartbeat = time.monotonic()
                session.hello_event.set()
            return True
        seq = int(message["seq"])
        finish_ack_seq: int | None = None
        with session.lock:
            if seq <= session.last_event_seq:
                # Duplicates and delayed low-priority events are safe to discard.
                return False
            if session.finished_seen:
                raise ProtocolError("event arrived after finished")
            session.last_event_seq = seq
            if message_type == "heartbeat":
                session.last_heartbeat = time.monotonic()
            elif message_type == "ready":
                session.ready_seen = True
                session.ready_event.set()
                with self._condition:
                    self._replace_snapshot_locked(ready_at=time.monotonic())
            elif message_type == "status":
                target = message.get("status")
                with self._condition:
                    if target == "running" and self._snapshot.state in {RunnerState.STARTING, RunnerState.PAUSED}:
                        self._replace_snapshot_locked(state=RunnerState.RUNNING, device_lease=DeviceLeaseState.RUNNER)
                    elif target == "paused" and self._snapshot.state in {RunnerState.RUNNING, RunnerState.STARTING}:
                        self._replace_snapshot_locked(state=RunnerState.PAUSED, device_lease=DeviceLeaseState.RUNNER)
                    elif target == "stopping" and self._snapshot.state is not RunnerState.IDLE:
                        self._replace_snapshot_locked(state=RunnerState.STOPPING)
            elif message_type == "task.started":
                task_id = message.get("taskId")
                if isinstance(task_id, str):
                    with self._condition:
                        self._replace_snapshot_locked(current_task_id=task_id)
            elif message_type == "finished":
                session.finished_seen = True
                session.finished_event.set()
                with self._condition:
                    self._replace_snapshot_locked(
                        state=RunnerState.RESTORING,
                        device_lease=DeviceLeaseState.RESTORING,
                        outcome=message.get("outcome"),
                        forced=bool(message.get("forced", False)) or session.forced,
                        error=message.get("error"),
                    )
                # Defer ACK until the callback has had a chance to persist the
                # finished frame, finalSeq and any preceding resource/config
                # obligations.  A callback failure therefore cannot be
                # acknowledged as durable work.  The actual write happens
                # outside ``session.lock`` so a blocked pipe cannot stall state
                # inspection or stop requests.
                finish_ack_seq = seq
            elif message_type == "error":
                error = message.get("error")
                if not isinstance(error, Mapping):
                    error = {"code": "RUNNER_INIT_FAILED", "message": str(message.get("message", "Runner error"))}
                with self._condition:
                    self._replace_snapshot_locked(error=dict(error))
        callback = self.event_callback
        if callback is not None:
            callback(message, payload)
        if finish_ack_seq is not None:
            # Keep the event-reader responsive even if a malicious/broken peer
            # has filled the command pipe.  A bounded timer force-closes a Runner
            # that never accepts the acknowledgement.
            threading.Thread(
                target=self._send_finish_ack,
                args=(session, finish_ack_seq),
                name=f"runner-finish-ack-{session.spec.run_id}",
                daemon=True,
            ).start()
            timer = threading.Timer(self.finish_ack_timeout, self._force_if_finish_ack_missing, args=(session,))
            timer.daemon = True
            timer.start()
        return True

    def _send_finish_ack(self, session: _Session, final_seq: int) -> None:
        try:
            self._send_command(session, "finishAck", finalSeq=final_seq)
            with session.lock:
                session.finish_ack_sent = True
            # ``finishAck`` is an acknowledgement of durable receipt, not proof
            # that the peer actually exited.  Keep a separate bounded reap timer
            # for a Runner that ignores the acknowledgement or gets stuck in
            # teardown.
            timer = threading.Timer(self.finish_ack_timeout, self._force_if_finished_process_alive, args=(session,))
            timer.daemon = True
            timer.start()
        except CommandQueueError as exc:
            self._on_command_queue_error(session, exc)
        except TimeoutError as exc:
            self._on_command_queue_error(session, CommandQueueError("IPC_BACKPRESSURE", str(exc)))
        except Exception as exc:
            self._on_command_queue_error(session, CommandQueueError("IPC_WRITE_FAILED", str(exc)))

    def _force_if_finish_ack_missing(self, session: _Session) -> None:
        with session.lock:
            if session.finalized or not session.finished_seen or session.finish_ack_sent:
                return
        process = session.process
        if process is None:
            return
        try:
            if process.poll() is None:
                self._force_process(session)
        except Exception:
            self._force_process(session)

    def _force_if_finished_process_alive(self, session: _Session) -> None:
        with session.lock:
            if session.finalized or not session.finished_seen or not session.finish_ack_sent:
                return
        process = session.process
        if process is None:
            return
        try:
            if process.poll() is None:
                session.forced = True
                self._force_process(session)
        except Exception:
            session.forced = True
            self._force_process(session)

    def _handle_protocol_failure(self, session: _Session, message: str) -> None:
        with self._condition:
            if self._session is not session or session.finalized:
                return
            self._replace_snapshot_locked(
                state=RunnerState.STOPPING,
                requested_by="watchdog",
                error={"code": "RUNNER_PROTOCOL_ERROR", "message": message},
            )
            session.stop_requested = True
            session.requested_by = session.requested_by or "watchdog"
            session.protocol_failed = True
        self._force_process(session)

    def _fail_start(self, session: _Session, error: SupervisorError) -> None:
        with self._condition:
            if self._session is not session or session.finalized:
                return
            session.stop_requested = True
            # Preserve a startup/IPC failure through the process-exit race;
            # otherwise ``_on_process_exit`` would classify the forced child as
            # an ordinary user stop merely because stop_requested is set.
            session.protocol_failed = True
            session.requested_by = session.requested_by or "watchdog"
            self._replace_snapshot_locked(
                state=RunnerState.STOPPING,
                error=error.as_dict(),
                outcome="failed",
            )
        self._force_process(session)

    def _force_process(self, session: _Session) -> None:
        process = session.process
        if process is None:
            outcome = "failed" if session.protocol_failed else ("stopped" if session.stop_requested else "failed")
            self._begin_restoring(session, outcome=outcome)
            return
        session.forced = True
        adapter = self.process_adapter
        try:
            if adapter is not None:
                adapter.terminate(process, grace=0.0)
            else:
                process.terminate()
        except Exception:
            try:
                if adapter is not None:
                    adapter.kill(process)
                else:
                    process.kill()
            except Exception:
                pass
        # Reaping is handled by _watch_process.  A fake process that does not wake
        # waiters still gets a bounded fallback finalization, while real process
        # adapters continue to drain their event pipe first.
        threading.Thread(target=self._wait_for_forced_exit, args=(session,), name=f"runner-kill-{session.spec.run_id}", daemon=True).start()

    def _wait_for_forced_exit(self, session: _Session) -> None:
        if session.process_exit.wait(self.kill_wait):
            return
        process = session.process
        if process is not None:
            try:
                if self.process_adapter is not None:
                    self.process_adapter.kill(process)
                else:
                    process.kill()
            except Exception:
                pass
        # Do not wait forever for a broken fake/native wrapper.  The process handle
        # is considered forced and the snapshot can still expose the failure.
        if not session.process_exit.wait(self.kill_wait):
            if session.protocol_failed:
                outcome = "failed"
            elif session.stop_requested:
                outcome = "stopped"
            else:
                outcome = "crashed"
            self._begin_restoring(session, outcome=outcome)

    def _on_process_exit(self, session: _Session) -> None:
        if session.protocol_failed:
            outcome = "failed"
        elif session.unresponsive:
            outcome = "crashed"
        elif session.finished_seen:
            outcome = self.snapshot.outcome or ("stopped" if session.stop_requested else "completed")
        elif session.stop_requested:
            outcome = "stopped"
        elif not session.ready_seen:
            # An unprompted process exit before ``ready`` is a startup/init
            # failure (for example, a missing task module), not a mid-run crash.
            outcome = "failed"
        elif session.reader_error is not None:
            outcome = "crashed"
        else:
            outcome = "crashed"
        self._begin_restoring(session, outcome=outcome)

    def _begin_restoring(self, session: _Session, *, outcome: str) -> None:
        with self._condition:
            if self._session is not session or session.finalized:
                return
            session.finalized = True
            current_error = self._snapshot.error
            if outcome == "crashed" and current_error is None:
                current_error = {"code": "RUNNER_CRASHED", "message": "Runner exited before finished"}
            elif outcome == "failed" and current_error is None and not session.ready_seen:
                current_error = {"code": "RUNNER_INIT_FAILED", "message": "Runner exited during startup"}
            stderr_tail = self._stderr_tail(session.process)
            if stderr_tail:
                current_error = dict(current_error or {"code": "RUNNER_CRASHED", "message": "Runner exited unexpectedly"})
                current_error.setdefault("stderrTail", stderr_tail)
            self._replace_snapshot_locked(
                state=RunnerState.RESTORING,
                device_lease=DeviceLeaseState.RESTORING,
                outcome=outcome,
                forced=self._snapshot.forced or session.forced,
                error=current_error,
            )
        # Exactly one thread executes cleanup/restore.  Exceptions are reflected in
        # deviceRestore but do not strand the sidecar in restoring forever.
        restore_error: dict[str, Any] | None = None
        try:
            if self.cleanup_callback is not None:
                self._call_callback(self.cleanup_callback, session, outcome)
        except Exception as exc:
            restore_error = {"code": "DEVICE_RESTORE_FAILED", "message": str(exc)}
        try:
            if self.restore_callback is not None:
                result = self._call_callback(self.restore_callback, session, outcome)
                if result in {"restored", "disconnected", "failed"}:
                    restore_state = str(result)
                elif isinstance(result, Mapping) and result.get("deviceRestore") in {"restored", "disconnected", "failed"}:
                    restore_state = str(result["deviceRestore"])
                else:
                    restore_state = "restored"
            else:
                restore_state = "restored"
        except Exception as exc:
            restore_state = "failed"
            restore_error = {"code": "DEVICE_RESTORE_FAILED", "message": str(exc)}
        finally:
            # Stop accepting new work first, then close the process streams to
            # unblock a writer that is currently in an OS write.  A second,
            # bounded join keeps normal shutdown leak-free without allowing a
            # hostile pipe to hold restoration forever.
            command_queue = session.command_queue
            if command_queue is not None:
                command_queue.close(timeout=0.0)
            process = session.process
            if process is not None:
                try:
                    process.close()
                except Exception:
                    pass
            if command_queue is not None:
                command_queue.close(timeout=max(0.1, min(self.kill_wait, 1.0)))
            with self._condition:
                if self._session is session:
                    self._replace_snapshot_locked(
                        state=RunnerState.IDLE,
                        current_task_id=None,
                        runner_pid=None,
                        device_lease=DeviceLeaseState.NONE,
                        outcome=outcome,
                        error=restore_error or self._snapshot.error,
                        device_restore=restore_state,
                    )
                    self._session = None

    @staticmethod
    def _stderr_tail(process: RunnerProcess | None) -> str | None:
        """Return a bounded diagnostic tail exposed by the pipe wrapper."""

        if process is None:
            return None
        try:
            value = getattr(process, "stderr_tail", b"")
            if callable(value):
                value = value()
            if not value:
                return None
            if isinstance(value, str):
                return value[-STDERR_TAIL_LIMIT:]
            if isinstance(value, (bytes, bytearray, memoryview)):
                return bytes(value)[-STDERR_TAIL_LIMIT:].decode("utf-8", errors="replace")
        except Exception:
            return None
        return None

    @staticmethod
    def _call_callback(callback: Callable[..., Any], session: _Session, outcome: str) -> Any:
        # Inspect before invocation.  Retrying after a TypeError raised by the
        # callback body can duplicate cleanup/restore side effects.
        candidates = (
            ((session.spec, outcome, session), {}),
            ((session.spec, outcome), {}),
            ((session.spec,), {}),
        )
        try:
            signature = inspect.signature(callback)
        except (TypeError, ValueError):
            return callback(session.spec, outcome, session)
        for args, kwargs in candidates:
            try:
                signature.bind(*args, **kwargs)
            except TypeError:
                continue
            return callback(*args, **kwargs)
        raise TypeError("supervisor callback has an unsupported signature")

    def request_pause(self, run_id: str) -> CommandResult:
        return self._pause_resume(run_id, paused=True)

    def pause(self, run_id: str) -> ExecutionSnapshot:
        result = self.request_pause(run_id)
        if not result.accepted:
            raise SupervisorError(result.error["code"], result.error["message"], snapshot=result.snapshot)
        return result.snapshot

    def request_resume(self, run_id: str) -> CommandResult:
        return self._pause_resume(run_id, paused=False)

    def resume(self, run_id: str) -> ExecutionSnapshot:
        result = self.request_resume(run_id)
        if not result.accepted:
            raise SupervisorError(result.error["code"], result.error["message"], snapshot=result.snapshot)
        return result.snapshot

    def _pause_resume(self, run_id: str, *, paused: bool) -> CommandResult:
        with self._condition:
            stale = self._check_run_locked(run_id)
            if stale is not None:
                return stale
            expected = RunnerState.RUNNING if paused else RunnerState.PAUSED
            if self._snapshot.state is not expected:
                error = {"code": "INVALID_EXECUTION_STATE", "message": f"{expected.value} is required"}
                return CommandResult(False, self._snapshot, error)
            session = self._session
            self._replace_snapshot_locked(state=RunnerState.PAUSED if paused else RunnerState.RUNNING)
        try:
            if session is None:
                raise FrameIOError("Runner session is unavailable")
            # WebSocket/RPC callers only perform queue admission.  The writer
            # owns the sequence and may be blocked by the child, while the
            # caller must remain responsive to stop and timeout requests.
            self._enqueue_command(session, "setPaused", paused=paused)
        except CommandQueueError as exc:
            self._on_command_queue_error(session, exc) if session is not None else None
            return CommandResult(False, self.snapshot, exc.as_dict())
        except Exception as exc:
            error = {"code": "RUNNER_SUPERVISION_FAILED", "message": str(exc)}
            with self._condition:
                self._replace_snapshot_locked(error=error)
            return CommandResult(False, self._snapshot, error)
        return CommandResult(True, self.snapshot)

    def request_stop(self, run_id: str, *, requested_by: str = "user") -> CommandResult:
        if requested_by not in {"user", "shutdown", "watchdog"}:
            return CommandResult(False, self.snapshot, {"code": "INVALID_REQUESTED_BY", "message": "invalid requestedBy"})
        with self._condition:
            stale = self._check_run_locked(run_id, allow_recent_idle=True)
            if stale is not None:
                return stale
            if self._snapshot.state is RunnerState.IDLE:
                return CommandResult(True, self._snapshot)
            if self._snapshot.state is RunnerState.STOPPING:
                return CommandResult(True, self._snapshot)
            session = self._session
            if session is not None:
                with session.lock:
                    session.stop_requested = True
                    session.requested_by = requested_by
            self._replace_snapshot_locked(state=RunnerState.STOPPING, requested_by=requested_by)
        # Command sending and timeout supervision are deliberately outside the
        # state lock.  A blocked pipe cannot block another stop/getState call.
        if session is None or session.process is None:
            self._begin_restoring(session, outcome="stopped") if session is not None else None
            return CommandResult(True, self.snapshot)
        # Before ``start`` is sent the bootstrap command grammar has no legal
        # stop command.  The launch thread will send attached/start and then call
        # ``_send_stop_if_ready``.  Once start is on the wire, enqueue stop now so
        # initialization can observe it without waiting for the ready timeout.
        # A full command pipe must not make the RPC handler wait.  The bounded
        # grace timer starts immediately; the dedicated daemon sender either
        # enqueues stop or is superseded by process-level termination.
        with session.lock:
            start_sent = session.start_sent
        if start_sent:
            threading.Thread(
                target=self._send_stop_if_ready,
                args=(session,),
                name=f"runner-stop-{session.spec.run_id}",
                daemon=True,
            ).start()
        timer = threading.Timer(self.stop_grace, self._force_if_still_running, args=(session,))
        timer.daemon = True
        timer.start()
        return CommandResult(True, self.snapshot)

    def stop(self, run_id: str, *, requested_by: str = "user") -> ExecutionSnapshot:
        result = self.request_stop(run_id, requested_by=requested_by)
        if not result.accepted:
            raise SupervisorError(result.error["code"], result.error["message"], snapshot=result.snapshot)
        return result.snapshot

    request_shutdown = request_stop

    def _force_if_still_running(self, session: _Session) -> None:
        with self._condition:
            if self._session is not session or session.finalized or self._snapshot.state is not RunnerState.STOPPING:
                return
        if session.process is None or session.process.poll() is not None:
            return
        self._force_process(session)

    def _check_run_locked(self, run_id: str, *, allow_recent_idle: bool = False) -> CommandResult | None:
        if not isinstance(run_id, str) or run_id != self._snapshot.run_id:
            return CommandResult(False, self._snapshot, {"code": "STALE_RUN", "message": "runId is stale"})
        if self._snapshot.state is RunnerState.IDLE and not allow_recent_idle:
            return CommandResult(False, self._snapshot, {"code": "INVALID_EXECUTION_STATE", "message": "execution is idle"})
        return None

    def wait_for_state(self, state: RunnerState | str, timeout: float = 5.0) -> ExecutionSnapshot:
        expected = RunnerState(state)
        deadline = time.monotonic() + max(0.0, float(timeout))
        with self._condition:
            while self._snapshot.state is not expected:
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    return self._snapshot
                self._condition.wait(remaining)
            return self._snapshot

    def wait_for_idle(self, timeout: float = 5.0) -> ExecutionSnapshot:
        return self.wait_for_state(RunnerState.IDLE, timeout)

    def close(self, timeout: float = 5.0) -> None:
        timeout = max(0.0, float(timeout))
        with self._condition:
            self._closed = True
            snapshot = self._snapshot
            run_id = snapshot.run_id
        if run_id is not None and snapshot.state is not RunnerState.IDLE:
            self.request_stop(run_id, requested_by="shutdown")
            self.wait_for_idle(timeout)
            # ``request_stop`` deliberately gives the child a grace period, but
            # supervisor.close itself must be bounded by its caller's timeout.
            # Force and close the queue/process handles when that period expires;
            # the normal finalizer remains responsible for the final snapshot.
            with self._condition:
                session = self._session
            if session is not None:
                self._force_process(session)
                command_queue = session.command_queue
                if command_queue is not None:
                    command_queue.close(timeout=0.0)
                process = session.process
                if process is not None:
                    try:
                        process.close()
                    except Exception:
                        pass
                if command_queue is not None:
                    command_queue.close(timeout=max(0.1, min(self.kill_wait, 1.0)))
                self.wait_for_idle(max(0.1, min(self.kill_wait * 2.0, 1.0)))


# Compatibility aliases make the intended actor discoverable without forcing the
# rest of the sidecar to depend on a particular class spelling.
ExecutionSupervisor = RunnerSupervisor
RunnerStateSnapshot = ExecutionSnapshot


__all__ = [
    "CommandResult",
    "DeviceLeaseState",
    "ExecutionSnapshot",
    "ExecutionSpec",
    "ExecutionSupervisor",
    "RunnerFactory",
    "RunnerHandshakeTimeout",
    "RunnerProcess",
    "RunnerProtocolFailure",
    "RunnerState",
    "RunnerStateSnapshot",
    "RunnerSupervisor",
    "SCHEMA_VERSION",
    "STDERR_TAIL_LIMIT",
    "SubprocessRunnerFactory",
    "SupervisorError",
]
