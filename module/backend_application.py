"""Headless application services exposed by the GPUI sidecar.

The sidecar is the narrow runtime context shared by RPC handlers and the
WebSocket event publisher.  Services are deliberately kept behind small
methods: the transport does not need to know where configuration, devices, or
task execution are implemented.
"""

from __future__ import annotations

import copy
import inspect
import math
import os
import re
import threading
import time
import uuid
from collections import OrderedDict
from collections.abc import Callable, Iterable, Mapping
from pathlib import Path
from typing import Any

from core.team_squad import validate_pseudo_solo_selection
from module.config.redaction import SENSITIVE_CONFIG_KEYS, redact_text
from module.config.theme_pack_catalog import theme_pack_display_name
from module.device_manager import DeviceError, DeviceManager, get_device_manager
from module.execution_stats import ExecutionStatsStore
from module.logger import log
from module.observe_ego_gift import normalize_observe_ego_gifts
from module.preview_capture import PreviewCapture, encode_screenshot_frame

SCHEMA_VERSION = 3
WINDOW_POSITIONS = frozenset({"free", "left_top", "right_top", "left_bottom", "right_bottom", "center"})
LEGACY_WINDOW_POSITIONS = {
    "0": "center",
    "1": "left_top",
    "2": "right_top",
    "3": "free",
    0: "center",
    1: "left_top",
    2: "right_top",
    3: "free",
}

SINNER_IDS = (
    "yi_sang",
    "faust",
    "don_quixote",
    "ryoshu",
    "meursault",
    "hong_lu",
    "heathcliff",
    "ishmael",
    "rodion",
    "sinclair",
    "outis",
    "gregor",
)
SINNER_NAMES = (
    "李箱",
    "浮士德",
    "堂吉诃德",
    "良秀",
    "默尔索",
    "鸿璐",
    "希斯克利夫",
    "以实玛利",
    "罗佳",
    "辛克莱",
    "奥提斯",
    "格雷戈尔",
)
SYSTEM_NAMES = (
    "burn",
    "bleed",
    "tremor",
    "rupture",
    "poise",
    "sinking",
    "charge",
    "slash",
    "pierce",
    "blunt",
)

TEAM_PURPOSES = frozenset({"mirror", "luxcavation", "general"})
TEAM_MIRROR_BOOL_FIELDS = frozenset(
    {
        "do_not_heal",
        "do_not_buy",
        "do_not_fuse",
        "do_not_sell",
        "do_not_enhance",
        "only_aggressive_fuse",
        "do_not_system_fuse",
        "only_system_fuse",
        "aggressive_also_enhance",
        "aggressive_save_systems",
        "after_level_IV",
        "second_system",
        "avoid_skill_3",
        "prioritize_skill_3",
        "use_damage_p",
        "re_formation_each_floor",
        "defense_first_round",
        "defense_for_solo",
        "skill_replacement",
        "use_starlight",
        "fixed_team_use",
        "use_team_code",
        "use_custom_theme_pack_weight",
        "observe_ego_gift",
        "reward_cards",
        "shopping_strategy",
        "opening_items",
    }
)
TEAM_MIRROR_INT_LIMITS = {
    "team_system": (0, len(SYSTEM_NAMES) - 1),
    "shop_strategy": (0, 2),
    "after_level_IV_select": (0, 3),
    "max_keyword_refresh": (0, 10),
    "max_normal_refresh": (0, 10),
    "second_system_select": (0, len(SYSTEM_NAMES) - 1),
    "second_system_setting": (0, 1),
    "defense_for_solo_turns": (1, 5),
    "skill_replacement_select": (0, 3),
    "skill_replacement_mode": (0, 2),
    "fixed_team_use_select": (0, 2),
    "reward_cards_select": (0, 3),
    "shopping_strategy_select": (0, 5),
    "opening_items_select": (0, 5),
    "opening_items_system": (0, len(SYSTEM_NAMES) - 1),
}
TEAM_MIRROR_ACTION_FIELDS = (
    "second_system_fuse_IV",
    "second_system_buy",
    "second_system_select_reward",
    "second_system_power_up",
)
EXECUTABLE_TASK_IDS = ("daily_task", "get_reward", "buy_enkephalin", "mirror")

# ``execution.start.options`` is intentionally a narrow extension point.  No
# Runner-specific option is currently part of the sidecar contract, so the
# allowlist is empty until an option has an explicit value/ownership contract.
# Keeping the set here (rather than forwarding an arbitrary mapping) makes a
# newly added option an auditable change at the security boundary.
_EXECUTION_START_OPTION_ALLOWLIST = frozenset()
_EXECUTION_START_RESERVED_FIELDS = frozenset(
    {
        "runId",
        "run_id",
        "protocol",
        "configPath",
        "config_path",
        "runtimeConfig",
        "runtime_config",
        "resourceRoot",
        "resource_root",
        "cleanupReservation",
        "cleanup_reservation",
        "deviceTarget",
        "device_target",
        "deviceLease",
        "device_lease",
        "allowEmulatorLaunch",
        "allow_emulator_launch",
        "parentPid",
        "parent_pid",
        "expectedParentPid",
        "expected_parent_pid",
        "configRevision",
        "config_revision",
        "baseConfigHash",
        "base_config_hash",
        "configBaseline",
        "config_baseline",
        "baseline",
        "baselineValues",
        "baseline_values",
        "taskId",
        "task_id",
    }
)
_RUNNER_SENSITIVE_SNAPSHOT_KEYS = frozenset(
    {str(key).casefold() for key in SENSITIVE_CONFIG_KEYS}
    | {
        "token",
        "password",
        "secret",
        "credential",
        "apikey",
        "accesskey",
        "privatekey",
    }
)


def _cleanup_root_from_config(config: Any | None = None) -> Path:
    """Return the private app-data directory used for execution journals.

    Runner configuration snapshots already live below ``.aalc-runs`` beside
    the configured YAML file.  Keep cleanup journals in a child directory so
    a scan cannot mistake a temporary config directory for a journal.  The
    fallback is resolved lazily because importing :mod:`module.config` can
    construct the process-wide configuration singleton.
    """

    raw_path = getattr(config, "config_path", None) if config is not None else None
    if raw_path is None:
        try:
            from module import CONFIG_PATH

            raw_path = CONFIG_PATH
        except Exception:  # pragma: no cover - bootstrap fallback
            raw_path = Path.cwd() / "config.yaml"
    return Path(raw_path).expanduser().resolve().parent / ".aalc-runs" / "cleanup"


def _safe_cleanup_filename(run_id: str) -> str:
    """Map a run id to one journal filename without allowing path traversal."""

    safe = re.sub(r"[^A-Za-z0-9_.-]+", "_", str(run_id)).strip("._")
    return (safe[:96] or "run") + ".json"


def _invoke_recovery_callback(callback: Callable[..., Any], variants: Iterable[tuple[tuple[Any, ...], dict[str, Any]]]) -> Any:
    """Call one injected recovery callback using a signature-safe variant.

    Compatibility adapters in this file intentionally inspect signatures
    before choosing a call shape.  A ``TypeError`` raised by the callback body
    therefore remains a real cleanup failure instead of being mistaken for a
    legacy signature.
    """

    try:
        signature = inspect.signature(callback)
    except (TypeError, ValueError):
        args, kwargs = next(iter(variants))
        return callback(*args, **kwargs)
    for args, kwargs in variants:
        try:
            signature.bind(*args, **kwargs)
        except TypeError:
            continue
        return callback(*args, **kwargs)
    raise TypeError("injected recovery callback has an unsupported signature")


def _build_default_cleanup_executor(run_id: str, ledger: Any) -> Any:
    """Create one production cleanup executor for one durable run.

    ``build_default_cleanup_executor`` validates and binds the executor to the
    exact ledger/run pair.  Keep the import lazy so compatibility/test builds
    that do not package device cleanup can still serve read-only RPC, while
    making the factory the single default for real Runner cleanup.
    """

    from module.resource_cleanup import build_default_cleanup_executor

    return build_default_cleanup_executor(run_id, ledger)


def _mark_cleanup_device_disconnected(ledger: Any, result: dict[str, Any]) -> None:
    """Retain a deferred remote-device cleanup as explicitly disconnected."""

    try:
        data = getattr(ledger, "data", {})
        target = data.get("target") or data.get("deviceTarget") if isinstance(data, Mapping) else None
        target_kind = target.get("kind") if isinstance(target, Mapping) else None
        if target_kind in {"adb", "mumu"}:
            reserve = getattr(ledger, "reserve", None)
            if callable(reserve):
                reserve(deviceDisposition="disconnected")
            result["deviceDisposition"] = "disconnected"
    except Exception as error:
        result.setdefault("errors", []).append(
            {"code": "CLEANUP_DISPOSITION_FAILED", "message": str(error)}
        )


def _default_process_identity_probe(pid: int) -> dict[str, Any]:
    """Read a process identity without opening or terminating the process."""

    try:
        import psutil  # type: ignore[import-not-found]
    except ImportError:
        return {"alive": None, "createTime": None, "error": "psutil unavailable"}
    try:
        process = psutil.Process(pid)
        return {
            "alive": bool(process.is_running()),
            "createTime": float(process.create_time()),
        }
    except (psutil.NoSuchProcess, psutil.ZombieProcess):
        return {"alive": False, "createTime": None}
    except (psutil.AccessDenied, OSError, ValueError) as error:
        # Access denied is deliberately not treated as proof of identity.  A
        # recovery caller may retry later with an injected privileged probe.
        return {"alive": None, "createTime": None, "error": str(error)}


class _LedgerRecoveryExecutor:
    """Guard process cleanup and delegate safe resource steps.

    ``ResourceCleanupExecutor`` intentionally leaves ``runner.process`` to its
    owning supervisor.  Startup recovery has no live supervisor, so this
    adapter verifies PID + creation time first and only calls an explicitly
    injected terminator.  It never invokes ``terminate``/``kill`` itself.
    """

    def __init__(
        self,
        delegate: Any = None,
        *,
        process_identity_probe: Callable[..., Any] | None = None,
        process_terminator: Callable[..., Any] | None = None,
    ) -> None:
        self.delegate = delegate
        self.process_identity_probe = process_identity_probe or _default_process_identity_probe
        self.process_terminator = process_terminator

    def execute(self, action: Mapping[str, Any], *, ledger: Any = None) -> Any:
        action_type = str(action.get("actionType", action.get("type", ""))).casefold()
        if action_type in {"runner.process", "runner_process", "runner-process"}:
            return self._process(action, ledger)
        delegate = self.delegate
        if delegate is None:
            # No concrete device backend is bound in a minimal compatibility
            # process.  Keeping the action pending is safer than claiming a
            # remote resource was released.
            return {"status": "deferred", "detail": "resource cleanup executor is not bound"}
        callback = getattr(delegate, "execute", None)
        if not callable(callback) and callable(delegate):
            callback = delegate
        if not callable(callback):
            return {"status": "deferred", "detail": "resource cleanup executor is not callable"}
        return _invoke_recovery_callback(
            callback,
            (
                ((action,), {"ledger": ledger}),
                ((action,), {}),
                ((action, ledger), {}),
            ),
        )

    def _process(self, action: Mapping[str, Any], ledger: Any) -> dict[str, Any]:
        payload = action.get("payload")
        values = dict(payload) if isinstance(payload, Mapping) else {}
        if ledger is not None:
            reservation = getattr(ledger, "reservation", None)
            if isinstance(reservation, Mapping):
                for key, value in reservation.items():
                    values.setdefault(key, value)
        raw_pid = values.get("pid", values.get("runnerPid"))
        if isinstance(raw_pid, bool):
            return {"status": "failed", "detail": "runner PID is invalid"}
        try:
            pid = int(raw_pid)
        except (TypeError, ValueError):
            return {"status": "failed", "detail": "runner PID is missing"}
        if pid <= 0:
            return {"status": "failed", "detail": "runner PID is invalid"}
        expected = values.get("createTime", values.get("runnerCreateTime"))
        try:
            expected_time = float(expected) if expected is not None else None
        except (TypeError, ValueError):
            return {"status": "failed", "detail": "runner creation time is invalid"}

        try:
            observed = _invoke_recovery_callback(
                self.process_identity_probe,
                (
                    ((pid,), {}),
                    ((), {"pid": pid}),
                ),
            )
        except Exception as error:
            return {"status": "deferred", "detail": f"runner identity probe failed: {error}"}
        identity = self._identity_values(observed)
        alive = identity.get("alive")
        if alive is False:
            return {"status": "done", "detail": "runner process is no longer alive"}
        if alive is not True:
            return {"status": "deferred", "detail": "runner process identity is unavailable"}
        actual = identity.get("createTime")
        if expected_time is None or actual is None:
            # PID-only cleanup is unsafe because the PID may already belong to
            # an unrelated process.  Keep the journal for an explicit probe.
            return {"status": "deferred", "detail": "runner PID lacks creation-time proof"}
        try:
            same_process = abs(float(actual) - expected_time) <= 0.001
        except (TypeError, ValueError):
            same_process = False
        if not same_process:
            return {
                "status": "done",
                "detail": {"status": "skipped", "code": "STALE_RUNNER_PID", "pid": pid},
            }
        terminator = self.process_terminator
        if not callable(terminator):
            return {"status": "deferred", "detail": "runner process termination is not injected"}
        try:
            result = _invoke_recovery_callback(
                terminator,
                (
                    ((pid, expected_time), {}),
                    ((pid,), {"create_time": expected_time}),
                    ((pid,), {"runner_create_time": expected_time}),
                    (({"pid": pid, "createTime": expected_time},), {}),
                ),
            )
        except Exception as error:
            return {"status": "failed", "detail": f"runner termination failed: {error}"}
        if result is False:
            return {"status": "deferred", "detail": "runner termination did not complete"}
        try:
            after = _invoke_recovery_callback(
                self.process_identity_probe,
                (
                    ((pid,), {}),
                    ((), {"pid": pid}),
                ),
            )
            if self._identity_values(after).get("alive") is True:
                return {"status": "deferred", "detail": "runner process remains alive"}
        except Exception:
            # An explicitly injected terminator is allowed to report success
            # even when the follow-up probe is unavailable; no unscoped kill is
            # attempted here.
            pass
        return {"status": "done", "detail": "runner process terminated by injected cleanup"}

    @staticmethod
    def _identity_values(value: Any) -> dict[str, Any]:
        if isinstance(value, Mapping):
            alive = value.get("alive", value.get("running"))
            create_time = value.get("createTime", value.get("create_time"))
            return {"alive": alive, "createTime": create_time}
        if isinstance(value, bool):
            return {"alive": value, "createTime": None}
        if isinstance(value, tuple) and len(value) >= 2:
            return {"alive": bool(value[0]), "createTime": value[1]}
        alive = getattr(value, "alive", getattr(value, "running", None))
        create_time = getattr(value, "createTime", getattr(value, "create_time", None))
        return {"alive": alive, "createTime": create_time}


def recover_pending_cleanups(
    root: str | Path | None = None,
    *,
    cleanup_executor: Any | None = None,
    resource_cleanup_executor: Any | None = None,
    ledger_factory: Callable[..., Any] | None = None,
    process_identity_probe: Callable[..., Any] | None = None,
    process_terminator: Callable[..., Any] | None = None,
) -> list[dict[str, Any]]:
    """Recover durable Runner journals before the RPC server accepts work.

    Recovery is intentionally a synchronous startup barrier.  It records a
    synthetic ``crashed`` finish for a journal with no durable ``finished``
    event, then reuses the same idempotent ledger executor used by normal
    Runner finalization.  A remote cleanup that cannot prove the device is
    online remains pending and is marked ``disconnected`` for the next
    connection attempt.

    ``ledger_factory`` is primarily a test/embedding seam.  Existing files are
    loaded through ``CleanupLedger`` itself so a factory cannot accidentally
    reinterpret an untrusted on-disk path during startup.
    """

    try:
        from module.execution.cleanup_ledger import CleanupLedger
    except Exception as error:  # pragma: no cover - packaging fallback
        log.warning("CleanupLedger 不可用，跳过启动恢复：%s", error)
        return []

    cleanup_root = Path(root).expanduser().resolve() if root is not None else _cleanup_root_from_config()
    if not cleanup_root.exists():
        return []
    if cleanup_executor is not None and resource_cleanup_executor is not None and cleanup_executor is not resource_cleanup_executor:
        raise ValueError("cleanup_executor and resource_cleanup_executor are mutually exclusive")
    # An injected executor is retained for compatibility with older embedders.
    # The production default is deliberately built per ledger below because
    # ``build_default_cleanup_executor`` binds its state to one runId.
    injected_delegate = cleanup_executor if cleanup_executor is not None else resource_cleanup_executor

    results: list[dict[str, Any]] = []
    # ``scan`` intentionally raises on a malformed journal.  Startup should
    # continue to inspect the remaining runs and expose each malformed path in
    # a structured result rather than silently accepting new work.
    paths = sorted(cleanup_root.glob("*.json"))
    for path in paths:
        try:
            if ledger_factory is None:
                ledger = CleanupLedger.load(path)
            else:
                ledger = _invoke_recovery_callback(
                    ledger_factory,
                    (
                        ((path,), {"create": False}),
                        ((path,), {}),
                        ((path.stem,), {"path": path}),
                        ((path.stem,), {}),
                    ),
                )
                if ledger is None:
                    raise RuntimeError("ledger factory returned no ledger")
        except Exception as error:
            results.append(
                {
                    "status": "failed",
                    "complete": False,
                    "pending": True,
                    "path": str(path),
                    "error": {"code": "CLEANUP_LEDGER_INVALID", "message": str(error)},
                }
            )
            log.warning("忽略无效 CleanupLedger：%s (%s)", path, error)
            continue
        if ledger.complete:
            completed_result: dict[str, Any] = {
                "runId": ledger.run_id,
                "status": "complete",
                "complete": True,
                "pending": False,
                "path": str(path),
            }
            try:
                ledger.delete()
            except Exception as error:
                completed_result["error"] = {
                    "code": "CLEANUP_LEDGER_DELETE_FAILED",
                    "message": str(error),
                }
            results.append(completed_result)
            continue

        try:
            if ledger.finished is None:
                ledger.record_finished(
                    {
                        "runId": ledger.run_id,
                        "outcome": "crashed",
                        "forced": True,
                        "requestedBy": "watchdog",
                        "error": {
                            "code": "SIDECAR_CRASHED",
                            "message": "sidecar restarted before Runner cleanup finished",
                            "phase": "recovery",
                            "recovery": "retry",
                        },
                    }
                )
        except Exception as error:
            results.append(
                {
                    "runId": ledger.run_id,
                    "status": "failed",
                    "complete": False,
                    "pending": True,
                    "path": str(path),
                    "error": {"code": "CLEANUP_LEDGER_FINISH_FAILED", "message": str(error)},
                }
            )
            continue

        delegate = injected_delegate
        if delegate is None:
            try:
                delegate = _build_default_cleanup_executor(ledger.run_id, ledger)
            except Exception as error:
                # A missing optional platform dependency or malformed target
                # must not consume the journal.  Keep it pending so a later
                # startup can retry after the device/backend is available.
                deferred = {
                    "runId": ledger.run_id,
                    "status": "deferred",
                    "complete": False,
                    "pending": True,
                    "path": str(path),
                    "error": {"code": "CLEANUP_EXECUTOR_UNAVAILABLE", "message": str(error)},
                }
                _mark_cleanup_device_disconnected(ledger, deferred)
                results.append(deferred)
                log.warning("创建默认资源清理执行器失败：%s", error)
                continue
        executor = _LedgerRecoveryExecutor(
            delegate,
            process_identity_probe=process_identity_probe,
            process_terminator=process_terminator,
        )
        try:
            recovery = ledger.recover(executor, delete_complete=False, retry_failed=True)
            recovery_value = recovery.to_dict() if hasattr(recovery, "to_dict") else dict(recovery)
            if not recovery.complete:
                # Preserve the pending journal and make a remote device state
                # explicit; no new lease should be admitted until a later
                # connection retries this run's scoped cleanup.
                _mark_cleanup_device_disconnected(ledger, recovery_value)
            else:
                try:
                    ledger.delete()
                except Exception as error:
                    recovery_value.setdefault("errors", []).append(
                        {"code": "CLEANUP_LEDGER_DELETE_FAILED", "message": str(error)}
                    )
            recovery_value.update({"runId": ledger.run_id, "path": str(path)})
            results.append(recovery_value)
        except Exception as error:
            results.append(
                {
                    "runId": ledger.run_id,
                    "status": "failed",
                    "complete": False,
                    "pending": True,
                    "path": str(path),
                    "error": {"code": "CLEANUP_RECOVERY_FAILED", "message": str(error)},
                }
            )
            log.exception("CleanupLedger 恢复失败：%s", path)
    return results


class BackendEventBus:
    """Thread-safe event publisher with a monotonically increasing sequence."""

    def __init__(self) -> None:
        self._lock = threading.RLock()
        self._listeners: list[Callable[[str, dict[str, Any], int], None]] = []
        self._sequence = 0

    def add_listener(self, listener: Callable[[str, dict[str, Any], int], None]) -> None:
        with self._lock:
            if listener not in self._listeners:
                self._listeners.append(listener)

    def remove_listener(self, listener: Callable[[str, dict[str, Any], int], None]) -> None:
        with self._lock:
            if listener in self._listeners:
                self._listeners.remove(listener)

    def emit(self, event: str, payload: Mapping[str, Any] | None = None) -> int:
        with self._lock:
            self._sequence += 1
            sequence = self._sequence
            listeners = list(self._listeners)
            value = dict(payload or {})
            # Listener callbacks are intentionally serialized with sequence
            # allocation so another emitter cannot publish seq N+1 first.
            for listener in listeners:
                try:
                    # A listener may enrich its local view; don't let that
                    # mutate the payload observed by later subscribers.
                    listener(event, dict(value), sequence)
                except Exception:
                    log.exception("sidecar 事件监听器执行失败：%s", event)
        return sequence


