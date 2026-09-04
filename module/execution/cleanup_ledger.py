"""Durable, idempotent cleanup journal for one Runner run.

The journal contains only conservative identifiers/snapshots.  Concrete Win32,
ADB and input cleanup is deliberately injected through :class:`CleanupExecutor`.
Every mutation is written through a temporary file and ``os.replace`` so a
restart can replay pending obligations safely.
"""

from __future__ import annotations

import copy
import inspect
import json
import os
import tempfile
import threading
import time
import uuid
from collections.abc import Callable, Mapping
from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path
from typing import Any, Protocol


class LedgerError(RuntimeError):
    """A cleanup journal is malformed or cannot be persisted."""


CleanupLedgerError = LedgerError


@dataclass(frozen=True, slots=True)
class ResourceKey:
    resource_type: str
    resource_id: str


class CleanupActionState(StrEnum):
    PENDING = "pending"
    EXECUTING = "executing"
    DONE = "done"
    FAILED = "failed"


@dataclass(frozen=True, slots=True)
class CleanupActionResult:
    action_id: str
    state: CleanupActionState
    detail: Any = None
    attempts: int = 0

    @property
    def ok(self) -> bool:
        return self.state is CleanupActionState.DONE

    def to_dict(self) -> dict[str, Any]:
        return {
            "actionId": self.action_id,
            "state": self.state.value,
            "detail": copy.deepcopy(self.detail),
            "attempts": self.attempts,
            "ok": self.ok,
        }


@dataclass(frozen=True, slots=True)
class CleanupRecoveryResult:
    status: str
    complete: bool
    actions: tuple[CleanupActionResult, ...] = ()
    errors: tuple[dict[str, Any], ...] = ()

    @property
    def pending(self) -> bool:
        return not self.complete

    def to_dict(self) -> dict[str, Any]:
        return {
            "status": self.status,
            "complete": self.complete,
            "pending": self.pending,
            "actions": [item.to_dict() for item in self.actions],
            "errors": [dict(item) for item in self.errors],
        }

    as_dict = to_dict

    def __getitem__(self, key: str) -> Any:
        return self.to_dict()[key]


class CleanupActionExecutor(Protocol):
    """Platform-independent action boundary (Win32/ADB are injected)."""

    def execute(self, action: Mapping[str, Any]) -> Any: ...


CleanupExecutor = CleanupActionExecutor

_SECRET_PARTS = (
    "token",
    "secret",
    "password",
    "credential",
    "authorization",
    "api_key",
    "apikey",
    "private_key",
    "spt",
)


def _safe_copy(value: Any) -> Any:
    def check(item: Any) -> None:
        if isinstance(item, Mapping):
            for key, child in item.items():
                normalized = str(key).lower().replace("-", "_")
                if any(part in normalized for part in _SECRET_PARTS):
                    raise LedgerError(f"secret-like field is not allowed in cleanup ledger: {key}")
                check(child)
        elif isinstance(item, (list, tuple)):
            for child in item:
                check(child)

    check(value)
    try:
        copied = copy.deepcopy(value)
        json.dumps(copied, ensure_ascii=False, allow_nan=False)
    except (TypeError, ValueError) as exc:
        raise LedgerError(f"ledger value is not JSON-safe: {exc}") from exc
    return copied


def _non_negative(value: Any, name: str, *, none_ok: bool = False) -> int | None:
    if value is None and none_ok:
        return None
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise LedgerError(f"{name} must be a non-negative integer")
    return int(value)


