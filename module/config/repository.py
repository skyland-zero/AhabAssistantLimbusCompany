"""Transactional configuration snapshots for sidecar/Runner execution.

``Config`` remains the compatibility facade used by the existing application.
This module adds the narrower repository contract needed by the execution
supervisor without changing the legacy ``cfg.get_value``/``cfg.set_value``
callers.  The repository owns revision/hash bookkeeping and is the only place
that applies Runner ``config.delta`` messages.
"""

from __future__ import annotations

import copy
import hashlib
import json
import math
import os
import re
import shutil
import tempfile
import threading
from collections.abc import Mapping, Sequence
from contextlib import nullcontext
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

from pydantic import BaseModel, ValidationError
from ruamel.yaml import YAML, YAMLError

from core.atomic_write import atomic_dump_yaml

from .redaction import SENSITIVE_CONFIG_KEYS


class ConfigRepositoryError(RuntimeError):
    """Base error for repository, snapshot, and persistence failures."""


class ConfigDeltaError(ConfigRepositoryError):
    """A Runner delta failed envelope, whitelist, or value validation."""


class ConfigConflictError(ConfigRepositoryError):
    """The in-memory or on-disk configuration changed concurrently."""


_ALLOWED_CHANGE_FIELDS = frozenset(
    {
        "last_auto_change",
        "hard_mirror",
        "hard_mirror_chance",
        "set_win_size",
    }
)
_ALLOWED_OPERATIONS = frozenset({"rotateTeamQueue"})
_DELTA_FIELDS = frozenset(
    {
        "type",
        "runId",
        "seq",
        "deltaId",
        "baseRevision",
        "baseConfigHash",
        "changes",
        "operations",
    }
)
_WINDOW_HEIGHTS = frozenset({720, 900, 1080, 1440, 1800, 2160})
_MISSING = object()


def _normalise_value(value: Any) -> Any:
    """Convert config-shaped values to deterministic JSON-compatible data."""

    if isinstance(value, BaseModel):
        value = value.model_dump(mode="python")
    if isinstance(value, Mapping):
        normalised: dict[str, Any] = {}
        for raw_key, nested in value.items():
            if not isinstance(raw_key, (str, int, float, bool)):
                raise ConfigRepositoryError(f"配置键不可序列化：{raw_key!r}")
            key = str(raw_key)
            if key in normalised:
                raise ConfigRepositoryError(f"配置键规范化后冲突：{key}")
            normalised[key] = _normalise_value(nested)
        return normalised
    if isinstance(value, (list, tuple)):
        return [_normalise_value(item) for item in value]
    if isinstance(value, set):
        # Sets are not part of ConfigModel, but accepting them here keeps the
        # wrapper useful for small test doubles without making hash order
        # observable in the canonical representation.
        return [_normalise_value(item) for item in sorted(value, key=repr)]
    if value is None or isinstance(value, (str, bool, int)):
        return value
    if isinstance(value, float):
        if not math.isfinite(value):
            raise ConfigRepositoryError("配置中不允许 NaN 或无穷浮点数")
        return value
    raise ConfigRepositoryError(f"配置值不可序列化：{type(value).__name__}")


def _canonical_json(value: Mapping[str, Any]) -> bytes:
    try:
        return json.dumps(
            value,
            ensure_ascii=False,
            sort_keys=True,
            separators=(",", ":"),
            allow_nan=False,
        ).encode("utf-8")
    except (TypeError, ValueError) as error:
        raise ConfigRepositoryError(f"配置无法生成规范化快照：{error}") from error


def _canonical_hash(value: Mapping[str, Any]) -> str:
    return hashlib.sha256(_canonical_json(value)).hexdigest()


def _without_sensitive(value: Any, sensitive_keys: frozenset[str]) -> Any:
    if isinstance(value, Mapping):
        return {
            key: _without_sensitive(nested, sensitive_keys)
            for key, nested in value.items()
            if str(key).casefold() not in sensitive_keys
        }
    if isinstance(value, list):
        return [_without_sensitive(item, sensitive_keys) for item in value]
    return copy.deepcopy(value)


@dataclass
class _RunContext:
    run_id: str
    directory: Path
    config_path: Path
    revision: int
    base_hash: str
    snapshot: dict[str, Any]
    baseline_values: dict[str, Any]
    baseline_queue: list[int]
    baseline_teams: tuple[tuple[str, int], ...]
    seen_delta_ids: set[str] = field(default_factory=set)
    seen_sequences: set[int] = field(default_factory=set)