class BackendApplication:
    """Runtime service context for one sidecar process.

    ``config`` and ``theme_list`` are injectable to keep protocol tests
    independent from the process-wide configuration singleton.  In the real
    sidecar they default to the existing compatibility-aware stores.
    """

    def __init__(
        self,
        device_manager: DeviceManager | Any | None = None,
        *,
        version: str = "unknown",
        shutdown: Callable[[], None] | None = None,
        config: Any | None = None,
        theme_list: Any | None = None,
        resource_service: Any | None = None,
        preview_capture: PreviewCapture | None = None,
        stats_path: str | Path | None = None,
        notifications: Any | None = None,
        runner_supervisor: Any | None = None,
        runner_factory: Any | None = None,
        runner_enabled: bool | None = None,
        config_repository: Any | None = None,
        repository: Any | None = None,
        after_completion_coordinator: Any | None = None,
        completion_coordinator: Any | None = None,
        after_completion_journal: Any | None = None,
        after_completion_handlers: Mapping[str, Callable[..., Any]] | None = None,
        after_completion_power_handler: Callable[..., Any] | None = None,
        after_completion: Any | None = None,
        cleanup_ledger_root: str | Path | None = None,
        ledger_root: str | Path | None = None,
        cleanup_ledger_factory: Callable[..., Any] | None = None,
        ledger_factory: Callable[..., Any] | None = None,
        cleanup_executor: Any | None = None,
        resource_cleanup_executor: Any | None = None,
        process_identity_probe: Callable[..., Any] | None = None,
        process_terminator: Callable[..., Any] | None = None,
    ) -> None:
        if after_completion is not None:
            if after_completion_coordinator is not None and after_completion_coordinator is not after_completion:
                raise ValueError("after_completion and after_completion_coordinator are mutually exclusive")
            if completion_coordinator is not None and completion_coordinator is not after_completion:
                raise ValueError("after_completion and completion_coordinator are mutually exclusive")
            after_completion_coordinator = after_completion
        if (
            after_completion_coordinator is not None
            and completion_coordinator is not None
            and after_completion_coordinator is not completion_coordinator
        ):
            raise ValueError("after_completion_coordinator and completion_coordinator are mutually exclusive")
        if after_completion_coordinator is None:
            after_completion_coordinator = completion_coordinator
        if cleanup_ledger_root is not None and ledger_root is not None:
            if Path(cleanup_ledger_root).expanduser().resolve() != Path(ledger_root).expanduser().resolve():
                raise ValueError("cleanup_ledger_root and ledger_root are mutually exclusive")
        if cleanup_ledger_factory is not None and ledger_factory is not None and cleanup_ledger_factory is not ledger_factory:
            raise ValueError("cleanup_ledger_factory and ledger_factory are mutually exclusive")
        if cleanup_executor is not None and resource_cleanup_executor is not None and cleanup_executor is not resource_cleanup_executor:
            raise ValueError("cleanup_executor and resource_cleanup_executor are mutually exclusive")
        self.version = version
        self.events = BackendEventBus()
        self._lock = threading.RLock()
        self._shutdown = shutdown
        self._config = config
        self._theme_list = theme_list
        self._resource_service = resource_service
        self._resource_check: Any | None = None
        self._resource_plan: Any | None = None
        self._resource_run: str | None = None
        self._execution_state = "idle"
        self._execution_state_revision = 0
        self._execution_supervisor_revision = 0
        self._execution_task_id: str | None = None
        self._execution_run_id: str | None = None
        self._execution_runner_pid: int | None = None
        self._execution_device_lease = "none"
        self._execution_lease_generation: int | None = None
        self._execution_outcome: str | None = None
        self._execution_forced = False
        self._execution_requested_by: str | None = None
        self._execution_error: dict[str, Any] | None = None
        self._execution_device_restore = "not_needed"
        self._execution_finalized_runs: set[str] = set()
        self._execution_after_completion_requests: dict[str, list[dict[str, Any]]] = {}
        self._execution_after_completion_results: dict[str, dict[str, Any]] = {}
        self._execution_stats_started: set[str] = set()
        self._execution_stats_finished: set[str] = set()
        self._execution_client_requests: OrderedDict[str, dict[str, Any]] = OrderedDict()
        self._execution_client_request_limit = 128
        self._execution_run_config: dict[str, Any] | None = None
        self._runner_config_cleanup_done: set[str] = set()
        self._runner_event_sequences: dict[str, set[int]] = {}
        self._runner_task_completed_events: set[tuple[str, int]] = set()
        self._runner_event_adapter: Any | None = None
        self._cleanup_ledger_root = cleanup_ledger_root if cleanup_ledger_root is not None else ledger_root
        if self._cleanup_ledger_root is not None:
            self._cleanup_ledger_root = Path(self._cleanup_ledger_root).expanduser().resolve()
        self._cleanup_ledger_factory = (
            cleanup_ledger_factory if cleanup_ledger_factory is not None else ledger_factory
        )
        self._cleanup_executor = (
            cleanup_executor if cleanup_executor is not None else resource_cleanup_executor
        )
        self._process_identity_probe = process_identity_probe
        self._process_terminator = process_terminator
        self._cleanup_ledgers: dict[str, Any] = {}
        self._execution_thread: threading.Thread | None = None
        self._execution_worker: Any | None = None
        self._execution_stop = threading.Event()
        env_runner = os.environ.get("AALC_EXECUTION_RUNNER", "").strip().lower()
        self._runner_enabled = (
            bool(runner_enabled)
            if runner_enabled is not None
            else runner_supervisor is not None or env_runner in {"1", "true", "yes", "on"}
        )
        self._runner_supervisor = runner_supervisor
        self._runner_factory = runner_factory
        self._runner_monitor: threading.Thread | None = None
        self._tools: dict[str, dict[str, Any]] = {}
        self._tool_workers: dict[str, tuple[threading.Thread, threading.Event, Any]] = {}
        self._tool_runtimes: dict[str, Any] = {}
        self._hotkey_enabled = True
        self._hotkey_listener: Any | None = None
        self._mediator_bindings: list[tuple[Any, Callable[..., Any]]] = []
        self._preview_capture = preview_capture or PreviewCapture(self.emit)
        self._preview_enabled = True
        self._preview_device_id: str | None = None
        self._closed = False
        if stats_path is None:
            from module import CONFIG_PATH

            stats_path = Path(CONFIG_PATH).with_name("runtime_stats.json")
        self.stats = ExecutionStatsStore(stats_path)
        if notifications is None:
            from module.notification.wxpusher import NotificationService

            notifications = NotificationService()
        self.notifications = notifications
        self._after_completion_coordinator = after_completion_coordinator
        if self._after_completion_coordinator is None:
            try:
                from module.after_completion import AfterCompletionCoordinator

                supplied_handlers = after_completion_handlers if isinstance(after_completion_handlers, Mapping) else {}
                supplied_handler_names = {str(key) for key in supplied_handlers}
                self._after_completion_coordinator = AfterCompletionCoordinator(
                    journal=after_completion_journal,
                    handlers=after_completion_handlers,
                    notification_handler=(
                        None
                        if supplied_handler_names.intersection({"notification", "notify"})
                        else self._after_completion_notification
                    ),
                    toast_handler=(None if "toast" in supplied_handler_names else self._after_completion_toast),
                    sound_handler=(None if "sound" in supplied_handler_names else self._after_completion_sound),
                    exit_request_handler=(
                        None
                        if supplied_handler_names.intersection({"exit_aalc", "exit_requested", "app.exitRequested"})
                        else self.emit
                    ),
                    power_handler=(
                        None
                        if supplied_handler_names.intersection({"power", "power_action"})
                        else after_completion_power_handler
                    ),
                )
            except Exception as error:
                # Completion actions are an optional Runner boundary.  A
                # packaging/compatibility failure must not disable execution;
                # the explicit dependency can still be supplied by embedders.
                log.warning("创建完成动作协调器失败：%s", error)
                self._after_completion_coordinator = None

        self.device_manager = device_manager or get_device_manager()
        if config_repository is not None and repository is not None and config_repository is not repository:
            raise ValueError("config_repository and repository are mutually exclusive")
        self._config_repository = config_repository if config_repository is not None else repository
        # A Runner must never silently fall back to the process-wide config
        # object.  Keep an internal, non-sensitive failure marker so the RPC
        # boundary can return a stable error after construction (the sidecar
        # still needs to start far enough to report the problem cleanly).
        self._runner_config_error: str | None = None
        if self._config_repository is None:
            try:
                config_object = self.config
                if getattr(config_object, "config_path", None) is None:
                    if self._runner_enabled:
                        raise RuntimeError("ConfigRepository requires config.config_path")
                else:
                    from module.config.repository import ConfigRepository

                    self._config_repository = ConfigRepository(config_object)
            except Exception:
                if self._runner_enabled:
                    self._runner_config_error = "ConfigRepository initialization failed"
                    # Do not include exception text here: a custom Config or
                    # import hook may accidentally put credential contents in
                    # its exception message.
                    log.warning("创建 ConfigRepository 失败，Runner 已 fail-closed")
                else:
                    log.debug("创建 ConfigRepository 失败，Runner 配置事务保持禁用")
        if self._runner_enabled and self._runner_supervisor is None:
            try:
                from module.execution.supervisor import RunnerSupervisor

                supervisor_kwargs: dict[str, Any] = {"event_callback": self._on_runner_event}
                if runner_factory is not None:
                    supervisor_kwargs["runner_factory"] = runner_factory
                self._runner_supervisor = RunnerSupervisor(**supervisor_kwargs)
            except Exception as error:
                # Keep the legacy engine available when an optional runner
                # dependency cannot be constructed in a compatibility build.
                log.warning("创建 RunnerSupervisor 失败，回退旧执行线程：%s", error)
                self._runner_enabled = False
        elif self._runner_supervisor is not None:
            callback = getattr(self._runner_supervisor, "event_callback", None)
            if callback is None:
                try:
                    setattr(self._runner_supervisor, "event_callback", self._on_runner_event)
                except Exception:
                    pass
        if self._runner_enabled:
            try:
                from module.execution.event_adapter import RunnerEventAdapter

                # The adapter is rebound to the current run's durable ledger
                # immediately before Runner start.  Keeping the adapter
                # instance stable preserves its sink contract while avoiding
                # any process-global journal state.
                self._runner_event_adapter = RunnerEventAdapter(self.emit)
            except Exception as error:
                log.debug("创建 RunnerEventAdapter 失败：%s", error)
        if hasattr(self.device_manager, "set_busy_checker"):
            self.device_manager.set_busy_checker(self.is_busy)
        if hasattr(self.device_manager, "add_status_listener"):
            self.device_manager.add_status_listener(self._on_device_event)
        if hasattr(self.device_manager, "add_notice_listener"):
            self.device_manager.add_notice_listener(self._on_device_event)
        self._bind_core_events()

        # Hotkeys are optional on non-Windows hosts and in headless test
        # environments.  A malformed user value must not prevent RPC startup.
        self._refresh_hotkeys()

    @property
    def config(self) -> Any:
        if self._config is None:
            from module.config import cfg

            self._config = cfg
        return self._config

    @property
    def theme_store(self) -> Any:
        if self._theme_list is None:
            from module.config import theme_list

            self._theme_list = theme_list
        return self._theme_list

    def add_event_listener(self, listener: Callable[[str, dict[str, Any], int], None]) -> None:
        self.events.add_listener(listener)

    def remove_event_listener(self, listener: Callable[[str, dict[str, Any], int], None]) -> None:
        self.events.remove_listener(listener)

    def set_busy_checker(self, checker: Callable[[], bool]) -> None:
        """Override the device switch guard for embedding/tests."""
        if hasattr(self.device_manager, "set_busy_checker"):
            self.device_manager.set_busy_checker(checker)

    def emit(self, event: str, payload: Mapping[str, Any] | None = None) -> int:
        return self.events.emit(event, payload)

    def app_ping(self) -> str:
        return "pong"

    def app_version(self) -> dict[str, Any]:
        return {"ui": "gpui", "backend": self.version, "schemaVersion": SCHEMA_VERSION}

    def app_shutdown(self) -> bool:
        if self._shutdown is not None:
            self._shutdown()
        return True

    def preview_set_enabled(self, params: Any) -> dict[str, bool]:
        values = self._require_mapping(params, "preview.setEnabled")
        enabled = values.get("enabled")
        if not isinstance(enabled, bool):
            raise ValueError("preview.setEnabled.enabled must be a boolean")

        with self._lock:
            if enabled != self._preview_enabled:
                self._preview_enabled = enabled
                if not enabled:
                    stop_and_wait = getattr(self._preview_capture, "stop_and_wait", None)
                    if callable(stop_and_wait):
                        self._invoke_compatible(stop_and_wait, (), {})
                    else:
                        self._preview_capture.stop()
                elif self._preview_device_id:
                    # A Runner owns the device during an execution lease.  A
                    # user may change the desired preview preference, but no
                    # preview worker may regain the device capability yet.
                    if not self._device_lease_active():
                        self._preview_capture.start(self._preview_device_id)

            fallback_running = self._preview_enabled and self._preview_device_id is not None
            running = bool(getattr(self._preview_capture, "running", fallback_running))
            return {"enabled": self._preview_enabled, "running": running}

    def stats_get_summary(self) -> dict[str, Any]:
        return self.stats.summary()

    def stats_get_daily_summary(self, params: Any = None) -> dict[str, Any]:
        values = params if isinstance(params, Mapping) else {}
        date_from = values.get("dateFrom")
        date_to = values.get("dateTo")
        if date_from is not None and not isinstance(date_from, str):
            raise ValueError("dateFrom must be a YYYY-MM-DD string")
        if date_to is not None and not isinstance(date_to, str):
            raise ValueError("dateTo must be a YYYY-MM-DD string")
        return self.stats.daily_summary(date_from=date_from, date_to=date_to)

    def tasks_get_config(self) -> dict[str, Any]:
        raw = self._config_snapshot()
        return {
            "schemaVersion": SCHEMA_VERSION,
            "enabledTasks": {
                "daily_task": self._json_bool(raw.get("daily_task"), False),
                "get_reward": self._json_bool(raw.get("get_reward"), False),
                "buy_enkephalin": self._json_bool(raw.get("buy_enkephalin"), False),
                "mirror": self._json_bool(raw.get("mirror"), False),
                "resonate_with_Ahab": self._json_bool(raw.get("resonate_with_Ahab"), False),
            },
            "set_windows": {
                key: (
                    self._normalise_window_position(raw.get(key, default))
                    if key == "set_win_position"
                    else self._json_bool(raw.get(key), default)
                    if isinstance(default, bool)
                    else raw.get(key, default)
                )
                for key, default in {
                    "set_win_size": 1080,
                    "set_win_position": "free",
                    "set_reduce_miscontact": True,
                    "screenshot_interval": 0.5,
                    "mouse_action_interval": 0.3,
                    "mouse_down_duration": 0.1,
                    "use_post_message": False,
                }.items()
            },
            "daily_task": {
                key: self._json_bool(raw.get(key), default) if isinstance(default, bool) else raw.get(key, default)
                for key, default in {
                    "set_EXP_count": 1,
                    "set_thread_count": 3,
                    "daily_teams": 1,
                    "use_continuous_combat": False,
                    "use_continuous_combat_select": 1,
                    "targeted_teaming_EXP": False,
                    "EXP_day_1_2": 1,
                    "EXP_day_3_4": 1,
                    "EXP_day_5_6": 1,
                    "EXP_day_7": 1,
                    "targeted_teaming_thread": False,
                    "thread_day_1": 1,
                    "thread_day_2": 1,
                    "thread_day_3": 1,
                    "thread_day_4": 1,
                    "thread_day_5": 1,
                    "thread_day_6": 1,
                    "thread_day_7": 1,
                }.items()
            },
            "get_reward": {"set_get_prize": raw.get("set_get_prize", 0)},
            "buy_enkephalin": {
                "set_lunacy_to_enkephalin": raw.get("set_lunacy_to_enkephalin", 2),
                "Dr_Grandet_mode": self._json_bool(raw.get("Dr_Grandet_mode"), False),
                "skip_enkephalin": self._json_bool(raw.get("skip_enkephalin"), False),
            },
            "mirror": {
                key: self._json_bool(raw.get(key), default) if isinstance(default, bool) else raw.get(key, default)
                for key, default in {
                    "set_mirror_count": 1,
                    "infinite_dungeons": False,
                    "hard_mirror": False,
                    "hard_mirror_target_floors": 5,
                    "no_weekly_bonuses": False,
                    "floor_3_exit": False,
                    "save_rewards": False,
                    "hard_mirror_single_bonuses": False,
                    "select_event_pack": False,
                    "skip_event_pack": False,
                    "re_claim_rewards": False,
                    "not_skip_whitegossypium": False,
                    "fight_to_last_man": False,
                    "mirror_keyboard_navigation": False,
                    "mirror_keyboard_simple_pathfinding": False,
                    "mirror_prefer_hatred_and_despair": False,
                    "mirror_minimize_non_boss_combat": False,
                }.items()
            },
            "resonate_with_Ahab": {"enabled": self._json_bool(raw.get("resonate_with_Ahab"), False)},
            "afterCompletion": {
                "actions": list(raw.get("after_completion_actions", []) or []),
                "powerAction": raw.get("after_completion_power_action", "none"),
                "keepAfterCompletion": self._json_bool(raw.get("keep_after_completion"), False),
            },
        }

    def tasks_set_config(self, params: Any) -> bool:
        values = self._require_mapping(params, "tasks.setConfig")
        updates: dict[str, Any] = {}
        known_flat_fields = {
            "daily_task",
            "get_reward",
            "buy_enkephalin",
            "mirror",
            "resonate_with_Ahab",
            "set_win_size",
            "set_win_position",
            "set_reduce_miscontact",
            "screenshot_interval",
            "mouse_action_interval",
            "mouse_down_duration",
            "use_post_message",
            "set_EXP_count",
            "set_thread_count",
            "daily_teams",
            "use_continuous_combat",
            "use_continuous_combat_select",
            "targeted_teaming_EXP",
            "EXP_day_1_2",
            "EXP_day_3_4",
            "EXP_day_5_6",
            "EXP_day_7",
            "targeted_teaming_thread",
            "thread_day_1",
            "thread_day_2",
            "thread_day_3",
            "thread_day_4",
            "thread_day_5",
            "thread_day_6",
            "thread_day_7",
            "set_get_prize",
            "set_lunacy_to_enkephalin",
            "Dr_Grandet_mode",
            "skip_enkephalin",
            "set_mirror_count",
            "infinite_dungeons",
            "hard_mirror",
            "hard_mirror_target_floors",
            "no_weekly_bonuses",
            "floor_3_exit",
            "save_rewards",
            "hard_mirror_single_bonuses",
            "select_event_pack",
            "skip_event_pack",
            "re_claim_rewards",
            "not_skip_whitegossypium",
            "fight_to_last_man",
            "mirror_keyboard_navigation",
            "mirror_keyboard_simple_pathfinding",
            "mirror_prefer_hatred_and_despair",
            "mirror_minimize_non_boss_combat",
            "after_completion_actions",
            "after_completion_power_action",
            "keep_after_completion",
        }
        updates.update(
            {key: value for key, value in values.items() if key in known_flat_fields and not isinstance(value, Mapping)}
        )
        enabled = values.get("enabledTasks")
        if isinstance(enabled, Mapping):
            updates.update(
                {
                    key: self._require_bool(enabled[key], f"enabledTasks.{key}")
                    for key in (
                        "daily_task",
                        "get_reward",
                        "buy_enkephalin",
                        "mirror",
                        "resonate_with_Ahab",
                    )
                    if key in enabled
                }
            )
        sections = {
            "set_windows": (
                "set_win_size",
                "set_win_position",
                "set_reduce_miscontact",
                "screenshot_interval",
                "mouse_action_interval",
                "mouse_down_duration",
                "use_post_message",
            ),
            "daily_task": (
                "set_EXP_count",
                "set_thread_count",
                "daily_teams",
                "use_continuous_combat",
                "use_continuous_combat_select",
                "targeted_teaming_EXP",
                "EXP_day_1_2",
                "EXP_day_3_4",
                "EXP_day_5_6",
                "EXP_day_7",
                "targeted_teaming_thread",
                "thread_day_1",
                "thread_day_2",
                "thread_day_3",
                "thread_day_4",
                "thread_day_5",
                "thread_day_6",
                "thread_day_7",
            ),
            "get_reward": ("set_get_prize",),
            "buy_enkephalin": ("set_lunacy_to_enkephalin", "Dr_Grandet_mode", "skip_enkephalin"),
            "mirror": (
                "set_mirror_count",
                "infinite_dungeons",
                "hard_mirror",
                "hard_mirror_target_floors",
                "no_weekly_bonuses",
                "floor_3_exit",
                "save_rewards",
                "hard_mirror_single_bonuses",
                "select_event_pack",
                "skip_event_pack",
                "re_claim_rewards",
                "not_skip_whitegossypium",
                "fight_to_last_man",
                "mirror_keyboard_navigation",
                "mirror_keyboard_simple_pathfinding",
                "mirror_prefer_hatred_and_despair",
                "mirror_minimize_non_boss_combat",
            ),
        }
        for section, fields in sections.items():
            section_values = values.get(section)
            if section_values is None:
                continue
            section_values = self._require_mapping(section_values, f"tasks.setConfig.{section}")
            for field in fields:
                if field in section_values:
                    updates[field] = section_values[field]

        completion = values.get("afterCompletion")
        if completion is not None:
            completion = self._require_mapping(completion, "tasks.setConfig.afterCompletion")
            if "actions" in completion:
                actions = completion["actions"]
                if not isinstance(actions, list) or not all(isinstance(item, str) for item in actions):
                    raise ValueError("afterCompletion.actions must be a string list")
                updates["after_completion_actions"] = list(actions)
            if "powerAction" in completion:
                updates["after_completion_power_action"] = self._require_string(
                    completion["powerAction"], "afterCompletion.powerAction"
                )
            if "keepAfterCompletion" in completion:
                updates["keep_after_completion"] = self._require_bool(
                    completion["keepAfterCompletion"], "afterCompletion.keepAfterCompletion"
                )

        resonate = values.get("resonate_with_Ahab")
        if isinstance(resonate, Mapping) and "enabled" in resonate:
            updates["resonate_with_Ahab"] = self._require_bool(resonate["enabled"], "resonate_with_Ahab.enabled")

        self._apply_config_updates(updates)
        return True

    @staticmethod
    def _execution_params(params: Any, method: str) -> dict[str, Any]:
        if params is None:
            return {}
        if not isinstance(params, Mapping):
            raise ValueError(f"{method} requires an object params value")
        return dict(params)

    @staticmethod
    def _execution_client_request_id(values: Mapping[str, Any]) -> str | None:
        value = values.get("clientRequestId")
        if value is None:
            return None
        if not isinstance(value, str) or not value.strip():
            raise ValueError("execution.start.clientRequestId must be a non-empty string")
        if len(value) > 256:
            raise ValueError("execution.start.clientRequestId is too long")
        return value

    @staticmethod
    def _execution_run_id_from_params(values: Mapping[str, Any]) -> str | None:
        value = values.get("runId")
        if value is None:
            return None
        if not isinstance(value, str) or not value.strip():
            raise ValueError("execution runId must be a non-empty string")
        return value

    @staticmethod
    def _execution_exception(code: str, message: str, *, retryable: bool = False) -> Exception:
        """Build the transport-level structured error without an import cycle."""

        numeric_codes = {
            "EXECUTION_BUSY": -32010,
            "INVALID_EXECUTION_STATE": -32011,
            "STALE_RUN": -32013,
            "INVALID_EXECUTION_OPTIONS": -32602,
            "RUNNER_CONFIG_FAILED": -32020,
            "DEVICE_TOOL_ACTIVE": -32020,
            "DEVICE_LEASE_ACTIVE": -32020,
            "RUNNER_SUPERVISION_FAILED": -32020,
            "DEVICE_RESTORE_FAILED": -32020,
        }
        try:
            from module.rpc_dispatcher import RpcDispatchError

            return RpcDispatchError(
                numeric_codes.get(code, -32020),
                message,
                {"code": code, "retryable": retryable, "userMessage": message},
            )
        except Exception:  # pragma: no cover - import-cycle/bootstrap fallback
            return DeviceError(message)

    @staticmethod
    def _validate_execution_start_options(options: Any) -> dict[str, Any]:
        """Validate the untrusted ``execution.start.options`` extension.

        The Runner spec contains process, lease, and configuration identity
        fields.  Forwarding a caller-provided mapping into that spec would let
        an RPC client replace those fields (even if later code happened to
        restore only some of them).  Until a concrete harmless option has a
        versioned contract, the explicit allowlist is intentionally empty.
        """

        if options is None:
            return {}
        if not isinstance(options, Mapping):
            raise ValueError("execution.start.options must be an object")
        for raw_key in options:
            if not isinstance(raw_key, str):
                raise BackendApplication._execution_exception(
                    "INVALID_EXECUTION_OPTIONS",
                    "execution.start.options field names must be strings",
                )
            if raw_key in _EXECUTION_START_RESERVED_FIELDS:
                raise BackendApplication._execution_exception(
                    "INVALID_EXECUTION_OPTIONS",
                    f"execution.start.options cannot override reserved field: {raw_key}",
                )
            if raw_key not in _EXECUTION_START_OPTION_ALLOWLIST:
                raise BackendApplication._execution_exception(
                    "INVALID_EXECUTION_OPTIONS",
                    f"execution.start.options contains unsupported field: {raw_key}",
                )
        return {key: copy.deepcopy(options[key]) for key in _EXECUTION_START_OPTION_ALLOWLIST if key in options}

    @staticmethod
    def _runner_snapshot_contains_sensitive(value: Any) -> bool:
        """Reject, rather than redact-and-run, unsafe injected snapshots."""

        if isinstance(value, Mapping):
            for raw_key, nested in value.items():
                key = str(raw_key).casefold()
                compact_key = re.sub(r"[^a-z0-9]", "", key)
                if key in _RUNNER_SENSITIVE_SNAPSHOT_KEYS or compact_key in _RUNNER_SENSITIVE_SNAPSHOT_KEYS:
                    return True
                if BackendApplication._runner_snapshot_contains_sensitive(nested):
                    return True
            return False
        if isinstance(value, (list, tuple, set)):
            return any(BackendApplication._runner_snapshot_contains_sensitive(item) for item in value)
        return False

    def _remember_execution_request_locked(self, request_id: str, response: Mapping[str, Any]) -> None:
        self._execution_client_requests[request_id] = copy.deepcopy(dict(response))
        self._execution_client_requests.move_to_end(request_id)
        while len(self._execution_client_requests) > self._execution_client_request_limit:
            self._execution_client_requests.popitem(last=False)

    def _set_execution_fields_locked(self, **changes: Any) -> dict[str, Any]:
        """Apply a schema-3 snapshot update while holding ``_lock``.

        The method is intentionally the only writer used by the new execution
        path.  Existing mediator callbacks still mutate a few legacy fields
        directly for compatibility, and the next explicit status update brings
        those fields back into the authoritative snapshot.
        """

        field_names = {
            "state": "_execution_state",
            "state_revision": "_execution_state_revision",
            "current_task_id": "_execution_task_id",
            "run_id": "_execution_run_id",
            "runner_pid": "_execution_runner_pid",
            "device_lease": "_execution_device_lease",
            "lease_generation": "_execution_lease_generation",
            "outcome": "_execution_outcome",
            "forced": "_execution_forced",
            "requested_by": "_execution_requested_by",
            "error": "_execution_error",
            "device_restore": "_execution_device_restore",
        }
        changed = False
        for key, value in changes.items():
            attribute = field_names.get(key)
            if attribute is None:
                continue
            if key == "state":
                value = getattr(value, "value", value)
                value = str(value)
            elif key == "device_lease":
                value = getattr(value, "value", value)
                value = str(value or "none")
            elif key == "error" and value is not None:
                value = dict(value) if isinstance(value, Mapping) else {"code": "EXECUTION_FAILED", "message": str(value)}
                if "message" not in value and value.get("userMessage"):
                    value["message"] = value["userMessage"]
                value.setdefault("code", "EXECUTION_FAILED")
                value.setdefault("phase", "runner")
                value.setdefault("recovery", "retry")
            if getattr(self, attribute) != value:
                setattr(self, attribute, value)
                changed = True
        if changed:
            self._execution_state_revision += 1
            self.emit("execution.status", self._execution_payload())
        return self._execution_payload()

    def _device_lease_state(self) -> str:
        value = getattr(self.device_manager, "lease_state", "none")
        try:
            value = value() if callable(value) else value
        except Exception:
            value = "none"
        if hasattr(value, "value"):
            value = value.value
        if value not in {"none", "acquiring", "runner", "restoring"}:
            return "none"
        return str(value)

    def _device_lease_active(self) -> bool:
        return self._execution_device_lease != "none" or self._device_lease_state() != "none"

    def _tool_is_active_locked(self) -> bool:
        for worker, _stop_event, _run_id in self._tool_workers.values():
            if worker.is_alive():
                return True
        return any(bool(runtime.get("running")) for runtime in self._tools.values() if isinstance(runtime, Mapping))

    def _assert_sidecar_device_available(self, operation: str) -> None:
        lease_state = self._device_lease_state()
        if self._execution_device_lease != "none" or lease_state != "none":
            run_id = self._execution_run_id or "unknown"
            raise self._execution_exception(
                "DEVICE_LEASE_ACTIVE",
                f"设备已由执行租约占用，拒绝{operation}（runId={run_id}）",
                retryable=True,
            )

    @staticmethod
    def _snapshot_mapping(snapshot: Any) -> dict[str, Any] | None:
        if snapshot is None:
            return None
        to_dict = getattr(snapshot, "to_dict", None)
        if callable(to_dict):
            try:
                value = to_dict()
                if isinstance(value, Mapping):
                    return dict(value)
            except Exception:
                return None
        if isinstance(snapshot, Mapping):
            return dict(snapshot)
        names = {
            "schemaVersion": "schema_version",
            "state": "state",
            "stateRevision": "state_revision",
            "currentTaskId": "current_task_id",
            "runId": "run_id",
            "runnerPid": "runner_pid",
            "deviceLease": "device_lease",
            "outcome": "outcome",
            "forced": "forced",
            "requestedBy": "requested_by",
            "error": "error",
            "deviceRestore": "device_restore",
        }
        value = {}
        for external, attribute in names.items():
            if hasattr(snapshot, attribute):
                item = getattr(snapshot, attribute)
                value[external] = getattr(item, "value", item)
        return value or None

    def _read_runner_snapshot(self) -> dict[str, Any] | None:
        supervisor = self._runner_supervisor
        if supervisor is None:
            return None
        getter = getattr(supervisor, "get_state", None)
        if callable(getter):
            try:
                return self._snapshot_mapping(getter())
            except Exception:
                log.debug("读取 Runner 状态失败", exc_info=True)
        try:
            return self._snapshot_mapping(getattr(supervisor, "snapshot", None))
        except Exception:
            return None

    def _runner_creation_time(self, runner_pid: Any) -> Any:
        """Read the supervisor's launch identity when it is not on the wire.

        ``RunnerSupervisor`` keeps the PID + creation-time pair on its private
        session because the public execution snapshot intentionally exposes
        only ``runnerPid``.  Consume that identity opportunistically here so a
        journal still has the PID-reuse guard without widening the execution
        module's public schema.  Compatibility supervisors may expose the
        same value through one of the public aliases below.
        """

        try:
            expected_pid = int(runner_pid)
        except (TypeError, ValueError):
            expected_pid = None
        supervisor = self._runner_supervisor
        if supervisor is None:
            return None
        candidates: list[Any] = []
        for name in ("process_identity", "runner_process_identity", "identity"):
            try:
                candidates.append(getattr(supervisor, name, None))
            except Exception:
                pass
        try:
            session = getattr(supervisor, "_session", None)
            if session is not None:
                candidates.append(getattr(session, "process_identity", None))
        except Exception:
            pass
        for candidate in candidates:
            if candidate is None:
                continue
            if isinstance(candidate, Mapping):
                candidate_pid = candidate.get("pid", candidate.get("runnerPid"))
                create_time = candidate.get("createTime", candidate.get("create_time"))
            else:
                candidate_pid = getattr(candidate, "pid", None)
                create_time = getattr(candidate, "createTime", getattr(candidate, "create_time", None))
            if expected_pid is not None and candidate_pid is not None:
                try:
                    if int(candidate_pid) != expected_pid:
                        continue
                except (TypeError, ValueError):
                    continue
            if create_time is not None:
                return create_time
        return None

    def _sync_runner_snapshot(self, snapshot: Any, *, emit: bool = True) -> dict[str, Any] | None:
        value = self._snapshot_mapping(snapshot)
        if value is None:
            return None
        run_id = value.get("runId")
        state_revision = value.get("stateRevision")
        try:
            state_revision = int(state_revision) if state_revision is not None else None
        except (TypeError, ValueError):
            state_revision = None
        with self._lock:
            if run_id is not None and self._execution_run_id not in {None, run_id}:
                # A supervisor callback from an old process must never alter a
                # newer run's state.
                return self._execution_payload()
            if state_revision is not None and state_revision <= self._execution_supervisor_revision:
                return self._execution_payload()
            if state_revision is not None:
                self._execution_supervisor_revision = state_revision
            state = value.get("state", self._execution_state)
            state = getattr(state, "value", state)
            state = str(state)
            manager_lease = self._device_lease_state()
            lease = value.get("deviceLease", self._execution_device_lease)
            lease = getattr(lease, "value", lease)
            lease = str(lease or "none")
            # DeviceManager is the capability authority.  A supervisor's
            # initial ``acquiring`` snapshot cannot downgrade a lease already
            # acquired by the sidecar, and an idle runner cannot hide pending
            # device restoration.
            if manager_lease != "none":
                lease = manager_lease
                if state == "idle":
                    state = "restoring"
            changes = {
                "state": state,
                "current_task_id": value.get("currentTaskId", self._execution_task_id),
                "run_id": run_id if run_id is not None else self._execution_run_id,
                "runner_pid": value.get("runnerPid"),
                "device_lease": lease,
                "outcome": value.get("outcome"),
                "forced": bool(value.get("forced", False)),
                "requested_by": value.get("requestedBy"),
                "error": value.get("error"),
                "device_restore": value.get("deviceRestore", self._execution_device_restore),
            }
            payload = self._set_execution_fields_locked(**changes)
            if state_revision is not None and state_revision > self._execution_state_revision:
                self._execution_state_revision = state_revision
                payload = self._execution_payload()
            runner_pid = value.get("runnerPid")
            runner_create_time = value.get("runnerCreateTime", value.get("runner_create_time"))
            if runner_create_time is None:
                runner_create_time = self._runner_creation_time(runner_pid)
            self._journal_runner_process(run_id, runner_pid, runner_create_time)
            if not emit:
                return payload
            return payload

    @staticmethod
    def _invoke_compatible(method: Callable[..., Any], args: tuple[Any, ...], kwargs: dict[str, Any]) -> Any:
        """Invoke a known compatibility variant without swallowing body errors."""

        try:
            signature = inspect.signature(method)
        except (TypeError, ValueError):
            return method(*args, **kwargs)
        try:
            signature.bind(*args, **kwargs)
        except TypeError:
            if kwargs:
                signature.bind(*args)
                return method(*args)
            raise
        return method(*args, **kwargs)

    def _runner_result_snapshot(self, result: Any) -> dict[str, Any] | None:
        snapshot = getattr(result, "snapshot", None)
        if snapshot is not None:
            return self._snapshot_mapping(snapshot)
        if isinstance(result, Mapping):
            return dict(result)
        return self._snapshot_mapping(result) or self._read_runner_snapshot()

    def _runner_result_payload(self, result: Any) -> dict[str, Any]:
        with self._lock:
            payload = self._execution_payload()
        accepted = getattr(result, "accepted", None)
        if accepted is None and isinstance(result, Mapping):
            accepted = result.get("accepted", True)
        payload["accepted"] = bool(True if accepted is None else accepted)
        error = getattr(result, "error", None)
        if error is None and isinstance(result, Mapping):
            error = result.get("error")
        if error is not None:
            payload["error"] = dict(error) if isinstance(error, Mapping) else {"message": str(error)}
        return payload

    def _call_runner_control(self, name: str, run_id: str, **kwargs: Any) -> Any:
        supervisor = self._runner_supervisor
        method = getattr(supervisor, name, None) if supervisor is not None else None
        if not callable(method):
            aliases = {"request_stop": "stop", "request_pause": "pause", "request_resume": "resume"}
            method = getattr(supervisor, aliases[name], None) if supervisor is not None else None
        if not callable(method):
            raise self._execution_exception("RUNNER_SUPERVISION_FAILED", f"Runner 不支持 {name}")
        return self._invoke_compatible(method, (run_id,), kwargs)

    def _call_runner_start(
        self,
        spec: Mapping[str, Any],
        *,
        run_id: str,
        client_request_id: str | None,
    ) -> Any:
        supervisor = self._runner_supervisor
        method = getattr(supervisor, "try_start", None) if supervisor is not None else None
        if not callable(method):
            method = getattr(supervisor, "start", None) if supervisor is not None else None
        if not callable(method):
            raise self._execution_exception("RUNNER_SUPERVISION_FAILED", "Runner 不支持启动")
        kwargs: dict[str, Any] = {"run_id": run_id}
        if client_request_id is not None:
            kwargs["client_request_id"] = client_request_id
        return self._invoke_compatible(method, (spec,), kwargs)

    def _create_runner_config(self, run_id: str) -> dict[str, Any]:
        """Create and validate one private, secret-free Runner config.

        A Runner launch without this manifest is unsafe: it would either have
        no config path or accidentally reuse the sidecar's live configuration.
        Every repository/shape failure therefore becomes the same structured
        ``RUNNER_CONFIG_FAILED`` error.  The repository cleanup hook is also
        attempted for malformed/partially-created manifests.
        """

        repository = self._config_repository
        if repository is None:
            raise self._execution_exception(
                "RUNNER_CONFIG_FAILED",
                "Runner 临时配置不可用",
                retryable=True,
            )
        creator = getattr(repository, "create_run_config", None)
        if not callable(creator):
            for name in ("create_runner_config", "create_execution_config"):
                creator = getattr(repository, name, None)
                if callable(creator):
                    break
        if not callable(creator):
            raise self._execution_exception(
                "RUNNER_CONFIG_FAILED",
                "Runner 临时配置不可用",
                retryable=True,
            )
        try:
            value = self._invoke_compatible(creator, (run_id,), {})
            if not isinstance(value, Mapping):
                raise ValueError("Runner config manifest is not an object")
            manifest = copy.deepcopy(dict(value))
            manifest_run_id = manifest.get("runId", run_id)
            config_path = manifest.get("configPath")
            snapshot = manifest.get("snapshot")
            baseline = manifest.get("baseline")
            if manifest_run_id != run_id:
                raise ValueError("Runner config runId does not match execution run")
            if isinstance(config_path, Path):
                config_path = str(config_path)
            if not isinstance(config_path, str) or not config_path.strip() or not Path(config_path).is_absolute():
                raise ValueError("Runner configPath must be an absolute path")
            if not isinstance(snapshot, Mapping):
                raise ValueError("Runner config snapshot is missing")
            if self._runner_snapshot_contains_sensitive(snapshot):
                raise ValueError("Runner config snapshot contains sensitive fields")
            if baseline is not None:
                if not isinstance(baseline, Mapping) or self._runner_snapshot_contains_sensitive(baseline):
                    raise ValueError("Runner config baseline is invalid")
            revision = manifest.get("configRevision", manifest.get("baseRevision", 0))
            if isinstance(revision, bool) or not isinstance(revision, int) or revision < 0:
                raise ValueError("Runner config revision is invalid")
            # Only retain the fields the sidecar contract consumes.  Unknown
            # repository metadata must not become an accidental Runner input.
            safe_manifest: dict[str, Any] = {
                "runId": run_id,
                "configPath": config_path,
                "configRevision": revision,
                "baseRevision": manifest.get("baseRevision", revision),
                "baseConfigHash": manifest.get("baseConfigHash"),
                "snapshot": dict(snapshot),
            }
            if baseline is not None:
                safe_manifest["baseline"] = dict(baseline)
            return safe_manifest
        except Exception as error:
            # The error sent to GPUI deliberately contains no repository
            # exception text; custom repositories may include credential data
            # in their exception messages.  Cleanup is best effort and is
            # repeated safely by normal finalize/recovery paths when needed.
            try:
                self._cleanup_runner_config(run_id)
            except Exception:
                log.debug("清理失败的 Runner 临时配置失败", exc_info=True)
            raise self._execution_exception(
                "RUNNER_CONFIG_FAILED",
                "Runner 临时配置创建失败",
                retryable=True,
            ) from error

    def _cleanup_runner_config(self, run_id: str) -> Exception | None:
        with self._lock:
            if run_id in self._runner_config_cleanup_done:
                return None
        repository = self._config_repository
        if repository is None:
            return None
        cleaner = getattr(repository, "cleanup_run_config", None)
        if not callable(cleaner):
            for name in ("remove_run_config", "cleanup_runner_config"):
                cleaner = getattr(repository, name, None)
                if callable(cleaner):
                    break
        if not callable(cleaner):
            return self._execution_exception("CONFIG_REPOSITORY_UNAVAILABLE", "ConfigRepository 不支持清理 Runner 配置")
        try:
            self._invoke_compatible(cleaner, (run_id,), {})
        except Exception as error:
            return error
        with self._lock:
            self._runner_config_cleanup_done.add(run_id)
        return None

    def _cleanup_ledger_directory(self) -> Path:
        root = self._cleanup_ledger_root
        if root is None:
            root = _cleanup_root_from_config(self.config)
            self._cleanup_ledger_root = root
        root.mkdir(parents=True, exist_ok=True)
        return root

    def _create_cleanup_ledger(
        self,
        run_id: str,
        *,
        target: Mapping[str, Any] | None = None,
        generation: int | None = None,
    ) -> Any:
        """Create one durable journal and bind the Runner event adapter."""

        target_id = target.get("id") if isinstance(target, Mapping) else None
        path = self._cleanup_ledger_directory() / _safe_cleanup_filename(run_id)
        factory = self._cleanup_ledger_factory
        if factory is None:
            from module.execution.cleanup_ledger import CleanupLedger

            factory = CleanupLedger
        kwargs: dict[str, Any] = {"run_id": run_id}
        if isinstance(target_id, str) and target_id:
            kwargs["target_id"] = target_id
        if isinstance(generation, int) and not isinstance(generation, bool) and generation >= 0:
            kwargs["generation"] = generation
        ledger = _invoke_recovery_callback(
            factory,
            (
                ((path,), kwargs),
                ((path, run_id), {}),
                ((run_id,), {"path": path}),
                ((path,), {}),
                ((run_id,), {}),
            ),
        )
        if ledger is None:
            raise RuntimeError("cleanup ledger factory returned no ledger")
        with self._lock:
            self._cleanup_ledgers[run_id] = ledger
        adapter = self._runner_event_adapter
        if adapter is not None:
            adapter.run_id = run_id
            adapter.ledger = ledger
            adapter.cleanup_ledger = ledger
        return ledger

    @staticmethod
    def _ledger_call(ledger: Any, name: str, *args: Any, **kwargs: Any) -> Any:
        callback = getattr(ledger, name, None)
        if not callable(callback):
            return None
        return _invoke_recovery_callback(callback, (((args), kwargs), ((args), {})))

    def _seed_cleanup_ledger(
        self,
        run_id: str,
        lease: Mapping[str, Any],
        *,
        generation: int | None = None,
        runner_pid: int | None = None,
        runner_create_time: Any = None,
    ) -> Any:
        """Persist target/lease identity before the Runner launch command."""

        target = lease.get("target") if isinstance(lease.get("target"), Mapping) else {}
        generation = lease.get("generation") if generation is None else generation
        generation = generation if isinstance(generation, int) and not isinstance(generation, bool) else None
        ledger = self._create_cleanup_ledger(run_id, target=target, generation=generation)
        metadata = {
            "runId": run_id,
            "target": dict(target),
            "deviceTarget": dict(target),
            "deviceLease": dict(lease),
        }
        if isinstance(metadata["target"], Mapping):
            # ResourceCleanupExecutor accepts both the compact target snapshot
            # and the flattened legacy reservation fields.
            if metadata["target"].get("kind") in {"adb", "mumu"} and metadata["target"].get("endpoint"):
                metadata["deviceSerial"] = metadata["target"]["endpoint"]
            if metadata["target"].get("kind") == "pc" and metadata["target"].get("hwnd") is not None:
                metadata["hwnd"] = metadata["target"]["hwnd"]
        self._ledger_call(ledger, "update_metadata", metadata)

        reservation: dict[str, Any] = {}
        if generation is not None:
            reservation["generation"] = generation
        target_kind = target.get("kind") if isinstance(target, Mapping) else None
        if target_kind in {"adb", "mumu"} and target.get("endpoint"):
            reservation["deviceSerial"] = str(target["endpoint"])
        if target_kind == "pc" and target.get("hwnd") is not None:
            reservation["hwnd"] = target["hwnd"]
        # A lease implementation may have already reserved scoped ADB/window
        # resources.  Copy only CleanupLedger's allow-listed identifiers.
        for key in (
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
        ):
            if key in lease and lease[key] is not None:
                reservation[key] = lease[key]
        if runner_pid is not None:
            reservation["runnerPid"] = runner_pid
            if runner_create_time is not None:
                reservation["runnerCreateTime"] = runner_create_time
        if reservation:
            self._ledger_call(ledger, "reserve", **reservation)
        return ledger

    def _cleanup_ledger_for_run(self, run_id: str | None) -> Any:
        if not isinstance(run_id, str) or not run_id:
            return None
        with self._lock:
            return self._cleanup_ledgers.get(run_id)

    def _journal_runner_event(self, message: Mapping[str, Any]) -> None:
        """Persist durable Runner obligations before forwarding the event."""

        run_id = message.get("runId")
        ledger = self._cleanup_ledger_for_run(run_id)
        if ledger is None:
            return
        message_type = str(message.get("type", ""))
        try:
            if message_type in {"resource.created", "resource.released"}:
                self._ledger_call(ledger, "record_resource_event", dict(message))
            elif message_type == "config.delta":
                self._ledger_call(ledger, "record_config_delta", dict(message))
            elif message_type == "finished":
                sequence = message.get("seq")
                if sequence is not None:
                    self._ledger_call(ledger, "record_final_seq", sequence)
                self._ledger_call(ledger, "record_finished", dict(message))
            elif message_type == "afterCompletion.requested":
                actions = message.get("actions")
                if isinstance(actions, Mapping):
                    actions = [actions]
                if not isinstance(actions, list):
                    actions = [message]
                for index, action in enumerate(actions):
                    if not isinstance(action, Mapping):
                        continue
                    value = dict(action)
                    value.setdefault(
                        "actionId",
                        f"{run_id}:after-completion:{message.get('seq', index)}:{index}",
                    )
                    value.setdefault("actionType", value.get("type", "after_completion"))
                    self._ledger_call(ledger, "record_action_pending", value)
            elif message_type in {"action", "afterCompletion.action", "action.state"}:
                action_id = message.get("actionId", message.get("id"))
                action_type = message.get("actionType", message.get("type", "after_completion"))
                state = str(message.get("state", message.get("status", "pending"))).casefold()
                if isinstance(action_id, str) and action_id:
                    if state == "pending":
                        self._ledger_call(
                            ledger,
                            "record_action_pending",
                            {"actionId": action_id, "actionType": str(action_type), "payload": dict(message)},
                        )
                    elif state in {"executing", "done", "failed"}:
                        self._ledger_call(ledger, "set_action_state", action_id, state, dict(message))
        except Exception as error:
            # A malformed optional Runner event must not kill the event reader;
            # retain a visible diagnostic while keeping the current run alive.
            log.warning("持久化 Runner 事件失败（%s）：%s", message_type, error)

    def _journal_finalization(
        self,
        run_id: str,
        *,
        outcome: Any,
        forced: bool,
        requested_by: Any = None,
        error: Mapping[str, Any] | None = None,
    ) -> None:
        ledger = self._cleanup_ledger_for_run(run_id)
        if ledger is None:
            return
        try:
            if getattr(ledger, "finished", None) is None:
                value: dict[str, Any] = {
                    "runId": run_id,
                    "outcome": getattr(outcome, "value", outcome),
                    "forced": bool(forced),
                }
                if requested_by is not None:
                    value["requestedBy"] = getattr(requested_by, "value", requested_by)
                if error is not None:
                    value["error"] = dict(error)
                self._ledger_call(ledger, "record_finished", value)
        except Exception as journal_error:
            log.warning("持久化 Runner 终态失败：%s", journal_error)

    def _mark_runner_process_done(self, run_id: str, ledger: Any) -> None:
        action_id = f"{run_id}:runner-process"
        try:
            action = getattr(ledger, "action", lambda _action_id: None)(action_id)
            if isinstance(action, Mapping) and action.get("state") != "done":
                self._ledger_call(ledger, "mark_action_done", action_id, {"status": "exited"})
        except Exception as error:
            log.warning("记录 Runner 进程清理完成失败：%s", error)

    def _journal_runner_process(self, run_id: str | None, pid: Any, create_time: Any = None) -> None:
        if not isinstance(run_id, str) or not run_id:
            return
        if isinstance(pid, bool):
            return
        try:
            process_id = int(pid)
        except (TypeError, ValueError):
            return
        if process_id <= 0:
            return
        ledger = self._cleanup_ledger_for_run(run_id)
        if ledger is None:
            return
        try:
            self._ledger_call(ledger, "reserve_process", process_id, create_time)
        except Exception as error:
            log.warning("持久化 Runner 进程身份失败：%s", error)

    def _run_cleanup_ledger(self, run_id: str) -> dict[str, Any] | None:
        ledger = self._cleanup_ledger_for_run(run_id)
        if ledger is None:
            return None
        self._mark_runner_process_done(run_id, ledger)
        delegate = self._cleanup_executor
        if delegate is None:
            try:
                delegate = _build_default_cleanup_executor(run_id, ledger)
            except Exception as error:
                log.warning("创建资源清理执行器失败：%s", error)
                value = {
                    "runId": run_id,
                    "status": "deferred",
                    "complete": False,
                    "pending": True,
                    "resourceComplete": False,
                    "error": {"code": "CLEANUP_EXECUTOR_UNAVAILABLE", "message": str(error)},
                }
                _mark_cleanup_device_disconnected(ledger, value)
                return value
        executor = _LedgerRecoveryExecutor(
            delegate,
            process_identity_probe=self._process_identity_probe,
            process_terminator=self._process_terminator,
        )
        try:
            recover = getattr(ledger, "recover", None)
            if callable(recover):
                result = recover(executor, delete_complete=False, retry_failed=True)
                value = result.to_dict() if hasattr(result, "to_dict") else dict(result)
                for action in getattr(result, "actions", ()):
                    if not isinstance(action, Mapping):
                        action = action.to_dict() if hasattr(action, "to_dict") else {}
                    state = str(action.get("state", "pending"))
                    outcome = "done" if state == "done" else "failed" if state == "failed" else "pending"
                    self._ledger_call(
                        ledger,
                        "record_cleanup_step",
                        str(action.get("actionId", "cleanup")),
                        outcome=outcome,
                        detail=action.get("detail"),
                    )
                actions = getattr(ledger, "actions", ())
                resource_actions = []
                for action in actions:
                    if not isinstance(action, Mapping):
                        continue
                    action_type = str(action.get("actionType", "")).casefold()
                    if action_type not in {
                        "after_completion",
                        "notification",
                        "toast",
                        "sound",
                        "exit_aalc",
                        "power",
                    }:
                        resource_actions.append(action)
                value["resourceComplete"] = all(item.get("state") == "done" for item in resource_actions)
            else:
                raw = executor.execute(ledger)
                value = dict(raw) if isinstance(raw, Mapping) else {"status": "success", "result": raw}
                value.setdefault("resourceComplete", value.get("status") in {"success", "done", "complete"})
            if value.get("complete") is True or getattr(ledger, "complete", False):
                try:
                    self._ledger_call(ledger, "delete")
                except Exception as error:
                    value.setdefault("errors", []).append(
                        {"code": "CLEANUP_LEDGER_DELETE_FAILED", "message": str(error)}
                    )
            else:
                _mark_cleanup_device_disconnected(ledger, value)
            return value
        except Exception as error:
            log.exception("CleanupLedger 执行失败：%s", run_id)
            return {
                "runId": run_id,
                "status": "failed",
                "complete": False,
                "pending": True,
                "error": {"code": "CLEANUP_RECOVERY_FAILED", "message": str(error)},
            }

    def _apply_runner_config_delta(self, message: Mapping[str, Any]) -> dict[str, Any] | None:
        repository = self._config_repository
        if repository is None:
            self.emit(
                "app.notice",
                {
                    "level": "warning",
                    "code": "CONFIG_REPOSITORY_UNAVAILABLE",
                    "message": "Runner config.delta 未应用：ConfigRepository 不可用",
                    "runId": message.get("runId"),
                },
            )
            return None
        apply_delta = getattr(repository, "apply_delta", None)
        if not callable(apply_delta):
            apply_delta = getattr(repository, "apply_config_delta", None)
        if not callable(apply_delta):
            self.emit(
                "app.notice",
                {
                    "level": "warning",
                    "code": "CONFIG_REPOSITORY_UNAVAILABLE",
                    "message": "Runner config.delta 未应用：ConfigRepository 不支持 apply_delta",
                    "runId": message.get("runId"),
                },
            )
            return None
        try:
            result = self._invoke_compatible(apply_delta, (dict(message),), {})
        except Exception as error:
            code = getattr(error, "code", None)
            if not code and error.__class__.__name__ == "ConfigConflictError":
                code = "CONFIG_CONFLICT"
            code = code or "CONFIG_DELTA_FAILED"
            self.emit(
                "app.notice",
                {
                    "level": "warning",
                    "code": str(code),
                    "message": f"Runner config.delta 未应用：{error}",
                    "runId": message.get("runId"),
                    "deltaId": message.get("deltaId"),
                },
            )
            return {"status": "error", "code": str(code), "message": str(error)}
        if isinstance(result, Mapping):
            result = dict(result)
            if result.get("conflicts") or result.get("warnings"):
                self.emit(
                    "app.notice",
                    {
                        "level": "warning",
                        "code": "CONFIG_DELTA_CONFLICT" if result.get("conflicts") else "CONFIG_DELTA_WARNING",
                        "message": "Runner config.delta 检测到并发配置变更",
                        "runId": message.get("runId"),
                        "deltaId": message.get("deltaId"),
                        "conflicts": list(result.get("conflicts", [])),
                        "warnings": list(result.get("warnings", [])),
                    },
                )
            return result
        return None

    def _runner_event_is_new_locked(self, run_id: str | None, message: Mapping[str, Any]) -> bool:
        if run_id is None:
            return False
        sequence = message.get("seq")
        if isinstance(sequence, bool) or not isinstance(sequence, int) or sequence < 0:
            return True
        seen = self._runner_event_sequences.setdefault(run_id, set())
        if sequence in seen or (seen and sequence < max(seen)):
            return False
        seen.add(sequence)
        # A Runner sequence is monotonic and bounded in practice.  Keep only a
        # recent window so a long execution cannot retain unbounded event IDs.
        if len(seen) > 4096:
            for old in sorted(seen)[: len(seen) - 2048]:
                seen.discard(old)
        return True

    def _start_runner_execution(
        self,
        run_id: str,
        task_id: str,
        tasks: Mapping[str, Any],
        values: Mapping[str, Any],
        *,
        client_request_id: str | None,
        start_options: Mapping[str, Any] | None = None,
    ) -> dict[str, Any]:
        deadline = time.monotonic() + 15.0
        stop_and_wait = getattr(self._preview_capture, "stop_and_wait", None)
        if callable(stop_and_wait):
            stopped = self._invoke_compatible(stop_and_wait, (), {"deadline": deadline})
            if stopped is False:
                raise self._execution_exception("PREVIEW_QUIESCE_TIMEOUT", "实时预览未能在截止时间前停止", retryable=True)
        else:
            self._preview_capture.stop()
        with self._lock:
            self._set_execution_fields_locked(device_lease="acquiring", device_restore="pending")

        suspend = getattr(self.device_manager, "suspend_for_execution", None)
        if not callable(suspend):
            raise self._execution_exception("DEVICE_LEASE_UNAVAILABLE", "当前设备管理器不支持执行租约")
        lease = self._invoke_compatible(suspend, (run_id,), {"deadline": deadline})
        lease_value = dict(lease) if isinstance(lease, Mapping) else self._snapshot_mapping(lease) or {}
        target = lease_value.get("target", {})
        generation = lease_value.get("generation")
        if generation is None:
            generation = getattr(self.device_manager, "execution_generation", None)
        with self._lock:
            self._set_execution_fields_locked(
                device_lease=lease_value.get("state", "runner"),
                lease_generation=generation,
            )
        # The journal is created only after the device lease is authoritative,
        # but always before Runner launch.  A crash during the launch window
        # therefore still leaves the exact target/generation available for
        # startup compensation.
        self._seed_cleanup_ledger(run_id, lease_value, generation=generation)

        # Acquire the lease before creating the private config so every
        # failure after resource handoff goes through the normal finalize path
        # and restores the lease.  In particular, a broken repository must not
        # leave a suspended device behind while the Runner is never started.
        run_config = self._create_runner_config(run_id)
        with self._lock:
            self._execution_run_config = run_config

        task_config = tasks.get(task_id, {})
        if not isinstance(task_config, Mapping):
            task_config = {}
        snapshot = run_config["snapshot"]
        config_path = run_config["configPath"]
        config_revision = run_config["configRevision"]
        base_config_hash = run_config.get("baseConfigHash")
        baseline = run_config.get("baseline")
        config_baseline = baseline if isinstance(baseline, Mapping) else snapshot
        # ``start_options`` has already crossed the explicit allowlist gate in
        # execution_start.  Keep this defensive validation for direct/internal
        # callers of this method as well.
        extra = self._validate_execution_start_options(start_options)
        target_value = dict(target) if isinstance(target, Mapping) else {}
        spec = {
            "runId": run_id,
            "taskId": task_id,
            "taskConfig": dict(task_config),
            "runtimeConfig": dict(snapshot),
            "configPath": config_path,
            "configRevision": config_revision,
            "baseConfigHash": base_config_hash,
            "configBaseline": dict(config_baseline),
            "baseline": dict(config_baseline),
            "baselineValues": dict(config_baseline),
            "resourceRoot": self._config_value("resource_root", None),
            "deviceTarget": target_value,
            "deviceLease": lease_value,
            # These sidecar-owned values are explicit even though the current
            # Runner implementation derives them from its launch context.
            # They can never be supplied through execution.start.options.
            "allowEmulatorLaunch": False,
            "parentPid": os.getpid(),
            **extra,
        }
        result = self._call_runner_start(spec, run_id=run_id, client_request_id=client_request_id)
        accepted = getattr(result, "accepted", None)
        if accepted is None and isinstance(result, Mapping):
            accepted = result.get("accepted", True)
        if accepted is False:
            error = getattr(result, "error", None)
            if error is None and isinstance(result, Mapping):
                error = result.get("error")
            if isinstance(error, Mapping):
                code = str(error.get("code", "RUNNER_SUPERVISION_FAILED"))
                message = str(error.get("message", code))
            else:
                code, message = "RUNNER_SUPERVISION_FAILED", "Runner 拒绝启动"
            raise self._execution_exception(code, message, retryable=code in {"EXECUTION_BUSY", "RUNNER_BUSY"})
        self._sync_runner_snapshot(self._runner_result_snapshot(result), emit=True)
        monitor = threading.Thread(
            target=self._monitor_runner_execution,
            args=(run_id,),
            name=f"AALCRunnerMonitor-{run_id}",
            daemon=True,
        )
        with self._lock:
            self._runner_monitor = monitor
        monitor.start()
        with self._lock:
            return {**self._execution_payload(), "accepted": True}

    def _monitor_runner_execution(self, run_id: str) -> None:
        while True:
            snapshot = self._read_runner_snapshot()
            if snapshot is not None:
                self._sync_runner_snapshot(snapshot, emit=True)
                state = str(snapshot.get("state", ""))
                snapshot_run = snapshot.get("runId")
                if snapshot_run not in {None, run_id}:
                    return
                if state == "idle":
                    self._finalize_runner_execution(run_id, snapshot=snapshot)
                    return
            with self._lock:
                if self._closed and self._execution_run_id != run_id:
                    return
            time.sleep(0.05)

    def _fail_runner_execution(self, run_id: str, error: BaseException) -> None:
        error_data = getattr(error, "data", None)
        if isinstance(error_data, Mapping) and error_data.get("code"):
            detail = dict(error_data)
        else:
            code = getattr(error, "code", "RUNNER_SUPERVISION_FAILED")
            detail = {"code": str(code), "message": redact_text(error, ())}
        detail.setdefault("message", str(error))
        detail.setdefault("phase", "runner")
        detail.setdefault("recovery", "retry")
        self._finalize_runner_execution(run_id, outcome="failed", error=detail)

    def _finalize_runner_execution(
        self,
        run_id: str,
        *,
        snapshot: Mapping[str, Any] | None = None,
        outcome: str | None = None,
        error: Mapping[str, Any] | None = None,
    ) -> dict[str, Any]:
        with self._lock:
            if run_id in self._execution_finalized_runs:
                return self._execution_payload(run_id=run_id)
            if self._execution_run_id not in {None, run_id}:
                return self._execution_payload()
            self._execution_finalized_runs.add(run_id)
            value = dict(snapshot or {})
            final_outcome = outcome or value.get("outcome") or ("stopped" if self._execution_stop.is_set() else "completed")
            final_error = dict(error or value.get("error")) if (error or value.get("error")) else None
            forced = bool(value.get("forced", self._execution_forced))
            requested_by = value.get("requestedBy", self._execution_requested_by)
            self._set_execution_fields_locked(
                state="restoring",
                device_lease="restoring",
                outcome=final_outcome,
                forced=forced,
                requested_by=requested_by,
                error=final_error,
                device_restore="pending",
            )
            generation = self._execution_lease_generation
            lease_value = getattr(self.device_manager, "execution_lease", None)
            if callable(lease_value):
                try:
                    lease_value = lease_value()
                except Exception:
                    lease_value = None
            if isinstance(lease_value, Mapping):
                generation = lease_value.get("generation", generation)
                target = lease_value.get("target")
            else:
                target = None
            lease_state = self._device_lease_state()
            # ``acquiring`` is only an application guard until DeviceManager
            # returns a concrete lease.  Do not report a failed restoration for
            # a suspend call that failed before capability transfer.
            if lease_state == "none" and not isinstance(lease_value, Mapping):
                lease_state = "none"
            elif lease_state == "none":
                lease_state = self._execution_device_lease
            if target is None and isinstance(value.get("deviceTarget"), Mapping):
                target = value.get("deviceTarget")

        # Preserve a synthetic finish when the supervisor only exposed its
        # terminal snapshot (for example after a sidecar restart or a fake
        # process exit).  If an actual ``finished`` frame was already journaled
        # this is a no-op and validates the same run identity.
        self._journal_finalization(
            run_id,
            outcome=final_outcome,
            forced=forced,
            requested_by=requested_by,
            error=final_error,
        )
        restore_state = "not_needed"
        if lease_state != "none" or isinstance(lease_value, Mapping):
            restore_state = "restored"
            resume = getattr(self.device_manager, "resume_after_execution", None)
            if callable(resume):
                try:
                    restored = self._invoke_compatible(
                        resume,
                        (run_id,),
                        {"generation": generation, "deadline": time.monotonic() + 15.0},
                    )
                    if isinstance(restored, Mapping) and restored.get("status") == "disconnected":
                        restore_state = "disconnected"
                except Exception as restore_error:
                    restore_state = "failed"
                    final_error = final_error or {
                        "code": "DEVICE_RESTORE_FAILED",
                        "message": redact_text(restore_error, ()),
                        "phase": "restore",
                        "recovery": "reconnect_device",
                    }
            else:
                restore_state = "failed"
                final_error = final_error or {
                    "code": "DEVICE_RESTORE_FAILED",
                    "message": "当前设备管理器不支持恢复执行租约",
                    "phase": "restore",
                    "recovery": "reconnect_device",
                }
            target_id = target.get("id") if isinstance(target, Mapping) else None
            if self._preview_enabled and isinstance(target_id, str) and target_id:
                try:
                    start = getattr(self._preview_capture, "start", None)
                    if callable(start):
                        self._invoke_compatible(start, (target_id,), {"generation": generation})
                        self._preview_device_id = target_id
                except Exception:
                    log.debug("执行完成后恢复实时预览失败", exc_info=True)

        cleanup_error = self._cleanup_runner_config(run_id)
        if cleanup_error is not None and final_error is None:
            final_error = {
                "code": "RUNNER_CONFIG_CLEANUP_FAILED",
                "message": redact_text(cleanup_error, ()),
                "phase": "cleanup",
                "recovery": "retry",
            }

        cleanup_result = self._run_cleanup_ledger(run_id)
        if isinstance(cleanup_result, Mapping):
            if cleanup_result.get("deviceDisposition") == "disconnected":
                restore_state = "disconnected"
            elif cleanup_result.get("status") == "failed":
                restore_state = "failed"
                if final_error is None:
                    final_error = {
                        "code": "CLEANUP_FAILED",
                        "message": "Runner 资源清理失败",
                        "phase": "cleanup",
                        "recovery": "retry",
                    }
            elif not cleanup_result.get("complete", cleanup_result.get("resourceComplete", False)):
                restore_state = "pending"

        stats_completed = True
        with self._lock:
            self._set_execution_fields_locked(
                state="idle",
                current_task_id=None,
                runner_pid=None,
                device_lease="none",
                run_id=run_id,
                outcome=final_outcome,
                forced=forced,
                requested_by=requested_by,
                error=final_error,
                device_restore=restore_state,
            )
            self._execution_thread = None
            self._execution_worker = None
            if self._execution_run_id == run_id:
                self._execution_run_config = None
            if run_id not in self._execution_stats_finished:
                self._execution_stats_finished.add(run_id)
                try:
                    self.emit("execution.stats", self.stats.finish_run(run_id))
                except Exception:
                    stats_completed = False
                    log.debug("完成 Runner 统计失败", exc_info=True)
            final_payload = self._execution_payload(run_id=run_id)

        # Config deltas have been applied before ``finished`` is delivered,
        # the per-run config snapshot is now cleaned, and statistics are
        # finalized above.  Only this fully successful boundary may invoke
        # sidecar completion actions.  The coordinator itself is also
        # outcome/forced aware, but keeping the gate here prevents an injected
        # compatibility coordinator from accidentally acting on a failed run.
        completion_result: Any = None
        final_outcome_value = getattr(final_outcome, "value", final_outcome)
        cleanup_completed = cleanup_result is None or bool(
            cleanup_result.get("resourceComplete", cleanup_result.get("complete", False))
        )
        can_execute_completion = (
            str(final_outcome_value).strip().lower() == "completed"
            and not forced
            and cleanup_error is None
            and cleanup_completed
            and stats_completed
            and final_error is None
        )
        if can_execute_completion:
            completion_result = self._execute_runner_after_completion(
                run_id,
                outcome=str(final_outcome_value),
                forced=forced,
            )
        else:
            completion_result = {
                "runId": run_id,
                "outcome": str(final_outcome_value),
                "forced": forced,
                "skipped": True,
                "reason": (
                    "completion actions require completed outcome, non-forced run, "
                    "successful Runner cleanup and statistics finalization"
                ),
            }
        with self._lock:
            if isinstance(completion_result, Mapping):
                self._execution_after_completion_results[run_id] = copy.deepcopy(dict(completion_result))
            self._execution_after_completion_requests.pop(run_id, None)
        return final_payload

    def _runner_completion_request(self, run_id: str, *, outcome: str, forced: bool) -> dict[str, Any]:
        """Build the sidecar completion request after Runner cleanup.

        Runner-owned ``exit_game``/``exit_emulator`` entries are deliberately
        excluded here.  They have already run (or failed) in the leased
        Runner and must never be replayed by the sidecar coordinator.
        """

        with self._lock:
            requests = [copy.deepcopy(value) for value in self._execution_after_completion_requests.get(run_id, [])]

        actions: list[Any] = []
        power_action: Any = None

        def is_sidecar_action(action: Any) -> bool:
            raw_action = (
                action.get("type", action.get("actionType", action.get("action", "")))
                if isinstance(action, Mapping)
                else action
            )
            return str(getattr(raw_action, "value", raw_action)) not in {"exit_game", "exit_emulator"}

        for request in requests:
            if any(key in request for key in ("type", "actionType", "action")) and not any(
                key in request for key in ("actions", "sidecarActions", "afterCompletion")
            ):
                if is_sidecar_action(request):
                    actions.append(request)
            raw_actions = request.get("sidecarActions")
            if raw_actions is None:
                raw_actions = request.get("afterCompletion", request.get("actions", []))
            if isinstance(raw_actions, Mapping):
                nested = raw_actions
                raw_actions = nested.get("sidecarActions", nested.get("actions", []))
                if power_action is None:
                    power_action = nested.get("powerAction", nested.get("power_action"))
            if isinstance(raw_actions, (str, Mapping)):
                raw_actions = [raw_actions]
            if isinstance(raw_actions, Iterable):
                actions.extend(
                    value
                    for value in raw_actions
                    if not isinstance(value, (bytes, bytearray)) and is_sidecar_action(value)
                )
            if power_action is None:
                power_action = request.get("powerAction", request.get("power_action"))

        # A Runner action event is authoritative when present; configuration
        # remains a compatibility fallback for supervisors that do not emit the
        # optional event yet.  Device-owned actions are not copied from the
        # config fallback either.
        if not requests:
            completion = self.tasks_get_config().get("afterCompletion", {})
            if isinstance(completion, Mapping):
                configured_actions = completion.get("actions", [])
                if isinstance(configured_actions, (str, Mapping)):
                    configured_actions = [configured_actions]
                if isinstance(configured_actions, Iterable):
                    actions.extend(
                        value
                        for value in configured_actions
                        if str(getattr(value, "value", value)) == "exit_aalc"
                    )
                power_action = completion.get("powerAction", "none")

        if power_action is None:
            configured_completion = self.tasks_get_config().get("afterCompletion", {})
            if isinstance(configured_completion, Mapping):
                power_action = configured_completion.get("powerAction", "none")

        # These are the sidecar equivalents of the historical in-process
        # success notification.  Disabled channels are omitted, while the
        # coordinator still permits explicit requests from future Runners.
        wxpusher_spt = self._config_value("wxpusher_spt", "")
        if isinstance(wxpusher_spt, str) and wxpusher_spt.strip():
            actions.append("notification")
        if os.name == "nt":
            actions.append("toast")
        if bool(self._config_value("resonate_with_Ahab", False)):
            actions.append("sound")

        action_aliases = {
            "notify": "notification",
            "notification": "notification",
            "toast": "toast",
            "sound": "sound",
            "exit_aalc": "exit_aalc",
            "exitAalc": "exit_aalc",
            "exit-aalc": "exit_aalc",
            "app.exitRequested": "exit_aalc",
        }
        action_order = {name: index for index, name in enumerate(("notification", "toast", "sound", "exit_aalc"))}

        def action_name(action: Any) -> str:
            raw_action = (
                action.get("type", action.get("actionType", action.get("action", "")))
                if isinstance(action, Mapping)
                else action
            )
            raw_action = str(getattr(raw_action, "value", raw_action))
            return action_aliases.get(raw_action, raw_action)

        unique_actions: list[Any] = []
        seen_actions: set[str] = set()
        for action in actions:
            canonical_action = action_name(action)
            if canonical_action in seen_actions:
                continue
            seen_actions.add(canonical_action)
            unique_actions.append(action)
        actions = sorted(unique_actions, key=lambda action: action_order.get(action_name(action), len(action_order)))

        return {
            "runId": run_id,
            "outcome": outcome,
            "forced": forced,
            "actions": actions,
            "powerAction": power_action if power_action is not None else "none",
        }

    def _execute_runner_after_completion(self, run_id: str, *, outcome: str, forced: bool) -> Any:
        coordinator = self._after_completion_coordinator
        if coordinator is None:
            return {
                "runId": run_id,
                "outcome": outcome,
                "forced": forced,
                "skipped": True,
                "reason": "completion coordinator unavailable",
            }
        try:
            request = self._runner_completion_request(run_id, outcome=outcome, forced=forced)
            execute = getattr(coordinator, "execute", None)
            if not callable(execute):
                for name in ("coordinate", "process", "handle_finished", "execute_after_completion"):
                    execute = getattr(coordinator, name, None)
                    if callable(execute):
                        break
            if callable(execute):
                result = self._invoke_compatible(execute, (request,), {})
            elif callable(coordinator):
                result = self._invoke_compatible(coordinator, (request,), {})
            else:
                raise TypeError("completion coordinator does not expose an execute method")
            return result if isinstance(result, Mapping) else {"runId": run_id, "result": result}
        except Exception as error:
            # Completion side effects are best effort after all durable Runner
            # work is done.  A broken adapter must not make a completed run
            # appear failed or cause a duplicate finalize on retry.
            log.warning("完成动作执行失败：%s", redact_text(error, ()))
            return {
                "runId": run_id,
                "outcome": outcome,
                "forced": forced,
                "skipped": True,
                "reason": "completion coordinator failed",
                "error": {"code": "AFTER_COMPLETION_FAILED", "message": redact_text(error, ())},
            }

    def execution_get_state(self) -> dict[str, Any]:
        # A supervisor owns the runner-side revision.  Pull it before taking the
        # application snapshot so a delayed getState cannot resurrect an older
        # state after a finished run.
        if self._runner_enabled and self._runner_supervisor is not None:
            self._sync_runner_snapshot(self._read_runner_snapshot(), emit=True)
        with self._lock:
            return self._execution_payload()

    def execution_start(self, params: Any = None) -> dict[str, Any]:
        values = self._execution_params(params, "execution.start")
        client_request_id = self._execution_client_request_id(values)
        requested_task = values.get("taskId")
        if requested_task is not None and not isinstance(requested_task, str):
            raise ValueError("execution.start.taskId must be a string")
        options = values.get("options", {})
        start_options = self._validate_execution_start_options(options)

        with self._lock:
            cached = self._execution_client_requests.get(client_request_id) if client_request_id else None
            if cached is not None:
                return copy.deepcopy(cached)
            if self._execution_state != "idle" or self._device_lease_active():
                raise self._execution_exception(
                    "EXECUTION_BUSY",
                    "任务或设备执行租约已经在运行",
                    retryable=True,
                )
            if self._tool_is_active_locked():
                raise self._execution_exception(
                    "DEVICE_TOOL_ACTIVE",
                    "工具运行期间不能启动任务",
                    retryable=True,
                )
            if self._runner_enabled and self._runner_supervisor is not None and (
                self._runner_config_error is not None or self._config_repository is None
            ):
                raise self._execution_exception(
                    "RUNNER_CONFIG_FAILED",
                    "Runner 配置仓储不可用",
                    retryable=True,
                )
            tasks = self.tasks_get_config()
            enabled = tasks["enabledTasks"]
            if not any(enabled.get(name, False) for name in EXECUTABLE_TASK_IDS):
                result = {
                    **self._execution_payload(),
                    "accepted": False,
                    "reason": "没有选择可执行任务",
                }
                if client_request_id:
                    self._remember_execution_request_locked(client_request_id, result)
                return result
            task_id = requested_task if requested_task in EXECUTABLE_TASK_IDS else None
            if task_id is None:
                task_id = next((name for name in EXECUTABLE_TASK_IDS if enabled.get(name)), None)
            if task_id is None:  # pragma: no cover - guarded by enabled check
                raise DeviceError("没有选择可执行任务")
            # The legacy path validates the current device here.  The Runner
            # path asks DeviceManager to acquire the authoritative lease below;
            # this also keeps the adapter usable with tiny lease-only fakes.
            if not self._runner_enabled:
                self._require_active_runtime()
            run_id = uuid.uuid4().hex
            self._execution_stop.clear()
            self._execution_finalized_runs.discard(run_id)
            self._execution_after_completion_requests[run_id] = []
            self._execution_after_completion_results.pop(run_id, None)
            self._execution_stats_finished.discard(run_id)
            self._set_execution_fields_locked(
                state="starting" if self._runner_enabled else "running",
                current_task_id=task_id,
                run_id=run_id,
                runner_pid=None,
                device_lease="none",
                outcome=None,
                forced=False,
                requested_by=None,
                error=None,
                device_restore="not_needed",
            )
            stats_payload = self.stats.start_run(
                run_id,
                self._execution_targets(tasks),
                current_task_id=task_id,
            )
            self._execution_stats_started.add(run_id)
            self.emit("execution.stats", stats_payload)
            accepted = self._execution_payload()
            accepted["accepted"] = True
            if client_request_id:
                self._remember_execution_request_locked(client_request_id, accepted)

        if self._runner_enabled and self._runner_supervisor is not None:
            try:
                return self._start_runner_execution(
                    run_id,
                    task_id,
                    tasks,
                    values,
                    client_request_id=client_request_id,
                    start_options=start_options,
                )
            except Exception as error:
                self._fail_runner_execution(run_id, error)
                raise

        worker = threading.Thread(target=self._run_execution, args=(run_id,), name="AALCExecution", daemon=True)
        with self._lock:
            self._execution_thread = worker
        worker.start()
        with self._lock:
            return {**self._execution_payload(), "accepted": True}

    def execution_stop(self, params: Any = None) -> dict[str, Any]:
        values = self._execution_params(params, "execution.stop")
        requested_run = self._execution_run_id_from_params(values)
        requested_by = values.get("requestedBy", "user")
        if requested_by not in {"user", "shutdown", "watchdog"}:
            raise ValueError("execution.stop.requestedBy is invalid")

        if self._runner_enabled and self._runner_supervisor is not None:
            with self._lock:
                current_run = self._execution_run_id
                if requested_run is not None and requested_run != current_run:
                    raise self._execution_exception("STALE_RUN", "runId is stale")
                if self._execution_state == "idle":
                    return {**self._execution_payload(), "accepted": bool(requested_run and requested_run == current_run)}
                run_id = current_run
            if run_id is None:  # pragma: no cover - defensive state guard
                raise self._execution_exception("INVALID_EXECUTION_STATE", "execution has no runId")
            result = self._call_runner_control("request_stop", run_id, requested_by=requested_by)
            self._sync_runner_snapshot(self._runner_result_snapshot(result), emit=True)
            return self._runner_result_payload(result)

        worker = None
        with self._lock:
            if requested_run is not None and requested_run != self._execution_run_id:
                raise self._execution_exception("STALE_RUN", "runId is stale")
            if self._execution_state == "idle":
                return {
                    **self._execution_payload(),
                    "accepted": bool(requested_run and requested_run == self._execution_run_id),
                }
            if self._execution_state == "stopping":
                return {**self._execution_payload(), "accepted": True}
            self._execution_stop.set()
            worker = self._execution_worker
            run_id = self._execution_run_id
            self._set_execution_fields_locked(state="stopping", requested_by=requested_by)
            self.emit("execution.stats", self.stats.set_state("stopping"))
        # The event is the primary cancellation mechanism.  Notify the legacy
        # worker as well so it can stop its retry monitor and wake any
        # worker-owned cancellation hooks without resorting to thread killing.
        self._request_worker_stop(worker)
        with self._lock:
            return {**self._execution_payload(), "accepted": True, "runId": run_id}

    @staticmethod
    def _request_worker_stop(worker: Any) -> None:
        if worker is None:
            return
        terminate = getattr(worker, "terminate", None)
        if callable(terminate):
            try:
                terminate()
            except Exception:
                log.debug("通知任务线程停止失败", exc_info=True)

    def execution_pause(self, params: Any = None) -> dict[str, Any]:
        values = self._execution_params(params, "execution.pause")
        requested_run = self._execution_run_id_from_params(values)
        if values and requested_run is None:
            raise ValueError("execution.pause.runId is required")
        if self._runner_enabled and self._runner_supervisor is not None:
            with self._lock:
                current_run = self._execution_run_id
                if requested_run is not None and requested_run != current_run:
                    raise self._execution_exception("STALE_RUN", "runId is stale")
                if current_run is None or self._execution_state != "running":
                    raise self._execution_exception("INVALID_EXECUTION_STATE", "任务当前不可暂停")
            result = self._call_runner_control("request_pause", current_run)
            self._sync_runner_snapshot(self._runner_result_snapshot(result), emit=True)
            return self._runner_result_payload(result)
        with self._lock:
            if requested_run is not None and requested_run != self._execution_run_id:
                raise self._execution_exception("STALE_RUN", "runId is stale")
            if self._execution_state != "running":
                raise self._execution_exception("INVALID_EXECUTION_STATE", "任务当前不可暂停")
            self._toggle_automation_pause()
            self._set_execution_fields_locked(state="paused")
            self.emit("execution.stats", self.stats.set_state("paused"))
            return {**self._execution_payload(), "accepted": True}

    def execution_resume(self, params: Any = None) -> dict[str, Any]:
        values = self._execution_params(params, "execution.resume")
        requested_run = self._execution_run_id_from_params(values)
        if values and requested_run is None:
            raise ValueError("execution.resume.runId is required")
        if self._runner_enabled and self._runner_supervisor is not None:
            with self._lock:
                current_run = self._execution_run_id
                if requested_run is not None and requested_run != current_run:
                    raise self._execution_exception("STALE_RUN", "runId is stale")
                if current_run is None or self._execution_state != "paused":
                    raise self._execution_exception("INVALID_EXECUTION_STATE", "任务当前不可恢复")
            result = self._call_runner_control("request_resume", current_run)
            self._sync_runner_snapshot(self._runner_result_snapshot(result), emit=True)
            return self._runner_result_payload(result)
        with self._lock:
            if requested_run is not None and requested_run != self._execution_run_id:
                raise self._execution_exception("STALE_RUN", "runId is stale")
            if self._execution_state != "paused":
                raise self._execution_exception("INVALID_EXECUTION_STATE", "任务当前不可恢复")
            self._toggle_automation_pause()
            self._set_execution_fields_locked(state="running")
            self.emit("execution.stats", self.stats.set_state("running"))
            return {**self._execution_payload(), "accepted": True}

    def team_list(self) -> list[dict[str, Any]]:
        config = self.config
        teams = getattr(getattr(config, "config", None), "teams", {}) or {}
        queue = {
            int(number)
            for number in (config.get_value("teams_active_queue", []) or [])
            if isinstance(number, int) and not isinstance(number, bool)
        }
        details: list[dict[str, Any]] = []
        for raw_number, setting in sorted(teams.items(), key=self._team_sort_key):
            try:
                number = int(raw_number)
            except (TypeError, ValueError):
                continue
            if number > 0:
                details.append(self._team_detail(number, setting, number in queue))
        return details

    def team_stats_get(self, params: Any) -> dict[str, Any]:
        values = self._require_mapping(params, "team.stats.get")
        team_id = self._require_string(values.get("id"), "team.stats.get.id")
        team_number = self._team_number_from_id(team_id, "team.stats.get.id")
        with self._lock:
            setting = (getattr(self.config.config, "teams", {}) or {}).get(str(team_number))
            if setting is None:
                raise ValueError("team.stats.get.id 对应的队伍不存在")
            return self._team_stats_detail(team_id, team_number, setting)

    def team_stats_clear(self, params: Any) -> dict[str, Any]:
        values = self._require_mapping(params, "team.stats.clear")
        team_id = self._require_string(values.get("id"), "team.stats.clear.id")
        team_number = self._team_number_from_id(team_id, "team.stats.clear.id")
        with self._lock:
            setting = (getattr(self.config.config, "teams", {}) or {}).get(str(team_number))
            if setting is None:
                raise ValueError("team.stats.clear.id 对应的队伍不存在")
            setting.total_mirror_time_hard = [0.0, 0.0, 0.0]
            setting.mirror_hard_count = 0
            setting.total_mirror_time_normal = [0.0, 0.0, 0.0]
            setting.mirror_normal_count = 0
            self._persist_config()
            return self._team_stats_detail(team_id, team_number, setting)

    def team_preset_list(self) -> list[dict[str, Any]]:
        """Return the read-only built-in team preset catalog.

        Presets are templates rather than persisted teams.  Their complete
        team payload is normalized through the same projection used by
        ``team.list`` so the GPUI picker can save it without knowing Python's
        legacy ``chosen_sinners``/``sinner_order`` representation.
        """

        from module.team_presets import builtin_team_presets

        return builtin_team_presets(self._team_detail)

    def team_save(self, params: Any) -> dict[str, Any]:
        values = self._require_mapping(params, "team.save")
        team_id = values.get("id", "")
        if not isinstance(team_id, str):
            raise ValueError("team.save.id must be a string")
        numbers = self._team_numbers()
        if team_id:
            match = re.fullmatch(r"team-(\d+)", team_id)
            if match is None:
                raise ValueError("team.save.id 无效")
            team_number = int(match.group(1))
            if "teamNumber" in values:
                raise ValueError("team.save.teamNumber 只允许用于新建队伍")
        else:
            if "teamNumber" in values:
                team_number = self._require_int(values["teamNumber"], "team.save.teamNumber", minimum=1)
                if team_number in numbers:
                    raise ValueError(f"编队编号 {team_number} 已被占用")
            else:
                team_number = max(numbers, default=0) + 1
        if team_number < 1:
            team_number = 1
        try:
            from module.config import TeamSetting
        except ImportError as error:
            raise RuntimeError(f"无法加载队伍配置模型：{error}") from error
        current = getattr(self.config.config, "teams", {}).get(str(team_number))
        setting = current.model_copy(deep=True) if current is not None else TeamSetting(team_number=team_number)
        current_name = getattr(setting, "remark_name", None) or f"编队 {team_number}"
        name = self._require_string(values.get("name", current_name), "team.save.name").strip()
        if not name:
            raise ValueError("队伍名称不能为空")
        setting.remark_name = name
        if "sinners" in values:
            self._write_team_sinners(setting, values["sinners"])
        purpose = getattr(setting, "purpose", "mirror") or "mirror"
        if purpose not in TEAM_PURPOSES:
            purpose = "mirror"
        if "purpose" in values:
            purpose = self._require_string(values["purpose"], "team.save.purpose")
            if purpose not in TEAM_PURPOSES:
                raise ValueError("team.save.purpose 无效")
        setting.purpose = purpose

        # Luxcavation teams deliberately do not expose mirror settings in the
        # GPUI contract.  A null mirrorConfig means "preserve the stored
        # mirror settings" when an existing team is changed back later; it is
        # never a request to erase the compatibility fields in TeamSetting.
        if purpose != "luxcavation":
            if "mirrorConfig" in values and values["mirrorConfig"] is not None:
                mirror = self._require_mapping(values["mirrorConfig"], "team.save.mirrorConfig")
                self._write_mirror_setting(setting, mirror)
            elif "accessoryScheme" in values:
                self._write_accessory_scheme(setting, values["accessoryScheme"])
            if getattr(setting, "defense_for_solo", False):
                validate_pseudo_solo_selection(setting.chosen_sinners)

        if hasattr(setting, "team_number"):
            setting.team_number = team_number
        requested_enabled = values.get("enabled", self._team_enabled(team_number))
        if not isinstance(requested_enabled, bool):
            raise ValueError("team.save.enabled requires a boolean")
        self.config.config.teams[str(team_number)] = setting
        if purpose == "luxcavation":
            self._remove_team_from_queue(team_number)
            enabled = False
        else:
            self._set_team_enabled(team_number, requested_enabled)
            enabled = requested_enabled
        self._persist_config()
        return self._team_detail(team_number, setting, enabled)

    def team_delete(self, params: Any) -> bool:
        values = self._require_mapping(params, "team.delete")
        team_id = self._require_string(values.get("id"), "team.delete.id")
        match = re.fullmatch(r"team-(\d+)", team_id)
        if match is None:
            raise ValueError("team.delete.id 无效")
        number = int(match.group(1))
        self.config.config.teams.pop(str(number), None)
        if hasattr(self.config, "remove_team_from_queue"):
            self.config.remove_team_from_queue(number)
        if hasattr(self.theme_store, "delete_team_weight_config"):
            self.theme_store.delete_team_weight_config(number)
        self._persist_config()
        return True

    def sinner_list(self) -> list[dict[str, str]]:
        return [{"id": sinner_id, "name": name} for sinner_id, name in zip(SINNER_IDS, SINNER_NAMES, strict=True)]

    def theme_pack_list(self) -> dict[str, Any]:
        store = self.theme_store
        config = getattr(store, "config", {}) or {}
        packs: list[dict[str, Any]] = []
        seen: set[str] = set()
        for section_name in ("theme_pack_list", "theme_pack_list_hard"):
            section = config.get(section_name, {}) or {}
            if not isinstance(section, Mapping):
                continue
            for pack_id, raw_weight in section.items():
                pack_id = str(pack_id)
                if pack_id in seen:
                    continue
                seen.add(pack_id)
                try:
                    raw_weight = int(raw_weight)
                except (TypeError, ValueError):
                    raw_weight = 0
                packs.append(
                    {
                        "id": pack_id,
                        "name": theme_pack_display_name(
                            pack_id,
                            hard=section_name.endswith("hard"),
                        ),
                        "weight": min(10, abs(raw_weight)),
                        "enabled": raw_weight >= 0,
                        "tier": "HARD" if section_name.endswith("hard") else "NORMAL",
                    }
                )
        return {
            "schemaVersion": SCHEMA_VERSION,
            "hardMirrorActive": bool(self._config_value("hard_mirror", False)),
            "packs": packs,
        }

    def theme_pack_update_all(self, params: Any) -> bool:
        values = self._require_mapping(params, "themePack.updateAll")
        packs = values.get("packs")
        if not isinstance(packs, list):
            raise ValueError("themePack.updateAll.packs must be a list")
        store = self.theme_store
        config = copy.deepcopy(getattr(store, "config", {}) or {})
        sections = [config.get(name, {}) for name in ("theme_pack_list", "theme_pack_list_hard")]
        known = {str(key) for section in sections if isinstance(section, Mapping) for key in section}
        for pack in packs:
            pack = self._require_mapping(pack, "themePack.updateAll.pack")
            pack_id = self._require_string(pack.get("id"), "themePack.id")
            if pack_id not in known:
                raise ValueError(f"未知主题包：{pack_id}")
            try:
                weight = int(pack.get("weight", 0))
            except (TypeError, ValueError) as error:
                raise ValueError(f"主题包权重无效：{pack_id}") from error
            if not 0 <= weight <= 10 or not isinstance(pack.get("enabled"), bool):
                raise ValueError(f"主题包权重或启用状态无效：{pack_id}")
            raw_weight = weight if pack["enabled"] else -max(1, weight)
            for section_name in ("theme_pack_list", "theme_pack_list_hard"):
                section = config.get(section_name)
                if isinstance(section, dict) and pack_id in section:
                    section[pack_id] = raw_weight
        store.config = config
        store.save_config(path=store.theme_pack_list_path, config_data=config)
        return True

    def theme_pack_reset_weights(self) -> dict[str, Any]:
        store = self.theme_store
        example_path = Path("assets/config/theme_pack_list.example.yaml")
        if example_path.is_file():
            default = store.yaml.load(example_path.read_text(encoding="utf-8")) or {}
            store.config = copy.deepcopy(default)
            store.save_config(path=store.theme_pack_list_path, config_data=store.config)
        return self.theme_pack_list()

    def resource_status(self) -> list[dict[str, Any]]:
        state_path = Path("assets/config/image_resource_state.json")
        local_version = "unknown"
        try:
            import json

            value = json.loads(state_path.read_text(encoding="utf-8"))
            local_version = str(value.get("last_applied_manifest_id") or "unknown")
        except (OSError, ValueError, TypeError):
            pass
        return [
            {
                "id": "images",
                "name": "图片资源",
                "localVersion": local_version,
                "remoteVersion": None,
                "lastSyncAt": None,
            }
        ]

    def resource_check_update(self) -> list[dict[str, Any]]:
        service = self._get_resource_service()
        result = service.check_for_updates(self._config_value("image_resource_source", "Auto"))
        self._resource_check = result
        groups = self.resource_status()
        if result.remote_manifest is not None:
            remote_version = result.remote_manifest.manifest_id
            for group in groups:
                group["remoteVersion"] = remote_version
        if result.status.value == "error":
            raise RuntimeError(result.message or "资源更新检查失败")
        return groups

    def resource_sync_start(self, params: Any = None) -> dict[str, Any]:
        with self._lock:
            if self._resource_run is not None:
                raise DeviceError("资源同步已经在运行")
            run_id = uuid.uuid4().hex
            self._resource_run = run_id
        scope = "all"
        if isinstance(params, Mapping) and isinstance(params.get("scope"), str):
            scope = params["scope"]
        worker = threading.Thread(
            target=self._run_resource_sync, args=(run_id, scope), name="AALCResourceSync", daemon=True
        )
        worker.start()
        return {"accepted": True, "runId": run_id}

    def tool_start(self, params: Any) -> dict[str, Any]:
        tool_id = self._tool_id(params)
        self._assert_sidecar_device_available("工具操作")
        self._require_active_runtime()
        with self._lock:
            existing = self._tool_workers.get(tool_id)
            if existing is not None and existing[0].is_alive():
                return {"accepted": False, "runId": existing[2]}
            run_id = uuid.uuid4().hex
            stop_event = threading.Event()
            runtime = {"runId": run_id, "running": True}
            self._tools[tool_id] = runtime
            self.emit("tool.status", {"toolId": tool_id, "running": True, "runId": run_id})
            worker = threading.Thread(
                target=self._run_tool,
                args=(tool_id, run_id, stop_event),
                name=f"AALCTool-{tool_id}",
                daemon=True,
            )
            self._tool_workers[tool_id] = (worker, stop_event, run_id)
            worker.start()
            return {"accepted": True, "runId": run_id}

    def tool_stop(self, params: Any) -> dict[str, Any]:
        tool_id = self._tool_id(params)
        with self._lock:
            runtime = self._tool_workers.get(tool_id)
            if runtime is None:
                self._tools[tool_id] = {"runId": None, "running": False}
                self.emit("tool.status", {"toolId": tool_id, "running": False})
                return {"accepted": False, "runId": None}
            run_id = runtime[2]
            runtime[1].set()
            tool_runtime = self._tool_runtimes.get(tool_id)
            if tool_runtime is not None and hasattr(tool_runtime, "running"):
                tool_runtime.running = False
            self.emit("tool.status", {"toolId": tool_id, "running": False, "runId": run_id})
            return {"accepted": True, "runId": run_id}

    def tool_screenshot(self) -> dict[str, Any]:
        self._assert_sidecar_device_available("截图")
        active_session = self._require_active_runtime()
        from module.automation import auto

        image = auto.take_screenshot(gray=False)
        if image is None:
            raise RuntimeError("截图失败，请确认设备已连接且游戏处于运行状态")
        directory = Path("screenshots")
        directory.mkdir(parents=True, exist_ok=True)
        path = directory / f"screenshot_{time.strftime('%Y%m%d_%H%M%S')}.jpg"
        image.save(path, format="JPEG", quality=92)
        try:
            device_id = active_session.target.info.id
            frame = encode_screenshot_frame(
                image,
                device_id,
                max_width=None,
                quality=80,
            )
            # ``PreviewCapture`` already attaches these identity fields to
            # monitor frames.  One-shot tool screenshots use the same wire
            # event, so attach the complete metadata here as well; GPUI uses
            # it to reject frames from a stale device/run/generation.
            frame.update(
                {
                    "deviceId": device_id,
                    "runId": getattr(self._preview_capture, "run_id", None),
                    "generation": getattr(self._preview_capture, "generation", 0),
                }
            )
            self.emit(
                "screenshot.frame",
                frame,
            )
        except Exception:
            log.debug("截图事件编码失败", exc_info=True)
        return {"path": str(path), "accepted": True}

    def _resolve_adb_device(self, session: Any) -> Any:
        controller = getattr(session, "controller", None)
        if hasattr(controller, "device") and controller.device is not None:
            return controller.device
        if hasattr(controller, "simulator_device") and controller.simulator_device is not None:
            return controller.simulator_device
        endpoint = getattr(session.target, "endpoint", None)
        if endpoint:
            from adbutils import adb

            try:
                if ":" in endpoint:
                    adb.connect(endpoint)
                return adb.device(endpoint)
            except Exception:
                pass
        from adbutils import adb

        devices = adb.device_list()
        if devices:
            return devices[0]
        return None

    def _ensure_resolution_change_allowed(self) -> None:
        if self._execution_state != "idle":
            raise DeviceError("任务运行期间不能修改设备分辨率，请先停止任务")
        self._assert_sidecar_device_available("设备分辨率操作")

    def _reconnect_scrcpy_after_resolution(self, active_session: Any, operation: str) -> bool:
        """Refresh Scrcpy after an Android display-size mutation."""
        if active_session.target.kind != "adb":
            return False

        reconnect = getattr(self.device_manager, "reconnect_active", None)
        if not callable(reconnect):
            raise DeviceError("当前设备管理器不支持 Scrcpy 重连")
        try:
            reconnected = bool(reconnect())
        except Exception as error:
            log.exception("%s后重连 Scrcpy 失败", operation)
            raise DeviceError(f"{operation}，但 Scrcpy 重连失败：{error}") from error
        if not reconnected:
            raise DeviceError(f"{operation}，但当前设备不是可重连的 Scrcpy 会话")
        return True

    def tool_resolution_set(self, _params: Any = None) -> dict[str, Any]:
        """将当前连接的 Android 设备分辨率修改为 1080P (手机基准 1080x1920，横屏时自动呈 1920x1080) 240 DPI。"""
        with self._lock:
            self._ensure_resolution_change_allowed()
            active_session = self._require_active_runtime()
            if active_session.target.kind == "pc":
                raise DeviceError("当前为 Windows 游戏窗口，该工具仅支持 Android 设备")

            device = self._resolve_adb_device(active_session)
            if device is None:
                raise DeviceError("未找到可操作的 ADB 设备")

            try:
                # 查询设备物理原生分辨率以判断原生方向（手机竖屏 vs 平板/模拟器横屏）
                size_output = device.shell(["wm", "size"]) or ""
                match = re.search(r"Physical size:\s*(\d+)x(\d+)", size_output)
                if match:
                    phys_w = int(match.group(1))
                    phys_h = int(match.group(2))
                    is_portrait_native = phys_w < phys_h
                else:
                    is_portrait_native = True

                # 手机真机原生为竖屏(Portrait)，基准分辨率必须是 1080x1920。
                # 当手机横屏（游戏）旋转 90 度后，系统自动计算为 1920x1080 满屏横屏。
                target_size = "1080x1920" if is_portrait_native else "1920x1080"
                device.shell(["wm", "size", target_size])
                device.shell(["wm", "density", "240"])
                try:
                    device.shell(["settings", "put", "system", "accelerometer_rotation", "1"])
                except Exception:
                    pass
            except Exception as error:
                log.exception("修改设备分辨率失败")
                raise RuntimeError(f"修改设备分辨率失败：{error}") from error

            reconnected = self._reconnect_scrcpy_after_resolution(active_session, "设备分辨率已修改")
            message = f"设备分辨率已修改为 {target_size} (240 DPI)"
            if reconnected:
                message += "，Scrcpy 已重连"
            self.emit("app.notice", {"level": "info", "message": message})
            log.info("已通过 ADB 将设备分辨率修改为 %s 240 DPI%s", target_size, "，并已重连 Scrcpy" if reconnected else "")
            return {"accepted": True, "size": target_size, "density": 240, "reconnected": reconnected}

    def tool_resolution_reset(self, _params: Any = None) -> dict[str, Any]:
        """还原当前连接的 Android 设备的默认分辨率与 DPI。"""
        with self._lock:
            self._ensure_resolution_change_allowed()
            active_session = self._require_active_runtime()
            if active_session.target.kind == "pc":
                raise DeviceError("当前为 Windows 游戏窗口，该工具仅支持 Android 设备")

            device = self._resolve_adb_device(active_session)
            if device is None:
                raise DeviceError("未找到可操作的 ADB 设备")

            try:
                device.shell(["wm", "size", "reset"])
                device.shell(["wm", "density", "reset"])
            except Exception as error:
                log.exception("还原设备分辨率失败")
                raise RuntimeError(f"还原设备分辨率失败：{error}") from error

            reconnected = self._reconnect_scrcpy_after_resolution(active_session, "设备分辨率已恢复默认")
            message = "设备分辨率与 DPI 已恢复默认"
            if reconnected:
                message += "，Scrcpy 已重连"
            self.emit("app.notice", {"level": "info", "message": message})
            log.info("已通过 ADB 将设备分辨率与 DPI 恢复默认%s", "，并已重连 Scrcpy" if reconnected else "")
            return {"accepted": True, "reconnected": reconnected}

    def hotkey_get(self) -> dict[str, Any]:
        return {
            "schemaVersion": SCHEMA_VERSION,
            "startStop": self._config_value("shutdown_hotkey", "<ctrl>+q"),
            "pauseResume": self._config_value("pause_hotkey", "<alt>+p"),
            "enabled": self._hotkey_enabled,
        }

    def hotkey_set(self, params: Any) -> bool:
        values = self._require_mapping(params, "hotkey.set")
        updates: dict[str, Any] = {}
        if "startStop" in values:
            updates["shutdown_hotkey"] = self._require_string(values["startStop"], "hotkey.startStop")
        if "pauseResume" in values:
            pause = values["pauseResume"]
            if pause is not None and not isinstance(pause, str):
                raise ValueError("hotkey.pauseResume must be a string or null")
            updates["pause_hotkey"] = pause or ""
            updates["resume_hotkey"] = pause or ""
        if "enabled" in values:
            self._hotkey_enabled = self._require_bool(values["enabled"], "hotkey.enabled")
        self._apply_config_updates(updates)
        self._refresh_hotkeys()
        return True

    def system_settings_get(self) -> dict[str, Any]:
        keys = (
            "simulator",
            "simulator_type",
            "simulator_port",
            "start_emulator_timeout",
            "memory_protection",
            "minimize_to_tray",
            "autostart",
            "experimental_keep_screen_awake",
            "experimental_hdr_warning",
            "enable_template_blur",
            "update_prerelease_enable",
            "update_source",
            "mirrorchyan_cdk",
            "wxpusher_spt",
        )
        result = {key: self._config_value(key, None) for key in keys}
        # Old injected/config snapshots may not have the newly introduced
        # credential field; keep the Rust model's string contract stable.
        result["wxpusher_spt"] = self._config_value("wxpusher_spt", "") or ""
        result["schemaVersion"] = SCHEMA_VERSION
        return result

    def system_settings_set(self, params: Any) -> bool:
        values = self._require_mapping(params, "systemSettings.set")
        allowed = {
            "simulator",
            "simulator_type",
            "simulator_port",
            "start_emulator_timeout",
            "memory_protection",
            "minimize_to_tray",
            "autostart",
            "experimental_keep_screen_awake",
            "experimental_hdr_warning",
            "enable_template_blur",
            "update_prerelease_enable",
            "update_source",
            "mirrorchyan_cdk",
            "wxpusher_spt",
        }
        self._apply_config_updates({key: value for key, value in values.items() if key in allowed})
        return True

    def notification_test(self, params: Any) -> dict[str, Any]:
        values = self._require_mapping(params, "notification.test")
        spt = values.get("spt", self._config_value("wxpusher_spt", ""))
        if not isinstance(spt, str):
            raise ValueError("SPT 必须是字符串")
        normalized_spt = spt.strip()
        if not normalized_spt:
            raise ValueError("SPT 未配置")
        if len(normalized_spt) <= 4 or not normalized_spt.startswith("SPT_"):
            raise ValueError("SPT 格式无效")
        try:
            self.notifications.send_test(normalized_spt)
        except Exception as error:
            raise ValueError(redact_text(error, (spt,))) from error
        return {"accepted": True}

    def app_check_update(self) -> dict[str, Any]:
        """Run the existing framework-free update checker synchronously.

        The caller already executes RPC work away from the WebSocket loop.  A
        synchronous checker here keeps the legacy source fallback and version
        comparison in one place; GPUI receives the result without importing
        update implementation details.
        """

        try:
            from module.update.checker import UpdateThread

            checker = UpdateThread(timeout=10, flag=False)
            checker.run()
            if not checker.new_version:
                return {
                    "schemaVersion": SCHEMA_VERSION,
                    "status": "failed",
                    "error": checker.error_msg or "无法从更新源获取版本信息",
                }
            latest = checker.new_version
            update_available = bool(latest != self.version and not checker.is_current_version_latest)
            return {
                "schemaVersion": SCHEMA_VERSION,
                "status": "available" if update_available else "up_to_date",
                "updateAvailable": update_available,
                "latest": latest,
            }
        except Exception as error:
            log.warning("更新检查失败：%s", error)
            return {"schemaVersion": SCHEMA_VERSION, "status": "failed", "error": str(error)}

    def close(self) -> None:
        with self._lock:
            if self._closed:
                return
            self._closed = True
        # RunnerSupervisor owns the process stop/kill deadline.  Close it
        # before tearing down preview/device resources so its final restoring
        # transition still has a live DeviceManager to call.
        if self._runner_enabled and self._runner_supervisor is not None:
            close_runner = getattr(self._runner_supervisor, "close", None)
            if callable(close_runner):
                try:
                    self._invoke_compatible(close_runner, (), {"timeout": 5.0})
                except Exception:
                    log.exception("有界关闭 RunnerSupervisor 失败")
                    try:
                        self.execution_stop({"requestedBy": "shutdown"})
                    except Exception:
                        log.debug("RunnerSupervisor 失败后的停止请求失败", exc_info=True)
            else:
                try:
                    self.execution_stop({"requestedBy": "shutdown"})
                except Exception:
                    log.debug("RunnerSupervisor 缺少 close 后的停止请求失败", exc_info=True)
            monitor = self._runner_monitor
            if monitor is not None and monitor is not threading.current_thread():
                monitor.join(timeout=5.0)
        else:
            try:
                self.execution_stop({"requestedBy": "shutdown"})
            except Exception:
                # The application may already be idle or a legacy task may
                # have left no run id; close still must release all resources.
                log.debug("关闭旧执行线程失败", exc_info=True)
        self._preview_capture.close()
        execution_thread = self._execution_thread
        if execution_thread is not None and execution_thread is not threading.current_thread():
            # Give cooperative cancellation a short grace period before
            # closing device resources.  The thread is daemonized so process
            # shutdown remains bounded even if legacy code is blocked in an
            # external API; no interpreter-level hard kill is attempted.
            execution_thread.join(timeout=2.0)
        close_notifications = getattr(self.notifications, "close", None)
        if callable(close_notifications):
            try:
                close_notifications()
            except Exception:
                log.exception("关闭通知服务失败")
        for tool_id in tuple(self._tool_workers):
            try:
                self.tool_stop({"id": tool_id})
            except Exception:
                log.exception("关闭工具失败：%s", tool_id)
        self._stop_hotkeys()
        for event, handler in self._mediator_bindings:
            try:
                event.disconnect(handler)
            except Exception:
                log.debug("解绑核心事件失败", exc_info=True)
        self._mediator_bindings.clear()
        if hasattr(self.device_manager, "remove_status_listener"):
            self.device_manager.remove_status_listener(self._on_device_event)
        if hasattr(self.device_manager, "remove_notice_listener"):
            self.device_manager.remove_notice_listener(self._on_device_event)
        if hasattr(self.device_manager, "close"):
            self.device_manager.close()
        flush = getattr(self.config, "flush", None)
        if callable(flush):
            flush()

    def is_busy(self) -> bool:
        with self._lock:
            if self._execution_state != "idle" or self._execution_device_lease != "none":
                return True
        return self._device_lease_state() != "none"

    def _require_active_runtime(self) -> Any:
        """Validate and rebind the device selected by the sidecar UI."""
        binder = getattr(self.device_manager, "bind_active_runtime", None)
        if callable(binder):
            return binder()

        session = getattr(self.device_manager, "active_session", None)
        if session is None:
            raise DeviceError("未连接设备，请先选择并连接设备")
        target = getattr(session, "target", None)
        if getattr(target, "kind", None) in ("mumu", "adb") and getattr(session, "controller", None) is None:
            raise DeviceError("当前设备会话缺少模拟器控制器")
        return session

    def _run_execution(self, run_id: str) -> None:
        worker = None
        completed_normally = False
        failure_message: str | None = None
        manual_stop = False
        try:
            from core.execution_control import bind_cancel_event
            from tasks.base.script_task_scheme import my_script_task

            bind_cancel_event(self._execution_stop)
            worker = my_script_task()
            with self._lock:
                if self._execution_run_id != run_id:
                    return
                self._execution_worker = worker
                stop_requested = self._execution_stop.is_set()
            if stop_requested:
                self._request_worker_stop(worker)
            worker.start()
            worker.join()
            worker_error = getattr(worker, "exception", None)
            if worker_error is not None:
                from module.my_error.my_error import userStopError

                manual_stop = isinstance(worker_error, userStopError)
                if not manual_stop:
                    raise worker_error
            manual_stop = manual_stop or self._execution_stop.is_set()
            if not manual_stop:
                completed_normally = True
                self.emit("app.notice", {"level": "info", "message": "所有任务已完成"})
        except Exception as error:
            manual_stop = manual_stop or self._execution_stop.is_set()
            if not manual_stop:
                failure_message = redact_text(
                    error,
                    (self._config_value("wxpusher_spt", ""),),
                )
                # The sanitized message is sufficient for the sidecar event;
                # avoid logging an unsanitized traceback that could contain a
                # credential supplied by a legacy task/plugin.
                log.error("任务执行失败：%s", failure_message)
                self.emit("app.notice", {"level": "error", "message": f"任务执行失败：{failure_message}"})
        finally:
            from core.execution_control import bind_cancel_event

            bind_cancel_event(None)
            if completed_normally:
                self._queue_run_notification(run_id, "final")
            elif failure_message is not None:
                self._queue_run_notification(run_id, "failure", failure_message)
            with self._lock:
                is_current = self._execution_run_id == run_id
                already_finalized = run_id in self._execution_finalized_runs
                if is_current and not already_finalized:
                    self._execution_finalized_runs.add(run_id)
                    outcome = "completed" if completed_normally else "stopped" if manual_stop else "failed"
                    final_error = (
                        {
                            "code": "EXECUTION_FAILED",
                            "message": failure_message,
                            "phase": "legacy",
                            "recovery": "retry",
                        }
                        if failure_message is not None
                        else None
                    )
                    self._set_execution_fields_locked(
                        state="idle",
                        current_task_id=None,
                        runner_pid=None,
                        device_lease="none",
                        run_id=run_id,
                        outcome=outcome,
                        forced=False,
                        requested_by=self._execution_requested_by,
                        error=final_error,
                        device_restore="not_needed",
                    )
                    self._execution_thread = None
                    self._execution_worker = None
                else:
                    is_current = False
            if is_current and run_id not in self._execution_stats_finished:
                self._execution_stats_finished.add(run_id)
                try:
                    self.emit("execution.stats", self.stats.finish_run(run_id))
                except Exception:
                    log.debug("完成旧执行统计失败", exc_info=True)

    def _run_resource_sync(self, run_id: str, scope: str) -> None:
        try:
            service = self._get_resource_service()
            result = service.check_for_updates(self._config_value("image_resource_source", "Auto"))
            self._resource_check = result
            if result.remote_manifest is None:
                raise RuntimeError(result.message or "远端资源清单不可用")
            plan = service.build_sync_plan(result.remote_manifest)
            self._resource_plan = plan
            if not plan.has_changes:
                service.accept_remote_state(result)
                self.emit("resource.sync.progress", {"scope": scope, "progress": 100, "runId": run_id})
                return

            def progress(current: int, total: int) -> None:
                value = 100 if total <= 0 else max(0, min(100, round(current * 100 / total)))
                self.emit("resource.sync.progress", {"scope": scope, "progress": value, "runId": run_id})

            service.apply_sync_plan(check_result=result, sync_plan=plan, progress_callback=progress)
            self.emit("resource.sync.progress", {"scope": scope, "progress": 100, "runId": run_id})
        except Exception as error:
            # Keep the GPUI progress indicator recoverable when a sync fails;
            # app.notice is intentionally routed to the log surface, while
            # this typed progress event lets the resources page clear its
            # in-flight state without parsing user-facing text.
            self.emit(
                "resource.sync.progress",
                {"scope": scope, "progress": 0, "runId": run_id, "error": str(error)},
            )
            self.emit("app.notice", {"level": "error", "message": f"资源同步失败：{error}", "runId": run_id})
            log.exception("资源同步失败")
        finally:
            with self._lock:
                if self._resource_run == run_id:
                    self._resource_run = None

    def _run_tool(self, tool_id: str, run_id: str, stop_event: threading.Event) -> None:
        runtime = None
        try:
            if tool_id == "infinite_battle":
                from tasks.base.script_task_scheme import init_game
                from tasks.battle.battle import Battle

                init_game()
                runtime = Battle(is_tool=True)
                with self._lock:
                    self._tool_runtimes[tool_id] = runtime
                runtime.fight(infinite_battle=True)
            elif tool_id == "enkephalin":
                from tasks.base.back_init_menu import back_init_menu
                from tasks.base.make_enkephalin_module import make_enkephalin_module

                while not stop_event.is_set():
                    back_init_menu(allow_restart=False)
                    make_enkephalin_module(cancel=False, skip=False)
                    stop_event.wait(1.0)
            elif tool_id == "screenshot":
                self.tool_screenshot()
            else:
                raise ValueError(f"未知工具：{tool_id}")
        except Exception as error:
            if not stop_event.is_set():
                self.emit("app.notice", {"level": "error", "message": f"工具 {tool_id} 失败：{error}"})
                log.exception("工具执行失败：%s", tool_id)
        finally:
            if runtime is not None and hasattr(runtime, "running"):
                runtime.running = False
            with self._lock:
                self._tools[tool_id] = {"runId": run_id, "running": False}
                self._tool_workers.pop(tool_id, None)
                self._tool_runtimes.pop(tool_id, None)
                self.emit("tool.status", {"toolId": tool_id, "running": False, "runId": run_id})

    def _get_resource_service(self) -> Any:
        if self._resource_service is None:
            from module.resource_sync.service import ResourceSyncService

            self._resource_service = ResourceSyncService()
        return self._resource_service

    def _toggle_automation_pause(self) -> None:
        try:
            from module.automation import auto

            auto.set_pause()
        except Exception as error:
            log.debug("切换自动化暂停状态失败：%s", error)

    def _config_snapshot(self) -> dict[str, Any]:
        config = getattr(self.config, "config", None)
        if config is None:
            return {}
        if hasattr(config, "model_dump"):
            return copy.deepcopy(config.model_dump())
        if isinstance(config, Mapping):
            return copy.deepcopy(dict(config))
        return {}

    def _config_value(self, key: str, default: Any = None) -> Any:
        try:
            value = self.config.get_value(key, default)
        except Exception:
            value = default
        return copy.deepcopy(value) if isinstance(value, (dict, list, set)) else value

    @staticmethod
    def _json_bool(value: Any, default: bool) -> bool:
        if isinstance(value, bool):
            return value
        if isinstance(value, int):
            return value != 0
        return default

    def _apply_config_updates(self, updates: Mapping[str, Any]) -> None:
        if not updates:
            return
        with self._lock:
            for key, value in updates.items():
                if key == "set_win_position":
                    value = self._canonical_window_position(value)
                self._validate_config_value(key, value)
                if hasattr(self.config, "unsaved_set_value"):
                    self.config.unsaved_set_value(key, copy.deepcopy(value), stacklevel=3)
                elif hasattr(self.config, "set_value"):
                    self.config.set_value(key, copy.deepcopy(value))
                else:
                    setattr(self.config.config, key, copy.deepcopy(value))
            self._persist_config()

    def _validate_config_value(self, key: str, value: Any) -> None:
        current = getattr(getattr(self.config, "config", None), key, None)
        if current is None:
            return
        if isinstance(current, bool):
            if not isinstance(value, bool):
                raise ValueError(f"{key} requires a boolean")
            return
        if isinstance(current, int) and not isinstance(current, bool):
            if not isinstance(value, int) or isinstance(value, bool):
                raise ValueError(f"{key} requires an integer")
            if key in {"set_mirror_count"} and not 1 <= value <= 99:
                raise ValueError(f"{key} must be between 1 and 99")
            if key in {"use_continuous_combat_select"} and not 1 <= value <= 10:
                raise ValueError(f"{key} must be between 1 and 10")
            if key in {"set_get_prize"} and not 0 <= value <= 2:
                raise ValueError(f"{key} must be between 0 and 2")
            return
        if isinstance(current, float):
            if not isinstance(value, (int, float)) or isinstance(value, bool):
                raise ValueError(f"{key} requires a number")
            return
        if isinstance(current, str) and not isinstance(value, str):
            raise ValueError(f"{key} requires a string")
        if key == "wxpusher_spt" and value.strip() and not value.strip().startswith("SPT_"):
            raise ValueError("SPT 格式无效")
        if key == "set_win_position" and value not in WINDOW_POSITIONS:
            raise ValueError(f"{key} contains an unknown position")

    @staticmethod
    def _canonical_window_position(value: Any) -> str:
        value = LEGACY_WINDOW_POSITIONS.get(value, value)
        if isinstance(value, str) and value in WINDOW_POSITIONS:
            return value
        raise ValueError("set_win_position contains an unknown position")

    @staticmethod
    def _normalise_window_position(value: Any) -> str:
        try:
            return BackendApplication._canonical_window_position(value)
        except ValueError:
            return "free"

    def _persist_config(self) -> None:
        self.config.save()

    def _execution_payload(self, *, run_id: str | None = None) -> dict[str, Any]:
        effective_state = self._execution_state
        current_task_id = self._execution_task_id
        runner_pid = self._execution_runner_pid
        if effective_state == "idle":
            current_task_id = None
            runner_pid = None
        return {
            "schemaVersion": SCHEMA_VERSION,
            "state": effective_state,
            "stateRevision": self._execution_state_revision,
            "currentTaskId": current_task_id,
            "runId": run_id if run_id is not None else self._execution_run_id,
            "runnerPid": runner_pid,
            "deviceLease": self._execution_device_lease,
            "outcome": self._execution_outcome,
            "forced": bool(self._execution_forced),
            "requestedBy": self._execution_requested_by,
            "error": None if self._execution_error is None else dict(self._execution_error),
            "deviceRestore": self._execution_device_restore,
        }

    @staticmethod
    def _execution_targets(tasks: Mapping[str, Any]) -> dict[str, Any]:
        enabled = tasks.get("enabledTasks", {})
        daily = tasks.get("daily_task", {})
        mirror = tasks.get("mirror", {})
        infinite = bool(mirror.get("infinite_dungeons", False))
        return {
            "exp": int(daily.get("set_EXP_count", 0)) if enabled.get("daily_task") else 0,
            "thread": int(daily.get("set_thread_count", 0)) if enabled.get("daily_task") else 0,
            "mirror": int(mirror.get("set_mirror_count", 0)) if enabled.get("mirror") and not infinite else 0,
            "mirrorInfinite": infinite and bool(enabled.get("mirror")),
        }

    def _on_runner_event(self, message: Mapping[str, Any], _payload: bytes = b"") -> None:
        """Bridge validated Runner events into the sidecar schema.

        ``RunnerSupervisor`` already rejects stale wire sequence/run events;
        this second boundary protects the application when an injected test or
        compatibility supervisor invokes its callback directly.
        """

        if not isinstance(message, Mapping):
            return
        run_id = message.get("runId")
        state_revision = message.get("stateRevision")
        try:
            state_revision = int(state_revision) if state_revision is not None else None
        except (TypeError, ValueError):
            state_revision = None
        with self._lock:
            if run_id is not None and run_id != self._execution_run_id:
                return
            if not self._runner_event_is_new_locked(run_id, message):
                return
            if state_revision is not None and state_revision <= self._execution_supervisor_revision:
                return
            if state_revision is not None:
                self._execution_supervisor_revision = state_revision
            message_type = str(message.get("type", ""))
        # Persist obligations before applying/forwarding the optional typed
        # event.  RunnerEventAdapter is also bound to this ledger and performs
        # a compatibility record, so duplicate delivery is harmless.
        self._journal_runner_event(message)
        self._journal_runner_process(
            run_id,
            message.get("pid", message.get("runnerPid")),
            message.get("createTime", message.get("runnerCreateTime")),
        )
        forwarded = dict(message)
        if message_type == "config.delta":
            result = self._apply_runner_config_delta(message)
            if result is not None:
                forwarded["applyResult"] = result
        elif message_type == "task.completed":
            sequence = message.get("seq")
            if isinstance(sequence, int) and not isinstance(sequence, bool) and run_id is not None:
                completion_key = (run_id, sequence)
                with self._lock:
                    duplicate_completion = completion_key in self._runner_task_completed_events
                    if not duplicate_completion:
                        self._runner_task_completed_events.add(completion_key)
                if duplicate_completion:
                    return
            result = message.get("result")
            result = result if isinstance(result, Mapping) else {}
            kind = message.get("kind", message.get("taskId", ""))
            count = result.get("count", message.get("count", 1))
            details = result.get("details", message.get("details"))
            self._on_task_completed(kind, count, details)

        with self._lock:
            if message_type == "task.started":
                task_id = message.get("taskId")
                if isinstance(task_id, str) and task_id in EXECUTABLE_TASK_IDS:
                    self._set_execution_fields_locked(current_task_id=task_id)
            elif message_type == "status":
                status = message.get("status")
                if status in {"starting", "running", "paused", "stopping", "restoring", "idle"}:
                    self._set_execution_fields_locked(state=status)
            elif message_type == "finished":
                self._set_execution_fields_locked(
                    state="restoring",
                    device_lease=self._device_lease_state() if self._device_lease_state() != "none" else "restoring",
                    outcome=message.get("outcome"),
                    forced=bool(message.get("forced", False)),
                    requested_by=message.get("requestedBy", self._execution_requested_by),
                    error=message.get("error"),
                    device_restore="pending",
                )
            elif message_type == "afterCompletion.requested" and isinstance(run_id, str) and run_id:
                self._execution_after_completion_requests.setdefault(run_id, []).append(dict(message))
            elif message_type == "error":
                error = message.get("error")
                if not isinstance(error, Mapping):
                    error = {"code": "RUNNER_INIT_FAILED", "message": str(message.get("message", "Runner error"))}
                self._set_execution_fields_locked(error=dict(error))

        # Status is already emitted by the authoritative snapshot update above;
        # forwarding it again would create duplicate revisions for the GPUI.
        if message_type != "status":
            self._emit_runner_typed_event(forwarded, _payload)

    def _emit_runner_typed_event(self, message: Mapping[str, Any], payload: bytes = b"") -> None:
        adapter = self._runner_event_adapter
        if adapter is None:
            return
        run_id = message.get("runId")
        try:
            if run_id is not None:
                adapter.run_id = str(run_id)
            adapter.forward(message, payload)
        except Exception:
            log.exception("Runner 事件适配失败：%s", message.get("type"))

    def _on_device_event(self, event: str, payload: dict[str, Any]) -> None:
        self.emit(event, payload)
        if event != "device.status":
            return

        status = payload.get("status")
        device_id = payload.get("deviceId")
        try:
            with self._lock:
                if status == "connected" and isinstance(device_id, str) and device_id:
                    self._preview_device_id = device_id
                    if self._preview_enabled and not self._device_lease_active():
                        self._preview_capture.start(device_id)
                elif status in {"connecting", "disconnected"}:
                    self._preview_device_id = None
                    stop_and_wait = getattr(self._preview_capture, "stop_and_wait", None)
                    if callable(stop_and_wait):
                        self._invoke_compatible(stop_and_wait, (), {})
                    else:
                        self._preview_capture.stop()
        except Exception as error:
            log.exception("实时预览生命周期处理失败")
            if isinstance(device_id, str) and device_id:
                preview_run_id = getattr(self._preview_capture, "run_id", None)
                preview_generation = getattr(self._preview_capture, "generation", 0)
                self.emit(
                    "preview.status",
                    {
                        "deviceId": device_id,
                        "runId": preview_run_id,
                        "generation": preview_generation,
                        "status": "error",
                        "error": str(error),
                    },
                )

    def _bind_core_events(self) -> None:
        """Bridge framework-free core events into the RPC event contract."""
        try:
            from core.events import mediator

            bindings = (
                (mediator.mirror_signal, self._on_mirror_signal),
                (mediator.mirror_floor_signal, self._on_mirror_floor),
                (mediator.task_started, self._on_task_started),
                (mediator.task_completed, self._on_task_completed),
                (mediator.warning, self._on_core_warning),
                (mediator.hdr_warning, self._on_hdr_warning),
                (mediator.update_progress, self._on_update_progress),
                (mediator.download_complete, self._on_download_complete),
            )
            for event, handler in bindings:
                event.connect(handler)
                self._mediator_bindings.append((event, handler))
        except Exception:
            # A core event bridge is an enhancement; it must not stop the
            # sidecar from serving read-only RPC when an optional module is
            # unavailable in a packaged environment.
            log.debug("核心事件桥接不可用", exc_info=True)

    def _on_mirror_floor(self, floor: Any, total: Any) -> None:
        try:
            floor_value = max(0, int(floor))
            total_value = max(0, int(total))
        except (TypeError, ValueError):
            return
        self.emit(
            "execution.mirrorFloor",
            {
                "floor": floor_value,
                "floorTotal": total_value,
                "runId": self._execution_run_id,
            },
        )

    def _on_mirror_signal(self, current: Any, total: Any) -> None:
        self.emit(
            "execution.mirrorProgress",
            {
                "current": max(0, int(current)),
                "total": max(0, int(total)),
                "isHard": bool(self._config_value("hard_mirror", False)),
                "isInfinite": bool(self._config_value("infinite_dungeons", False)),
                "runId": self._execution_run_id,
            },
        )

    def _on_task_started(self, task_id: Any) -> None:
        if not isinstance(task_id, str) or task_id not in EXECUTABLE_TASK_IDS:
            return
        with self._lock:
            run_id = self._execution_run_id
            if self._execution_state != "running" or run_id is None:
                return
            if self._execution_task_id == task_id:
                return
            stats_payload = self.stats.set_current_task(task_id, run_id=run_id)
            if stats_payload is None:
                return
            self._set_execution_fields_locked(current_task_id=task_id)
            self.emit("execution.stats", stats_payload)

    def _on_task_completed(self, kind: Any, count: Any = 1, details: Any = None) -> None:
        run_id = self._execution_run_id
        if run_id is None:
            return
        task_kind = str(kind)
        mirror_details = details if task_kind == "mirror" and isinstance(details, Mapping) else None
        try:
            amount = int(count)
            payload = self.stats.record_completion(
                task_kind,
                amount,
                run_id=run_id,
                details=mirror_details,
            )
        except (TypeError, ValueError):
            return
        if payload is not None:
            self.emit("execution.stats", payload)
            if self._execution_state == "running" and not self._execution_stop.is_set():
                notification_details = payload.get("lastMirror") if task_kind == "mirror" else None
                if isinstance(notification_details, Mapping):
                    # 仅结算超时视为失败通知；其他 failed（如中途放弃/未100%但已回到主界面）不按失败推送，避免正常领取被误报为错误
                    if notification_details.get("failed"):
                        if notification_details.get("failureReason") == "settlement_timeout":
                            # 使用失败通知，附带统计明细
                            from module.notification.wxpusher import _mirror_detail_lines as _detail
                            details_text = "\n".join(_detail(notification_details))
                            reason = notification_details.get("failureReason") or "结算超时"
                            content = f"AALC 镜牢结算失败\n原因：{reason}\n" + details_text
                            # 直接入队失败通知，复用失败通道但携带统计
                            spt = self._config_value("wxpusher_spt", "")
                            if isinstance(spt, str) and spt.strip():
                                try:
                                    from module.notification.wxpusher import NotificationService
                                    svc = NotificationService()
                                    svc._enqueue(spt, content, "AALC 镜牢结算失败")
                                except Exception:
                                    pass
                            # 仍更新 lastMirror，不再发成功通知
                        else:
                            # 非超时的 failed（如检测到非100%但已回到主界面），不推送失败/成功，避免正常领取被误报
                            pass
                    else:
                        self._queue_notification("enqueue_completion", task_kind, amount, notification_details)
                else:
                    self._queue_notification("enqueue_completion", task_kind, amount)

    def _queue_run_notification(self, run_id: str, outcome: str, error: str | None = None) -> None:
        summary = self.stats.summary()
        current_run = summary.get("currentRun", {})
        if current_run.get("runId") != run_id:
            return
        if outcome == "final":
            final_run = dict(current_run)
            last_mirror = summary.get("lastMirror")
            if isinstance(last_mirror, Mapping) and last_mirror.get("runId") == run_id:
                final_run["lastMirror"] = dict(last_mirror)
            self._queue_notification("enqueue_final", final_run)
        elif outcome == "failure" and error is not None:
            self._queue_notification("enqueue_failure", error)

    def _queue_notification(self, operation: str, *args: Any) -> None:
        spt = self._config_value("wxpusher_spt", "")
        if not isinstance(spt, str) or not spt.strip():
            return
        try:
            getattr(self.notifications, operation)(spt, *args)
        except Exception as error:
            log.warning("任务通知入队失败：%s", redact_text(error, (spt,)))

    def _after_completion_notification(self, record: Mapping[str, Any]) -> dict[str, Any]:
        """Queue the final WxPusher message through the existing service."""

        spt = self._config_value("wxpusher_spt", "")
        if not isinstance(spt, str) or not spt.strip():
            return {"configured": False, "queued": False}
        run_id = record.get("runId")
        summary = self.stats.summary()
        current_run = summary.get("currentRun", {})
        if not isinstance(current_run, Mapping) or current_run.get("runId") != run_id:
            return {"configured": True, "queued": False, "reason": "run statistics unavailable"}
        enqueue = getattr(self.notifications, "enqueue_final", None)
        if not callable(enqueue):
            raise RuntimeError("notification service lacks enqueue_final")
        queued = enqueue(spt, dict(current_run))
        return {"configured": True, "queued": bool(queued)}

    @staticmethod
    def _after_completion_toast(record: Mapping[str, Any]) -> dict[str, Any]:
        """Send the historical completion toast via the existing facade."""

        if os.name != "nt":
            return {"configured": False, "sent": False, "reason": "unsupported platform"}
        from module.notification.toast import TemplateToast, send_toast

        payload = record.get("payload", {})
        payload = payload if isinstance(payload, Mapping) else {}
        title = payload.get("title", "AALC 运行结束")
        message = payload.get("message", ["所有任务已完成"])
        sent = send_toast(title, message, template=TemplateToast.NormalTemplate)
        return {"configured": True, "sent": bool(sent)}

    def _after_completion_sound(self, record: Mapping[str, Any]) -> dict[str, Any]:
        """Play the opt-in completion sound without importing task modules."""

        if not bool(self._config_value("resonate_with_Ahab", False)):
            return {"configured": False, "played": False}
        if os.name != "nt":
            return {"configured": True, "played": False, "reason": "unsupported platform"}
        from random import randint

        from playsound3 import playsound

        payload = record.get("payload", {})
        payload = payload if isinstance(payload, Mapping) else {}
        path = payload.get("path", f"assets/audio/This_is_all_your_fault_{randint(1, 4)}.mp3")
        playsound(path, block=False)
        return {"configured": True, "played": True}

    def _on_core_warning(self, message: Any) -> None:
        self.emit("app.notice", {"level": "warn", "message": str(message)})

    def _on_hdr_warning(self, acknowledgement: Any) -> None:
        self.emit(
            "app.notice",
            {"level": "warn", "message": "检测到游戏所在显示器已开启 HDR，可能影响图像识别"},
        )
        if hasattr(acknowledgement, "set"):
            acknowledgement.set()

    def _on_update_progress(self, progress: Any) -> None:
        self.emit("app.notice", {"level": "info", "message": f"更新下载进度：{int(progress)}%"})

    def _on_download_complete(self, file_name: Any) -> None:
        self.emit("app.notice", {"level": "info", "message": f"更新下载完成：{file_name}"})

    def _team_numbers(self) -> list[int]:
        teams = getattr(self.config.config, "teams", {}) or {}
        numbers: list[int] = []
        for key in teams:
            try:
                number = int(key)
            except (TypeError, ValueError):
                continue
            if number > 0:
                numbers.append(number)
        return sorted(numbers)

    @staticmethod
    def _team_sort_key(item: tuple[Any, Any]) -> tuple[int, str]:
        try:
            return int(item[0]), str(item[0])
        except (TypeError, ValueError):
            return 0, str(item[0])

    def _team_detail(self, number: int, setting: Any, enabled: bool) -> dict[str, Any]:
        values = setting.model_dump() if hasattr(setting, "model_dump") else dict(setting)
        defaults = self._team_default()
        purpose = values.get("purpose", "mirror")
        if purpose not in TEAM_PURPOSES:
            purpose = "mirror"
        chosen = values.get("chosen_sinners", []) or []
        order = values.get("sinner_order", []) or []
        selected_indexes = [
            index for index, selected in enumerate(chosen[: len(SINNER_IDS)]) if selected and index < len(SINNER_IDS)
        ]
        ordered_indexes: dict[int, int] = {}
        for index in selected_indexes:
            position = order[index] if index < len(order) else 0
            if (
                isinstance(position, int)
                and not isinstance(position, bool)
                and 1 <= position <= len(selected_indexes)
                and position not in ordered_indexes
            ):
                ordered_indexes[position] = index
        ordered_selected = [ordered_indexes[position] for position in sorted(ordered_indexes)]
        ordered_selected.extend(index for index in selected_indexes if index not in ordered_selected)
        sinners = [SINNER_IDS[index] for index in ordered_selected]

        team_system = self._team_int_value(
            values.get("team_system", getattr(defaults, "team_system", 0)),
            default=0,
            minimum=0,
            maximum=len(SYSTEM_NAMES) - 1,
        )
        mirror: dict[str, Any] = {
            "team_system": team_system,
            "shop_strategy": self._team_int_value(
                values.get("shop_strategy", getattr(defaults, "shop_strategy", 0)),
                default=0,
                minimum=0,
                maximum=2,
            ),
            "discard_systems": {
                name: self._team_bool_value(
                    values.get(f"system_{name}", False),
                    default=False,
                )
                for name in SYSTEM_NAMES
            },
        }

        for field in TEAM_MIRROR_BOOL_FIELDS:
            mirror[field] = self._team_bool_value(
                values.get(field, getattr(defaults, field, False)),
                default=False,
            )
        for field, (minimum, maximum) in TEAM_MIRROR_INT_LIMITS.items():
            mirror[field] = self._team_int_value(
                values.get(field, getattr(defaults, field, minimum)),
                default=minimum,
                minimum=minimum,
                maximum=maximum,
            )

        opening_bonus = values.get("opening_bonus", getattr(defaults, "opening_bonus", [])) or []
        if not isinstance(opening_bonus, list):
            opening_bonus = []
        mirror["opening_bonus"] = [
            self._team_int_value(value, default=0, minimum=0, maximum=3) for value in opening_bonus[:10]
        ]
        mirror["opening_bonus"] += [0] * (10 - len(mirror["opening_bonus"]))

        ignore_shop = values.get("ignore_shop", getattr(defaults, "ignore_shop", [])) or []
        if not isinstance(ignore_shop, list):
            ignore_shop = []
        mirror["ignore_shop"] = [self._team_bool_value(value, default=False) for value in ignore_shop[:5]]
        mirror["ignore_shop"] += [False] * (5 - len(mirror["ignore_shop"]))

        team_code = values.get("team_code", getattr(defaults, "team_code", ""))
        mirror["team_code"] = team_code if isinstance(team_code, str) else ""
        route_profile = values.get(
            "mirror_route_profile",
            getattr(defaults, "mirror_route_profile", ""),
        )
        mirror["mirror_route_profile"] = route_profile if isinstance(route_profile, str) else ""
        selected_gifts = (
            values.get(
                "observe_ego_gift_selected",
                getattr(defaults, "observe_ego_gift_selected", []),
            )
            or []
        )
        mirror["observe_ego_gift_selected"] = (
            normalize_observe_ego_gifts(selected_gifts) if isinstance(selected_gifts, list) else []
        )

        actions = values.get("second_system_action", getattr(defaults, "second_system_action", [])) or []
        if not isinstance(actions, list):
            actions = []
        default_actions = getattr(defaults, "second_system_action", [0, 0, 0, 0]) or [0, 0, 0, 0]
        for index, field in enumerate(TEAM_MIRROR_ACTION_FIELDS):
            raw_value = actions[index] if index < len(actions) else default_actions[index]
            mirror[field] = self._team_bool_value(raw_value, default=False)

        return {
            "schemaVersion": SCHEMA_VERSION,
            "id": f"team-{number}",
            "name": values.get("remark_name") or f"编队 {number}",
            "sinners": sinners,
            "purpose": purpose,
            "accessoryScheme": SYSTEM_NAMES[team_system],
            "enabled": False if purpose == "luxcavation" else bool(enabled),
            "mirrorConfig": None if purpose == "luxcavation" else mirror,
        }

    @staticmethod
    def _team_default() -> Any:
        from module.config import TeamSetting

        return TeamSetting()

    @staticmethod
    def _team_number_from_id(team_id: str, name: str) -> int:
        match = re.fullmatch(r"team-(\d+)", team_id)
        if match is None or not 1 <= int(match.group(1)) <= 0xFFFFFFFF:
            raise ValueError(f"{name} 无效")
        return int(match.group(1))

    @staticmethod
    def _team_stats_bucket(setting: Any, mode: str) -> dict[str, Any]:
        time_values = getattr(setting, f"total_mirror_time_{mode}", [])
        if not isinstance(time_values, list):
            time_values = []
        averages: list[float] = []
        for value in time_values[:3]:
            if not isinstance(value, (int, float)) or isinstance(value, bool):
                averages.append(0.0)
                continue
            try:
                numeric = float(value)
            except (TypeError, ValueError, OverflowError):
                averages.append(0.0)
                continue
            if math.isfinite(numeric):
                averages.append(max(0.0, numeric))
            else:
                averages.append(0.0)
        averages.extend([0.0] * (3 - len(averages)))
        count = getattr(setting, f"mirror_{mode}_count", 0)
        if not isinstance(count, int) or isinstance(count, bool):
            count = 0
        return {
            "count": min(0xFFFFFFFF, max(0, count)),
            "averageSeconds": averages[0],
            "last5AverageSeconds": averages[1],
            "last10AverageSeconds": averages[2],
        }

    @classmethod
    def _team_stats_detail(cls, team_id: str, team_number: int, setting: Any) -> dict[str, Any]:
        hard = cls._team_stats_bucket(setting, "hard")
        normal = cls._team_stats_bucket(setting, "normal")
        return {
            "schemaVersion": 1,
            "teamId": team_id,
            "teamNumber": team_number,
            "totalCount": min(0xFFFFFFFF, hard["count"] + normal["count"]),
            "hard": hard,
            "normal": normal,
        }

    def _write_team_sinners(self, setting: Any, sinners: Any) -> None:
        if not isinstance(sinners, list) or not all(isinstance(item, str) for item in sinners):
            raise ValueError("team.save.sinners must be a string list")
        if len(sinners) > len(SINNER_IDS):
            raise ValueError(f"team.save.sinners 最多允许 {len(SINNER_IDS)} 名人格")
        if len(set(sinners)) != len(sinners):
            raise ValueError("team.save.sinners 不允许重复人格")
        selected = [0] * len(SINNER_IDS)
        order = [0] * len(SINNER_IDS)
        for position, sinner_id in enumerate(sinners, start=1):
            if sinner_id not in SINNER_IDS:
                raise ValueError(f"未知人格：{sinner_id}")
            index = SINNER_IDS.index(sinner_id)
            selected[index] = 1
            order[index] = position
        setting.chosen_sinners = selected
        setting.sinner_order = order
        setting.sinners_be_select = sum(selected)

    def _team_enabled(self, number: int) -> bool:
        return number in {
            int(value)
            for value in (self.config.get_value("teams_active_queue", []) or [])
            if isinstance(value, int) and not isinstance(value, bool)
        }

    def _set_team_enabled(self, number: int, enabled: bool) -> None:
        if hasattr(self.config, "set_team_enabled"):
            self.config.set_team_enabled(number, enabled)
            return
        queue = [
            int(value)
            for value in (self.config.get_value("teams_active_queue", []) or [])
            if isinstance(value, int) and not isinstance(value, bool) and int(value) > 0
        ]
        if enabled and number not in queue:
            queue.append(number)
        if not enabled:
            queue = [value for value in queue if value != number]
        self._write_team_queue(queue)

    def _remove_team_from_queue(self, number: int) -> None:
        if hasattr(self.config, "remove_team_from_queue"):
            self.config.remove_team_from_queue(number)
            return
        queue = [
            int(value)
            for value in (self.config.get_value("teams_active_queue", []) or [])
            if isinstance(value, int) and not isinstance(value, bool) and int(value) != number
        ]
        self._write_team_queue(queue)

    def _write_team_queue(self, queue: list[int]) -> None:
        normalized: list[int] = []
        for value in queue:
            if value > 0 and value not in normalized:
                normalized.append(value)
        if hasattr(self.config, "unsaved_set_value"):
            self.config.unsaved_set_value("teams_active_queue", normalized, stacklevel=3)
        else:
            setattr(self.config.config, "teams_active_queue", normalized)

    @staticmethod
    def _write_accessory_scheme(setting: Any, value: Any) -> None:
        scheme = BackendApplication._require_string(value, "team.save.accessoryScheme")
        if scheme not in SYSTEM_NAMES:
            raise ValueError("team.save.accessoryScheme 无效")
        setting.team_system = SYSTEM_NAMES.index(scheme)

    @staticmethod
    def _write_mirror_setting(setting: Any, mirror: Mapping[str, Any]) -> None:
        discard = mirror.get("discard_systems")
        if discard is not None and not isinstance(discard, Mapping):
            raise ValueError("team.save.mirrorConfig.discard_systems must be an object")
        for key, value in mirror.items():
            if key == "discard_systems" or key in TEAM_MIRROR_ACTION_FIELDS:
                continue
            if key in TEAM_MIRROR_BOOL_FIELDS:
                setattr(setting, key, BackendApplication._require_bool(value, f"team.save.mirrorConfig.{key}"))
                continue
            if key in TEAM_MIRROR_INT_LIMITS:
                minimum, maximum = TEAM_MIRROR_INT_LIMITS[key]
                setattr(
                    setting,
                    key,
                    BackendApplication._require_int(
                        value,
                        f"team.save.mirrorConfig.{key}",
                        minimum=minimum,
                        maximum=maximum,
                    ),
                )
                continue
            if key == "team_code":
                setattr(setting, key, BackendApplication._require_string(value, f"team.save.mirrorConfig.{key}"))
                continue
            if key == "mirror_route_profile":
                setattr(
                    setting,
                    key,
                    BackendApplication._require_string(value, f"team.save.mirrorConfig.{key}").strip(),
                )
                continue
            if key == "opening_bonus":
                values = BackendApplication._require_int_list(value, f"team.save.mirrorConfig.{key}")
                if len(values) > 10 or any(item < 0 or item > 3 for item in values):
                    raise ValueError(
                        "team.save.mirrorConfig.opening_bonus must contain 0-3 values and have at most 10 items"
                    )
                values += [0] * (10 - len(values))
                setattr(setting, key, values)
                continue
            if key == "observe_ego_gift_selected":
                values = BackendApplication._require_string_list(value, f"team.save.mirrorConfig.{key}")
                setattr(setting, key, normalize_observe_ego_gifts(values))
                continue
            if key == "ignore_shop":
                values = BackendApplication._require_bool_list(value, f"team.save.mirrorConfig.{key}")
                if len(values) > 5:
                    raise ValueError("team.save.mirrorConfig.ignore_shop must have at most 5 items")
                values += [False] * (5 - len(values))
                setattr(setting, key, [1 if item else 0 for item in values])
                continue
            if key == "second_system_action":
                values = BackendApplication._require_int_list(value, f"team.save.mirrorConfig.{key}")
                if len(values) > 4 or any(item not in {0, 1} for item in values):
                    raise ValueError(
                        "team.save.mirrorConfig.second_system_action must contain 0/1 values and have at most 4 items"
                    )
                values += [0] * (4 - len(values))
                setattr(setting, key, values)
        if isinstance(discard, Mapping):
            for system in SYSTEM_NAMES:
                if system in discard:
                    setattr(
                        setting,
                        f"system_{system}",
                        BackendApplication._require_bool(
                            discard[system], f"team.save.mirrorConfig.discard_systems.{system}"
                        ),
                    )
        if any(key in mirror for key in TEAM_MIRROR_ACTION_FIELDS):
            actions = list(getattr(setting, "second_system_action", [0, 0, 0, 0]) or [])[:4]
            actions += [0] * (4 - len(actions))
            for index, key in enumerate(TEAM_MIRROR_ACTION_FIELDS):
                if key in mirror:
                    actions[index] = (
                        1 if BackendApplication._require_bool(mirror[key], f"team.save.mirrorConfig.{key}") else 0
                    )
            setting.second_system_action = actions

    @staticmethod
    def _require_mapping(value: Any, name: str) -> dict[str, Any]:
        if not isinstance(value, Mapping):
            raise ValueError(f"{name} requires an object")
        return dict(value)

    @staticmethod
    def _require_string(value: Any, name: str) -> str:
        if not isinstance(value, str):
            raise ValueError(f"{name} requires a string")
        return value

    @staticmethod
    def _require_int(value: Any, name: str, *, minimum: int = 0, maximum: int | None = None) -> int:
        if not isinstance(value, int) or isinstance(value, bool):
            raise ValueError(f"{name} requires an integer")
        if value < minimum or (maximum is not None and value > maximum):
            if maximum is None:
                raise ValueError(f"{name} must be at least {minimum}")
            raise ValueError(f"{name} must be between {minimum} and {maximum}")
        return value

    @staticmethod
    def _require_int_list(value: Any, name: str) -> list[int]:
        if not isinstance(value, list) or not all(
            isinstance(item, int) and not isinstance(item, bool) for item in value
        ):
            raise ValueError(f"{name} must be an integer list")
        return list(value)

    @staticmethod
    def _require_bool_list(value: Any, name: str) -> list[bool]:
        if not isinstance(value, list) or not all(isinstance(item, bool) for item in value):
            raise ValueError(f"{name} must be a boolean list")
        return list(value)

    @staticmethod
    def _require_string_list(value: Any, name: str) -> list[str]:
        if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
            raise ValueError(f"{name} must be a string list")
        return list(value)

    @staticmethod
    def _require_bool(value: Any, name: str) -> bool:
        if not isinstance(value, bool):
            raise ValueError(f"{name} requires a boolean")
        return value

    @staticmethod
    def _team_bool_value(value: Any, *, default: bool) -> bool:
        if isinstance(value, bool):
            return value
        if isinstance(value, int):
            return value != 0
        return default

    @staticmethod
    def _team_int_value(value: Any, *, default: int, minimum: int, maximum: int) -> int:
        if not isinstance(value, int) or isinstance(value, bool):
            return default
        return min(max(value, minimum), maximum)

    @staticmethod
    def _tool_id(params: Any) -> str:
        values = BackendApplication._require_mapping(params, "tool call")
        tool_id = values.get("id")
        if tool_id not in {"infinite_battle", "enkephalin", "screenshot"}:
            raise ValueError("tool call requires a valid id")
        return str(tool_id)

    def _refresh_hotkeys(self) -> None:
        self._stop_hotkeys()
        if not self._hotkey_enabled or os.name != "nt":
            return
        try:
            from module.hotkey_listener import ExactGlobalHotKeys

            start_stop = self._config_value("shutdown_hotkey", "")
            pause = self._config_value("pause_hotkey", "")
            resume = self._config_value("resume_hotkey", "")
            hotkeys = {}
            if start_stop:
                hotkeys[start_stop] = self._hotkey_start_stop
            if pause:
                hotkeys[pause] = self._hotkey_pause_resume
            if resume and resume != pause:
                hotkeys[resume] = self._hotkey_pause_resume
            if hotkeys:
                listener = ExactGlobalHotKeys(hotkeys)
                listener.start()
                self._hotkey_listener = listener
        except (ImportError, OSError, ValueError) as error:
            log.warning("全局快捷键未启动：%s", error)

    def _stop_hotkeys(self) -> None:
        listener = self._hotkey_listener
        self._hotkey_listener = None
        if listener is not None:
            try:
                listener.stop()
            except Exception:
                log.debug("停止全局快捷键失败", exc_info=True)

    def _hotkey_start_stop(self) -> None:
        if self.is_busy():
            self.execution_stop()
        else:
            self.execution_start()

    def _hotkey_pause_resume(self) -> None:
        try:
            if self._execution_state == "running":
                self.execution_pause()
            elif self._execution_state == "paused":
                self.execution_resume()
        except Exception:
            log.exception("快捷键切换任务状态失败")


__all__ = ["BackendApplication", "BackendEventBus", "SCHEMA_VERSION", "recover_pending_cleanups"]