class CleanupLedger:
    """A per-run recoverable cleanup journal."""

    FORMAT_VERSION = 1
    STATUS_PENDING = "cleanup_pending"
    STATUS_COMPLETE = "complete"
    _RESERVED = {
        "runnerPid",
        "runnerCreateTime",
        "targetId",
        "generation",
        "scid",
        "socketName",
        "adbForwardPort",
        "deviceSerial",
        "hwnd",
        "originalWindowState",
        "originalWindowRect",
        "inputObligations",
        "inputState",
        "deviceDisposition",
        "reservation",
    }
    _ALIASES = {
        "runner_pid": "runnerPid",
        "runner_create_time": "runnerCreateTime",
        "target_id": "targetId",
        "socket_name": "socketName",
        "adb_forward_port": "adbForwardPort",
        "reserved_scrcpy_scid": "scid",
        "reserved_socket_name": "socketName",
        "reserved_adb_forward_port": "adbForwardPort",
        "device_serial": "deviceSerial",
        "original_window_state": "originalWindowState",
        "original_window_rect": "originalWindowRect",
        "input_obligations": "inputObligations",
        "input_state": "inputState",
        "device_disposition": "deviceDisposition",
    }

    def __init__(
        self,
        path: str | os.PathLike[str],
        *,
        run_id: str | None = None,
        target_id: str | None = None,
        generation: int | None = None,
        initial: Mapping[str, Any] | None = None,
        create: bool = True,
    ) -> None:
        self.path = Path(path).expanduser()
        self._lock = threading.RLock()
        existed = self.path.exists()
        if initial is not None:
            self._data = self._validate(initial)
        elif existed:
            self._data = self._read(self.path)
        else:
            if not run_id:
                raise ValueError("run_id is required when creating a ledger")
            now = time.time()
            self._data = {
                "formatVersion": self.FORMAT_VERSION,
                "runId": str(run_id),
                "targetId": target_id,
                "generation": generation,
                "status": self.STATUS_PENDING,
                "createdAt": now,
                "updatedAt": now,
                "reservation": {},
                "resources": [],
                "configDeltas": [],
                "finalSeq": None,
                "finished": None,
                "actions": [],
                "cleanup": [],
            }
        if run_id is not None and self._data["runId"] != str(run_id):
            raise LedgerError("cleanup ledger runId does not match requested run")
        if target_id is not None and self._data.get("targetId") is None:
            self._data["targetId"] = str(target_id)
        if generation is not None and self._data.get("generation") is None:
            self._data["generation"] = generation
        if create and not existed:
            self._persist()

    @classmethod
    def create(
        cls,
        path: str | os.PathLike[str],
        *,
        run_id: str | None = None,
        target_id: str | None = None,
        generation: int | None = None,
        **metadata: Any,
    ) -> "CleanupLedger":
        ledger = cls(path, run_id=run_id or str(uuid.uuid4()), target_id=target_id, generation=generation)
        if metadata:
            ledger.update_metadata(metadata)
        return ledger

    @classmethod
    def load(cls, path: str | os.PathLike[str]) -> "CleanupLedger":
        try:
            return cls(path, create=False)
        except ValueError as exc:
            raise LedgerError(f"cannot load cleanup ledger {path}: {exc}") from exc

    @staticmethod
    def _read(path: Path) -> dict[str, Any]:
        try:
            with path.open("r", encoding="utf-8") as stream:
                value = json.load(stream)
        except (OSError, json.JSONDecodeError) as exc:
            raise LedgerError(f"cannot read cleanup ledger {path}: {exc}") from exc
        return CleanupLedger._validate(value)

    @classmethod
    def _validate(cls, value: Mapping[str, Any]) -> dict[str, Any]:
        if not isinstance(value, Mapping):
            raise LedgerError("cleanup ledger root must be an object")
        data = dict(value)
        if (
            isinstance(data.get("formatVersion"), bool)
            or not isinstance(data.get("formatVersion"), int)
            or data.get("formatVersion") != cls.FORMAT_VERSION
        ):
            raise LedgerError("unsupported cleanup ledger format")
        if not isinstance(data.get("runId"), str) or not data["runId"]:
            raise LedgerError("cleanup ledger runId is missing")
        if data.get("status") not in {cls.STATUS_PENDING, cls.STATUS_COMPLETE}:
            raise LedgerError("cleanup ledger status is invalid")
        for key, default in {
            "targetId": None,
            "generation": None,
            "createdAt": time.time(),
            "updatedAt": time.time(),
            "reservation": {},
            "resources": [],
            "configDeltas": [],
            "finalSeq": None,
            "finished": None,
            "actions": [],
            "cleanup": [],
        }.items():
            data.setdefault(key, default)
        if not isinstance(data["reservation"], Mapping):
            raise LedgerError("cleanup ledger reservation must be an object")
        for key in ("resources", "configDeltas", "actions", "cleanup"):
            if not isinstance(data[key], list):
                raise LedgerError(f"cleanup ledger {key} must be a list")
        if data["finalSeq"] is not None:
            _non_negative(data["finalSeq"], "finalSeq")
        if data["finished"] is not None and not isinstance(data["finished"], Mapping):
            raise LedgerError("cleanup ledger finished must be an object")
        if isinstance(data["finished"], Mapping):
            finished_run = data["finished"].get("runId", data["finished"].get("run_id"))
            if finished_run is not None and str(finished_run) != data["runId"]:
                raise LedgerError("cleanup ledger finished runId does not match")
            if data["finished"].get("seq") is not None:
                _non_negative(data["finished"]["seq"], "finished seq")
        resource_keys: set[tuple[str, str]] = set()
        for item in data["resources"]:
            if not isinstance(item, Mapping) or not isinstance(item.get("resourceType"), str) or not item.get("resourceType"):
                raise LedgerError("cleanup ledger resource is invalid")
            if not isinstance(item.get("resourceId"), str) or not item.get("resourceId"):
                raise LedgerError("cleanup ledger resourceId is invalid")
            if not isinstance(item.get("released", False), bool):
                raise LedgerError("cleanup ledger resource released must be boolean")
            resource_key = (item["resourceType"], item["resourceId"])
            if resource_key in resource_keys:
                raise LedgerError("cleanup ledger contains duplicate resource")
            resource_keys.add(resource_key)
            if item.get("metadata") is not None and not isinstance(item.get("metadata"), Mapping):
                raise LedgerError("cleanup ledger resource metadata must be an object")
        delta_ids: set[str] = set()
        delta_sequences: set[int] = set()
        for item in data["configDeltas"]:
            if not isinstance(item, Mapping):
                raise LedgerError("cleanup ledger config delta is invalid")
            delta_id = item.get("deltaId", item.get("delta_id"))
            if not isinstance(delta_id, str) or not delta_id:
                raise LedgerError("cleanup ledger config deltaId is invalid")
            if delta_id in delta_ids:
                raise LedgerError("cleanup ledger contains duplicate config delta")
            delta_ids.add(delta_id)
            if item.get("seq") is not None:
                sequence = _non_negative(item["seq"], "config delta seq")
                assert sequence is not None
                if sequence in delta_sequences:
                    raise LedgerError("cleanup ledger contains duplicate config delta seq")
                delta_sequences.add(sequence)
        valid_states = {state.value for state in CleanupActionState}
        action_ids: set[str] = set()
        for item in data["actions"]:
            if not isinstance(item, Mapping) or not isinstance(item.get("actionId"), str) or not item.get("actionId"):
                raise LedgerError("cleanup ledger action is invalid")
            if item.get("state", CleanupActionState.PENDING.value) not in valid_states:
                raise LedgerError("cleanup ledger action state is invalid")
            action_id = item["actionId"]
            if action_id in action_ids:
                raise LedgerError("cleanup ledger contains duplicate action")
            action_ids.add(action_id)
            if item.get("actionType") is not None and (
                not isinstance(item.get("actionType"), str) or not item.get("actionType")
            ):
                raise LedgerError("cleanup ledger actionType is invalid")
            if item.get("payload") is not None and not isinstance(item.get("payload"), Mapping):
                raise LedgerError("cleanup ledger action payload must be an object")
            if item.get("attempts") is not None:
                _non_negative(item["attempts"], "cleanup action attempts")
        if data["status"] == cls.STATUS_COMPLETE and any(
            item.get("state", CleanupActionState.PENDING.value) != CleanupActionState.DONE.value
            for item in data["actions"]
        ):
            raise LedgerError("complete cleanup ledger contains unresolved actions")
        return _safe_copy(data)

    @property
    def run_id(self) -> str:
        return str(self._data["runId"])

    @property
    def target_id(self) -> str | None:
        with self._lock:
            value = self._data.get("targetId")
            return None if value is None else str(value)

    @property
    def generation(self) -> int | None:
        with self._lock:
            value = self._data.get("generation")
            return None if value is None else int(value)

    @property
    def status(self) -> str:
        with self._lock:
            return str(self._data["status"])

    @property
    def pending(self) -> bool:
        return self.status == self.STATUS_PENDING

    @property
    def complete(self) -> bool:
        return self.status == self.STATUS_COMPLETE

    @property
    def data(self) -> dict[str, Any]:
        with self._lock:
            return _safe_copy(self._data)

    def snapshot(self) -> dict[str, Any]:
        return self.data

    @property
    def reservation(self) -> dict[str, Any]:
        with self._lock:
            value = dict(self._data.get("reservation", {}))
            for key in self._RESERVED - {"reservation"}:
                if key in self._data and key not in value:
                    value[key] = self._data[key]
            return _safe_copy(value)

    def _touch(self) -> None:
        self._data["updatedAt"] = time.time()

    def _persist(self) -> None:
        with self._lock:
            try:
                payload = json.dumps(self._data, ensure_ascii=False, allow_nan=False, sort_keys=True, indent=2).encode("utf-8")
            except (TypeError, ValueError) as exc:
                raise LedgerError(f"cleanup ledger is not JSON-safe: {exc}") from exc
            parent = self.path.parent
            temporary: str | None = None
            try:
                parent.mkdir(parents=True, exist_ok=True)
                if os.name != "nt":
                    try:
                        os.chmod(parent, 0o700)
                    except OSError:
                        pass
                fd, temporary = tempfile.mkstemp(prefix=f".{self.path.name}.", suffix=".tmp", dir=parent)
                if os.name != "nt":
                    try:
                        os.fchmod(fd, 0o600)
                    except OSError:
                        pass
                with os.fdopen(fd, "wb") as stream:
                    stream.write(payload)
                    stream.flush()
                    os.fsync(stream.fileno())
                os.replace(temporary, self.path)
                temporary = None
                if os.name != "nt":
                    try:
                        os.chmod(self.path, 0o600)
                    except OSError:
                        pass
                try:
                    directory_fd = os.open(parent, os.O_RDONLY)
                except OSError:
                    directory_fd = None
                if directory_fd is not None:
                    try:
                        os.fsync(directory_fd)
                    except OSError:
                        pass
                    finally:
                        os.close(directory_fd)
            except (OSError, ValueError) as exc:
                raise LedgerError(f"cannot persist cleanup ledger {self.path}: {exc}") from exc
            finally:
                if temporary is not None:
                    try:
                        os.unlink(temporary)
                    except (FileNotFoundError, OSError):
                        pass

    def reload(self) -> "CleanupLedger":
        with self._lock:
            self._data = self._read(self.path)
        return self

    def update_metadata(self, values: Mapping[str, Any]) -> None:
        if not isinstance(values, Mapping):
            raise TypeError("values must be a mapping")
        with self._lock:
            self._data.update(_safe_copy(dict(values)))
            self._touch()
            self._persist()

    def reserve(self, **values: Any) -> None:
        normalized: dict[str, Any] = {}
        for key, value in values.items():
            canonical = self._ALIASES.get(key, key)
            if canonical not in self._RESERVED:
                raise LedgerError(f"unsupported reservation field: {key}")
            if canonical == "reservation":
                if not isinstance(value, Mapping):
                    raise LedgerError("reservation must be an object")
                for nested_key, nested_value in value.items():
                    nested_canonical = self._ALIASES.get(nested_key, nested_key)
                    if nested_canonical not in self._RESERVED - {"reservation"}:
                        raise LedgerError(f"unsupported reservation field: {nested_key}")
                    normalized[nested_canonical] = nested_value
            else:
                normalized[canonical] = value
        if "runnerPid" in normalized and (
            isinstance(normalized["runnerPid"], bool)
            or not isinstance(normalized["runnerPid"], int)
            or normalized["runnerPid"] <= 0
        ):
            raise LedgerError("runnerPid must be a positive integer")
        if "generation" in normalized:
            _non_negative(normalized["generation"], "generation")
        if "adbForwardPort" in normalized and (
            isinstance(normalized["adbForwardPort"], bool)
            or not isinstance(normalized["adbForwardPort"], int)
            or not 0 < normalized["adbForwardPort"] <= 65535
        ):
            raise LedgerError("adbForwardPort must be a valid TCP port")
        copied = _safe_copy(normalized)
        with self._lock:
            reservation = dict(self._data.get("reservation", {}))
            reservation.update(copied)
            self._data["reservation"] = reservation
            self._data.update(copied)
            self._data["status"] = self.STATUS_PENDING
            self._ensure_reservation_actions()
            self._touch()
            self._persist()

    reserve_start = reserve

    def reserve_process(self, pid: int, create_time: Any = None) -> None:
        values: dict[str, Any] = {"runnerPid": pid}
        if create_time is not None:
            values["runnerCreateTime"] = create_time
        self.reserve(**values)

    reserve_runner = reserve_process

    def _resource_index(self, resource_type: str, resource_id: str) -> int | None:
        for index, item in enumerate(self._data["resources"]):
            if item.get("resourceType") == resource_type and item.get("resourceId") == resource_id:
                return index
        return None

    @staticmethod
    def _resource_action_id(resource_type: str, resource_id: str) -> str:
        return f"resource:{resource_type}:{resource_id}"

    def _upsert_action(self, action_id: str, action_type: str, payload: Mapping[str, Any] | None = None) -> dict[str, Any]:
        for item in self._data["actions"]:
            if item.get("actionId") == action_id:
                return item
        now = time.time()
        item = {
            "actionId": str(action_id),
            "actionType": str(action_type),
            "payload": _safe_copy(dict(payload or {})),
            "state": CleanupActionState.PENDING.value,
            "attempts": 0,
            "lastError": None,
            "result": None,
            "createdAt": now,
            "updatedAt": now,
        }
        self._data["actions"].append(item)
        return item

    def _ensure_resource_action(self, item: Mapping[str, Any]) -> dict[str, Any]:
        return self._upsert_action(
            self._resource_action_id(str(item["resourceType"]), str(item["resourceId"])),
            "resource",
            {
                "resourceType": str(item["resourceType"]),
                "resourceId": str(item["resourceId"]),
                "metadata": dict(item.get("metadata", {})),
            },
        )

    def record_resource_created(self, resource_type: str, resource_id: str, *, metadata: Mapping[str, Any] | None = None) -> bool:
        resource_type, resource_id = str(resource_type), str(resource_id)
        if not resource_type or not resource_id:
            raise ValueError("resource_type and resource_id are required")
        metadata = _safe_copy(dict(metadata or {}))
        with self._lock:
            index = self._resource_index(resource_type, resource_id)
            if index is not None:
                item = self._data["resources"][index]
                if item.get("released") is True:
                    return False
                prior = dict(item.get("metadata", {}))
                prior.update(metadata)
                changed = prior != item.get("metadata", {})
                item["metadata"] = prior
                action_count = len(self._data["actions"])
                self._ensure_resource_action(item)
                # Older journals may contain a resource without its derived
                # cleanup action.  Persist that repair even when the duplicate
                # ``resource.created`` carried no new metadata.
                changed = changed or len(self._data["actions"]) != action_count
            else:
                item = {
                    "resourceType": resource_type,
                    "resourceId": resource_id,
                    "released": False,
                    "metadata": metadata,
                    "createdAt": time.time(),
                }
                self._data["resources"].append(item)
                self._ensure_resource_action(item)
                changed = True
            if changed:
                self._touch()
                self._persist()
            return changed

    record_resource = record_resource_created

    def record_resource_released(self, resource_type: str, resource_id: str) -> bool:
        resource_type, resource_id = str(resource_type), str(resource_id)
        if not resource_type or not resource_id:
            raise ValueError("resource_type and resource_id are required")
        with self._lock:
            index = self._resource_index(resource_type, resource_id)
            if index is None:
                item = {
                    "resourceType": resource_type,
                    "resourceId": resource_id,
                    "released": True,
                    "metadata": {},
                    "createdAt": time.time(),
                }
                self._data["resources"].append(item)
            else:
                item = self._data["resources"][index]
                if item.get("released") is True:
                    return False
                item["released"] = True
            action = self._ensure_resource_action(item)
            self._mark_action(action, CleanupActionState.DONE, {"released": True})
            self._touch()
            self._persist()
            return True

    release_resource = record_resource_released

    def record_resource_event(self, event: Mapping[str, Any]) -> bool:
        if not isinstance(event, Mapping):
            raise TypeError("resource event must be a mapping")
        event_type = event.get("type")
        if event_type == "resource.created":
            return self.record_resource_created(
                str(event.get("resourceType", "")),
                str(event.get("resourceId", "")),
                metadata=event.get("metadata") if isinstance(event.get("metadata"), Mapping) else {},
            )
        if event_type == "resource.released":
            return self.record_resource_released(str(event.get("resourceType", "")), str(event.get("resourceId", "")))
        raise LedgerError(f"unsupported resource event type: {event_type}")

    @property
    def resources(self) -> list[dict[str, Any]]:
        with self._lock:
            return _safe_copy(self._data["resources"])

    def active_resources(self) -> list[dict[str, Any]]:
        return [item for item in self.resources if not item.get("released", False)]

    def _check_run(self, value: Mapping[str, Any]) -> None:
        event_run = value.get("runId", value.get("run_id"))
        if event_run is not None and str(event_run) != self.run_id:
            raise LedgerError("event runId does not match cleanup ledger")

    def record_config_delta(self, delta: Mapping[str, Any]) -> bool:
        if not isinstance(delta, Mapping):
            raise TypeError("delta must be a mapping")
        value = dict(delta)
        self._check_run(value)
        delta_id = value.get("deltaId", value.get("delta_id"))
        if not isinstance(delta_id, str) or not delta_id:
            raise LedgerError("config delta requires deltaId")
        sequence = value.get("seq")
        if sequence is not None:
            sequence = _non_negative(sequence, "config delta seq")
        value.update({"deltaId": delta_id, "runId": self.run_id})
        if sequence is not None:
            value["seq"] = sequence
        copied = _safe_copy(value)
        with self._lock:
            if any(item.get("deltaId") == delta_id or (sequence is not None and item.get("seq") == sequence) for item in self._data["configDeltas"]):
                return False
            copied["receivedAt"] = time.time()
            self._data["configDeltas"].append(copied)
            self._touch()
            self._persist()
            return True

    record_delta = record_config_delta

    @property
    def config_deltas(self) -> list[dict[str, Any]]:
        with self._lock:
            return _safe_copy(self._data["configDeltas"])

    def record_final_seq(self, final_seq: int) -> bool:
        sequence = _non_negative(final_seq, "finalSeq")
        assert sequence is not None
        with self._lock:
            if self._data.get("finalSeq") is not None and int(self._data["finalSeq"]) >= sequence:
                return False
            self._data["finalSeq"] = sequence
            self._touch()
            self._persist()
            return True

    set_final_seq = record_final_seq

    @property
    def final_seq(self) -> int | None:
        with self._lock:
            value = self._data.get("finalSeq")
            return None if value is None else int(value)

    def record_finished(self, finished: Mapping[str, Any]) -> bool:
        if not isinstance(finished, Mapping):
            raise TypeError("finished must be a mapping")
        value = dict(finished)
        self._check_run(value)
        value["runId"] = self.run_id
        if value.get("seq") is not None:
            value["seq"] = _non_negative(value["seq"], "finished seq")
        copied = _safe_copy(value)
        with self._lock:
            if self._data.get("finished") is not None:
                if self._data["finished"] == copied:
                    return False
                raise LedgerError("conflicting finished event for cleanup ledger")
            self._data["finished"] = copied
            sequence = copied.get("seq")
            if sequence is not None and (self._data.get("finalSeq") is None or sequence > self._data["finalSeq"]):
                self._data["finalSeq"] = sequence
            self._touch()
            self._persist()
            return True

    record_finished_event = record_finished

    @property
    def finished(self) -> dict[str, Any] | None:
        with self._lock:
            value = self._data.get("finished")
            return None if value is None else _safe_copy(value)

    def _normalise_action(
        self,
        action: Mapping[str, Any] | str,
        *,
        action_type: str | None = None,
        payload: Mapping[str, Any] | None = None,
        **fields: Any,
    ) -> tuple[str, str, dict[str, Any]]:
        if isinstance(action, Mapping):
            value = dict(action)
            action_id = value.pop("actionId", value.pop("action_id", value.pop("id", None)))
            action_type = value.pop("actionType", value.pop("action_type", action_type))
            embedded = value.pop("payload", None)
            if payload is None and isinstance(embedded, Mapping):
                payload = dict(embedded)
            if action_type is None:
                action_type = value.pop("type", None)
            fields = {**value, **fields}
        else:
            action_id = action
        if not isinstance(action_id, str) or not action_id:
            raise LedgerError("cleanup action requires actionId")
        if not isinstance(action_type, str) or not action_type:
            raise LedgerError("cleanup action requires actionType")
        result = dict(payload or {})
        result.update(fields)
        return action_id, action_type, _safe_copy(result)

    def record_action_pending(
        self,
        action: Mapping[str, Any] | str,
        *,
        action_type: str | None = None,
        payload: Mapping[str, Any] | None = None,
        **fields: Any,
    ) -> CleanupActionResult:
        action_id, action_type, payload = self._normalise_action(action, action_type=action_type, payload=payload, **fields)
        with self._lock:
            item = self._upsert_action(action_id, action_type, payload)
            if item["state"] != CleanupActionState.DONE.value:
                item["state"] = CleanupActionState.PENDING.value
                item["lastError"] = None
                item["updatedAt"] = time.time()
            self._touch()
            self._persist()
            return self._action_result(item)

    request_action = record_action_pending
    add_action = record_action_pending

    def _action_index(self, action_id: str) -> int | None:
        for index, item in enumerate(self._data["actions"]):
            if item.get("actionId") == action_id:
                return index
        return None

    @staticmethod
    def _action_result(item: Mapping[str, Any]) -> CleanupActionResult:
        return CleanupActionResult(
            str(item["actionId"]),
            CleanupActionState(str(item.get("state", CleanupActionState.PENDING.value))),
            copy.deepcopy(item.get("result") if item.get("result") is not None else item.get("lastError")),
            int(item.get("attempts", 0)),
        )

    def _mark_action(self, item: dict[str, Any], state: CleanupActionState, detail: Any = None) -> CleanupActionResult:
        item["state"] = state.value
        item["updatedAt"] = time.time()
        if state is CleanupActionState.EXECUTING:
            item["attempts"] = int(item.get("attempts", 0)) + 1
        elif state is CleanupActionState.DONE:
            item["result"] = _safe_copy(detail)
            item["lastError"] = None
            item["completedAt"] = time.time()
        elif state is CleanupActionState.FAILED:
            item["lastError"] = _safe_copy(detail)
        elif state is CleanupActionState.PENDING and detail is not None:
            item["lastError"] = _safe_copy(detail)
        return self._action_result(item)

    def set_action_state(self, action_id: str, state: CleanupActionState | str, detail: Any = None) -> CleanupActionResult:
        with self._lock:
            index = self._action_index(str(action_id))
            if index is None:
                raise LedgerError(f"unknown cleanup action: {action_id}")
            result = self._mark_action(self._data["actions"][index], CleanupActionState(state), detail)
            self._touch()
            self._persist()
            return result

    mark_action = set_action_state

    def mark_action_pending(self, action_id: str, detail: Any = None) -> CleanupActionResult:
        return self.set_action_state(action_id, CleanupActionState.PENDING, detail)

    def mark_action_executing(self, action_id: str) -> CleanupActionResult:
        return self.set_action_state(action_id, CleanupActionState.EXECUTING)

    def mark_action_done(self, action_id: str, detail: Any = None) -> CleanupActionResult:
        return self.set_action_state(action_id, CleanupActionState.DONE, detail)

    def mark_action_failed(self, action_id: str, detail: Any = None) -> CleanupActionResult:
        return self.set_action_state(action_id, CleanupActionState.FAILED, detail)

    @property
    def actions(self) -> list[dict[str, Any]]:
        with self._lock:
            return _safe_copy(self._data["actions"])

    def action(self, action_id: str) -> dict[str, Any] | None:
        with self._lock:
            index = self._action_index(str(action_id))
            return None if index is None else _safe_copy(self._data["actions"][index])

    def _ensure_reservation_actions(self) -> None:
        reservation = self.reservation

        def ensure(action_id: str, action_type: str, payload: Mapping[str, Any]) -> None:
            self._upsert_action(action_id, action_type, payload)

        if reservation.get("runnerPid") is not None:
            ensure(f"{self.run_id}:runner-process", "runner.process", {"pid": reservation.get("runnerPid"), "createTime": reservation.get("runnerCreateTime")})
        if reservation.get("scid") or reservation.get("socketName"):
            ensure(
                f"{self.run_id}:scrcpy-server",
                "scrcpy.server",
                {key: reservation.get(key) for key in ("scid", "socketName", "deviceSerial") if reservation.get(key) is not None},
            )
        if reservation.get("adbForwardPort") is not None:
            ensure(
                f"{self.run_id}:adb-forward",
                "adb.forward",
                {key: reservation.get(key) for key in ("adbForwardPort", "deviceSerial") if reservation.get(key) is not None},
            )
        if reservation.get("hwnd") is not None and (reservation.get("originalWindowState") is not None or reservation.get("originalWindowRect") is not None):
            ensure(
                f"{self.run_id}:window-restore",
                "window.restore",
                {key: reservation.get(key) for key in ("hwnd", "originalWindowState", "originalWindowRect") if reservation.get(key) is not None},
            )
        obligations = reservation.get("inputObligations", reservation.get("inputState"))
        if obligations:
            ensure(f"{self.run_id}:input-release", "input.release", {"obligations": obligations})

    def record_cleanup_step(self, step: str, *, outcome: str, detail: Any = None) -> None:
        if not step:
            raise ValueError("step is required")
        if outcome not in {"pending", "done", "failed", "skipped"}:
            raise ValueError("invalid cleanup step outcome")
        with self._lock:
            self._data["cleanup"].append({"step": str(step), "outcome": outcome, "detail": _safe_copy(detail), "at": time.time()})
            self._touch()
            self._persist()

    def cleanup_steps(self) -> list[dict[str, Any]]:
        with self._lock:
            return _safe_copy(self._data["cleanup"])

    @staticmethod
    def _invoke_executor(executor: Any, action: Mapping[str, Any], ledger: "CleanupLedger") -> Any:
        if isinstance(executor, Mapping):
            callback = executor.get(action.get("actionType"), executor.get("*"))
        else:
            callback = getattr(executor, "execute", None)
            if not callable(callback) and callable(executor):
                callback = executor
        if not callable(callback):
            raise LedgerError("cleanup executor is not callable")
        try:
            signature = inspect.signature(callback)
        except (TypeError, ValueError):
            return callback(action)
        for args, kwargs in (((action,), {"ledger": ledger}), ((action,), {}), ((action, ledger), {})):
            try:
                signature.bind(*args, **kwargs)
            except TypeError:
                continue
            return callback(*args, **kwargs)
        raise LedgerError("cleanup executor has an unsupported signature")

    @staticmethod
    def _normalize_result(action_id: str, attempts: int, value: Any) -> CleanupActionResult:
        if isinstance(value, CleanupActionResult):
            return CleanupActionResult(action_id, value.state, value.detail, attempts)
        if value is None or value is True:
            return CleanupActionResult(action_id, CleanupActionState.DONE, None, attempts)
        if value is False:
            return CleanupActionResult(action_id, CleanupActionState.FAILED, {"code": "CLEANUP_ACTION_FAILED"}, attempts)
        if isinstance(value, Mapping):
            raw = value.get("state", value.get("status", value.get("outcome")))
            if raw is None and "ok" in value:
                raw = "done" if value.get("ok") else "failed"
            raw = str(raw or "done").lower()
            if raw in {"done", "ok", "success", "complete", "completed"}:
                state = CleanupActionState.DONE
            elif raw in {"pending", "retry", "deferred", "executing", "in_progress"}:
                state = CleanupActionState.PENDING
            else:
                state = CleanupActionState.FAILED
            detail = value.get("detail", value.get("error", dict(value)))
            return CleanupActionResult(action_id, state, detail, attempts)
        return CleanupActionResult(action_id, CleanupActionState.DONE, value, attempts)

    def recover(
        self,
        executor: CleanupActionExecutor | Callable[..., Any] | Mapping[str, Callable[..., Any]] | None = None,
        *,
        delete_complete: bool = True,
        retry_failed: bool = True,
        stop_on_error: bool = False,
        deadline: float | None = None,
    ) -> CleanupRecoveryResult:
        with self._lock:
            if self.complete:
                if delete_complete:
                    self._delete_locked()
                return CleanupRecoveryResult(self.STATUS_COMPLETE, True)
            before = len(self._data["actions"])
            self._ensure_reservation_actions()
            changed = len(self._data["actions"]) != before
            for item in self._data["actions"]:
                if item.get("state") == CleanupActionState.EXECUTING.value:
                    self._mark_action(item, CleanupActionState.PENDING, {"code": "RECOVERY_RETRY"})
                    changed = True
            if changed:
                self._touch()
                self._persist()
        if executor is None:
            # A journal can already be clean when all obligations were
            # acknowledged by event adapters (for example, a released resource
            # recorded just before a sidecar restart).  Do not leave such a run
            # permanently ``cleanup_pending`` merely because there is no action
            # callback to invoke.
            with self._lock:
                all_done = all(item.get("state") == CleanupActionState.DONE.value for item in self._data["actions"])
                if all_done:
                    self._data["status"] = self.STATUS_COMPLETE
                    self._touch()
                    self._persist()
                    if delete_complete:
                        self._delete_locked()
                    return CleanupRecoveryResult(self.STATUS_COMPLETE, True)
            return CleanupRecoveryResult(self.STATUS_PENDING, False, tuple(self._action_result(item) for item in self.actions))

        results: list[CleanupActionResult] = []
        errors: list[dict[str, Any]] = []
        for action in self.actions:
            state = action.get("state")
            if state == CleanupActionState.DONE.value or (state == CleanupActionState.FAILED.value and not retry_failed):
                results.append(self._action_result(action))
                continue
            if deadline is not None and time.monotonic() >= deadline:
                detail = {"code": "CLEANUP_DEADLINE_EXPIRED", "message": "cleanup deadline expired"}
                results.append(self.mark_action_pending(str(action["actionId"]), detail))
                errors.append(detail)
                break
            action_id = str(action["actionId"])
            self.mark_action_executing(action_id)
            try:
                value = self._invoke_executor(executor, action, self)
                current = self.action(action_id) or {"attempts": 1}
                normalized = self._normalize_result(action_id, int(current.get("attempts", 1)), value)
            except TimeoutError as exc:
                current = self.action(action_id) or {"attempts": 1}
                normalized = CleanupActionResult(action_id, CleanupActionState.PENDING, {"code": "CLEANUP_ACTION_TIMEOUT", "message": str(exc)}, int(current.get("attempts", 1)))
            except Exception as exc:
                current = self.action(action_id) or {"attempts": 1}
                normalized = CleanupActionResult(action_id, CleanupActionState.FAILED, {"code": "CLEANUP_ACTION_FAILED", "message": str(exc)}, int(current.get("attempts", 1)))
            results.append(self.set_action_state(action_id, normalized.state, normalized.detail))
            if normalized.state is not CleanupActionState.DONE:
                error = dict(normalized.detail) if isinstance(normalized.detail, Mapping) else {"code": "CLEANUP_ACTION_FAILED", "message": str(normalized.detail)}
                errors.append(error)
                if stop_on_error:
                    break

        with self._lock:
            all_done = all(item.get("state") == CleanupActionState.DONE.value for item in self._data["actions"])
            if all_done:
                self._data["status"] = self.STATUS_COMPLETE
                self._touch()
                self._persist()
                if delete_complete:
                    self._delete_locked()
                return CleanupRecoveryResult(self.STATUS_COMPLETE, True, tuple(results), tuple(errors))
            self._data["status"] = self.STATUS_PENDING
            self._touch()
            self._persist()
            return CleanupRecoveryResult(self.STATUS_PENDING, False, tuple(results), tuple(errors))

    execute_cleanup = recover

    def mark_complete(self, *, delete: bool = False, require_clean: bool = False) -> None:
        with self._lock:
            # A complete journal is safe to delete on the next startup.  It must
            # therefore never be persisted while an obligation is unresolved;
            # retain ``require_clean`` as a compatibility keyword but fail closed
            # regardless of its value.
            if any(item.get("state") != CleanupActionState.DONE.value for item in self._data["actions"]):
                raise LedgerError("cleanup actions are unresolved")
            self._data["status"] = self.STATUS_COMPLETE
            self._touch()
            self._persist()
            if delete:
                self._delete_locked()

    complete_cleanup = mark_complete

    def _delete_locked(self) -> None:
        try:
            self.path.unlink()
        except FileNotFoundError:
            pass
        except OSError as exc:
            raise LedgerError(f"cannot delete cleanup ledger: {exc}") from exc

    def delete(self) -> None:
        with self._lock:
            self._delete_locked()

    @classmethod
    def scan(cls, directory: str | os.PathLike[str], *, include_complete: bool = False) -> list["CleanupLedger"]:
        root = Path(directory)
        if not root.exists():
            return []
        ledgers = [cls.load(path) for path in sorted(root.glob("*.json"))]
        return [ledger for ledger in ledgers if include_complete or ledger.pending]

    @classmethod
    def scan_pending(cls, directory: str | os.PathLike[str]) -> list["CleanupLedger"]:
        return cls.scan(directory)

    @classmethod
    def recover_pending(cls, directory: str | os.PathLike[str], executor: Any = None, **kwargs: Any) -> list[CleanupRecoveryResult]:
        return [ledger.recover(executor, **kwargs) for ledger in cls.scan_pending(directory)]

    recover_all = recover_pending
    startup_recover = recover_pending


CleanupJournal = CleanupLedger

__all__ = [
    "CleanupActionExecutor",
    "CleanupActionResult",
    "CleanupActionState",
    "CleanupExecutor",
    "CleanupLedger",
    "CleanupLedgerError",
    "CleanupJournal",
    "CleanupRecoveryResult",
    "LedgerError",
    "ResourceKey",
]
