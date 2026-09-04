"""Sidecar-owned completion actions.

The Runner can request completion actions, but it must not own sidecar
notifications or the application/power lifecycle.  This module provides a
small, dependency-injected coordinator for those actions.  In particular, it
does not import :mod:`module.system_actions`: a caller has to explicitly wire
the real operating-system and notification operations into the coordinator.

The journal interface is intentionally structural.  A callable accepting one
record is enough for an embedder, while objects exposing ``transition`` or
``record`` are also supported.  This keeps the coordinator usable before the
full CleanupLedger is wired into ``BackendApplication``.
"""

from __future__ import annotations

import inspect
import threading
from collections.abc import Callable, Iterable, Mapping, MutableMapping
from dataclasses import dataclass
from enum import StrEnum
from typing import Any, Protocol

from module.after_completion_types import POWER_ACTIONS, PowerAction

NOTIFICATION_ACTION = "notification"
TOAST_ACTION = "toast"
SOUND_ACTION = "sound"
EXIT_AALC_ACTION = "exit_aalc"
POWER_ACTION = "power"

APP_EXIT_REQUESTED = "app.exitRequested"

ACTION_ORDER: tuple[str, ...] = (
    NOTIFICATION_ACTION,
    TOAST_ACTION,
    SOUND_ACTION,
    EXIT_AALC_ACTION,
    POWER_ACTION,
)
ALLOWED_ACTIONS = frozenset(ACTION_ORDER)
POWER_ACTION_VALUES = frozenset(action.value for action in POWER_ACTIONS)
DESTRUCTIVE_ACTIONS = frozenset({EXIT_AALC_ACTION, POWER_ACTION})


class ActionState(StrEnum):
    """Durable state of one completion action."""

    PENDING = "pending"
    EXECUTING = "executing"
    DONE = "done"
    UNKNOWN = "unknown"
    SKIPPED = "skipped"
    FAILED = "failed"


class ActionJournal(Protocol):
    """Minimal optional protocol understood by :class:`AfterCompletionCoordinator`.

    Implementations may expose only ``transition``.  ``load``/``records`` are
    optional and are discovered at runtime to make recovery easy to test with
    a tiny in-memory fake.
    """

    def transition(self, action_id: str, state: str, **fields: Any) -> Any: ...


@dataclass(frozen=True, slots=True)
class ActionResult:
    """Typed result for one action, serializable at the RPC boundary."""

    run_id: str
    action_id: str
    action_type: str
    state: ActionState
    executed: bool = False
    deduplicated: bool = False
    recovered: bool = False
    power_action: str | None = None
    event: Mapping[str, Any] | None = None
    result: Any = None
    error: Mapping[str, Any] | None = None

    @property
    def status(self) -> str:
        return self.state.value

    def to_dict(self) -> dict[str, Any]:
        value: dict[str, Any] = {
            "runId": self.run_id,
            "actionId": self.action_id,
            "actionType": self.action_type,
            "state": self.state.value,
            "status": self.state.value,
            "executed": self.executed,
            "deduplicated": self.deduplicated,
            "recovered": self.recovered,
        }
        if self.power_action is not None:
            value["powerAction"] = self.power_action
        if self.event is not None:
            value["event"] = dict(self.event)
        if self.result is not None:
            value["result"] = self.result
        if self.error is not None:
            value["error"] = dict(self.error)
        return value


CompletionActionResult = ActionResult
AfterCompletionActionState = ActionState


@dataclass(frozen=True, slots=True)
class _ActionSpec:
    action_type: str
    payload: Mapping[str, Any]
    valid: bool = True
    invalid_reason: str | None = None


class _ActionRejected(RuntimeError):
    """A handler explicitly declined an action by returning ``False``."""


def _coerce_bool(value: Any, *, default: bool = False) -> bool:
    if isinstance(value, bool):
        return value
    if isinstance(value, str):
        normalized = value.strip().lower()
        if normalized in {"1", "true", "yes", "on", "y"}:
            return True
        if normalized in {"0", "false", "no", "off", "n", ""}:
            return False
    if value is None:
        return default
    return bool(value)


def _enum_value(value: Any) -> Any:
    return getattr(value, "value", value)


