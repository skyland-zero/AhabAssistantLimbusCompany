"""One-shot task host used after bootstrap handshake.

This module is deliberately imported only after ``runner_bootstrap.py`` has
validated ``attached`` and ``start``.  It owns a fresh ExecutionControl, event
writer and command reader for exactly one run; no sidecar application singleton is
created here.
"""

from __future__ import annotations

import importlib
import inspect
import json
import logging
import os
import threading
import time
import uuid
from collections.abc import Callable, Mapping
from dataclasses import dataclass, field
from typing import Any

from .execution_control import ExecutionCancelled, ExecutionControl
from .ipc_protocol import Frame, FrameIOError, FrameWriter, ProtocolError, read_frame, validate_message
from .mediator_bridge import MediatorBridge
from .supervisor import ExecutionSpec

_MISSING = object()

# ``RunnerTaskHost`` normally lives in its own one-shot process, but keeping a
# direct environment snapshot makes the host safe to exercise in-process too.
# This also prevents a test or an embedding caller from inheriting the
# previous run's fail-closed Runner policy after host teardown.
_RUNNER_ENV_KEYS = (
    "AALC_CONFIG_PATH",
    "AALC_RESOURCE_ROOT",
    "AALC_RUN_ID",
    "AALC_RUNNER_MODE",
    "AALC_EXECUTION_RUNNER",
    "AALC_ALLOW_EMULATOR_LAUNCH",
    "AALC_RUNNER_POLICY",
)


class RunnerTaskError(RuntimeError):
    """Task initialization or execution failed inside the Runner."""


@dataclass(slots=True)
class RunnerTaskContext:
    """Explicit dependencies exposed to a task implementation."""

    spec: ExecutionSpec
    execution_control: ExecutionControl
    emit_event: Callable[..., int]
    config: Any = None
    mediator_bridge: Any = None
    runner_device_runtime: Any = None
    runner_owned_controller: Any = None
    resource_ledger_client: Any = None
    _host: "RunnerTaskHost | None" = field(default=None, repr=False, compare=False)

    @property
    def control(self) -> ExecutionControl:
        return self.execution_control

    @property
    def device_runtime(self) -> Any:
        """The private per-run device runtime, when one was requested."""

        return self.runner_device_runtime

    def checkpoint(self) -> None:
        self.execution_control.checkpoint()

    def interruptible_sleep(self, seconds: float) -> None:
        self.execution_control.interruptible_sleep(seconds)

    sleep = interruptible_sleep

    def event(self, message_type: str, **fields: Any) -> int:
        return self.emit_event(message_type, **fields)

    def resource_created(self, resource_type: str, resource_id: str, **metadata: Any) -> int:
        result = self.emit_event(
            "resource.created",
            resourceType=resource_type,
            resourceId=resource_id,
            metadata=metadata,
        )
        self._ledger_call("record_resource_created", resource_type, resource_id, metadata=metadata)
        return result

    def resource_released(self, resource_type: str, resource_id: str) -> int:
        result = self.emit_event(
            "resource.released",
            resourceType=resource_type,
            resourceId=resource_id,
        )
        self._ledger_call("record_resource_released", resource_type, resource_id)
        return result

    def config_delta(
        self,
        delta_id: str,
        *,
        base_revision: int,
        base_config_hash: str,
        changes: Mapping[str, Any] | None = None,
        operations: list[Mapping[str, Any]] | None = None,
    ) -> int:
        result = self.emit_event(
            "config.delta",
            deltaId=delta_id,
            baseRevision=base_revision,
            baseConfigHash=base_config_hash,
            changes=dict(changes or {}),
            operations=[dict(value) for value in (operations or [])],
        )
        self._ledger_call(
            "record_config_delta",
            {
                "type": "config.delta",
                "runId": self.spec.run_id,
                "deltaId": delta_id,
                "baseRevision": base_revision,
                "baseConfigHash": base_config_hash,
                "changes": dict(changes or {}),
                "operations": [dict(value) for value in (operations or [])],
            },
        )
        return result

    def defer_config_delta(
        self,
        delta_id: str,
        *,
        base_revision: int,
        base_config_hash: str,
        changes: Mapping[str, Any] | None = None,
        operations: list[Mapping[str, Any]] | None = None,
    ) -> None:
        """Queue a final config delta for the graceful completion sequence."""

        if self._host is None:
            self.config_delta(
                delta_id,
                base_revision=base_revision,
                base_config_hash=base_config_hash,
                changes=changes,
                operations=operations,
            )
            return
        self._host.queue_config_delta(
            delta_id,
            base_revision=base_revision,
            base_config_hash=base_config_hash,
            changes=changes,
            operations=operations,
        )

    queue_config_delta = defer_config_delta

    def defer_resource_released(self, resource_type: str, resource_id: str) -> None:
        """Queue a resource release for finalization (idempotence is sidecar-owned)."""

        if self._host is None:
            self.resource_released(resource_type, resource_id)
            return
        self._host.queue_resource_released(resource_type, resource_id)

    queue_resource_released = defer_resource_released

    def after_completion(self, action: Mapping[str, Any] | None = None, **fields: Any) -> None:
        """Queue typed sidecar-owned completion actions."""

        value: dict[str, Any] = dict(action or {})
        value.update(fields)
        if self._host is None:
            self.emit_event("afterCompletion.requested", **value)
            return
        self._host.queue_after_completion(value)

    queue_after_completion = after_completion

    def set_device_disposition(self, disposition: str) -> None:
        if self._host is None:
            return
        self._host.set_device_disposition(disposition)

    def _ledger_call(self, method: str, *args: Any, **kwargs: Any) -> None:
        client = self.resource_ledger_client
        callback = getattr(client, method, None) if client is not None else None
        if not callable(callback):
            return
        # The event remains the transport source of truth.  A local ledger client
        # is best-effort here; the sidecar can still journal the accepted frame.
        try:
            callback(*args, **kwargs)
        except Exception:
            return


class _IPCLogHandler(logging.Handler):
    """Forward structured logs without writing the Runner's stdout."""

    def __init__(self, emit: Callable[..., int]) -> None:
        super().__init__()
        self._emit = emit

    def emit(self, record: logging.LogRecord) -> None:
        try:
            self._emit(
                "log.entry",
                timestamp=time.time(),
                level=record.levelname.lower(),
                logger=record.name,
                message=self.format(record),
            )
        except Exception:
            # Logging must never recursively crash a task or block its stop path.
            return


