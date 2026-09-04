"""Bridge the framework-free task mediator to Runner IPC events.

The bridge is intentionally small and owns no application state.  A Runner
creates one instance after the bootstrap ``start`` command has been accepted,
connects it to that run's mediator, and disconnects it before the process exits.
Importing this module is safe before the business packages exist: ``core.events``
is resolved lazily by :meth:`start`.

The legacy ``hdr_warning`` signal carries a :class:`threading.Event` because the
old in-process sidecar acknowledged it synchronously.  Events cannot cross the
Runner pipe, so this bridge sends a typed message and sets the supplied event as
soon as the event writer accepts it.  A failed writer leaves the event unset and
lets the caller decide whether to abort the run.
"""

from __future__ import annotations

import importlib
import inspect
import threading
import time
from collections.abc import Callable, Mapping
from typing import Any

EmitEvent = Callable[..., Any]


class MediatorBridge:
    """Map legacy mediator signals to typed, per-run Runner events.

    ``emit_event`` is normally :meth:`RunnerTaskHost.emit_event`, but accepting a
    generic callable keeps the adapter independently testable.  The callback is
    considered successful unless it raises or explicitly returns ``False``;
    ``None`` and sequence number ``0`` are valid for sink-based test hosts.
    """

    EVENT_NAMES = {
        "task_started": "task.started",
        "task_completed": "task.completed",
        "mirror_signal": "mirror.progress",
        "mirror_floor_signal": "mirror.floor",
        "warning": "warning",
        "hdr_warning": "hdr.warning",
        "request_focus": "app.focusRequested",
    }

    def __init__(
        self,
        emit_event: EmitEvent,
        *,
        mediator: Any = None,
        run_id: str | None = None,
        config: Any = None,
    ) -> None:
        if not callable(emit_event):
            raise TypeError("emit_event must be callable")
        self.emit_event = emit_event
        self.mediator = mediator
        self.run_id = run_id
        self.config = config
        self._bindings: list[tuple[Any, Callable[..., Any]]] = []
        self._started = False
        self._lock = threading.RLock()

    @property
    def started(self) -> bool:
        with self._lock:
            return self._started

    def start(self) -> "MediatorBridge":
        """Resolve and connect the mediator exactly once.

        Missing optional signals are ignored so a small test mediator or a
        packaged build with a reduced event surface remains usable.  A real
        connection failure is not swallowed: initialization must not claim a
        bridge exists when it cannot deliver events.
        """

        with self._lock:
            if self._started:
                return self
            mediator = self.mediator
            if mediator is None:
                mediator = importlib.import_module("core.events").mediator
                self.mediator = mediator
            for attribute, handler in (
                ("task_started", self.on_task_started),
                ("task_completed", self.on_task_completed),
                ("mirror_signal", self.on_mirror_signal),
                ("mirror_floor_signal", self.on_mirror_floor),
                ("warning", self.on_warning),
                ("hdr_warning", self.on_hdr_warning),
                ("request_focus", self.on_focus_requested),
            ):
                signal = getattr(mediator, attribute, None)
                connect = getattr(signal, "connect", None)
                disconnect = getattr(signal, "disconnect", None)
                if not callable(connect) or not callable(disconnect):
                    continue
                connect(handler)
                self._bindings.append((signal, handler))
            self._started = True
            return self

    def close(self) -> None:
        """Disconnect all handlers; repeated calls are harmless."""

        with self._lock:
            bindings, self._bindings = self._bindings, []
            self._started = False
        for signal, handler in bindings:
            try:
                signal.disconnect(handler)
            except Exception:
                # Teardown should not mask the task's result.  EventBus itself
                # already treats disconnect as best-effort.
                continue

    disconnect = close

    def _enqueue(self, message_type: str, **fields: Any) -> bool:
        if self.run_id is not None:
            fields.setdefault("runId", self.run_id)
        try:
            result = self._call_emit(message_type, fields)
        except Exception:
            return False
        return result is not False

    def _call_emit(self, message_type: str, fields: Mapping[str, Any]) -> Any:
        """Support both ``host.emit_event(type, **fields)`` and sink callbacks."""

        try:
            signature = inspect.signature(self.emit_event)
        except (TypeError, ValueError):
            return self.emit_event(message_type, **dict(fields))
        try:
            signature.bind(message_type, **dict(fields))
        except TypeError:
            try:
                signature.bind(message_type, dict(fields), b"")
            except TypeError:
                return self.emit_event({"type": message_type, **dict(fields)})
            return self.emit_event(message_type, dict(fields), b"")
        return self.emit_event(message_type, **dict(fields))

    @staticmethod
    def _int(value: Any, *, default: int = 0) -> int:
        try:
            # bool is technically an int, but progress counters should not become
            # surprising 0/1 values when a malformed legacy signal is emitted.
            if isinstance(value, bool):
                raise ValueError
            return max(0, int(value))
        except (TypeError, ValueError):
            return default

    def on_task_started(self, task_id: Any) -> bool:
        return self._enqueue(
            self.EVENT_NAMES["task_started"],
            taskId=str(task_id),
            timestamp=time.time(),
        )

    def on_task_completed(self, kind: Any, count: Any = 1, details: Any = None) -> bool:
        task_kind = str(kind)
        amount = self._int(count, default=1)
        result: dict[str, Any] = {"count": amount}
        if details is not None:
            result["details"] = details
        # ``kind/count/details`` remain top-level compatibility fields for the
        # sidecar's existing statistics callback; ``result`` is the protocol's
        # canonical task result.
        return self._enqueue(
            self.EVENT_NAMES["task_completed"],
            taskId=task_kind,
            result=result,
            kind=task_kind,
            count=amount,
            details=details,
        )

    def on_mirror_signal(self, current: Any, total: Any) -> bool:
        completed = self._int(current)
        total_value = self._int(total)
        config = self.config
        is_hard = bool(self._config_value(config, "hard_mirror", False))
        is_infinite = bool(self._config_value(config, "infinite_dungeons", False))
        return self._enqueue(
            self.EVENT_NAMES["mirror_signal"],
            completed=completed,
            current=completed,
            total=total_value,
            isHard=is_hard,
            isInfinite=is_infinite,
        )

    def on_mirror_floor(self, floor: Any, total: Any = 0) -> bool:
        floor_value = self._int(floor)
        total_value = self._int(total)
        return self._enqueue(
            self.EVENT_NAMES["mirror_floor_signal"],
            floor=floor_value,
            total=total_value,
            floorTotal=total_value,
        )

    def on_warning(self, message: Any) -> bool:
        return self._enqueue(
            self.EVENT_NAMES["warning"],
            code="CORE_WARNING",
            message=str(message),
        )

    def on_hdr_warning(self, acknowledgement: Any = None) -> bool:
        accepted = self._enqueue(
            self.EVENT_NAMES["hdr_warning"],
            code="HDR_WARNING",
            message="检测到游戏所在显示器已开启 HDR，可能影响图像识别",
        )
        # A local Event is never serialized.  It is acknowledged only after the
        # writer/sink accepted the message, which mirrors old synchronous behavior
        # without waiting for UI work in another process.
        if accepted and hasattr(acknowledgement, "set"):
            try:
                acknowledgement.set()
            except Exception:
                return False
        return accepted

    def on_focus_requested(self, reason: Any = None) -> bool:
        return self._enqueue(
            self.EVENT_NAMES["request_focus"],
            reason=str(reason) if reason is not None else "task",
        )

    @staticmethod
    def _config_value(config: Any, key: str, default: Any) -> Any:
        if config is None:
            return default
        getter = getattr(config, "get_value", None)
        if callable(getter):
            try:
                return getter(key, default)
            except Exception:
                return default
        if isinstance(config, Mapping):
            return config.get(key, default)
        value = getattr(config, key, default)
        return default if value is None else value


# Descriptive aliases make the bridge discoverable to integration code without
# forcing it to depend on one historical class name.
MediatorEventBridge = MediatorBridge
ExecutionMediatorBridge = MediatorBridge
RunnerMediatorBridge = MediatorBridge


__all__ = [
    "ExecutionMediatorBridge",
    "MediatorBridge",
    "MediatorEventBridge",
    "RunnerMediatorBridge",
]