def _error(code: str, message: str, *, retryable: bool = False, **extra: Any) -> dict[str, Any]:
    value: dict[str, Any] = {
        "code": code,
        "message": message,
        "retryable": retryable,
    }
    value.update(extra)
    return value


def _handler_call(handler: Callable[..., Any], candidates: Iterable[tuple[tuple[Any, ...], dict[str, Any]]]) -> Any:
    """Call a compatibility handler without hiding an exception from its body."""

    try:
        signature = inspect.signature(handler)
    except (TypeError, ValueError):
        first_args, first_kwargs = next(iter(candidates))
        return handler(*first_args, **first_kwargs)

    for args, kwargs in candidates:
        try:
            signature.bind(*args, **kwargs)
        except TypeError:
            continue
        return handler(*args, **kwargs)
    raise TypeError("injected completion handler has an unsupported signature")


def _callable_result(value: Any) -> Any:
    if isinstance(value, Mapping):
        return dict(value)
    return value


class AfterCompletionCoordinator:
    """Execute sidecar completion actions with durable, idempotent states.

    All side effects are injected.  The common constructor form is::

        AfterCompletionCoordinator(
            journal=ledger,
            notification_handler=send_notification,
            toast_handler=send_toast,
            sound_handler=play_sound,
            exit_request_handler=emit_app_event,
            power_handler=perform_power_action,
            cleanup_handler=cleanup,
        )

    Handler callbacks receive a typed action record.  A power handler may use
    the shorter ``handler(power_action)`` form, and an exit handler may use the
    event-sink form ``handler(event_name, payload)``.  Missing handlers are
    reported as structured ``skipped`` results and never trigger an implicit
    operating-system action.
    """

    def __init__(
        self,
        journal: ActionJournal | Callable[..., Any] | MutableMapping[str, Any] | None = None,
        *,
        handlers: Mapping[str, Callable[..., Any]] | None = None,
        action_handlers: Mapping[str, Callable[..., Any]] | None = None,
        notification_handler: Callable[..., Any] | None = None,
        toast_handler: Callable[..., Any] | None = None,
        sound_handler: Callable[..., Any] | None = None,
        exit_request_handler: Callable[..., Any] | None = None,
        emit_exit_requested: Callable[..., Any] | None = None,
        event_sink: Callable[..., Any] | None = None,
        power_handler: Callable[..., Any] | None = None,
        cleanup_handler: Callable[..., Any] | None = None,
        cleanup_callback: Callable[..., Any] | None = None,
        **aliases: Any,
    ) -> None:
        if journal is None:
            journal = aliases.pop("journal_callback", None) or aliases.pop("journal_writer", None)
        self.journal = journal
        self._lock = threading.RLock()
        self._states: dict[str, dict[str, Any]] = {}
        self._journal_errors: list[dict[str, Any]] = []

        merged: dict[str, Callable[..., Any]] = {}
        callback_aliases = aliases.pop("callbacks", None)
        for source in (handlers, action_handlers, callback_aliases):
            if isinstance(source, Mapping):
                for key, callback in source.items():
                    if callable(callback):
                        canonical = self._canonical_type(key)
                        merged[canonical if canonical in ALLOWED_ACTIONS else str(key)] = callback

        # Keep aliases here rather than importing notification/system modules;
        # this module remains safe to instantiate in tests and on non-Windows.
        notification_handler = notification_handler or aliases.pop("notification", None)
        notification_handler = notification_handler or aliases.pop("notify", None)
        notification_handler = notification_handler or aliases.pop("notify_handler", None)
        toast_handler = toast_handler or aliases.pop("toast", None)
        sound_handler = sound_handler or aliases.pop("sound", None)
        exit_request_handler = exit_request_handler or aliases.pop("exit_aalc", None)
        exit_request_handler = exit_request_handler or aliases.pop("exit_requested", None)
        exit_request_handler = exit_request_handler or aliases.pop("exit_request", None)
        exit_request_handler = exit_request_handler or aliases.pop("emit_event", None)
        exit_request_handler = exit_request_handler or emit_exit_requested or event_sink
        power_handler = power_handler or aliases.pop("power", None)
        power_handler = power_handler or aliases.pop("power_action_handler", None)
        power_handler = power_handler or aliases.pop("power_action", None)
        cleanup_handler = cleanup_handler or cleanup_callback or aliases.pop("cleanup", None)

        for key, callback in (
            (NOTIFICATION_ACTION, notification_handler),
            (TOAST_ACTION, toast_handler),
            (SOUND_ACTION, sound_handler),
            (EXIT_AALC_ACTION, exit_request_handler),
            (POWER_ACTION, power_handler),
        ):
            if callable(callback):
                merged[key] = callback

        # A typo in an injection should not silently select a dangerous action.
        # Keep the constructor permissive for forward-compatible aliases, but
        # expose the leftovers for diagnostics.
        self.unknown_injections = tuple(sorted(str(key) for key in aliases))
        self.handlers = merged
        self.cleanup_handler = cleanup_handler if callable(cleanup_handler) else merged.get("cleanup")

        # Recovery is best effort and only changes journal state; it does not
        # replay anything during construction.
        self.recover()

    @staticmethod
    def action_id(run_id: str, action_type: str) -> str:
        """Return the stable ``runId + actionType`` journal key."""

        return f"{run_id}:{action_type}"

    make_action_id = action_id

    def execute(
        self,
        run_id_or_request: str | Mapping[str, Any] | None = None,
        *,
        run_id: str | None = None,
        outcome: Any = None,
        forced: Any = None,
        actions: Iterable[Any] | Mapping[str, Any] | Any | None = None,
        power_action: Any = None,
        notification: Any = None,
        toast: Any = None,
        sound: Any = None,
        cleanup: Callable[..., Any] | None = None,
        device_lease_valid: bool | None = None,
    ) -> dict[str, Any]:
        """Execute a completion request and return a serializable result.

        The first argument may be a run id or a finished/request mapping.  The
        latter accepts both camelCase protocol keys and snake_case aliases.
        ``device_lease_valid`` is accepted for integration compatibility; the
        sidecar-owned actions do not use it to authorize device-owned
        ``exit_game``/``exit_emulator`` actions.
        """

        del device_lease_valid  # device actions intentionally are not accepted here
        request: Mapping[str, Any] | None = None
        if isinstance(run_id_or_request, Mapping):
            request = run_id_or_request
            run_id = run_id or self._request_value(request, "runId", "run_id")
            if outcome is None:
                outcome = self._request_value(request, "outcome", default="completed")
            if forced is None:
                forced = self._request_value(request, "forced", default=False)
            if actions is None:
                actions = self._request_value(request, "actions", "afterCompletion", "after_completion")
            if power_action is None:
                power_action = self._request_value(request, "powerAction", "power_action")
            if notification is None and "notification" in request:
                notification = request["notification"]
            if toast is None and "toast" in request:
                toast = request["toast"]
            if sound is None and "sound" in request:
                sound = request["sound"]
        if isinstance(actions, Mapping) and "actions" in actions:
            # ``afterCompletion`` is sometimes passed as the wrapper object
            # rather than its inner action list.
            wrapper = actions
            actions = wrapper.get("actions")
            if power_action is None:
                power_action = wrapper.get("powerAction", wrapper.get("power_action"))
        if not isinstance(run_id_or_request, Mapping) and run_id is None:
            run_id = run_id_or_request

        normalized_run_id = self._validate_run_id(run_id)
        normalized_outcome = str(_enum_value(outcome if outcome is not None else "completed")).strip().lower()
        normalized_forced = _coerce_bool(forced, default=False)
        specs = self._build_specs(
            actions,
            power_action=power_action,
            notification=notification,
            toast=toast,
            sound=sound,
        )

        with self._lock:
            eligible = normalized_outcome == "completed" and not normalized_forced
            action_results: list[dict[str, Any]] = []
            for spec in specs:
                action_results.append(self._execute_spec(spec, normalized_run_id, eligible, normalized_outcome, normalized_forced))

            cleanup_result = self._execute_cleanup(
                cleanup or self.cleanup_handler,
                normalized_run_id,
                normalized_outcome,
                normalized_forced,
                action_results,
            )
            failed = any(item.get("state") == ActionState.FAILED.value for item in action_results)
            result: dict[str, Any] = {
                "runId": normalized_run_id,
                "outcome": normalized_outcome,
                "forced": normalized_forced,
                "eligible": eligible,
                "accepted": eligible and not failed,
                "actions": action_results,
                "cleanup": cleanup_result,
            }
            if request is not None:
                result["request"] = True
            if self._journal_errors:
                result["journalErrors"] = [dict(value) for value in self._journal_errors]
            return result

    # Names useful at the BackendApplication boundary and in embedders.
    coordinate = execute
    process = execute
    handle_finished = execute
    execute_after_completion = execute

    def recover(self, run_id: str | None = None) -> dict[str, Any]:
        """Reconcile journal ``executing`` entries without replaying hazards.

        Notification-like actions are returned to ``pending`` so a later
        ``execute`` can idempotently retry them.  ``exit_aalc`` and power
        actions are marked ``unknown`` and are never replayed automatically.
        """

        with self._lock:
            recovered: list[dict[str, Any]] = []
            for value in self._journal_records():
                record = self._normalize_record(value)
                if record is None:
                    continue
                action_id = record["actionId"]
                if run_id is not None and record.get("runId") != run_id:
                    continue
                self._states[action_id] = record
                state = record.get("state", ActionState.PENDING.value)
                if state != ActionState.EXECUTING.value:
                    continue
                action_type = str(record.get("actionType", ""))
                if action_type in DESTRUCTIVE_ACTIONS or action_type.startswith("power"):
                    next_state = ActionState.UNKNOWN
                    reason = _error(
                        "ACTION_UNKNOWN_AFTER_RESTART",
                        "destructive completion action was executing when the sidecar stopped",
                    )
                else:
                    next_state = ActionState.PENDING
                    reason = _error(
                        "ACTION_RETRY_AFTER_RESTART",
                        "idempotent completion action will be retried",
                        retryable=True,
                    )
                self._transition(record, next_state, error=reason, recovered=True)
                recovered.append(
                    {
                        "runId": record.get("runId"),
                        "actionId": action_id,
                        "actionType": action_type,
                        "state": next_state.value,
                        "status": next_state.value,
                        "recovered": True,
                        "error": reason,
                    }
                )
            return {"recovered": recovered}

    recover_inflight = recover

    @staticmethod
    def _request_value(request: Mapping[str, Any], *keys: str, default: Any = None) -> Any:
        for key in keys:
            if key in request:
                return request[key]
        return default

    @staticmethod
    def _validate_run_id(run_id: Any) -> str:
        run_id = _enum_value(run_id)
        if not isinstance(run_id, str) or not run_id.strip():
            raise ValueError("runId is required for completion actions")
        return run_id.strip()

    @classmethod
    def _build_specs(
        cls,
        actions: Iterable[Any] | Mapping[str, Any] | Any | None,
        *,
        power_action: Any,
        notification: Any,
        toast: Any,
        sound: Any,
    ) -> list[_ActionSpec]:
        raw_specs = cls._coerce_specs(actions)
        existing_types = {spec.action_type for spec in raw_specs}
        if notification is not None and NOTIFICATION_ACTION not in existing_types:
            raw_specs.append(_ActionSpec(NOTIFICATION_ACTION, cls._payload_value(notification)))
        if toast is not None and TOAST_ACTION not in existing_types:
            raw_specs.append(_ActionSpec(TOAST_ACTION, cls._payload_value(toast)))
        if sound is not None and SOUND_ACTION not in existing_types:
            raw_specs.append(_ActionSpec(SOUND_ACTION, cls._payload_value(sound)))

        if power_action is not None and POWER_ACTION not in existing_types:
            if _enum_value(power_action) != PowerAction.NONE.value:
                raw_specs.append(_ActionSpec(POWER_ACTION, {"powerAction": _enum_value(power_action)}))
        elif power_action is not None and _enum_value(power_action) != PowerAction.NONE.value:
            # A request may name ``power`` in ``actions`` and carry the value
            # in the separate protocol field.
            raw_specs = [
                (
                    _ActionSpec(POWER_ACTION, {"powerAction": _enum_value(power_action)})
                    if spec.action_type == POWER_ACTION and not spec.payload.get("powerAction")
                    else spec
                )
                for spec in raw_specs
            ]

        # One run/actionType is one journal key.  Keep the first payload and
        # apply the documented execution order regardless of request order.
        unique: dict[str, _ActionSpec] = {}
        for spec in raw_specs:
            unique.setdefault(spec.action_type, spec)
        order = {name: index for index, name in enumerate(ACTION_ORDER)}
        return sorted(unique.values(), key=lambda spec: (order.get(spec.action_type, len(order)), spec.action_type))

    @classmethod
    def _coerce_specs(cls, actions: Iterable[Any] | Mapping[str, Any] | Any | None) -> list[_ActionSpec]:
        if actions is None:
            return []
        if isinstance(actions, Mapping):
            if any(key in actions for key in ("type", "actionType", "action", "name")):
                values: list[Any] = [actions]
            else:
                values = [{"type": key, "payload": value} for key, value in actions.items()]
        elif isinstance(actions, (str, PowerAction)):
            values = [actions]
        else:
            try:
                values = list(actions)
            except TypeError:
                values = [actions]

        specs: list[_ActionSpec] = []
        for value in values:
            if isinstance(value, Mapping):
                raw_type = value.get("type", value.get("actionType", value.get("action", value.get("name"))))
                payload_value = value.get("payload", value.get("data"))
                if payload_value is None:
                    payload_value = {
                        str(key): item
                        for key, item in value.items()
                        if key not in {"type", "actionType", "action", "name", "enabled"}
                    }
                payload = cls._payload_value(payload_value)
                if value.get("enabled") is False:
                    # Keep an explicit skip in the action list/journal.
                    canonical = cls._canonical_type(raw_type)
                    specs.append(_ActionSpec(canonical, payload, valid=False, invalid_reason="disabled"))
                    continue
            else:
                raw_type = value
                payload = {}
            canonical = cls._canonical_type(raw_type)
            if canonical == POWER_ACTION:
                raw_power = payload.get("powerAction", payload.get("power_action"))
                if raw_power is None:
                    raw_power = payload.get("value")
                raw_name = _enum_value(raw_type)
                if raw_power is None and isinstance(raw_name, str):
                    prefix = raw_name.lower().replace(":", ".")
                    if prefix.startswith("power."):
                        raw_power = raw_name.replace(":", ".", 1).split(".", 1)[1]
                if raw_power is None and isinstance(raw_name, str) and raw_name in POWER_ACTION_VALUES:
                    raw_power = raw_name
                payload = dict(payload)
                payload["powerAction"] = _enum_value(raw_power)
            valid = canonical in ALLOWED_ACTIONS
            reason = None if valid else "action is not allowed for sidecar completion"
            specs.append(_ActionSpec(canonical, payload, valid=valid, invalid_reason=reason))
        return specs

    @staticmethod
    def _payload_value(value: Any) -> dict[str, Any]:
        if isinstance(value, Mapping):
            return dict(value)
        if value is None:
            return {}
        return {"value": value}

    @staticmethod
    def _canonical_type(value: Any) -> str:
        value = _enum_value(value)
        if not isinstance(value, str):
            return str(value) if value is not None else ""
        normalized = value.strip()
        aliases = {
            "notify": NOTIFICATION_ACTION,
            "notification": NOTIFICATION_ACTION,
            "toast": TOAST_ACTION,
            "sound": SOUND_ACTION,
            "exitAalc": EXIT_AALC_ACTION,
            "exit-aalc": EXIT_AALC_ACTION,
            "app.exitRequested": EXIT_AALC_ACTION,
            "powerAction": POWER_ACTION,
            "power_action": POWER_ACTION,
        }
        if normalized in aliases:
            return aliases[normalized]
        power_prefix = normalized.lower().replace(":", ".")
        if power_prefix.startswith("power."):
            return POWER_ACTION
        if normalized.lower() in {action.value for action in PowerAction if action is not PowerAction.NONE}:
            return POWER_ACTION
        return normalized

    def _execute_spec(
        self,
        spec: _ActionSpec,
        run_id: str,
        eligible: bool,
        outcome: str,
        forced: bool,
    ) -> dict[str, Any]:
        action_id = self.action_id(run_id, spec.action_type)
        existing = self._states.get(action_id)
        if existing is not None:
            state = str(existing.get("state", ""))
            if state == ActionState.DONE.value:
                return self._result_from_record(existing, deduplicated=True)
            if state == ActionState.UNKNOWN.value and spec.action_type in DESTRUCTIVE_ACTIONS:
                return self._result_from_record(
                    existing,
                    recovered=True,
                    error=existing.get(
                        "error",
                        _error("ACTION_UNKNOWN_AFTER_RESTART", "destructive action will not be replayed"),
                    ),
                )

        record = {
            "runId": run_id,
            "actionId": action_id,
            "actionType": spec.action_type,
            "payload": dict(spec.payload),
        }
        if spec.action_type == POWER_ACTION:
            raw_power = spec.payload.get("powerAction", spec.payload.get("power_action"))
            record["powerAction"] = str(_enum_value(raw_power)) if raw_power is not None else ""

        if not eligible:
            reason_code = "FORCED_RUN" if forced else "OUTCOME_NOT_COMPLETED"
            reason = _error(reason_code, "completion actions are disabled for this outcome")
            return self._finish_skipped(record, reason)
        if not spec.valid:
            return self._finish_skipped(record, _error("ACTION_NOT_ALLOWED", spec.invalid_reason or "action is not allowed"))
        if spec.action_type == POWER_ACTION:
            power_value = record.get("powerAction", "")
            if power_value not in POWER_ACTION_VALUES or power_value == PowerAction.NONE.value:
                return self._finish_skipped(record, _error("INVALID_POWER_ACTION", "power action is not allowlisted"))

        handler = self.handlers.get(spec.action_type)
        if not callable(handler):
            return self._finish_skipped(
                record,
                _error("HANDLER_UNAVAILABLE", f"no injected handler for {spec.action_type}", retryable=True),
            )

        prior_state = existing.get("state") if existing is not None else None
        if prior_state != ActionState.PENDING.value:
            self._transition(record, ActionState.PENDING)
        self._transition(record, ActionState.EXECUTING)
        try:
            value = self._invoke_action_handler(handler, record)
            if value is False:
                raise _ActionRejected(f"{spec.action_type} handler declined the action")
        except Exception as exc:
            reason = _error("ACTION_FAILED", str(exc) or type(exc).__name__, retryable=spec.action_type not in DESTRUCTIVE_ACTIONS)
            self._transition(record, ActionState.FAILED, error=reason)
            return self._result_from_record(record, executed=False, error=reason)

        result = _callable_result(value)
        self._transition(record, ActionState.DONE, result=result, executed=True)
        return self._result_from_record(record, executed=True, result=result)

    def _invoke_action_handler(self, handler: Callable[..., Any], record: Mapping[str, Any]) -> Any:
        action_type = record["actionType"]
        payload = dict(record.get("payload", {}))
        if action_type == POWER_ACTION:
            power_value = record.get("powerAction")
            return _handler_call(
                handler,
                (
                    ((power_value,), {}),
                    ((record,), {}),
                    ((record["runId"], record["actionId"], power_value), {}),
                    ((record["runId"], record), {}),
                    ((), {}),
                ),
            )
        if action_type == EXIT_AALC_ACTION:
            event_payload = {
                "type": APP_EXIT_REQUESTED,
                "runId": record["runId"],
                "actionId": record["actionId"],
                "reason": "completion",
            }
            # Preserve the typed event in the journal/result for a sink that
            # only returns None.
            if isinstance(record, dict):
                record["event"] = event_payload
            return _handler_call(
                handler,
                (
                    ((APP_EXIT_REQUESTED, event_payload), {}),
                    ((event_payload,), {}),
                    ((record,), {}),
                    ((record["runId"], event_payload), {}),
                    ((), {}),
                ),
            )
        return _handler_call(
            handler,
            (
                ((record,), {}),
                ((payload,), {}),
                ((record["runId"], record), {}),
                ((record["runId"], record["actionId"], payload), {}),
                ((), {}),
            ),
        )

    def _finish_skipped(self, record: dict[str, Any], reason: Mapping[str, Any]) -> dict[str, Any]:
        self._transition(record, ActionState.SKIPPED, error=reason)
        return self._result_from_record(record, error=reason)

    def _execute_cleanup(
        self,
        handler: Callable[..., Any] | None,
        run_id: str,
        outcome: str,
        forced: bool,
        action_results: list[Mapping[str, Any]],
    ) -> dict[str, Any]:
        if not callable(handler):
            return {"status": "skipped", "executed": False}
        cleanup_record = {
            "runId": run_id,
            "outcome": outcome,
            "forced": forced,
            "actions": [dict(value) for value in action_results],
        }
        try:
            value = _handler_call(
                handler,
                (
                    ((cleanup_record,), {}),
                    ((run_id, cleanup_record), {}),
                    ((run_id,), {}),
                    ((), {}),
                ),
            )
            if value is False:
                raise _ActionRejected("cleanup handler declined cleanup")
            return {"status": "done", "executed": True, "result": _callable_result(value)}
        except Exception as exc:
            return {
                "status": "failed",
                "executed": False,
                "error": _error("CLEANUP_FAILED", str(exc) or type(exc).__name__, retryable=True),
            }

    def _result_from_record(
        self,
        record: Mapping[str, Any],
        *,
        executed: bool | None = None,
        deduplicated: bool = False,
        recovered: bool = False,
        result: Any = None,
        error: Mapping[str, Any] | None = None,
    ) -> dict[str, Any]:
        raw_state = str(record.get("state", ActionState.SKIPPED.value))
        try:
            state = ActionState(raw_state)
        except ValueError:
            state = ActionState.UNKNOWN
        if executed is None:
            executed = bool(record.get("executed", False))
        if result is None and "result" in record:
            result = record.get("result")
        if error is None and isinstance(record.get("error"), Mapping):
            error = record["error"]
        return ActionResult(
            run_id=str(record.get("runId", "")),
            action_id=str(record.get("actionId", "")),
            action_type=str(record.get("actionType", "")),
            state=state,
            executed=executed,
            deduplicated=deduplicated,
            recovered=recovered or bool(record.get("recovered", False)),
            power_action=record.get("powerAction"),
            event=record.get("event") if isinstance(record.get("event"), Mapping) else None,
            result=result,
            error=error,
        ).to_dict()

    def _transition(self, record: dict[str, Any], state: ActionState, **fields: Any) -> None:
        record["state"] = state.value
        record["status"] = state.value
        if state in {ActionState.PENDING, ActionState.EXECUTING, ActionState.DONE} and "error" not in fields:
            record.pop("error", None)
        record.update(fields)
        self._states[str(record["actionId"])] = dict(record)
        self._persist(record)

    def _persist(self, record: Mapping[str, Any]) -> None:
        journal = self.journal
        if journal is None:
            return
        value = dict(record)
        try:
            if isinstance(journal, MutableMapping):
                journal[str(value["actionId"])] = value
                return
            method = None
            for name in ("transition", "record", "append", "update", "set_state"):
                candidate = getattr(journal, name, None)
                if callable(candidate):
                    method = candidate
                    break
            if method is None and callable(journal):
                _handler_call(
                    journal,
                    (
                        ((value,), {}),
                        ((value["actionId"], value["state"], value), {}),
                        (
                            (value["actionId"], value["state"]),
                            {
                                "run_id": value.get("runId"),
                                "action_type": value.get("actionType"),
                                "record": value,
                            },
                        ),
                        ((value["actionId"], value["state"]), {"metadata": value}),
                        ((value["actionId"], value["state"]), {}),
                    ),
                )
                return
            if method is None:
                return
            _handler_call(
                method,
                (
                    ((value,), {}),
                    ((value["actionId"], value["state"], value), {}),
                    (
                        (value["actionId"], value["state"]),
                        {
                            "run_id": value.get("runId"),
                            "action_type": value.get("actionType"),
                            "record": value,
                        },
                    ),
                    ((value["actionId"], value["state"]), {"metadata": value}),
                    ((value["actionId"], value["state"]), {}),
                ),
            )
        except Exception as exc:
            self._journal_errors.append(
                {
                    "code": "JOURNAL_WRITE_FAILED",
                    "message": str(exc) or type(exc).__name__,
                    "retryable": True,
                    "actionId": value.get("actionId"),
                    "state": value.get("state"),
                }
            )

    def _journal_records(self) -> list[Mapping[str, Any]]:
        journal = self.journal
        if journal is None or isinstance(journal, Mapping) or callable(journal):
            if isinstance(journal, Mapping):
                return [self._mapping_with_id(key, value) for key, value in journal.items()]
            return []

        values: Any = None
        for name in ("load", "snapshot", "entries", "records", "all"):
            candidate = getattr(journal, name, None)
            if callable(candidate):
                try:
                    values = _handler_call(candidate, (((), {}),))
                except (TypeError, ValueError):
                    continue
                break
            if candidate is not None:
                values = candidate
                break
        if values is None:
            return []
        if isinstance(values, Mapping):
            if self._looks_like_record(values):
                return [values]
            nested = values.get("actions")
            if isinstance(nested, Iterable) and not isinstance(nested, (str, bytes, bytearray, Mapping)):
                return [item for item in nested if isinstance(item, Mapping)]
            return [self._mapping_with_id(key, value) for key, value in values.items()]
        if isinstance(values, Iterable) and not isinstance(values, (str, bytes, bytearray)):
            return [value for value in values if isinstance(value, Mapping)]
        return []

    @staticmethod
    def _mapping_with_id(key: Any, value: Any) -> Mapping[str, Any]:
        if isinstance(value, Mapping):
            if "actionId" in value or "action_id" in value:
                return value
            result = dict(value)
            result["actionId"] = key
            return result
        return {"actionId": key, "state": value}

    @staticmethod
    def _looks_like_record(value: Mapping[str, Any]) -> bool:
        return any(key in value for key in ("actionId", "action_id", "actionType", "action_type", "state", "status"))

    @staticmethod
    def _normalize_record(value: Mapping[str, Any]) -> dict[str, Any] | None:
        raw_action_id = value.get("actionId", value.get("action_id"))
        raw_run_id = value.get("runId", value.get("run_id"))
        raw_action_type = value.get("actionType", value.get("action_type"))
        if raw_action_id is None:
            if raw_run_id is None or raw_action_type is None:
                return None
            raw_action_id = AfterCompletionCoordinator.action_id(str(raw_run_id), str(raw_action_type))
        action_id = str(raw_action_id)
        if raw_run_id is None and ":" in action_id:
            raw_run_id = action_id.rsplit(":", 1)[0]
        if raw_action_type is None and ":" in action_id:
            raw_action_type = action_id.rsplit(":", 1)[1]
        if isinstance(raw_action_type, str):
            normalized_action_type = raw_action_type.strip()
            if normalized_action_type.lower() in POWER_ACTION_VALUES or normalized_action_type.lower().startswith("power."):
                raw_action_type = POWER_ACTION
                if raw_run_id:
                    action_id = AfterCompletionCoordinator.action_id(str(raw_run_id), POWER_ACTION)
        state = str(value.get("state", value.get("status", ActionState.PENDING.value)))
        if state not in {item.value for item in ActionState}:
            state = ActionState.UNKNOWN.value
        normalized: dict[str, Any] = dict(value)
        normalized.update(
            {
                "actionId": action_id,
                "runId": str(raw_run_id or ""),
                "actionType": str(raw_action_type or ""),
                "state": state,
                "status": state,
            }
        )
        return normalized


def coordinate_after_completion(
    request: Mapping[str, Any],
    *,
    coordinator: AfterCompletionCoordinator,
) -> dict[str, Any]:
    """Small integration helper for callers that already own a coordinator."""

    return coordinator.execute(request)


__all__ = [
    "ACTION_ORDER",
    "ALLOWED_ACTIONS",
    "APP_EXIT_REQUESTED",
    "ActionJournal",
    "ActionResult",
    "ActionState",
    "AfterCompletionActionState",
    "AfterCompletionCoordinator",
    "CompletionActionResult",
    "DESTRUCTIVE_ACTIONS",
    "EXIT_AALC_ACTION",
    "NOTIFICATION_ACTION",
    "POWER_ACTION",
    "POWER_ACTION_VALUES",
    "SOUND_ACTION",
    "TOAST_ACTION",
    "coordinate_after_completion",
]