class RunnerTaskHost:
    """Run one task with command-driven pause/stop and typed event output."""

    def __init__(
        self,
        spec: ExecutionSpec | Mapping[str, Any],
        *,
        event_stream: Any = None,
        command_stream: Any = None,
        execution_control: ExecutionControl | None = None,
        task_factory: Callable[..., Any] | None = None,
        controller_factory: Callable[..., Any] | None = None,
        config: Any = None,
        mediator_bridge: Any = None,
        device_runtime_factory: Callable[..., Any] | None = None,
        runtime_factory: Callable[..., Any] | None = None,
        runner_owned_controller: Any = None,
        resource_ledger_client: Any = None,
        event_sink: Callable[[str, Mapping[str, Any], bytes], Any] | None = None,
        initial_command_seq: int = 0,
        finalization_callback: Callable[..., Any] | None = None,
        after_completion_callback: Callable[..., Any] | None = None,
        device_cleanup_timeout: float = 20.0,
    ) -> None:
        self.spec = spec if isinstance(spec, ExecutionSpec) else ExecutionSpec.from_mapping(spec)
        self.control = execution_control or ExecutionControl()
        self.command_stream = command_stream
        self.event_stream = event_stream
        self.task_factory = task_factory
        self.controller_factory = controller_factory
        self.config = config
        self.mediator_bridge = mediator_bridge
        if device_runtime_factory is not None and runtime_factory is not None and device_runtime_factory is not runtime_factory:
            raise TypeError("device_runtime_factory and runtime_factory cannot both be set")
        self.device_runtime_factory = device_runtime_factory or runtime_factory
        self.runner_owned_controller = runner_owned_controller
        self.resource_ledger_client = resource_ledger_client
        self.event_sink = event_sink
        self.finalization_callback = finalization_callback or after_completion_callback
        try:
            self.device_cleanup_timeout = max(0.0, float(device_cleanup_timeout))
        except (TypeError, ValueError) as exc:
            raise ValueError("device_cleanup_timeout must be a non-negative number") from exc
        self._event_writer = FrameWriter(event_stream, run_id=self.spec.run_id) if event_stream is not None else None
        self._command_seq = max(0, int(initial_command_seq))
        self._command_lock = threading.RLock()
        self._started = False
        self._closed = threading.Event()
        self._finished = threading.Event()
        self._finish_ack = threading.Event()
        self._command_thread: threading.Thread | None = None
        self._heartbeat_thread: threading.Thread | None = None
        self._task_thread: threading.Thread | None = None
        self._task_result: Any = None
        self._task_error: BaseException | None = None
        self._finished_sent = False
        self._finishing = False
        self._finish_lock = threading.RLock()
        self._logger_handler: logging.Handler | None = None
        self._cancel_binding: Any = None
        self._cancel_binding_active = False
        self._bridge_owned = False
        self._bridge_active = False
        self._environment_snapshot: dict[str, str | None] | None = None
        self._environment_restored = False
        self._default_task_factory = False
        self.device_runtime: Any = None
        self._runtime_reservation: dict[str, Any] = dict(self.spec.cleanup_reservation)
        self._runtime_resources: list[dict[str, Any]] = []
        self._released_runtime_resources: set[tuple[str, str]] = set()
        self._runtime_closed = False
        self._runtime_close_error: BaseException | None = None
        self._pending_config_deltas: list[dict[str, Any]] = []
        self._pending_resource_releases: list[dict[str, Any]] = []
        self._pending_after_completion: list[dict[str, Any]] = []
        self._device_disposition = "restore"
        self._finalization_prepared = False
        self._config_delta_counter = 0
        self._config_delta_fingerprints: set[str] = set()
        self._emitted_delta_ids: set[str] = set()

    @property
    def finished(self) -> bool:
        return self._finished.is_set()

    @property
    def task_error(self) -> BaseException | None:
        return self._task_error

    def initialize(self) -> None:
        """Construct task/controller dependencies after start validation."""

        # This method is called only after bootstrap accepted ``start``.  Keep
        # environment setup first: importing module.config, core.events, a
        # controller or tasks before this point would bind process globals to the
        # sidecar's configuration by accident.
        self._configure_environment()
        self._validate_start_requirements()
        # Construct the private runtime before importing the config/task graph.
        # A production task without a concrete session must fail closed instead
        # of allowing legacy helpers to fall back to a process-global manager.
        self._ensure_device_runtime()
        self._require_production_session()
        self._ensure_mediator_bridge()
        self._install_logging_handler()
        self._bind_legacy_cancel_event()
        if self.config is None:
            self.config = self._load_config()
        if self.mediator_bridge is not None and hasattr(self.mediator_bridge, "config"):
            try:
                self.mediator_bridge.config = self.config
            except Exception:
                pass
        if self.controller_factory is not None and self.runner_owned_controller is None:
            self.runner_owned_controller = self._invoke_factory(self.controller_factory, self._make_context())
        if self.runner_owned_controller is not None:
            self._bind_task_control(self.runner_owned_controller)
        # Task factory resolution is deferred until this point so importing the
        # bootstrap never imports tasks or module configuration.
        if self.task_factory is None:
            self.task_factory = self._load_task_factory()

    @staticmethod
    def _absolute_path(value: Any, field: str) -> str | None:
        if value is None:
            return None
        if not isinstance(value, (str, os.PathLike)):
            raise RunnerTaskError(f"{field} must be a path")
        raw = os.fspath(value)
        if not raw:
            raise RunnerTaskError(f"{field} must be a non-empty path")
        path = os.path.abspath(os.path.expanduser(raw))
        if not os.path.isabs(path):  # pragma: no cover - abspath always returns abs
            raise RunnerTaskError(f"{field} must be absolute")
        return path

    def _configure_environment(self) -> None:
        if self._environment_snapshot is None:
            self._environment_snapshot = {name: os.environ.get(name) for name in _RUNNER_ENV_KEYS}
        config_path = self._absolute_path(self.spec.config_path, "configPath")
        resource_root = self._absolute_path(self.spec.resource_root, "resourceRoot")
        if config_path is not None:
            os.environ["AALC_CONFIG_PATH"] = config_path
        else:
            # Never inherit a sidecar/global config path when this run did not
            # receive an explicit temporary config snapshot.
            os.environ.pop("AALC_CONFIG_PATH", None)
        if resource_root is not None:
            os.environ["AALC_RESOURCE_ROOT"] = resource_root
        else:
            os.environ.pop("AALC_RESOURCE_ROOT", None)
        os.environ["AALC_RUN_ID"] = self.spec.run_id
        # Both names are intentionally set for every Runner.  The former is
        # consumed by task code while the latter is retained for compatibility
        # with older sidecar integrations; neither may be inherited from the
        # parent process.
        os.environ["AALC_RUNNER_MODE"] = "1"
        os.environ["AALC_EXECUTION_RUNNER"] = "1"
        allow_emulator_launch = bool(self.spec.allow_emulator_launch)
        os.environ["AALC_ALLOW_EMULATOR_LAUNCH"] = "1" if allow_emulator_launch else "0"
        try:
            policy = dict(self.spec.platform_policy)
        except (TypeError, ValueError):
            policy = {}
        # Keep the serialized policy and dedicated environment switch in lockstep
        # with the immutable ExecutionSpec snapshot.
        policy["runnerMode"] = True
        policy["allowEmulatorLaunch"] = allow_emulator_launch
        os.environ["AALC_RUNNER_POLICY"] = json.dumps(policy, ensure_ascii=False, separators=(",", ":"))

    def _restore_environment(self) -> None:
        """Restore variables changed by this host when it is embedded in-process."""

        if self._environment_restored or self._environment_snapshot is None:
            return
        for name, value in self._environment_snapshot.items():
            if value is None:
                os.environ.pop(name, None)
            else:
                os.environ[name] = value
        self._environment_restored = True

    def _is_default_production_task(self) -> bool:
        """Whether this host would lazily import the real task implementation."""

        return self.task_factory is None and not self._is_noop_mode()

    def _validate_start_requirements(self) -> None:
        """Reject unsafe implicit config/session fallbacks before business imports."""

        if self.spec.config_path is None and not self._is_noop_mode() and (
            self._is_default_production_task() or self.config is None
        ):
            raise RunnerTaskError("Runner production tasks require an explicit configPath")
        if self._is_default_production_task():
            target = self.spec.device_target
            if not isinstance(target, Mapping) or not target:
                raise RunnerTaskError("Runner production tasks require an explicit device session target")

    def _require_production_session(self) -> None:
        """Ensure the default real task has a private runtime session."""

        if not self._is_default_production_task():
            return
        runtime = self.device_runtime
        try:
            session = getattr(runtime, "session", None)
        except Exception as exc:
            raise RunnerTaskError(f"Runner device session is unavailable: {exc}") from exc
        if session is None:
            raise RunnerTaskError("Runner production task has no private device session")

    def _ensure_mediator_bridge(self) -> None:
        if self.mediator_bridge is None:
            self.mediator_bridge = MediatorBridge(
                self.emit_event,
                run_id=self.spec.run_id,
                config=self.config,
            )
            self._bridge_owned = True
        start = getattr(self.mediator_bridge, "start", None)
        if callable(start):
            start()
        # Even a custom bridge without a start() hook is an explicit event owner;
        # suppress host-generated duplicate task events in that case.
        self._bridge_active = True

    def _load_config(self) -> Any:
        # Explicit no-op/fake modes are useful for protocol smoke tests and avoid
        # importing the application's singleton.  Production's default task path
        # intentionally imports module.config only here, after environment setup.
        if self._is_noop_mode():
            return None
        if self.spec.config_path is None:
            raise RunnerTaskError("Runner production tasks require an explicit configPath")
        try:
            module = importlib.import_module("module.config")
            return getattr(module, "cfg")
        except (ImportError, AttributeError) as exc:
            raise RunnerTaskError(f"cannot load Runner config: {exc}") from exc

    def _runner_policy(self) -> Any:
        """Build the explicit, fail-closed policy for this Runner process."""

        try:
            module = importlib.import_module("module.automation.input_handlers.simulator.runner_policy")
            policy_type = getattr(module, "RunnerPolicy")
            return policy_type(
                runner_mode=True,
                allow_emulator_launch=self.spec.allow_emulator_launch,
            )
        except (ImportError, AttributeError, TypeError, ValueError):
            # ``create_runner_runtime`` also defaults to a fail-closed policy;
            # retaining ``None`` keeps minimal/no-op packaged runners usable.
            return None

    @staticmethod
    def _invoke_runtime_factory(
        factory: Callable[..., Any],
        target: Mapping[str, Any],
        run_id: str,
        reservation: Mapping[str, Any],
        policy: Any,
    ) -> Any:
        """Invoke injected and production factories without retrying effects."""

        try:
            signature = inspect.signature(factory)
        except (TypeError, ValueError):
            return factory(target, run_id, reservation, runner_policy=policy)
        candidates = (
            ((target, run_id, reservation), {"runner_policy": policy}),
            ((target, run_id), {"cleanup_reservation": reservation, "runner_policy": policy}),
            ((target, run_id, reservation), {}),
            ((target, run_id), {"cleanup_reservation": reservation}),
            ((target, run_id), {}),
        )
        for args, kwargs in candidates:
            try:
                signature.bind(*args, **kwargs)
            except TypeError:
                continue
            return factory(*args, **kwargs)
        raise RunnerTaskError("device runtime factory has an unsupported signature")

    @staticmethod
    def _reservation_alias(reservation: Mapping[str, Any], *names: str) -> Any:
        for name in names:
            if name in reservation and reservation[name] is not None:
                return reservation[name]
        return None

    def _ensure_device_runtime(self) -> None:
        """Create one private runtime from the serialized target snapshot."""

        target = self.spec.device_target
        if not isinstance(target, Mapping) or not target:
            return
        # An explicitly supplied controller is the legacy/test escape hatch.  A
        # caller that supplies a runtime factory still wins and gets a real
        # per-run runtime even if it also supplied a placeholder controller.
        if self.device_runtime_factory is None and self.runner_owned_controller is not None:
            return
        factory = self.device_runtime_factory
        runner_policy = None
        if factory is None:
            try:
                module = importlib.import_module("module.device_manager")
                factory = getattr(module, "create_runner_runtime")
            except (ImportError, AttributeError) as exc:
                raise RunnerTaskError(f"cannot load Runner device runtime: {exc}") from exc
            runner_policy = self._runner_policy()
        # An injected runtime factory is an explicit dependency boundary.  Do
        # not import the automation package merely to build a policy object for
        # a fake/test factory; the authoritative allow/deny value is already
        # present in the per-run environment and serialized policy.
        if not callable(factory):
            raise RunnerTaskError("device runtime factory is not callable")
        reservation = dict(self.spec.cleanup_reservation)
        runtime = self._invoke_runtime_factory(
            factory,
            dict(target),
            self.spec.run_id,
            reservation,
            runner_policy,
        )
        if runtime is None:
            raise RunnerTaskError("device runtime factory returned no runtime")
        self.device_runtime = runtime
        runtime_reservation = getattr(runtime, "reservation", None)
        if isinstance(runtime_reservation, Mapping):
            reservation.update(dict(runtime_reservation))
        self._runtime_reservation = reservation
        if self.runner_owned_controller is None:
            controller = getattr(runtime, "controller", None)
            if controller is None:
                session = getattr(runtime, "session", None)
                controller = getattr(session, "controller", None)
            self.runner_owned_controller = controller

    def _runtime_resource_specs(self) -> list[dict[str, Any]]:
        """Return conservative resource records derived from one reservation."""

        reservation = self._runtime_reservation
        serial = self._reservation_alias(reservation, "deviceSerial", "serial", "device_serial")
        scid = self._reservation_alias(reservation, "scid", "reservedScrcpyScid", "reserved_scid")
        socket_name = self._reservation_alias(
            reservation,
            "socketName",
            "reservedSocketName",
            "reserved_socket_name",
            "remoteSocketName",
        )
        port = self._reservation_alias(
            reservation,
            "adbForwardPort",
            "forwardPort",
            "reservedAdbForwardPort",
            "reserved_adb_forward_port",
        )
        records: list[dict[str, Any]] = []
        if scid is not None or socket_name is not None:
            resource_id = str(scid if scid is not None else socket_name)
            records.append(
                {
                    "resourceType": "scrcpy.server",
                    "resourceId": resource_id,
                    "metadata": {
                        key: value
                        for key, value in {
                            "scid": scid,
                            "socketName": socket_name,
                            "deviceSerial": serial,
                        }.items()
                        if value is not None
                    },
                }
            )
        if port is not None:
            records.append(
                {
                    "resourceType": "adb.forward",
                    "resourceId": str(port),
                    "metadata": {
                        key: value
                        for key, value in {"port": port, "deviceSerial": serial}.items()
                        if value is not None
                    },
                }
            )
        raw_resources = reservation.get("resources")
        if isinstance(raw_resources, Mapping):
            raw_resources = [raw_resources]
        if isinstance(raw_resources, (list, tuple)):
            for value in raw_resources:
                if not isinstance(value, Mapping):
                    continue
                resource_type = value.get("resourceType", value.get("resource_type"))
                resource_id = value.get("resourceId", value.get("resource_id"))
                if resource_type is None or resource_id is None:
                    continue
                records.append(
                    {
                        "resourceType": str(resource_type),
                        "resourceId": str(resource_id),
                        "metadata": dict(value.get("metadata", {})) if isinstance(value.get("metadata"), Mapping) else {},
                    }
                )
        unique: list[dict[str, Any]] = []
        seen: set[tuple[str, str]] = set()
        for record in records:
            key = (record["resourceType"], record["resourceId"])
            if key not in seen:
                seen.add(key)
                unique.append(record)
        return unique

    def _emit_runtime_resources(self) -> None:
        for record in self._runtime_resource_specs():
            key = (record["resourceType"], record["resourceId"])
            if key in {(item["resourceType"], item["resourceId"]) for item in self._runtime_resources}:
                continue
            self.emit_event(
                "resource.created",
                resourceType=record["resourceType"],
                resourceId=record["resourceId"],
                metadata=dict(record.get("metadata", {})),
            )
            self._ledger_call(
                "record_resource_created",
                record["resourceType"],
                record["resourceId"],
                metadata=dict(record.get("metadata", {})),
            )
            self._runtime_resources.append(record)

    @staticmethod
    def _invoke_runtime_close(close: Callable[..., Any], deadline: float, disposition: str) -> Any:
        try:
            signature = inspect.signature(close)
        except (TypeError, ValueError):
            return close(deadline=deadline, device_disposition=disposition)
        remaining = max(0.0, deadline - time.monotonic())
        candidates = (
            ((), {"deadline": deadline, "device_disposition": disposition}),
            ((), {"timeout": remaining, "device_disposition": disposition}),
            ((), {"device_disposition": disposition}),
            ((deadline,), {}),
            ((), {}),
        )
        for args, kwargs in candidates:
            try:
                signature.bind(*args, **kwargs)
            except TypeError:
                continue
            return close(*args, **kwargs)
        raise RunnerTaskError("device runtime close has an unsupported signature")

    def _close_device_runtime(self, *, emit_events: bool) -> bool:
        runtime = self.device_runtime
        if runtime is None or self._runtime_closed:
            return self._runtime_close_error is None
        close = getattr(runtime, "close", None) or getattr(runtime, "cleanup", None)
        if not callable(close):
            self._runtime_close_error = RunnerTaskError("device runtime has no close/cleanup method")
            return False
        deadline = time.monotonic() + self.device_cleanup_timeout
        try:
            result = self._invoke_runtime_close(close, deadline, self._device_disposition)
            if result is False:
                raise RunnerTaskError("device runtime cleanup returned failure")
            if isinstance(result, Mapping) and str(result.get("status", "")).lower() in {"failed", "error"}:
                raise RunnerTaskError(str(result.get("message", "device runtime cleanup failed")))
            self._runtime_closed = True
            runtime_reservation = getattr(runtime, "reservation", None)
            if isinstance(runtime_reservation, Mapping):
                self._runtime_reservation.update(dict(runtime_reservation))
            if emit_events:
                for record in self._runtime_resources:
                    key = (record["resourceType"], record["resourceId"])
                    if key in self._released_runtime_resources:
                        continue
                    self.emit_event(
                        "resource.released",
                        resourceType=record["resourceType"],
                        resourceId=record["resourceId"],
                    )
                    self._ledger_call("record_resource_released", record["resourceType"], record["resourceId"])
                    self._released_runtime_resources.add(key)
            return True
        except BaseException as exc:
            self._runtime_close_error = exc
            return False

    def _load_task_factory(self) -> Callable[..., Any]:
        extra = dict(self.spec.extra)
        module_name = extra.get("taskModule") or extra.get("task_module")
        attribute = extra.get("taskCallable") or extra.get("task_callable")
        if module_name:
            if attribute is None:
                attribute = "my_script_task" if str(module_name) == "tasks.base.script_task_scheme" else "run"
            module = importlib.import_module(str(module_name))
            factory = getattr(module, str(attribute), None)
            if not callable(factory):
                raise RunnerTaskError(f"task callable {module_name}.{attribute} is not callable")
            return factory
        if self._is_noop_mode():
            return lambda _context: None
        # The real task is imported lazily and only after config/environment setup.
        # This module must not be imported by BackendApplication or WebSocket code.
        module = importlib.import_module("tasks.base.script_task_scheme")
        factory = getattr(module, "my_script_task", None)
        if not callable(factory):
            raise RunnerTaskError("tasks.base.script_task_scheme.my_script_task is not callable")
        self._default_task_factory = True
        return factory

    def _is_noop_mode(self) -> bool:
        extra = dict(self.spec.extra)
        mode = extra.get("runnerMode", extra.get("runner_mode", ""))
        return bool(extra.get("noOp", extra.get("no_op", False))) or str(mode).lower() in {
            "noop",
            "fake",
            "test",
        }

    def _make_context(self) -> RunnerTaskContext:
        return RunnerTaskContext(
            spec=self.spec,
            execution_control=self.control,
            emit_event=self.emit_event,
            config=self.config,
            mediator_bridge=self.mediator_bridge,
            runner_device_runtime=self.device_runtime,
            runner_owned_controller=self.runner_owned_controller,
            resource_ledger_client=self.resource_ledger_client,
            _host=self,
        )

    def _bind_legacy_cancel_event(self) -> None:
        """Bind legacy task helpers to this run's one cancellation Event."""

        try:
            bridge = importlib.import_module("core.execution_control")
            current = getattr(bridge, "current_cancel_event", None)
            binder = getattr(bridge, "bind_cancel_event", None)
            if not callable(binder):
                return
            self._cancel_binding = current() if callable(current) else None
            binder(self.control.cancel_event)
            self._cancel_binding_active = True
        except (ImportError, AttributeError):
            # Minimal packaged/test builds may omit the legacy bridge.  The new
            # ExecutionControl remains authoritative for injected task factories.
            return

    def _restore_legacy_cancel_event(self) -> None:
        if not self._cancel_binding_active:
            return
        try:
            bridge = importlib.import_module("core.execution_control")
            binder = getattr(bridge, "bind_cancel_event", None)
            if callable(binder):
                binder(self._cancel_binding)
        except (ImportError, AttributeError):
            pass
        finally:
            self._cancel_binding_active = False
            self._cancel_binding = None

    def _install_logging_handler(self) -> None:
        if self._logger_handler is not None:
            return
        handler = _IPCLogHandler(self.emit_event)
        handler.setFormatter(logging.Formatter("%(message)s"))
        logging.getLogger().addHandler(handler)
        self._logger_handler = handler

    def _remove_logging_handler(self) -> None:
        handler = self._logger_handler
        self._logger_handler = None
        if handler is None:
            return
        try:
            logging.getLogger().removeHandler(handler)
        finally:
            handler.close()

    def _bind_task_control(self, task: Any) -> Any:
        """Bind a legacy task's cancel event/control to this Runner instance."""

        if task is None:
            return task
        if hasattr(task, "cancel_event"):
            try:
                setattr(task, "cancel_event", self.control.cancel_event)
            except Exception:
                pass
        return self.control.bind(task)

    @staticmethod
    def _invoke_factory(factory: Callable[..., Any], context: RunnerTaskContext) -> Any:
        # Prefer explicit dependency injection.  If a legacy zero-argument factory
        # is supplied, inspect its signature before invocation to avoid retrying a
        # side-effectful factory after a TypeError from its body.
        try:
            signature = inspect.signature(factory)
        except (TypeError, ValueError):
            return factory(context)
        try:
            signature.bind(context)
        except TypeError:
            return factory()
        return factory(context)

    def queue_config_delta(
        self,
        delta_id: str,
        *,
        base_revision: int,
        base_config_hash: str,
        changes: Mapping[str, Any] | None = None,
        operations: list[Mapping[str, Any]] | None = None,
    ) -> None:
        value = {
            "deltaId": str(delta_id),
            "baseRevision": int(base_revision),
            "baseConfigHash": str(base_config_hash),
            "changes": dict(changes or {}),
            "operations": [dict(item) for item in (operations or [])],
        }
        if not any(item.get("deltaId") == value["deltaId"] for item in self._pending_config_deltas):
            self._pending_config_deltas.append(value)

    def queue_resource_released(self, resource_type: str, resource_id: str) -> None:
        value = {"resourceType": str(resource_type), "resourceId": str(resource_id)}
        if not any(
            item.get("resourceType") == value["resourceType"] and item.get("resourceId") == value["resourceId"]
            for item in self._pending_resource_releases
        ):
            self._pending_resource_releases.append(value)

    def queue_after_completion(self, action: Mapping[str, Any] | None = None, **fields: Any) -> None:
        value = dict(action or {})
        value.update(fields)
        if value not in self._pending_after_completion:
            self._pending_after_completion.append(value)

    def set_device_disposition(self, disposition: str) -> None:
        if disposition not in {"restore", "game_closed", "emulator_closed"}:
            raise ValueError("invalid device disposition")
        self._device_disposition = disposition

    def emit_event(self, message_type: str, payload: bytes = b"", **fields: Any) -> int:
        with self._finish_lock:
            if self._finished_sent:
                raise FrameIOError("cannot emit events after finished")
            if self._finishing and message_type not in {
                "config.delta",
                "resource.released",
                "afterCompletion.requested",
                "finished",
            }:
                raise FrameIOError("cannot emit non-final event while finishing")
            if self._event_writer is not None:
                return self._event_writer.send_event(message_type, payload=payload, **fields)
            if self.event_sink is not None:
                result = self.event_sink(message_type, dict(fields), bytes(payload))
                if result is False:
                    raise FrameIOError("event sink rejected message")
                return int(result) if isinstance(result, int) else 0
            return 0

    def start(self, *, ready_metadata: Mapping[str, Any] | None = None) -> None:
        if self._started:
            raise RunnerTaskError("RunnerTaskHost.start called twice")
        try:
            self.initialize()
        except BaseException:
            self._close_device_runtime(emit_events=False)
            self._teardown_runtime()
            raise
        self._started = True
        self.emit_event("ready", device=dict(ready_metadata or {}))
        self.emit_event("status", status="running")
        # The resource-created records follow ready/running, so a consumer can
        # safely associate them with an initialized run.  They are emitted
        # before task.started and are never inferred after the task has ended.
        try:
            self._emit_runtime_resources()
        except BaseException:
            self._close_device_runtime(emit_events=False)
            self._teardown_runtime()
            raise
        self._command_thread = threading.Thread(target=self._command_loop, name=f"runner-command-{self.spec.run_id}", daemon=True)
        self._heartbeat_thread = threading.Thread(target=self._heartbeat_loop, name=f"runner-heartbeat-{self.spec.run_id}", daemon=True)
        self._task_thread = threading.Thread(target=self._task_loop, name=f"runner-task-{self.spec.run_id}", daemon=True)
        self._command_thread.start()
        self._heartbeat_thread.start()
        self._task_thread.start()

    def run(self, *, ready_metadata: Mapping[str, Any] | None = None) -> None:
        self.start(ready_metadata=ready_metadata)
        task_thread = self._task_thread
        if task_thread is not None:
            task_thread.join()
        self._finished.wait(1.1)
        self.close()

    def _command_loop(self) -> None:
        if self.command_stream is None:
            # Event-sink hosts are often driven directly in tests; no pipe reader is
            # needed there.
            return
        try:
            while not self._closed.is_set() and not self._finished.is_set():
                frame = read_frame(self.command_stream)
                if frame is None:
                    self.control.request_stop()
                    return
                if not isinstance(frame, Frame):
                    raise ProtocolError("command stream returned invalid frame")
                message = validate_message(frame.header, direction="command", expected_run_id=self.spec.run_id)
                command_seq = int(message["commandSeq"])
                with self._command_lock:
                    if command_seq <= self._command_seq:
                        continue
                    self._command_seq = command_seq
                self._handle_command(message)
                # No further command is meaningful after the sidecar has
                # acknowledged ``finished``.  Returning here avoids racing the
                # task thread into a second blocking read while ``run()`` is
                # closing the inherited command pipe.
                if message["type"] == "finishAck":
                    return
        except (ProtocolError, FrameIOError, OSError) as exc:
            self._task_error = exc
            self.control.request_stop()

    def _handle_command(self, message: Mapping[str, Any]) -> None:
        message_type = message["type"]
        if message_type == "setPaused":
            self.control.set_paused(bool(message["paused"]))
            self.emit_event("status", status="paused" if self.control.paused else "running")
        elif message_type == "stop":
            self.control.request_stop()
            # status=stopping is a report, not a replacement for finished.
            self.emit_event("status", status="stopping")
        elif message_type == "finishAck":
            self._finish_ack.set()
        elif message_type == "shutdown":
            self.control.request_stop()
        elif message_type in {"attached", "start"}:
            # Duplicate handshake commands are harmless after bootstrap consumed
            # the originals; commandSeq still makes them idempotent.
            return

    def _heartbeat_loop(self) -> None:
        while not self._closed.wait(1.0):
            if self._finished_sent:
                return
            try:
                self.emit_event("heartbeat", monotonic=time.monotonic())
            except (FrameIOError, OSError):
                self.control.request_stop()
                return

    def _task_loop(self) -> None:
        outcome = "completed"
        error: dict[str, Any] | None = None
        task_object: Any = None
        try:
            if not self._default_task_factory:
                self.emit_event("task.started", taskId=self.spec.task_id, timestamp=time.time())
            context = self._make_context()
            result = self._invoke_factory(self.task_factory or (lambda _ctx: None), context)
            task_object = self._bind_task_control(result)
            if hasattr(task_object, "run") and callable(task_object.run):
                result = self._invoke_factory(task_object.run, context)
                task_exception = getattr(task_object, "exception", None)
                if task_exception is not None:
                    if isinstance(task_exception, ExecutionCancelled) or type(task_exception).__name__ == "userStopError":
                        raise ExecutionCancelled(str(task_exception))
                    raise RunnerTaskError(str(task_exception)) from task_exception
            self.control.checkpoint()
            self._task_result = result
            self._capture_after_completion_request(task_object, result)
            if not self._default_task_factory:
                self.emit_event("task.completed", taskId=self.spec.task_id, result=result)
            self._collect_config_delta("task.completed")
            self._prepare_finalization(outcome)
        except ExecutionCancelled as exc:
            outcome = "stopped"
            error = {"code": "EXECUTION_STOP_REQUESTED", "message": str(exc)}
        except BaseException as exc:  # task exceptions are reported, not leaked
            self._task_error = exc
            outcome = "failed"
            error = {
                "code": "RUNNER_INIT_FAILED" if not self._started else "TASK_FAILED",
                "message": str(exc),
                "phase": "task",
            }
        finally:
            if not self._close_device_runtime(emit_events=True):
                cleanup_error = {
                    "code": "RUNNER_DEVICE_CLEANUP_FAILED",
                    "message": str(self._runtime_close_error or "device runtime cleanup failed"),
                    "phase": "cleanup",
                }
                if outcome == "completed":
                    outcome = "failed"
                    error = cleanup_error
                elif error is None:
                    error = cleanup_error
                else:
                    error = {**error, "cleanupError": cleanup_error}
            self._send_finished(outcome, error)

    def _capture_after_completion_request(self, task_object: Any, result: Any) -> None:
        """Capture the task's sidecar-owned completion handoff exactly once."""

        request: Any = None
        getter = getattr(task_object, "get_after_completion_request", None)
        if callable(getter):
            request = getter()
        if request is None and isinstance(task_object, Mapping):
            request = task_object.get("afterCompletionRequest", task_object.get("after_completion_request"))
        if request is None and isinstance(result, Mapping):
            request = result.get("afterCompletionRequest", result.get("after_completion_request"))
        if isinstance(request, Mapping):
            value = dict(request)
            disposition = value.get("deviceDisposition", value.get("device_disposition"))
            if disposition is not None:
                self.set_device_disposition(str(disposition))
            self.queue_after_completion(value)
        disposition = getattr(task_object, "device_disposition", None)
        if disposition is not None and request is not None:
            self.set_device_disposition(str(disposition))

    @staticmethod
    def _same_config_value(left: Any, right: Any) -> bool:
        try:
            return json.dumps(left, ensure_ascii=False, sort_keys=True, allow_nan=False) == json.dumps(
                right,
                ensure_ascii=False,
                sort_keys=True,
                allow_nan=False,
            )
        except (TypeError, ValueError):
            return left == right

    def _config_snapshot(self) -> Mapping[str, Any] | None:
        config = self.config
        if config is None:
            return None
        snapshot = getattr(config, "runner_snapshot", None)
        if callable(snapshot):
            value = snapshot()
            return value if isinstance(value, Mapping) else None
        snapshot = getattr(config, "snapshot", None)
        if callable(snapshot):
            try:
                value = snapshot()
            except TypeError:
                value = snapshot
            if isinstance(value, Mapping):
                return value
        if isinstance(config, Mapping):
            return config
        value = getattr(config, "config", None)
        if isinstance(value, Mapping):
            return value
        model_dump = getattr(value, "model_dump", None)
        if callable(model_dump):
            dumped = model_dump(mode="python")
            return dumped if isinstance(dumped, Mapping) else None
        model_dump = getattr(config, "model_dump", None)
        if callable(model_dump):
            dumped = model_dump(mode="python")
            return dumped if isinstance(dumped, Mapping) else None
        try:
            attributes = vars(config)
        except TypeError:
            return None
        return attributes if isinstance(attributes, Mapping) else None

    def _config_value(self, key: str, snapshot: Mapping[str, Any] | None) -> Any:
        if snapshot is not None and key in snapshot:
            return snapshot[key]
        getter = getattr(self.config, "get_value", None) if self.config is not None else None
        if callable(getter):
            try:
                return getter(key)
            except (KeyError, TypeError, AttributeError):
                return _MISSING
        return _MISSING

    def _config_baseline(self) -> Mapping[str, Any] | None:
        extra = dict(self.spec.extra)
        value = extra.get("baselineValues", extra.get("baseline_values"))
        if value is None:
            runtime = self.spec.runtime_config
            if isinstance(runtime, Mapping):
                value = runtime.get("baselineValues", runtime.get("baseline_values"))
        return value if isinstance(value, Mapping) else None

    def _config_base_hash(self) -> str:
        extra = dict(self.spec.extra)
        value = extra.get("baseConfigHash", extra.get("base_config_hash"))
        if value is None and isinstance(self.spec.runtime_config, Mapping):
            value = self.spec.runtime_config.get("baseConfigHash", self.spec.runtime_config.get("base_config_hash"))
        return str(value or "")

    def _config_base_revision(self) -> int:
        extra = dict(self.spec.extra)
        value = extra.get("baseRevision", extra.get("base_revision", self.spec.config_revision))
        if isinstance(value, bool) or not isinstance(value, int) or value < 0:
            return self.spec.config_revision
        return value

    def _rotate_team_operation(
        self,
        baseline: Mapping[str, Any],
        snapshot: Mapping[str, Any] | None,
    ) -> list[dict[str, Any]]:
        if snapshot is None or baseline.get("teams_active_queue", _MISSING) == _MISSING:
            return []
        current_queue = snapshot.get("teams_active_queue", _MISSING)
        if current_queue is _MISSING or self._same_config_value(current_queue, baseline.get("teams_active_queue")):
            return []
        extra = dict(self.spec.extra)
        completed_id = extra.get("completedTeamId", extra.get("completed_team_id"))
        if completed_id is None:
            completed_id = snapshot.get("completedTeamId", snapshot.get("completed_team_id"))
        if isinstance(completed_id, bool) or not isinstance(completed_id, (str, int)) or not str(completed_id):
            # A changed queue without a stable completed team id cannot be
            # reconstructed safely; never send a raw replacement for the queue.
            return []
        return [{"op": "rotateTeamQueue", "completedTeamId": completed_id}]

    def _collect_config_delta(self, checkpoint: str) -> None:
        """Emit only baseline-relative, whitelisted configuration changes."""

        baseline = self._config_baseline()
        snapshot = self._config_snapshot()
        if not baseline or snapshot is None:
            return
        changes: dict[str, Any] = {}
        for key in ("last_auto_change", "hard_mirror", "hard_mirror_chance", "set_win_size"):
            if key not in baseline:
                continue
            current = self._config_value(key, snapshot)
            if current is not _MISSING and not self._same_config_value(current, baseline[key]):
                changes[key] = current
        operations = self._rotate_team_operation(baseline, snapshot)
        if not changes and not operations:
            return
        fingerprint = json.dumps(
            {"changes": changes, "operations": operations},
            ensure_ascii=False,
            sort_keys=True,
            allow_nan=False,
            default=repr,
        )
        if fingerprint in self._config_delta_fingerprints:
            return
        self._config_delta_fingerprints.add(fingerprint)
        self._config_delta_counter += 1
        delta_id = f"{self.spec.run_id}:config:{self._config_delta_counter}:{uuid.uuid4().hex}"
        fields = {
            "deltaId": delta_id,
            "baseRevision": self._config_base_revision(),
            "baseConfigHash": self._config_base_hash(),
            "changes": changes,
            "operations": operations,
            "checkpoint": checkpoint,
        }
        sequence = self.emit_event("config.delta", **fields)
        self._emitted_delta_ids.add(delta_id)
        self._ledger_call(
            "record_config_delta",
            {"type": "config.delta", "runId": self.spec.run_id, "seq": sequence, **fields},
        )

    def _prepare_finalization(self, outcome: str) -> None:
        """Run completion handoff before closing the leased device runtime."""

        if self._finalization_prepared or outcome != "completed":
            return
        callback = self.finalization_callback
        if callback is not None:
            result = self._invoke_completion_callback(callback, self._make_context(), outcome)
            self._collect_finalization_result(result)
        self._finalization_prepared = True

    def _send_finished(self, outcome: str, error: Mapping[str, Any] | None) -> None:
        with self._finish_lock:
            if self._finished_sent or self._finishing:
                return
            self._finishing = True
            try:
                # Final sidecar-owned actions are deliberately emitted before
                # ``finished``.  The event pipe is FIFO, so the supervisor can
                # journal each accepted item and acknowledge the final frame.
                self._emit_finalization_events(outcome)
                fields: dict[str, Any] = {
                    "outcome": outcome,
                    "forced": False,
                    "deviceDisposition": self._device_disposition,
                }
                if error is not None:
                    fields["error"] = dict(error)
                self.emit_event("finished", **fields)
                self._finished_sent = True
            except Exception as exc:
                self._task_error = self._task_error or exc
                self._finished_sent = True
                self._finished.set()
                return
            finally:
                self._finishing = False
        self._finish_ack.wait(1.0)
        self._finished.set()

    def _emit_finalization_events(self, outcome: str) -> None:
        if not self._finalization_prepared:
            self._prepare_finalization(outcome)

        if outcome == "completed":
            self._collect_config_delta("finished")

        for delta in self._pending_config_deltas:
            if str(delta.get("deltaId")) in self._emitted_delta_ids:
                continue
            self.emit_event("config.delta", **dict(delta))
            self._emitted_delta_ids.add(str(delta.get("deltaId")))
            self._ledger_call("record_config_delta", {"type": "config.delta", "runId": self.spec.run_id, **delta})
        for release in self._pending_resource_releases:
            self.emit_event("resource.released", **dict(release))
            self._ledger_call(
                "record_resource_released",
                release["resourceType"],
                release["resourceId"],
            )
        if outcome == "completed":
            for action in self._pending_after_completion:
                self.emit_event("afterCompletion.requested", **dict(action))

    @staticmethod
    def _invoke_completion_callback(
        callback: Callable[..., Any], context: RunnerTaskContext, outcome: str
    ) -> Any:
        try:
            signature = inspect.signature(callback)
        except (TypeError, ValueError):
            return callback(context, outcome)
        for args in ((context, outcome), (context,), ()):
            try:
                signature.bind(*args)
            except TypeError:
                continue
            return callback(*args)
        raise RunnerTaskError("finalization callback has an unsupported signature")

    def _collect_finalization_result(self, result: Any) -> None:
        if result is None:
            return
        if not isinstance(result, Mapping):
            raise RunnerTaskError("finalization callback must return a mapping")
        for value in result.get("configDeltas", result.get("config_deltas", [])) or []:
            if not isinstance(value, Mapping):
                raise RunnerTaskError("finalization config delta must be a mapping")
            delta = dict(value)
            delta_id = delta.pop("deltaId", delta.pop("delta_id", None))
            if delta_id is None:
                raise RunnerTaskError("finalization config delta requires deltaId")
            self.queue_config_delta(
                str(delta_id),
                base_revision=delta.pop("baseRevision", delta.pop("base_revision", 0)),
                base_config_hash=delta.pop("baseConfigHash", delta.pop("base_config_hash", "")),
                changes=delta.pop("changes", {}),
                operations=delta.pop("operations", []),
            )
        for value in result.get("resourceReleases", result.get("resource_releases", [])) or []:
            if not isinstance(value, Mapping):
                raise RunnerTaskError("finalization resource release must be a mapping")
            resource_type = value.get("resourceType", value.get("resource_type"))
            resource_id = value.get("resourceId", value.get("resource_id"))
            if resource_type is None or resource_id is None:
                raise RunnerTaskError("finalization resource release requires type and id")
            self.queue_resource_released(str(resource_type), str(resource_id))
        actions = result.get("afterCompletion", result.get("after_completion", []))
        if isinstance(actions, Mapping):
            actions = [actions]
        for value in actions or []:
            if not isinstance(value, Mapping):
                raise RunnerTaskError("finalization action must be a mapping")
            self.queue_after_completion(value)
        disposition = result.get("deviceDisposition", result.get("device_disposition"))
        if disposition is not None:
            self.set_device_disposition(str(disposition))

    def _ledger_call(self, method: str, *args: Any, **kwargs: Any) -> None:
        callback = getattr(self.resource_ledger_client, method, None) if self.resource_ledger_client is not None else None
        if not callable(callback):
            return
        try:
            callback(*args, **kwargs)
        except Exception:
            return

    def close(self) -> None:
        self._closed.set()
        self.control.request_stop() if not self._finished_sent else None
        if self._event_writer is not None:
            self._event_writer.close()
        # Close both protocol endpoints.  In particular, the command reader can
        # be blocked in ``read_frame`` after finishAck; leaving that descriptor
        # open keeps a native pipe handle alive until interpreter teardown and
        # makes direct host tests/frozen shutdown unnecessarily nondeterministic.
        for stream in (self.command_stream, self.event_stream):
            try:
                if stream is not None:
                    stream.close()
            except (OSError, ValueError):
                pass
        self._teardown_runtime()

    def _teardown_runtime(self) -> None:
        self._remove_logging_handler()
        bridge = self.mediator_bridge
        if bridge is not None:
            close = getattr(bridge, "close", None)
            if callable(close) and (self._bridge_owned or self._bridge_active):
                try:
                    close()
                except Exception:
                    pass
        self._bridge_active = False
        self._restore_legacy_cancel_event()
        self._restore_environment()


__all__ = ["RunnerTaskContext", "RunnerTaskError", "RunnerTaskHost"]