class ConfigRepository:
    """Wrap an existing :class:`Config` with a transactional Runner API.

    The wrapped object is intentionally duck-typed.  A production ``Config``
    instance is supported, while a small object exposing ``config``,
    ``config_path`` and ``save`` is enough for integration tests.  Existing
    callers continue to use the original object directly.
    """

    def __init__(
        self,
        config: Any | None = None,
        *,
        temp_root: str | Path | None = None,
        sensitive_keys: Sequence[str] | None = None,
    ) -> None:
        if config is None:
            # Import lazily so importing ``module.config.repository`` cannot
            # create the global Config singleton while package initialisation
            # is still defining ``cfg``.
            from module.config import cfg

            config = cfg
        self.config = config
        raw_path = getattr(config, "config_path", None)
        if raw_path is None:
            raise ValueError("ConfigRepository requires config.config_path")
        self.config_path = Path(raw_path).expanduser().resolve()
        self.temp_root = (
            Path(temp_root).expanduser().resolve()
            if temp_root is not None
            else (self.config_path.parent / ".aalc-runs").resolve()
        )
        configured_sensitive = {str(key).casefold() for key in (sensitive_keys or SENSITIVE_CONFIG_KEYS)}
        self._sensitive_keys = frozenset(configured_sensitive)
        self._lock = threading.RLock()
        self._yaml = YAML()
        self._revision = 0
        self._last_snapshot = self._read_config_snapshot()
        self._canonical_hash = _canonical_hash(self._last_snapshot)
        self._file_hash = self._read_file_hash()
        self._runs: dict[str, _RunContext] = {}

    # ------------------------------------------------------------------
    # Snapshot and revision API
    # ------------------------------------------------------------------
    @property
    def revision(self) -> int:
        with self._lock:
            self._synchronise_locked()
            return self._revision

    @property
    def config_revision(self) -> int:
        return self.revision

    @property
    def canonical_hash(self) -> str:
        with self._lock:
            self._synchronise_locked()
            return self._canonical_hash

    @property
    def canonical_sha256(self) -> str:
        return self.canonical_hash

    @property
    def file_hash(self) -> str | None:
        with self._lock:
            self._synchronise_locked()
            return self._file_hash

    def snapshot(self, *, include_sensitive: bool = False) -> dict[str, Any]:
        """Return a fresh normalized snapshot.

        Sensitive root or nested keys are excluded by default.  The complete
        canonical hash remains available through :attr:`canonical_hash` and is
        never itself a disclosure of credential contents.
        """

        with self._lock:
            self._synchronise_locked()
            snapshot = self._last_snapshot if include_sensitive else _without_sensitive(
                self._last_snapshot, self._sensitive_keys
            )
            return copy.deepcopy(snapshot)

    def runner_snapshot(self) -> dict[str, Any]:
        """Alias for the safe snapshot intended for a Runner config file."""

        return self.snapshot()

    def authoritative_snapshot(self) -> dict[str, Any]:
        """Return the complete normalized sidecar snapshot."""

        return self.snapshot(include_sensitive=True)

    def state_snapshot(self) -> dict[str, Any]:
        with self._lock:
            self._synchronise_locked()
            return {
                "revision": self._revision,
                "configRevision": self._revision,
                "canonicalHash": self._canonical_hash,
                "baseConfigHash": self._canonical_hash,
                "snapshot": copy.deepcopy(_without_sensitive(self._last_snapshot, self._sensitive_keys)),
            }

    def refresh_from_disk(self) -> dict[str, Any]:
        """Refresh after an external edit, or raise a conflict if unsafe."""

        with self._lock:
            self._synchronise_locked()
            return copy.deepcopy(_without_sensitive(self._last_snapshot, self._sensitive_keys))

    # ------------------------------------------------------------------
    # Temporary Runner config files
    # ------------------------------------------------------------------
    def create_run_config(
        self,
        run_id: str,
        *,
        temp_root: str | Path | None = None,
    ) -> dict[str, Any]:
        """Write a run-private, secret-free ``config.yaml`` and register it."""

        self._validate_run_id(run_id)
        with self._lock:
            if run_id in self._runs:
                raise ConfigRepositoryError(f"runId 已存在：{run_id}")
            self._synchronise_locked()
            snapshot = copy.deepcopy(self._last_snapshot)
            safe_snapshot = _without_sensitive(snapshot, self._sensitive_keys)
            root = Path(temp_root).expanduser().resolve() if temp_root is not None else self.temp_root
            root.mkdir(parents=True, exist_ok=True)
            self._restrict_mode(root, 0o700)
            directory: Path | None = None
            try:
                safe_name = re.sub(r"[^A-Za-z0-9_.-]+", "_", run_id)[:48] or "run"
                directory = Path(tempfile.mkdtemp(prefix=f"{safe_name}-", dir=root)).resolve()
                self._restrict_mode(directory, 0o700)
                config_path = directory / "config.yaml"
                atomic_dump_yaml(self._yaml, config_path, safe_snapshot)
                self._restrict_mode(config_path, 0o600)
            except Exception:
                if directory is not None:
                    shutil.rmtree(directory, ignore_errors=True)
                raise

            context = _RunContext(
                run_id=run_id,
                directory=directory,
                config_path=config_path,
                revision=self._revision,
                base_hash=self._canonical_hash,
                snapshot=copy.deepcopy(snapshot),
                baseline_values={
                    key: copy.deepcopy(snapshot[key]) if key in snapshot else _MISSING for key in _ALLOWED_CHANGE_FIELDS
                },
                baseline_queue=self._queue_from_snapshot(snapshot),
                baseline_teams=self._team_signature(snapshot),
            )
            self._runs[run_id] = context
            return {
                "runId": run_id,
                "configPath": str(config_path),
                "configRevision": self._revision,
                "baseRevision": self._revision,
                "baseConfigHash": self._canonical_hash,
                "snapshot": copy.deepcopy(safe_snapshot),
            }

    # Common integration spellings kept as aliases rather than duplicate state.
    create_runner_config = create_run_config
    create_execution_config = create_run_config

    def cleanup_run_config(self, run: str | Mapping[str, Any]) -> bool:
        """Remove one registered run directory, refusing untracked paths."""

        with self._lock:
            run_id = run.get("runId") if isinstance(run, Mapping) else run
            if not isinstance(run_id, str):
                raise ConfigRepositoryError("cleanup_run_config 需要 runId")
            context = self._runs.get(run_id)
            if context is None:
                return False
            try:
                shutil.rmtree(context.directory)
            except FileNotFoundError:
                pass
            except OSError as error:
                raise ConfigRepositoryError(f"清理 Runner 临时目录失败：{error}") from error
            self._runs.pop(run_id, None)
            return True

    remove_run_config = cleanup_run_config
    cleanup_runner_config = cleanup_run_config

    # ------------------------------------------------------------------
    # Runner config.delta API
    # ------------------------------------------------------------------
    def apply_delta(self, delta: Mapping[str, Any]) -> dict[str, Any]:
        """Validate and merge one Runner ``config.delta`` transaction."""

        with self._lock:
            run_id, sequence, delta_id, base_revision, base_hash, changes, operations = self._validate_delta(delta)
            context = self._runs.get(run_id)
            if context is None:
                raise ConfigDeltaError(f"未知的 config.delta runId：{run_id}")

            if delta_id in context.seen_delta_ids or sequence in context.seen_sequences:
                return self._delta_result(
                    run_id,
                    sequence,
                    delta_id,
                    status="duplicate",
                    applied_changes=[],
                    conflicts=[],
                    warnings=[],
                )

            self._synchronise_locked()
            current = copy.deepcopy(self._last_snapshot)
            revision_matches = base_revision == self._revision and base_hash == self._canonical_hash
            candidate = copy.deepcopy(current)
            applied_changes: list[str] = []
            conflicts: list[str] = []
            warnings: list[str] = []

            for key, value in changes.items():
                if revision_matches or self._same_value(current.get(key, _MISSING), context.baseline_values.get(key, _MISSING)):
                    if current.get(key, _MISSING) != value:
                        candidate[key] = copy.deepcopy(value)
                        applied_changes.append(key)
                else:
                    conflicts.append(key)

            self._merge_queue_operations(
                candidate,
                current,
                context,
                operations,
                revision_matches=revision_matches,
                conflicts=conflicts,
                warnings=warnings,
                applied_changes=applied_changes,
            )
            candidate = _normalise_value(candidate)
            self._validate_candidate(candidate)

            if candidate != current:
                previous = current
                try:
                    self._replace_config_locked(candidate)
                    self._persist_locked(candidate)
                except Exception:
                    # Keep the in-memory facade coherent if an atomic write or
                    # model validation fails after replacement.
                    try:
                        self._replace_config_locked(previous)
                    except Exception:
                        pass
                    raise
                self._last_snapshot = copy.deepcopy(candidate)
                self._canonical_hash = _canonical_hash(candidate)
                self._revision += 1
                self._file_hash = self._read_file_hash()

            context.seen_delta_ids.add(delta_id)
            context.seen_sequences.add(sequence)
            status = "applied" if applied_changes or candidate != current else "accepted"
            return self._delta_result(
                run_id,
                sequence,
                delta_id,
                status=status,
                applied_changes=applied_changes,
                conflicts=conflicts,
                warnings=warnings,
            )

    apply_config_delta = apply_delta
    merge_delta = apply_delta

    def validate_delta(self, delta: Mapping[str, Any]) -> dict[str, Any]:
        """Return normalized delta fields without applying the transaction."""

        run_id, sequence, delta_id, base_revision, base_hash, changes, operations = self._validate_delta(delta)
        return {
            "type": "config.delta",
            "runId": run_id,
            "seq": sequence,
            "deltaId": delta_id,
            "baseRevision": base_revision,
            "baseConfigHash": base_hash,
            "changes": copy.deepcopy(changes),
            "operations": copy.deepcopy(operations),
        }

    # ------------------------------------------------------------------
    # Internal synchronization and persistence
    # ------------------------------------------------------------------
    def _config_lock(self):
        lock = getattr(self.config, "_lock", None)
        return lock if lock is not None else nullcontext()

    def _read_config_snapshot(self) -> dict[str, Any]:
        with self._config_lock():
            config_obj = getattr(self.config, "config", self.config)
            if isinstance(config_obj, BaseModel):
                value = config_obj.model_dump(mode="python")
            elif isinstance(config_obj, Mapping):
                value = config_obj
            else:
                raise ConfigRepositoryError("Config 对象缺少可序列化 config 映射")
        normalised = _normalise_value(value)
        if not isinstance(normalised, dict):  # pragma: no cover - guarded above
            raise ConfigRepositoryError("配置根节点必须是 mapping")
        return normalised

    def _read_file_hash(self) -> str | None:
        try:
            return hashlib.sha256(self.config_path.read_bytes()).hexdigest()
        except FileNotFoundError:
            return None

    def _read_disk_snapshot(self) -> dict[str, Any]:
        try:
            with self.config_path.open("r", encoding="utf-8") as stream:
                loaded = YAML(typ="safe").load(stream)
        except (OSError, YAMLError, UnicodeError) as error:
            raise ConfigConflictError(f"无法读取外部配置变更：{error}") from error
        if loaded is None:
            loaded = {}
        if not isinstance(loaded, Mapping):
            raise ConfigConflictError("外部配置根节点必须是 mapping")

        # A hand-edited file may omit fields that Config would source from the
        # example defaults.  Start with the current complete model so the
        # repository never saves a partial candidate accidentally.
        merged = copy.deepcopy(self._last_snapshot)
        try:
            merged.update(_normalise_value(loaded))
            return self._validate_candidate(_normalise_value(merged))
        except ConfigRepositoryError as error:
            raise ConfigConflictError(f"外部配置变更未通过配置校验：{error}") from error

    def _synchronise_locked(self) -> None:
        current = self._read_config_snapshot()
        current_hash = _canonical_hash(current)
        disk_hash = self._read_file_hash()

        if disk_hash != self._file_hash:
            if disk_hash is None:
                raise ConfigConflictError("配置文件被外部删除，拒绝覆盖")
            disk_snapshot = self._read_disk_snapshot()
            disk_canonical_hash = _canonical_hash(disk_snapshot)
            if current_hash != self._canonical_hash and disk_canonical_hash != current_hash:
                raise ConfigConflictError("配置同时存在未保存的内存修改和外部文件修改")
            if current_hash == self._canonical_hash and disk_canonical_hash != current_hash:
                self._replace_config_locked(disk_snapshot)
                current = disk_snapshot
                current_hash = disk_canonical_hash
                self._revision += 1
                self._last_snapshot = copy.deepcopy(current)
                self._canonical_hash = current_hash
            self._file_hash = disk_hash

        if current_hash != self._canonical_hash:
            self._revision += 1
            self._last_snapshot = copy.deepcopy(current)
            self._canonical_hash = current_hash
        elif not self._last_snapshot:
            self._last_snapshot = copy.deepcopy(current)

    def _replace_config_locked(self, snapshot: Mapping[str, Any]) -> None:
        candidate = _normalise_value(snapshot)
        with self._config_lock():
            config_obj = getattr(self.config, "config", None)
            if isinstance(config_obj, BaseModel):
                try:
                    replacement = type(config_obj)(**candidate)
                except ValidationError as error:
                    raise ConfigDeltaError(f"配置模型校验失败：{error}") from error
                self.config.config = replacement
            elif isinstance(config_obj, dict):
                config_obj.clear()
                config_obj.update(copy.deepcopy(candidate))
            elif hasattr(self.config, "unsaved_set_value"):
                for key, value in candidate.items():
                    self.config.unsaved_set_value(key, copy.deepcopy(value))
            else:
                raise ConfigRepositoryError("Config 对象不支持替换配置快照")

    def _persist_locked(self, snapshot: Mapping[str, Any]) -> None:
        # This check closes the race between the initial synchronization and
        # the atomic replace: an editor changing the file in that small window
        # must be reported, never silently overwritten.
        if self._read_file_hash() != self._file_hash:
            raise ConfigConflictError("配置文件在合并期间发生外部修改")
        save = getattr(self.config, "save", None)
        if callable(save):
            try:
                save(instant=True)
            except TypeError:
                save()
        else:
            self.config_path.parent.mkdir(parents=True, exist_ok=True)
            atomic_dump_yaml(self._yaml, self.config_path, snapshot)

    # ------------------------------------------------------------------
    # Delta validation and semantic queue merge
    # ------------------------------------------------------------------
    @staticmethod
    def _validate_run_id(run_id: Any) -> None:
        if not isinstance(run_id, str) or not run_id.strip() or len(run_id) > 256:
            raise ConfigRepositoryError("runId 必须是 1-256 个字符的非空字符串")

    def _validate_delta(
        self, delta: Mapping[str, Any]
    ) -> tuple[str, int, str, int, str, dict[str, Any], list[dict[str, Any]]]:
        if not isinstance(delta, Mapping):
            raise ConfigDeltaError("config.delta 必须是 mapping")
        unknown = set(delta) - _DELTA_FIELDS
        if unknown:
            raise ConfigDeltaError(f"config.delta 含未知字段：{sorted(map(str, unknown))}")
        if delta.get("type") != "config.delta":
            raise ConfigDeltaError("config.delta type 无效")
        run_id = delta.get("runId")
        try:
            self._validate_run_id(run_id)
        except ConfigRepositoryError as error:
            raise ConfigDeltaError(str(error)) from error

        sequence = delta.get("seq")
        if isinstance(sequence, bool) or not isinstance(sequence, int) or sequence < 0:
            raise ConfigDeltaError("config.delta seq 必须是非负整数")
        delta_id = delta.get("deltaId")
        if not isinstance(delta_id, str) or not delta_id.strip() or len(delta_id) > 256:
            raise ConfigDeltaError("config.delta deltaId 无效")
        base_revision = delta.get("baseRevision")
        if isinstance(base_revision, bool) or not isinstance(base_revision, int) or base_revision < 0:
            raise ConfigDeltaError("config.delta baseRevision 无效")
        base_hash = delta.get("baseConfigHash")
        if (
            not isinstance(base_hash, str)
            or len(base_hash) != 64
            or re.fullmatch(r"[0-9a-fA-F]{64}", base_hash) is None
        ):
            raise ConfigDeltaError("config.delta baseConfigHash 必须是 SHA-256")

        raw_changes = delta.get("changes", {})
        if not isinstance(raw_changes, Mapping):
            raise ConfigDeltaError("config.delta changes 必须是 mapping")
        changes: dict[str, Any] = {}
        for key, value in raw_changes.items():
            if key not in _ALLOWED_CHANGE_FIELDS:
                raise ConfigDeltaError(f"config.delta 字段不在白名单：{key}")
            changes[key] = self._validate_change(key, value)

        raw_operations = delta.get("operations", [])
        if not isinstance(raw_operations, Sequence) or isinstance(raw_operations, (str, bytes, bytearray)):
            raise ConfigDeltaError("config.delta operations 必须是数组")
        if len(raw_operations) > 64:
            raise ConfigDeltaError("config.delta operations 数量过多")
        operations: list[dict[str, Any]] = []
        for operation in raw_operations:
            if not isinstance(operation, Mapping):
                raise ConfigDeltaError("config.delta operation 必须是 mapping")
            if set(operation) - {"op", "completedTeamId"}:
                raise ConfigDeltaError("config.delta operation 含未知字段")
            if operation.get("op") not in _ALLOWED_OPERATIONS:
                raise ConfigDeltaError(f"不支持的 config.delta operation：{operation.get('op')}")
            completed_id = operation.get("completedTeamId")
            if not isinstance(completed_id, (str, int)) or isinstance(completed_id, bool):
                raise ConfigDeltaError("rotateTeamQueue.completedTeamId 无效")
            if isinstance(completed_id, str) and (not completed_id.strip() or len(completed_id) > 128):
                raise ConfigDeltaError("rotateTeamQueue.completedTeamId 无效")
            operations.append({"op": "rotateTeamQueue", "completedTeamId": completed_id})
        return run_id, sequence, delta_id, base_revision, base_hash.lower(), changes, operations

    @staticmethod
    def _validate_change(key: str, value: Any) -> Any:
        if key == "last_auto_change":
            if isinstance(value, bool) or not isinstance(value, (int, float)) or not math.isfinite(float(value)):
                raise ConfigDeltaError("last_auto_change 必须是有限数字")
            if value < 0:
                raise ConfigDeltaError("last_auto_change 不能为负数")
            return value
        if key == "hard_mirror":
            if isinstance(value, bool):
                return value
            if isinstance(value, int) and value in (0, 1):
                return value
            raise ConfigDeltaError("hard_mirror 必须是 bool 或 0/1")
        if key == "hard_mirror_chance":
            if isinstance(value, bool) or not isinstance(value, int) or not 0 <= value <= 100:
                raise ConfigDeltaError("hard_mirror_chance 必须是 0-100 的整数")
            return value
        if key == "set_win_size":
            if isinstance(value, bool) or not isinstance(value, int) or value not in _WINDOW_HEIGHTS:
                raise ConfigDeltaError(f"set_win_size 必须是 {_WINDOW_HEIGHTS} 中的整数")
            return value
        raise ConfigDeltaError(f"字段不在 config.delta 白名单：{key}")

    @staticmethod
    def _same_value(left: Any, right: Any) -> bool:
        if left is _MISSING or right is _MISSING:
            return left is right
        return _canonical_json({"value": left}) == _canonical_json({"value": right})

    @staticmethod
    def _queue_from_snapshot(snapshot: Mapping[str, Any]) -> list[int]:
        queue = snapshot.get("teams_active_queue", [])
        if not isinstance(queue, list):
            return []
        return [value for value in queue if isinstance(value, int) and not isinstance(value, bool) and value > 0]

    @staticmethod
    def _team_signature(snapshot: Mapping[str, Any]) -> tuple[tuple[str, int], ...]:
        teams = snapshot.get("teams", {})
        if not isinstance(teams, Mapping):
            return ()
        signature: list[tuple[str, int]] = []
        for raw_key, raw_setting in teams.items():
            key = str(raw_key)
            number: int | None = None
            if isinstance(raw_setting, Mapping):
                raw_number = raw_setting.get("team_number")
                if isinstance(raw_number, int) and not isinstance(raw_number, bool) and raw_number > 0:
                    number = raw_number
            if number is None:
                try:
                    parsed = int(key)
                except (TypeError, ValueError):
                    parsed = 0
                if parsed > 0:
                    number = parsed
            if number is not None:
                signature.append((key, number))
        return tuple(sorted(signature))

    @staticmethod
    def _team_number_for_id(snapshot: Mapping[str, Any], completed_id: str | int) -> int | None:
        teams = snapshot.get("teams", {})
        if not isinstance(teams, Mapping):
            return None
        wanted = str(completed_id)
        for raw_key, raw_setting in teams.items():
            key = str(raw_key)
            number: int | None = None
            if isinstance(raw_setting, Mapping):
                raw_number = raw_setting.get("team_number")
                if isinstance(raw_number, int) and not isinstance(raw_number, bool) and raw_number > 0:
                    number = raw_number
            if number is None:
                try:
                    parsed = int(key)
                except (TypeError, ValueError):
                    parsed = 0
                if parsed > 0:
                    number = parsed
            if number is None:
                continue
            if wanted in {key, str(number), f"team-{number}"}:
                return number
        if isinstance(completed_id, int) and not isinstance(completed_id, bool) and completed_id > 0:
            return completed_id
        match = re.fullmatch(r"team-(\d+)", wanted)
        if match is not None:
            return int(match.group(1))
        return None

    def _merge_queue_operations(
        self,
        candidate: dict[str, Any],
        current: Mapping[str, Any],
        context: _RunContext,
        operations: Sequence[Mapping[str, Any]],
        *,
        revision_matches: bool,
        conflicts: list[str],
        warnings: list[str],
        applied_changes: list[str],
    ) -> None:
        if not operations:
            return
        current_queue = self._queue_from_snapshot(current)
        queue_safe = revision_matches or (
            current_queue == context.baseline_queue and self._team_signature(current) == context.baseline_teams
        )
        if not queue_safe:
            conflicts.append("rotateTeamQueue")
            warnings.append("队伍队列在 Runner 快照后被用户重构，跳过 rotateTeamQueue")
            return
        queue = list(current_queue)
        for operation in operations:
            completed_id = operation["completedTeamId"]
            team_number = self._team_number_for_id(current, completed_id)
            if team_number is None or team_number not in queue:
                conflicts.append("rotateTeamQueue")
                warnings.append(f"找不到已完成队伍，跳过 rotateTeamQueue：{completed_id}")
                continue
            index = queue.index(team_number)
            queue = queue[index + 1 :] + queue[: index + 1]
        if queue != current_queue:
            self._rebuild_team_queue(candidate, queue)
            applied_changes.append("rotateTeamQueue")

    @staticmethod
    def _rebuild_team_queue(snapshot: dict[str, Any], queue: list[int]) -> None:
        teams = snapshot.get("teams", {})
        numbers: set[int] = set()
        if isinstance(teams, Mapping):
            for raw_key, raw_setting in teams.items():
                number = None
                if isinstance(raw_setting, Mapping):
                    candidate = raw_setting.get("team_number")
                    if isinstance(candidate, int) and not isinstance(candidate, bool) and candidate > 0:
                        number = candidate
                if number is None:
                    try:
                        parsed = int(raw_key)
                    except (TypeError, ValueError):
                        parsed = 0
                    if parsed > 0:
                        number = parsed
                if number is not None:
                    numbers.add(number)
        max_number = max(numbers, default=max(queue, default=0))
        selected = [False] * max_number
        order = [0] * max_number
        for index, team_number in enumerate(queue, start=1):
            if 1 <= team_number <= max_number:
                selected[team_number - 1] = True
                order[team_number - 1] = index
        snapshot["teams_active_queue"] = list(queue)
        snapshot["teams_be_select"] = selected
        snapshot["teams_order"] = order
        snapshot["teams_be_select_num"] = len(queue)

    def _validate_candidate(self, candidate: Mapping[str, Any]) -> dict[str, Any]:
        normalized = _normalise_value(candidate)
        if not isinstance(normalized, dict):
            raise ConfigDeltaError("配置候选快照必须是 mapping")
        config_obj = getattr(self.config, "config", None)
        if isinstance(config_obj, BaseModel):
            try:
                type(config_obj)(**normalized)
            except ValidationError as error:
                raise ConfigDeltaError(f"配置模型校验失败：{error}") from error
        return normalized

    def _delta_result(
        self,
        run_id: str,
        sequence: int,
        delta_id: str,
        *,
        status: str,
        applied_changes: list[str],
        conflicts: list[str],
        warnings: list[str],
    ) -> dict[str, Any]:
        return {
            "status": status,
            "runId": run_id,
            "seq": sequence,
            "deltaId": delta_id,
            "duplicate": status == "duplicate",
            "configRevision": self._revision,
            "baseConfigHash": self._canonical_hash,
            "applied": list(dict.fromkeys(applied_changes)),
            "conflicts": list(dict.fromkeys(conflicts)),
            "warnings": list(dict.fromkeys(warnings)),
        }

    @staticmethod
    def _restrict_mode(path: Path, mode: int) -> None:
        try:
            os.chmod(path, mode)
        except OSError:
            # Windows ACL inheritance from a private temp directory is the
            # effective restriction; chmod is still useful on POSIX and in
            # tests running on Windows-compatible filesystems.
            pass


__all__ = [
    "ConfigConflictError",
    "ConfigDeltaError",
    "ConfigRepository",
    "ConfigRepositoryError",
]
