"""Adapters from Runner protocol events to the sidecar event vocabulary.

``RunnerSupervisor`` deliberately deals in protocol headers.  Existing
``BackendApplication`` instances, however, publish events through an
``emit(name, payload)`` callback.  This adapter keeps that compatibility at the
boundary and does not import the backend (or WebSocket) module itself.
"""

from __future__ import annotations

import inspect
from collections.abc import Callable, Mapping
from typing import Any


class RunnerEventAdapter:
    """Translate a decoded Runner event to a sidecar event callback.

    The callback normally has the shape ``backend.emit(event, payload)``.  A
    one-argument callback is also accepted for lightweight tests and embedders;
    it receives ``{"event": ..., "payload": ...}``.
    """

    EVENT_MAP = {
        "task.started": "execution.taskStarted",
        "task.completed": "execution.taskCompleted",
        "mirror.progress": "execution.mirrorProgress",
        "mirror.floor": "execution.mirrorFloor",
        "warning": "app.notice",
        "hdr.warning": "app.notice",
        "app.focusRequested": "app.focusRequested",
        "ready": "execution.runnerReady",
        "status": "execution.status",
        "finished": "execution.finished",
        "error": "execution.error",
        "log.entry": "execution.log",
        "config.delta": "execution.configDelta",
        "resource.created": "execution.resourceCreated",
        "resource.released": "execution.resourceReleased",
        "afterCompletion.requested": "execution.afterCompletionRequested",
    }

    def __init__(
        self,
        sink: Callable[..., Any],
        *,
        run_id: str | None = None,
        ledger: Any = None,
        cleanup_ledger: Any = None,
    ) -> None:
        if not callable(sink):
            raise TypeError("sink must be callable")
        self.sink = sink
        self.run_id = run_id
        self.ledger = ledger if ledger is not None else cleanup_ledger
        self.cleanup_ledger = self.ledger

    def __call__(self, event: Mapping[str, Any], payload: bytes = b"") -> Any:
        return self.forward(event, payload)

    def forward(self, event: Mapping[str, Any], payload: bytes = b"") -> Any:
        if not isinstance(event, Mapping):
            raise TypeError("Runner event must be a mapping")
        message_type = event.get("type")
        if not isinstance(message_type, str) or not message_type:
            raise ValueError("Runner event type is required")
        destination = self.EVENT_MAP.get(message_type, message_type)
        fields = self._payload(message_type, event, payload)
        self._record_ledger(message_type, event, fields)
        return self._call_sink(destination, fields)

    def _record_ledger(self, message_type: str, event: Mapping[str, Any], fields: Mapping[str, Any]) -> None:
        ledger = self.ledger
        if ledger is None:
            return
        original = dict(event)
        if self.run_id is not None:
            original.setdefault("runId", self.run_id)
        if message_type in {"resource.created", "resource.released"}:
            callback = getattr(ledger, "record_resource_event", None)
            if callable(callback):
                callback(original)
            elif message_type == "resource.created":
                callback = getattr(ledger, "record_resource_created", None)
                if callable(callback):
                    callback(
                        str(fields.get("resourceType", "")),
                        str(fields.get("resourceId", "")),
                        metadata=fields.get("metadata") if isinstance(fields.get("metadata"), Mapping) else {},
                    )
            else:
                callback = getattr(ledger, "record_resource_released", None)
                if callable(callback):
                    callback(str(fields.get("resourceType", "")), str(fields.get("resourceId", "")))
        elif message_type == "config.delta":
            callback = getattr(ledger, "record_config_delta", None)
            if callable(callback):
                callback(original)
        elif message_type == "finished":
            sequence = original.get("seq")
            if sequence is not None:
                callback = getattr(ledger, "record_final_seq", None)
                if callable(callback):
                    callback(sequence)
            callback = getattr(ledger, "record_finished", None)
            if callable(callback):
                callback(original)
        elif message_type == "afterCompletion.requested":
            callback = getattr(ledger, "record_action_pending", None)
            if not callable(callback):
                return
            actions = fields.get("actions")
            if isinstance(actions, Mapping):
                actions = [actions]
            if not isinstance(actions, list):
                actions = [fields]
            for index, action in enumerate(actions):
                if not isinstance(action, Mapping):
                    continue
                value = dict(action)
                value.setdefault(
                    "actionId",
                    f"{self.run_id or original.get('runId', 'run')}:after-completion:{original.get('seq', index)}:{index}",
                )
                value.setdefault("actionType", value.get("type", "after_completion"))
                callback(value)

    def _payload(self, message_type: str, event: Mapping[str, Any], payload: bytes) -> dict[str, Any]:
        # Protocol bookkeeping must not leak into the public backend payload.
        fields = {
            key: value
            for key, value in event.items()
            if key not in {"type", "protocol", "seq", "commandSeq", "binaryLength"}
        }
        run_id = fields.get("runId") or self.run_id
        if run_id is not None:
            fields["runId"] = run_id
        if payload:
            fields["payload"] = bytes(payload)

        if message_type == "task.completed":
            result = fields.get("result")
            if not isinstance(result, Mapping):
                result = {key: fields[key] for key in ("count", "details") if key in fields}
            fields["result"] = dict(result)
            # Current BackendApplication statistics code consumes these names.
            fields.setdefault("kind", fields.get("taskId", ""))
            fields.setdefault("count", result.get("count", 1))
        elif message_type == "mirror.progress":
            completed = fields.get("completed", fields.get("current", 0))
            try:
                completed = max(0, int(completed))
            except (TypeError, ValueError):
                completed = 0
            fields["completed"] = completed
            fields["current"] = completed
            try:
                fields["total"] = max(0, int(fields.get("total", 0)))
            except (TypeError, ValueError):
                fields["total"] = 0
        elif message_type == "mirror.floor":
            try:
                fields["floor"] = max(0, int(fields.get("floor", 0)))
            except (TypeError, ValueError):
                fields["floor"] = 0
            floor_total = fields.get("floorTotal", fields.get("total", 0))
            try:
                floor_total = max(0, int(floor_total))
            except (TypeError, ValueError):
                floor_total = 0
            fields["floorTotal"] = floor_total
            fields.setdefault("total", floor_total)
        elif message_type in {"warning", "hdr.warning"}:
            fields.setdefault("level", "warn")
            fields.setdefault("message", "Runner warning")
        return fields

    def _call_sink(self, destination: str, fields: Mapping[str, Any]) -> Any:
        try:
            signature = inspect.signature(self.sink)
        except (TypeError, ValueError):
            return self.sink(destination, dict(fields))
        try:
            signature.bind(destination, dict(fields))
        except TypeError:
            return self.sink({"event": destination, "payload": dict(fields)})
        return self.sink(destination, dict(fields))


BackendEventAdapter = RunnerEventAdapter
ExecutionEventAdapter = RunnerEventAdapter


__all__ = [
    "BackendEventAdapter",
    "ExecutionEventAdapter",
    "RunnerEventAdapter",
]
