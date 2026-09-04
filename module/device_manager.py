"""Runtime device selection for the GPUI sidecar.

The legacy application decides between a Windows game window and an Android
emulator through ``cfg.simulator``.  The GPUI client instead selects an opaque
stable device id (for example ``pc:limbus`` or ``mumu:0``).  This module is the
small compatibility layer between those two models.  It owns exactly one
active runtime session and keeps HWND/ADB objects out of the IPC contract.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import threading
import time
import uuid
from collections.abc import Mapping
from dataclasses import dataclass, field
from functools import wraps
from typing import Any, Callable, Literal

from module.config import cfg
from module.logger import log

DeviceKind = Literal["pc", "mumu", "adb"]


class DeviceError(RuntimeError):
    """A user-correctable device discovery or connection error."""


@dataclass(frozen=True)
class DeviceInfo:
    """Public, serializable device information exposed to the UI."""

    id: str
    name: str
    detail: str | None = None

    def to_dict(self) -> dict[str, Any]:
        value: dict[str, Any] = {"id": self.id, "name": self.name}
        if self.detail is not None:
            value["detail"] = self.detail
        return value


@dataclass(frozen=True)
class DeviceTarget:
    """A discovered target plus private connection parameters."""

    info: DeviceInfo
    kind: DeviceKind
    hwnd: int | None = None
    endpoint: str | None = None
    instance_number: int | None = None

    def to_snapshot(self) -> dict[str, Any]:
        """Return a serializable snapshot suitable for an execution lease.

        The snapshot deliberately contains only stable target metadata and
        private connection parameters.  Live controller objects are never
        exposed or retained by the lease protocol.
        """

        value: dict[str, Any] = {
            "id": self.info.id,
            "name": self.info.name,
            "kind": self.kind,
        }
        if self.info.detail is not None:
            value["detail"] = self.info.detail
        if self.hwnd is not None:
            value["hwnd"] = self.hwnd
        if self.endpoint is not None:
            value["endpoint"] = self.endpoint
        if self.instance_number is not None:
            value["instanceNumber"] = self.instance_number
        return value

    # ``to_dict`` is the naming used by the existing device RPC models.
    to_dict = to_snapshot

    @classmethod
    def from_snapshot(cls, payload: Mapping[str, Any]) -> "DeviceTarget":
        """Reconstruct a target from :meth:`to_snapshot` output."""

        if not isinstance(payload, Mapping):
            raise DeviceError("设备目标快照必须是对象")
        target_id = payload.get("id")
        name = payload.get("name")
        kind = payload.get("kind")
        if not isinstance(target_id, str) or not target_id:
            raise DeviceError("设备目标快照缺少有效 id")
        if not isinstance(name, str) or not name:
            raise DeviceError("设备目标快照缺少有效 name")
        if kind not in ("pc", "mumu", "adb"):
            raise DeviceError(f"设备目标快照 kind 无效：{kind}")

        detail = payload.get("detail")
        if detail is not None and not isinstance(detail, str):
            raise DeviceError("设备目标快照 detail 无效")
        hwnd = payload.get("hwnd")
        if hwnd is not None and (isinstance(hwnd, bool) or not isinstance(hwnd, int)):
            raise DeviceError("设备目标快照 hwnd 无效")
        endpoint = payload.get("endpoint")
        if endpoint is not None and not isinstance(endpoint, str):
            raise DeviceError("设备目标快照 endpoint 无效")
        instance_number = payload.get("instanceNumber", payload.get("instance_number"))
        if instance_number is not None and (
            isinstance(instance_number, bool) or not isinstance(instance_number, int) or instance_number < 0
        ):
            raise DeviceError("设备目标快照 instanceNumber 无效")
        return cls(
            DeviceInfo(target_id, name, detail),
            kind,
            hwnd=hwnd,
            endpoint=endpoint,
            instance_number=instance_number,
        )

    from_dict = from_snapshot


@dataclass(frozen=True)
class DeviceLease:
    """The serializable capability handed to one execution run."""

    run_id: str
    generation: int
    target: DeviceTarget
    state: Literal["acquiring", "runner", "restoring"] = "runner"

    def to_dict(self) -> dict[str, Any]:
        return {
            "runId": self.run_id,
            "generation": self.generation,
            "state": self.state,
            "target": self.target.to_snapshot(),
        }

    def __getitem__(self, key: str) -> Any:
        """Allow transitional callers to use ``lease["generation"]``."""

        return self.to_dict()[key]


@dataclass
class DeviceSession:
    target: DeviceTarget
    controller: Any = None


@dataclass
class RunnerDeviceRuntime:
    """One process-private device session owned by a single Runner run.

    The runtime deliberately owns a fresh :class:`DeviceManager` instead of
    borrowing the sidecar's process-wide manager.  Its public session remains
    the same ``DeviceSession`` shape consumed by legacy automation, while the
    install/close methods make ownership and bounded cleanup explicit.
    """

    manager: "DeviceManager"
    run_id: str
    session: DeviceSession
    reservation: dict[str, Any] = field(default_factory=dict)
    _closed: bool = field(default=False, init=False, repr=False)
    _close_result: dict[str, Any] | None = field(default=None, init=False, repr=False)
    _cleanup_thread: threading.Thread | None = field(default=None, init=False, repr=False)
    _cleanup_errors: list[BaseException] = field(default_factory=list, init=False, repr=False)
    _lock: threading.RLock = field(default_factory=threading.RLock, init=False, repr=False)

    @property
    def target(self) -> DeviceTarget:
        return self.session.target

    @property
    def controller(self) -> Any:
        return self.session.controller

    @property
    def closed(self) -> bool:
        with self._lock:
            return self._closed

    def install(self) -> "RunnerDeviceRuntime":
        """Install this session as the only Runner runtime in the process."""

        with self._lock:
            if self._closed:
                raise DeviceError("Runner 运行时已关闭，不能重新安装")
            with _runner_runtime_lock:
                existing = _runner_runtime
                # Read the flag while holding the registry lock instead of
                # taking ``existing._lock`` here: close() takes the runtime
                # lock before clearing the registry, so acquiring the locks
                # in the opposite order would deadlock concurrent install and
                # cleanup calls.
                if existing is not None and existing is not self and not existing._closed:
                    raise DeviceError(f"已有 Runner 运行时正在使用设备（runId={existing.run_id}）")
                # The explicit session is passed through, so this never asks
                # discovery or a legacy class-level controller to choose a
                # different target.
                self.manager._activate_runtime(self.session, allow_lease=True)
                with self.manager._lock:
                    self.manager._targets[self.target.info.id] = self.target
                    self.manager._active = self.session
                _set_runner_runtime(self)
        return self

    def close(
        self,
        deadline: float | None = None,
        *,
        device_disposition: str = "restore",
    ) -> dict[str, Any]:
        """Boundedly clean this runtime and report the task disposition.

        ``deadline`` is an absolute ``time.monotonic()`` deadline.  A timed
        out cleanup does not mark the runtime closed, allowing the owning host
        or CleanupLedger to retry without creating another controller.
        """

        if device_disposition not in {"restore", "game_closed", "emulator_closed"}:
            raise DeviceError(f"无效的 deviceDisposition：{device_disposition}")
        with self._lock:
            if self._closed:
                result = dict(self._close_result or {})
                result["alreadyClosed"] = True
                return result
            if deadline is not None and self.manager._deadline_remaining(deadline) <= 0:
                raise DeviceError("Runner 设备清理截止时间已过期")
            cleanup_thread = self._cleanup_thread
            if cleanup_thread is None:
                self._cleanup_errors.clear()

                def cleanup() -> None:
                    try:
                        self.manager._cleanup_session(self.session)
                    except BaseException as error:  # pragma: no cover - native cleanup boundary
                        self._cleanup_errors.append(error)

                cleanup_thread = threading.Thread(target=cleanup, name="AALCRunnerDeviceCleanup", daemon=True)
                self._cleanup_thread = cleanup_thread
                cleanup_thread.start()
            if cleanup_thread is threading.current_thread():
                raise DeviceError("Runner 设备清理不能从清理线程自身等待")
            remaining = self.manager._deadline_remaining(deadline)
            if remaining <= 0:
                raise DeviceError("Runner 设备清理截止时间已过期")
            cleanup_thread.join(timeout=remaining)
            if cleanup_thread.is_alive():
                raise DeviceError("Runner 设备控制器未能在截止时间前退出")
            if self._cleanup_errors:
                raise DeviceError(f"Runner 设备清理失败：{self._cleanup_errors[0]}") from self._cleanup_errors[0]

            try:
                # The private manager snapshots the sidecar-visible runtime
                # configuration before the target-specific values are
                # installed.  Restore it only after controller cleanup has
                # completed, so a timed-out close cannot claim a clean state.
                self.manager._restore_runtime_config()
            except Exception as error:
                raise DeviceError(f"Runner 运行时配置恢复失败：{error}") from error

            with self.manager._lock:
                if self.manager._active is self.session:
                    self.manager._active = None
            result = {
                "status": "closed",
                "runId": self.run_id,
                "deviceId": self.target.info.id,
                "deviceDisposition": device_disposition,
                "cleanup": True,
                "alreadyClosed": False,
            }
            self._close_result = dict(result)
            self._closed = True
            _clear_runner_runtime(self)
            return result

    cleanup = close


StatusListener = Callable[[str, dict[str, Any]], None]


def _device_write(method):
    """Reject manager-owned device writes while an execution owns the lease."""

    @wraps(method)
    def guarded(self: "DeviceManager", *args, **kwargs):
        self._begin_device_operation(method.__name__)
        try:
            return method(self, *args, **kwargs)
        finally:
            self._end_device_operation()

    return guarded


class DeviceManager:
    """Discover and manage the single device selected by the GPUI client.

    Discovery is deliberately side-effect-light.  In particular, listing a
    MuMu target never constructs ``MumuControl`` and therefore never launches
    an emulator.  Expensive work happens only in ``connect``.
    """

    _RUNTIME_CONFIG_KEYS = (
        "simulator",
        "simulator_type",
        "simulator_port",
        "mumu_instance_number",
    )

    def __init__(self) -> None:
        self._lock = threading.RLock()
        self._operation_condition = threading.Condition(self._lock)
        self._device_operations = 0
        self._status_lock = threading.RLock()
        self._targets: dict[str, DeviceTarget] = {}
        self._active: DeviceSession | None = None
        self._connecting = False
        self._connect_generation = 0
        self._connect_cancel: threading.Event | None = None
        self._connect_thread: threading.Thread | None = None
        self._pending_session: DeviceSession | None = None
        self._status_listeners: list[StatusListener] = []
        self._notice_listeners: list[StatusListener] = []
        self._busy_checker: Callable[[], bool] = lambda: False
        self._runtime_snapshot = {key: cfg.get_value(key) for key in self._RUNTIME_CONFIG_KEYS}
        # Execution lease state is independent of the connection request
        # generation.  A new lease always receives a strictly newer
        # capability generation so stale callbacks cannot regain authority.
        self._execution_lease: DeviceLease | None = None
        self._lease_state: Literal["none", "acquiring", "runner", "restoring"] = "none"
        self._lease_run_id: str | None = None
        self._lease_generation = 0
        self._lease_target: DeviceTarget | None = None
        self._lease_error: str | None = None
        self._lease_recovery: str | None = None
        self._lease_cleanup_thread: threading.Thread | None = None
        self._lease_pending_thread: threading.Thread | None = None
        self._connect_target: DeviceTarget | None = None

    @staticmethod
    def create_runner_runtime(
        target_snapshot: DeviceTarget | Mapping[str, Any],
        run_id: str,
        cleanup_reservation: Mapping[str, Any] | None = None,
        *,
        runner_policy: Any = None,
        remote_cleanup: Callable[[dict[str, object]], object] | None = None,
    ) -> RunnerDeviceRuntime:
        """Create and install an isolated Runner-owned device runtime.

        This is a static entry point intentionally: calling it through the
        sidecar's ``DeviceManager`` instance still creates a fresh private
        manager and cannot borrow or switch the sidecar's active session.
        """

        return create_runner_runtime(
            target_snapshot,
            run_id,
            cleanup_reservation,
            runner_policy=runner_policy,
            remote_cleanup=remote_cleanup,
        )

    @staticmethod
    def install_runner_session(runtime: RunnerDeviceRuntime) -> RunnerDeviceRuntime:
        """Install a previously constructed private Runner runtime."""

        return install_runner_session(runtime)

    @property
    def active_id(self) -> str | None:
        with self._lock:
            return self._active.target.info.id if self._active else None

    @property
    def active_session(self) -> DeviceSession | None:
        with self._lock:
            return self._active

    @property
    def lease_state(self) -> Literal["none", "acquiring", "runner", "restoring"]:
        """Current execution lease phase (``none`` when sidecar owns device)."""

        with self._lock:
            return self._lease_state

    @property
    def execution_generation(self) -> int:
        """Monotonic execution capability generation."""

        with self._lock:
            return self._lease_generation

    @property
    def execution_lease(self) -> dict[str, Any] | None:
        """Return a copy of the serializable current lease snapshot."""

        with self._lock:
            if self._execution_lease is None:
                return None
            value = self._execution_lease.to_dict()
            if self._lease_error is not None:
                # Keep the wire-level lease state in the existing
                # ``restoring`` vocabulary (the backend treats every
                # non-``none`` state as fail-closed), while making a terminal
                # failure explicit.  A failed acquisition must not look like
                # a lease that can still be resumed.
                value["error"] = {
                    "code": "DEVICE_LEASE_FAILED",
                    "message": self._lease_error,
                    "recovery": self._lease_recovery or "restart_backend",
                }
            return value

    @property
    def lease_error(self) -> dict[str, str] | None:
        """Return the terminal lease failure, if recovery is unsafe.

        ``lease_state`` intentionally remains ``restoring`` for compatibility
        with the execution status contract.  Callers can inspect this value to
        distinguish an in-progress restore from a lease that requires a
        backend restart and must never create another controller.
        """

        with self._lock:
            if self._lease_error is None:
                return None
            return {
                "code": "DEVICE_LEASE_FAILED",
                "message": self._lease_error,
                "recovery": self._lease_recovery or "restart_backend",
            }

    @property
    def lease_recovery(self) -> str | None:
        """Return the required recovery action for a failed lease."""

        with self._lock:
            return self._lease_recovery

    @property
    def device_lease(self) -> dict[str, Any] | None:
        """Compatibility alias for :attr:`execution_lease`."""

        return self.execution_lease

    def get_execution_lease(self) -> dict[str, Any] | None:
        """Method form for integrations that avoid property access in RPC code."""

        return self.execution_lease

    def _begin_device_operation(self, operation: str) -> None:
        with self._operation_condition:
            if self._lease_state != "none":
                run_id = self._lease_run_id or "unknown"
                raise DeviceError(f"设备已由执行租约占用，拒绝 {operation}（runId={run_id}）")
            self._device_operations += 1

    def _end_device_operation(self) -> None:
        with self._operation_condition:
            self._device_operations = max(0, self._device_operations - 1)
            self._operation_condition.notify_all()

    def _assert_device_writable(self, operation: str, *, allow_lease: bool = False) -> None:
        with self._lock:
            if self._lease_state != "none" and allow_lease and self._lease_error is not None:
                raise DeviceError(
                    "设备执行租约已失败，拒绝"
                    f" {operation}（需{self._lease_recovery or 'restart_backend'}）"
                )
            if self._lease_state != "none" and not allow_lease:
                run_id = self._lease_run_id or "unknown"
                raise DeviceError(f"设备已由执行租约占用，拒绝 {operation}（runId={run_id}）")

    def _set_lease_locked(self, state: Literal["acquiring", "runner", "restoring"]) -> None:
        self._lease_state = state
        if self._lease_run_id is None or self._lease_target is None:
            self._execution_lease = None
            return
        self._execution_lease = DeviceLease(
            self._lease_run_id,
            self._lease_generation,
            self._lease_target,
            state,
        )

    def _invalidate_connection_locked(self) -> None:
        """Invalidate every outstanding connection callback.

        This runs while ``_operation_condition``/``_lock`` is held.  Clearing
        the request identity before publishing a failed lease means a late
        MuMu worker can only clean its own session; it can never install a
        controller into a failed manager.
        """

        cancel = self._connect_cancel
        if cancel is not None:
            cancel.set()
        self._connect_generation += 1
        self._connecting = False
        self._connect_cancel = None
        self._connect_thread = None
        self._pending_session = None
        self._connect_target = None

    def _lease_failure(self, message: str) -> DeviceError:
        with self._operation_condition:
            self._lease_error = message
            if self._lease_run_id is not None and self._lease_target is not None:
                self._lease_recovery = "restart_backend"
                self._invalidate_connection_locked()
                self._set_lease_locked("restoring")
            else:
                # No target means no capability was handed to a Runner.  Do
                # not strand the manager in an acquiring/leased state when a
                # connection vanished before it became logically selected.
                self._invalidate_connection_locked()
                self._lease_state = "none"
                self._execution_lease = None
                self._lease_run_id = None
                self._lease_target = None
                self._lease_error = None
                self._lease_recovery = None
            self._operation_condition.notify_all()
        return DeviceError(message)

    @staticmethod
    def _deadline_remaining(deadline: float | None, *, default: float = 10.0) -> float:
        if deadline is None:
            return max(0.0, default)
        return max(0.0, float(deadline) - time.monotonic())

    def require_active_session(self) -> DeviceSession:
        """Return the selected runtime session or raise a user-facing error.

        The GPUI sidecar must never silently fall back to the persisted legacy
        simulator configuration. Legacy callers that do not use the device
        manager can keep their compatibility path, but sidecar execution
        entry points should use this guard.
        """
        self._assert_device_writable("访问活动设备")
        with self._lock:
            session = self._active
        if session is None:
            raise DeviceError("未连接设备，请先选择并连接设备")
        if session.target.kind in ("mumu", "adb") and session.controller is None:
            raise DeviceError("当前设备会话缺少模拟器控制器")
        if session.target.kind == "pc" and not self._valid_hwnd(session.target.hwnd):
            raise DeviceError("当前设备窗口已失效，请重新连接设备")
        return session

    def require_active_controller(self) -> Any:
        """Return the controller belonging to the selected simulator session."""
        session = self.require_active_session()
        if session.target.kind not in ("mumu", "adb"):
            raise DeviceError("当前选中设备不是模拟器")
        return session.controller

    @_device_write
    def bind_active_runtime(self) -> DeviceSession:
        """Rebind automation to the currently selected runtime session."""
        session = self.require_active_session()
        self._activate_runtime(session)
        return session

    def set_busy_checker(self, checker: Callable[[], bool]) -> None:
        self._busy_checker = checker

    def add_status_listener(self, listener: StatusListener) -> None:
        with self._lock:
            if listener not in self._status_listeners:
                self._status_listeners.append(listener)

    def remove_status_listener(self, listener: StatusListener) -> None:
        with self._lock:
            if listener in self._status_listeners:
                self._status_listeners.remove(listener)

    def add_notice_listener(self, listener: StatusListener) -> None:
        with self._lock:
            if listener not in self._notice_listeners:
                self._notice_listeners.append(listener)

    def remove_notice_listener(self, listener: StatusListener) -> None:
        with self._lock:
            if listener in self._notice_listeners:
                self._notice_listeners.remove(listener)

    def list_devices(self) -> list[dict[str, Any]]:
        """Return currently discoverable targets without opening them."""
        targets: dict[str, DeviceTarget] = {}

        # 1. 优先扫描 MuMu 模拟器
        for target in self._discover_mumu_targets():
            targets[target.info.id] = target

        # 2. 扫描已连接的 ADB 设备（真机与第三方模拟器，排除已发现的 MuMu 实例）
        for target in self._discover_adb_targets():
            # A configured MuMu endpoint should be represented by the stable
            # MuMu id rather than by a second generic ADB row.
            if target.endpoint and any(
                known.endpoint == target.endpoint and known.kind == "mumu" for known in targets.values()
            ):
                continue
            targets[target.info.id] = target

        # 3. 扫描 Windows 游戏窗口
        pc_target = self._discover_pc_window()
        if pc_target is not None:
            targets[pc_target.info.id] = pc_target

        with self._lock:
            self._targets = targets
            active_id = self._active.target.info.id if self._active else None

        result = [target.info.to_dict() for target in targets.values()]
        if active_id and active_id not in targets:
            # Keep the selected entry visible while a transient rescan is in
            # progress.  The next connect/refresh will resolve it again.
            with self._lock:
                active = self._active
            if active is not None:
                result.insert(0, active.target.info.to_dict())
        return result

    def _wait_for_device_operations(self, deadline: float | None) -> bool:
        """Wait for manager-owned device calls to leave the in-flight set."""

        with self._operation_condition:
            while self._device_operations:
                remaining = self._deadline_remaining(deadline)
                if remaining <= 0:
                    return False
                self._operation_condition.wait(min(0.05, remaining))
            return True

    def _lease_target_locked(self) -> DeviceTarget | None:
        if self._active is not None:
            return self._active.target
        if self._pending_session is not None:
            return self._pending_session.target
        return self._connect_target

    def _cleanup_session_bounded(self, session: DeviceSession, deadline: float | None) -> bool:
        """Run controller cleanup without claiming success after a timeout."""

        errors: list[BaseException] = []

        def cleanup() -> None:
            try:
                self._cleanup_session(session)
            except BaseException as error:  # pragma: no cover - defensive native cleanup boundary
                errors.append(error)

        thread = threading.Thread(target=cleanup, name="AALCDeviceCleanup", daemon=True)
        thread.start()
        remaining = self._deadline_remaining(deadline)
        if remaining <= 0:
            with self._operation_condition:
                self._lease_cleanup_thread = thread
            return False
        thread.join(timeout=remaining)
        if thread.is_alive():
            with self._operation_condition:
                self._lease_cleanup_thread = thread
            return False
        if errors:
            raise DeviceError(f"设备控制器清理失败：{errors[0]}") from errors[0]
        return True

    def suspend_for_execution(self, run_id: str, deadline: float | None = None) -> dict[str, Any]:
        """Transfer the selected device controller to one execution lease.

        The method first publishes an ``acquiring`` lease, which makes all
        manager-owned device writes fail closed.  Existing manager calls are
        allowed to finish, and a pending MuMu connection is cancelled and
        joined before the active controller is cleaned up.  The logical target
        remains selected as a controller-less :class:`DeviceSession` so the
        sidecar can restore the same target later.

        ``deadline`` is an absolute ``time.monotonic()`` deadline.  A missing
        deadline uses a conservative ten-second bound.  If quiescence or
        native cleanup cannot be proven complete, the lease remains in
        ``restoring`` with a terminal ``restart_backend`` recovery marker and
        a :class:`DeviceError` is raised; no new device operation or resume is
        admitted.
        """

        if not isinstance(run_id, str) or not run_id.strip():
            raise DeviceError("执行租约 runId 不能为空")
        if deadline is not None and self._deadline_remaining(deadline) <= 0:
            raise DeviceError("设备租约截止时间已过期")

        with self._operation_condition:
            if self._lease_state != "none":
                raise DeviceError(f"设备已处于执行租约状态：{self._lease_state}")
            target = self._lease_target_locked()
            self._lease_generation += 1
            self._lease_run_id = run_id
            self._lease_target = target
            self._lease_error = None
            self._lease_recovery = None
            self._lease_cleanup_thread = None
            self._lease_pending_thread = None
            self._lease_state = "acquiring"
            self._set_lease_locked("acquiring")
            self._operation_condition.notify_all()

        if not self._wait_for_device_operations(deadline):
            raise self._lease_failure("设备操作未能在截止时间前安静下来")

        # A background MuMu launch can outlive the connect RPC itself.  Cancel
        # it only after synchronous manager calls have quiesced, avoiding an
        # unbounded wait on the status lock held by a still-running connect.
        try:
            pending_thread = self._cancel_pending_connection()
        except Exception as error:
            raise self._lease_failure(f"取消后台设备连接失败：{error}") from error
        if pending_thread is not None and pending_thread is not threading.current_thread():
            with self._operation_condition:
                self._lease_pending_thread = pending_thread
            remaining = self._deadline_remaining(deadline)
            if remaining <= 0:
                raise self._lease_failure("后台设备连接未能在截止时间前取消")
            pending_thread.join(timeout=remaining)
            if pending_thread.is_alive():
                raise self._lease_failure("后台设备连接线程未能退出")
            with self._operation_condition:
                if self._lease_pending_thread is pending_thread:
                    self._lease_pending_thread = None

        with self._operation_condition:
            if target is None:
                target = self._lease_target_locked()
                if target is None:
                    raise self._lease_failure("未连接设备，无法建立执行租约")
                self._lease_target = target
                self._set_lease_locked("acquiring")
            self._targets[target.info.id] = target

        with self._status_lock:
            with self._lock:
                session = self._active
                self._active = None
            if session is not None:
                try:
                    cleaned = self._cleanup_session_bounded(session, deadline)
                except Exception as error:
                    with self._operation_condition:
                        self._active = DeviceSession(target, None)
                    raise self._lease_failure(str(error)) from error
                if not cleaned:
                    with self._operation_condition:
                        self._active = DeviceSession(target, None)
                    raise self._lease_failure("设备控制器未能在截止时间前退出")

        with self._operation_condition:
            # No connect worker is allowed to publish after acquisition starts;
            # this identity check also protects against a stale callback that
            # raced the cancellation/join path.
            self._active = DeviceSession(target, None)
            self._connect_target = None
            self._set_lease_locked("runner")
            self._operation_condition.notify_all()
            lease = self._execution_lease
        if lease is None:  # pragma: no cover - target was validated above
            raise self._lease_failure("执行租约创建失败")
        return lease.to_dict()

    def resume_after_execution(
        self,
        run_id: str,
        generation: int | None = None,
        *,
        deadline: float | None = None,
    ) -> dict[str, Any]:
        """Recreate a fresh controller for a completed execution lease."""

        if not isinstance(run_id, str) or not run_id.strip():
            raise DeviceError("执行租约 runId 不能为空")
        wait_for_operations = False
        with self._operation_condition:
            lease = self._execution_lease
            if self._lease_state == "none" or lease is None:
                raise DeviceError("当前没有可恢复的执行租约")
            if self._lease_state == "acquiring":
                raise DeviceError("执行租约仍在获取中，暂不能恢复")
            if self._lease_state == "restoring":
                if self._lease_error is not None:
                    raise DeviceError(
                        "设备执行租约恢复已失败，禁止创建新控制器"
                        f"（需{self._lease_recovery or 'restart_backend'}）"
                    )
                raise DeviceError("设备租约正在恢复")
            if self._lease_run_id != run_id:
                raise DeviceError("执行租约 runId 不匹配")
            if generation is not None and generation != self._lease_generation:
                raise DeviceError("执行租约 generation 不匹配")
            cleanup_thread = self._lease_cleanup_thread
            if cleanup_thread is not None and cleanup_thread.is_alive():
                raise DeviceError("设备控制器仍在清理，暂不能恢复")
            pending_thread = self._lease_pending_thread
            if pending_thread is not None and pending_thread.is_alive():
                raise DeviceError("后台设备连接仍在退出，暂不能恢复")
            target = self._lease_target
            if target is None:
                raise DeviceError("执行租约缺少设备目标快照")
            if deadline is not None and self._deadline_remaining(deadline) <= 0:
                raise DeviceError("设备恢复截止时间已过期")
            # Claim the single recovery transition before releasing the
            # condition.  A second resume caller must see ``restoring`` with
            # no error and fail closed instead of opening a second controller
            # for the same lease while this caller waits for old operations.
            self._lease_error = None
            self._lease_recovery = None
            self._set_lease_locked("restoring")
            # A suspend timeout can leave an already-admitted manager call in
            # flight while the lease remains fail-closed in ``restoring``.
            # Never construct a replacement controller until that call has
            # released its operation slot.  New writes are rejected by the
            # non-``none`` lease state, so waiting here cannot admit another
            # operation after the counter reaches zero.
            wait_for_operations = bool(self._device_operations)

        def resume_failure(message: str) -> None:
            with self._operation_condition:
                current = self._execution_lease
                if (
                    current is not None
                    and current.run_id == lease.run_id
                    and current.generation == lease.generation
                    and current.target == lease.target
                ):
                    self._lease_error = message
                    self._lease_recovery = "restart_backend"
                    self._set_lease_locked("restoring")
                    self._operation_condition.notify_all()

        if wait_for_operations and not self._wait_for_device_operations(deadline):
            resume_failure("设备操作仍在执行，无法安全恢复")
            raise DeviceError("设备操作仍在执行，无法安全恢复")

        with self._operation_condition:
            # Re-check all capability and cleanup identities after the wait;
            # this closes the small window in which another recovery caller
            # could have consumed the lease while this thread was waiting.
            current_lease = self._execution_lease
            if (
                current_lease is None
                or current_lease.run_id != lease.run_id
                or current_lease.generation != lease.generation
                or current_lease.target != lease.target
                or self._lease_run_id != run_id
            ):
                # The original lease is no longer ours; do not touch the new
                # caller's state.  The identity check itself is fail-closed.
                raise DeviceError("执行租约在恢复等待期间发生变化")
            if generation is not None and generation != self._lease_generation:
                resume_failure("执行租约 generation 在恢复等待期间发生变化")
                raise DeviceError("执行租约 generation 在恢复等待期间发生变化")
            cleanup_thread = self._lease_cleanup_thread
            if cleanup_thread is not None and cleanup_thread.is_alive():
                resume_failure("设备控制器仍在清理，暂不能恢复")
                raise DeviceError("设备控制器仍在清理，暂不能恢复")
            pending_thread = self._lease_pending_thread
            if pending_thread is not None and pending_thread.is_alive():
                resume_failure("后台设备连接仍在退出，暂不能恢复")
                raise DeviceError("后台设备连接仍在退出，暂不能恢复")
            if self._device_operations:
                # This should only be reachable if a non-cooperating caller
                # manipulated the manager internals; fail closed regardless.
                resume_failure("设备操作仍在执行，无法安全恢复")
                raise DeviceError("设备操作仍在执行，无法安全恢复")
            if deadline is not None and self._deadline_remaining(deadline) <= 0:
                resume_failure("设备恢复截止时间已过期")
                raise DeviceError("设备恢复截止时间已过期")
            self._lease_error = None
            self._set_lease_locked("restoring")
            self._operation_condition.notify_all()

        session: DeviceSession | None = None
        try:
            session = self._open_target(target, allow_lease=True)
            self._activate_runtime(session, allow_lease=True)
        except Exception as error:
            if session is not None:
                try:
                    self._cleanup_session(session)
                except Exception:
                    log.exception("恢复失败后的设备清理失败")
            self._restore_runtime_config()
            raise self._lease_failure(f"设备恢复失败：{error}") from error

        with self._operation_condition:
            self._active = session
            self._targets[target.info.id] = target
            self._lease_state = "none"
            self._execution_lease = None
            self._lease_run_id = None
            self._lease_target = None
            self._lease_error = None
            self._lease_recovery = None
            self._lease_cleanup_thread = None
            self._lease_pending_thread = None
            self._operation_condition.notify_all()
        return {
            "status": "connected",
            "resumed": True,
            "deviceId": target.info.id,
            "runId": run_id,
            "generation": lease.generation,
        }

    @_device_write
    def connect(self, device_id: str) -> dict[str, Any]:
        """Connect the selected target and rebind the legacy automation layer."""
        if not device_id:
            raise DeviceError("设备 ID 不能为空")
        if self._busy_checker():
            raise DeviceError("任务运行期间不能切换设备")

        with self._status_lock:
            with self._lock:
                if self._connecting:
                    raise DeviceError("已有设备连接任务正在进行")
                if self._active and self._active.target.info.id == device_id:
                    active_session = self._active
                else:
                    active_session = None
                if active_session is None:
                    self._connecting = True
                    self._connect_generation += 1
                    generation = self._connect_generation
                    cancel = threading.Event()
                    self._connect_cancel = cancel
                    run_id = uuid.uuid4().hex
                else:
                    generation = 0
                    cancel = None
                    run_id = None
            if active_session is None:
                # Publish the transition after installing the request state,
                # but without holding the manager lock while preview listeners
                # stop their worker.
                self._emit_status(device_id, "connecting", run_id=run_id)

        if active_session is not None:
            # Rebind the compatibility layer even when the selected id is
            # unchanged. This repairs old callers that cleared a class-level
            # pointer without creating another controller.
            try:
                self._activate_runtime(active_session)
            except Exception as error:
                log.exception("重新绑定设备运行时失败：%s", device_id)
                raise DeviceError(f"重新绑定设备失败：{error}") from error
            return {
                "deviceId": device_id,
                "status": "connected",
                "alreadyConnected": True,
            }

        session: DeviceSession | None = None
        background_started = False
        try:
            target = self._resolve_target(device_id)
            with self._lock:
                self._connect_target = target
            with self._status_lock:
                self._disconnect_active(restore_config=False)

            if target.kind == "mumu":
                # Constructing a MuMu controller with auto_start disabled is
                # cheap: it only resolves the installation.  This lets us
                # decide whether the instance is already running before doing
                # any launch/wait work.
                session = self._open_target(target, auto_start=False)
                controller = session.controller
                if controller is None:
                    raise DeviceError("MuMu 设备缺少控制器")

                if not controller.can_attach_without_launch():
                    with self._status_lock:
                        with self._lock:
                            if not self._connection_is_current_locked(generation, cancel):
                                raise DeviceError("设备连接已取消")
                            self._pending_session = session
                            thread = threading.Thread(
                                target=self._run_mumu_connection,
                                args=(target, session, device_id, run_id, generation, cancel),
                                name="AALCMumuConnect",
                                daemon=True,
                            )
                            self._connect_thread = thread
                        try:
                            thread.start()
                        except Exception:
                            with self._lock:
                                if self._pending_session is session:
                                    self._pending_session = None
                            raise
                    background_started = True
                    return {
                        "deviceId": device_id,
                        "status": "connecting",
                        "accepted": True,
                        "runId": run_id,
                    }

                controller.attach_existing()
            else:
                session = self._open_target(target)

            if not self._finish_connection(
                session,
                device_id,
                run_id,
                generation,
                cancel,
            ):
                raise DeviceError("设备连接已取消")
            return {"deviceId": device_id, "status": "connected"}
        except Exception as error:
            log.exception("连接设备失败：%s", device_id)
            self._fail_connection(session, device_id, run_id, generation, cancel, error)
            if isinstance(error, DeviceError):
                raise
            raise DeviceError(f"连接设备失败：{error}") from error
        finally:
            if not background_started:
                self._reset_connection_state(generation, cancel)

    def _run_mumu_connection(
        self,
        target: DeviceTarget,
        session: DeviceSession,
        device_id: str,
        run_id: str,
        generation: int,
        cancel: threading.Event,
    ) -> None:
        """Start a cold MuMu instance without holding up the RPC worker."""
        try:
            if cancel.is_set():
                self._cleanup_session(session)
                return

            controller = session.controller
            if controller is None:
                raise DeviceError("MuMu 设备缺少控制器")
            controller.start()

            if cancel.is_set():
                self._cleanup_session(session)
                return

            if not self._finish_connection(
                session,
                device_id,
                run_id,
                generation,
                cancel,
            ):
                self._cleanup_session(session)
                return
        except Exception as error:
            log.exception("后台连接 MuMu 失败：%s", target.info.id)
            self._fail_connection(session, device_id, run_id, generation, cancel, error)

    def _finish_connection(
        self,
        session: DeviceSession,
        device_id: str,
        run_id: str,
        generation: int,
        cancel: threading.Event,
    ) -> bool:
        """Publish a connected session only if it is still the current request."""
        with self._status_lock:
            with self._lock:
                if not self._connection_is_current_locked(generation, cancel):
                    current = False
                else:
                    # Keep activation serialized with disconnect/cancel so a stale
                    # worker can never leave the global automation binding behind.
                    self._activate_runtime(session)
                    self._active = session
                    self._pending_session = None
                    self._connecting = False
                    self._connect_cancel = None
                    self._connect_thread = None
                    self._connect_target = None
                    current = True
            if current:
                # Keep status ordering serialized, but do not hold the manager
                # lock while preview listeners perform their own cleanup.
                self._emit_status(device_id, "connected", run_id=run_id)
                self._emit_connection_details(session)

        if not current:
            # The caller owns cleanup for a stale session.  Keeping it here
            # would make the synchronous path clean the same controller twice
            # after it turns the false result into DeviceError.
            return False
        return True

    def _fail_connection(
        self,
        session: DeviceSession | None,
        device_id: str,
        run_id: str | None,
        generation: int,
        cancel: threading.Event | None,
        error: Exception,
    ) -> None:
        """Clean up a failed connection and report it if it is still current."""
        current = False
        pending: DeviceSession | None = None
        if cancel is not None:
            with self._status_lock:
                with self._lock:
                    current = self._connection_is_current_locked(generation, cancel)
                    if current:
                        pending = self._pending_session
                        self._pending_session = None
                        self._connecting = False
                        self._connect_cancel = None
                        self._connect_thread = None
                        self._connect_target = None

                if current:
                    # Keep failure ordering consistent with the successful
                    # path and with an explicit disconnect.  Cleanup follows
                    # this event so preview consumers stop before controller
                    # teardown, and no later connected event can overtake it.
                    self._restore_runtime_config()
                    self._emit_status(device_id, "disconnected", run_id=run_id)
                    self._emit_notice("error", f"连接设备失败：{error}")

        cleanup = session or pending
        if cleanup is not None:
            try:
                self._cleanup_session(cleanup)
            except Exception:
                log.exception("清理失败的设备连接时出错")

    def _connection_is_current_locked(
        self,
        generation: int,
        cancel: threading.Event | None,
    ) -> bool:
        return (
            cancel is not None
            and self._connecting
            and self._lease_state == "none"
            and not cancel.is_set()
            and self._connect_generation == generation
            and self._connect_cancel is cancel
        )

    def _reset_connection_state(
        self,
        generation: int,
        cancel: threading.Event | None,
    ) -> None:
        with self._lock:
            if self._connect_generation != generation or self._connect_cancel is not cancel:
                return
            self._connecting = False
            self._connect_cancel = None
            self._connect_thread = None
            self._pending_session = None
            self._connect_target = None

    def _cancel_pending_connection(self) -> threading.Thread | None:
        """Cancel a cold-start worker and return it for bounded joining."""
        with self._status_lock:
            with self._lock:
                cancel = self._connect_cancel
                if not self._connecting or cancel is None:
                    return None
                cancel.set()
                self._connect_generation += 1
                thread = self._connect_thread
                self._connecting = False
                self._connect_cancel = None
                self._connect_thread = None
                self._pending_session = None
                self._connect_target = None
            self._restore_runtime_config()
            return thread

    @_device_write
    def disconnect(self) -> dict[str, Any]:
        """Release the active target without closing a game or emulator."""
        return self._release_active(check_busy=True, notice=True)

    @_device_write
    def reconnect_active(self) -> bool:
        """Reconnect the active Scrcpy session without changing the device.

        Android ``wm size`` changes invalidate the dimensions negotiated during
        the current Scrcpy handshake.  Reusing the selected target here keeps
        the device identity and ADB endpoint stable while forcing a fresh
        Scrcpy handshake against the new display dimensions.

        Non-ADB sessions (for example MuMu IPC sessions) do not use
        ``ScrcpyControl`` and therefore do not need this reconnect path.
        """
        if self._busy_checker():
            raise DeviceError("任务运行期间不能重连设备")

        with self._status_lock:
            with self._lock:
                session = self._active
                if session is None:
                    raise DeviceError("未连接设备，请先选择并连接设备")
                if session.target.kind != "adb":
                    return False
                device_id = session.target.info.id

            # Stop preview consumers before tearing down the old controller.
            # ``connect`` will publish the following ``connecting`` and
            # ``connected`` transitions for the replacement session.
            self._emit_status(None, "disconnected")
            self._disconnect_active(restore_config=False)
            self.connect(device_id)
            return True

    @_device_write
    def release_after_task(self) -> dict[str, Any]:
        """Release a session after an explicit task-owned shutdown action.

        ``disconnect`` is intentionally blocked while execution is marked
        busy. A task that explicitly closes its emulator still needs a
        manager-owned cleanup after the action, otherwise the manager keeps a
        dead session and the preview thread keeps retrying it.
        """
        return self._release_active(check_busy=False, notice=False)

    def _release_active(self, *, check_busy: bool, notice: bool) -> dict[str, Any]:
        if check_busy and self._busy_checker():
            raise DeviceError("任务运行期间不能断开设备")
        with self._status_lock:
            pending_thread = self._cancel_pending_connection()
            with self._lock:
                active_id = self._active.target.info.id if self._active else None

            # Stop consumers before tearing down the controller. BackendApplication
            # maps this event to PreviewCapture.stop(), preventing a completed
            # task's preview loop from racing the connection cleanup.
            self._emit_status(None, "disconnected")
            self._disconnect_active(restore_config=True)
        if pending_thread is not None and pending_thread is not threading.current_thread():
            pending_thread.join(timeout=2.0)
        if notice and active_id:
            self._emit_notice("info", "设备已断开连接")
        return {"status": "disconnected"}

    def close(self) -> None:
        """Best-effort shutdown hook for the sidecar process."""
        self._assert_device_writable("关闭设备连接")
        with self._status_lock:
            pending_thread = self._cancel_pending_connection()
            try:
                self._disconnect_active(restore_config=True)
            except Exception:
                log.exception("关闭设备管理器时清理连接失败")
        if pending_thread is not None and pending_thread is not threading.current_thread():
            pending_thread.join(timeout=2.0)

    def _resolve_target(self, device_id: str) -> DeviceTarget:
        with self._lock:
            target = self._targets.get(device_id)
        if target is not None:
            if target.kind == "pc" and not self._valid_hwnd(target.hwnd):
                target = self._discover_pc_window()
                if target is not None:
                    with self._lock:
                        self._targets[device_id] = target
            if target is not None:
                return target

        if device_id == "pc:limbus":
            target = self._discover_pc_window()
        elif device_id.startswith("mumu:"):
            instance_text = device_id.removeprefix("mumu:")
            try:
                instance = int(instance_text)
            except ValueError as error:
                raise DeviceError(f"无效的 MuMu 设备 ID：{device_id}") from error
            target = self._make_mumu_target(instance)
        elif device_id.startswith("adb:"):
            endpoint = device_id.removeprefix("adb:")
            target = self._make_adb_target(endpoint)
        else:
            target = None

        if target is None:
            raise DeviceError(f"未找到设备：{device_id}")
        with self._lock:
            self._targets[device_id] = target
        return target

    def _open_target(
        self,
        target: DeviceTarget,
        *,
        auto_start: bool = True,
        allow_lease: bool = False,
    ) -> DeviceSession:
        self._assert_device_writable("创建设备控制器", allow_lease=allow_lease)
        if target.kind == "pc":
            if not self._valid_hwnd(target.hwnd):
                raise DeviceError("未找到有效的 Limbus Company 游戏窗口")
            return DeviceSession(target)

        if target.kind == "mumu":
            if target.instance_number is None:
                raise DeviceError("MuMu 实例编号缺失")
            self._set_target_runtime_config(target)
            from module.automation.input_handlers.simulator.mumu_control import MumuControl

            if auto_start:
                controller = MumuControl(instance_number=target.instance_number)
            else:
                controller = MumuControl(instance_number=target.instance_number, auto_start=False)
            return DeviceSession(target, controller)

        if target.endpoint is None:
            raise DeviceError("ADB 设备地址缺失")
        self._set_target_runtime_config(target)
        from module.automation.input_handlers.simulator.scrcpy_control import ScrcpyControl

        try:
            return DeviceSession(target, ScrcpyControl(endpoint=target.endpoint))
        except Exception as error:
            log.exception("启动 Scrcpy 控制器失败（%s）", target.endpoint)
            raise DeviceError(f"Scrcpy 设备连接失败：{error}") from error

    def _activate_runtime(self, session: DeviceSession, *, allow_lease: bool = False) -> None:
        self._assert_device_writable("绑定设备运行时", allow_lease=allow_lease)
        # Legacy modules still read these values. Keep them synchronized with
        # the selected session whenever a task or an RPC rebinds the runtime.
        self._set_target_runtime_config(session.target)
        if session.target.kind == "pc":
            from module.game_and_screen import screen

            screen.handle.bind(session.target.hwnd or 0)
        from module.automation import auto

        # ``auto`` is created during module import, before the GPUI selection
        # exists.  Rebuild only its input binding after activating the target.
        auto.init_input(session=session)

    def _set_target_runtime_config(self, target: DeviceTarget) -> None:
        if target.kind == "pc":
            self._set_runtime_config(simulator=False)
            return

        if target.kind == "mumu":
            if target.instance_number is None:
                raise DeviceError("MuMu 实例编号缺失")
            try:
                port = (
                    self._port_from_endpoint(target.endpoint)
                    if target.endpoint
                    else self._mumu_port(target.instance_number)
                )
            except DeviceError:
                port = self._mumu_port(target.instance_number)
            self._set_runtime_config(
                simulator=True,
                simulator_type=0,
                simulator_port=port,
                mumu_instance_number=target.instance_number,
            )
            return

        if target.endpoint is None:
            raise DeviceError("ADB 设备地址缺失")
        if ":" in target.endpoint:
            port = self._port_from_endpoint(target.endpoint)
        else:
            try:
                port = int(cfg.get_value("simulator_port", 0) or 0)
            except (TypeError, ValueError):
                port = 0
        self._set_runtime_config(
            simulator=True,
            simulator_type=10,
            simulator_port=port,
            mumu_instance_number=-1,
        )

    def _disconnect_active(self, *, restore_config: bool) -> None:
        with self._lock:
            session = self._active
            self._active = None
        if session is None:
            return

        try:
            self._cleanup_session(session)
        except Exception as error:
            log.warning("释放设备连接失败：%s", error)
        finally:
            if restore_config:
                self._restore_runtime_config()

    @staticmethod
    def _cleanup_session(session: DeviceSession) -> None:
        if session.target.kind == "mumu":
            controller = session.controller
            if controller is not None:
                try:
                    controller.disconnect()
                except Exception as error:
                    log.debug("断开 MuMu NemuIpc 失败：%s", error)
                try:
                    controller.adb_disconnect()
                except Exception as error:
                    log.debug("断开 MuMu ADB 失败：%s", error)
            from module.automation.input_handlers.simulator.mumu_control import MumuControl

            if MumuControl.connection_device is controller:
                MumuControl.connection_device = None
        elif session.target.kind == "adb":
            controller = session.controller
            if controller is not None:
                cleanup = getattr(controller, "cleanup_session", None)
                stop = getattr(controller, "stop", None)
                if callable(cleanup):
                    try:
                        cleanup()
                    except Exception as error:
                        log.debug("停止 Scrcpy 失败：%s", error)
                elif callable(stop):
                    try:
                        stop()
                    except Exception as error:
                        log.debug("停止 Scrcpy 失败：%s", error)
                else:
                    adb_disconnect = getattr(controller, "adb_disconnect", None)
                    if not callable(adb_disconnect):
                        adb_disconnect = None
                    if callable(adb_disconnect):
                        try:
                            adb_disconnect()
                        except Exception as error:
                            log.debug("断开 ADB 失败：%s", error)
            try:
                from module.automation.input_handlers.simulator.scrcpy_control import ScrcpyControl

                if ScrcpyControl.connection_device is controller:
                    ScrcpyControl.connection_device = None
            except ImportError:
                pass
            try:
                from module.automation.input_handlers.simulator.simulator_control import SimulatorControl

                if SimulatorControl.connection_device is controller:
                    SimulatorControl.connection_device = None
            except ImportError:
                pass
        else:
            from module.game_and_screen import screen

            screen.handle.clear()

    def _set_runtime_config(self, **values: Any) -> None:
        for key, value in values.items():
            if key in self._RUNTIME_CONFIG_KEYS:
                cfg.unsaved_set_value(key, value, stacklevel=3)

    def _restore_runtime_config(self) -> None:
        self._set_runtime_config(**self._runtime_snapshot)

    def _emit_status(
        self,
        device_id: str | None,
        status: str,
        *,
        run_id: str | None = None,
    ) -> None:
        payload = {"deviceId": device_id, "status": status}
        if run_id:
            payload["runId"] = run_id
        with self._status_lock:
            self._emit(self._status_listeners, "device.status", payload)

    def _emit_notice(self, level: str, message: str) -> None:
        self._emit(self._notice_listeners, "app.notice", {"level": level, "message": message})

    def _emit_connection_details(self, session: DeviceSession) -> None:
        connection, screenshot, input_method = self._connection_methods(session.target)
        self._emit_notice("info", f"已连接设备：{session.target.info.name}")
        self._emit_notice("info", f"连接方式：{connection}")
        self._emit_notice("info", f"截图方式：{screenshot}；输入方式：{input_method}")

    @staticmethod
    def _connection_methods(target: DeviceTarget) -> tuple[str, str, str]:
        if target.kind == "mumu":
            return (
                "MuMu IPC（NemuIpc）+ ADB 辅助",
                "MuMu IPC（NemuIpc）",
                "MuMu IPC（NemuIpc）",
            )

        if target.kind == "adb":
            endpoint = target.endpoint or "未知设备"
            return (
                f"Scrcpy（{endpoint}）",
                "Scrcpy 硬件视频流（H.264）",
                "Scrcpy 触控注入（Control Socket）",
            )

        screenshot = (
            "默认窗口截图（PrintWindow）"
            if bool(cfg.get_value("background_click", True))
            else "默认窗口截图（GDI，失败回退 pyautogui）"
        )
        input_method = {
            "background": "Windows 后台输入（pywin32）",
            "foreground": "Windows 前台输入（pyautogui）",
            "window_move": "Windows 窗口移动输入",
        }.get(str(cfg.get_value("win_input_type", "background")), "Windows 后台输入（pywin32）")
        return "Windows 游戏窗口", screenshot, input_method

    @staticmethod
    def _emit(listeners: list[StatusListener], event: str, payload: dict[str, Any]) -> None:
        for listener in list(listeners):
            try:
                listener(event, payload)
            except Exception:
                log.exception("设备事件监听器执行失败：%s", event)

    @staticmethod
    def _valid_hwnd(hwnd: int | None) -> bool:
        if not hwnd:
            return False
        try:
            import win32gui

            return bool(win32gui.IsWindow(hwnd))
        except Exception:
            return False

    def _discover_pc_window(self) -> DeviceTarget | None:
        try:
            import win32gui
            import win32process
        except ImportError:
            return None

        wanted_title = str(cfg.get_value("game_title_name", "LimbusCompany"))
        matches: list[DeviceTarget] = []

        def callback(hwnd: int, _extra: object) -> bool:
            try:
                if not win32gui.IsWindowVisible(hwnd):
                    return True
                title = win32gui.GetWindowText(hwnd)
                class_name = win32gui.GetClassName(hwnd)
                if class_name != "UnityWndClass" or title != wanted_title:
                    return True
                _, pid = win32process.GetWindowThreadProcessId(hwnd)
                _, _, width, height = win32gui.GetClientRect(hwnd)
                detail = f"Windows · {width}×{height} · PID {pid}"
                matches.append(
                    DeviceTarget(
                        DeviceInfo("pc:limbus", "Limbus Company", detail),
                        "pc",
                        hwnd=hwnd,
                    )
                )
            except Exception:
                return True
            return True

        try:
            win32gui.EnumWindows(callback, None)
        except Exception:
            return None
        if len(matches) > 1:
            log.warning("发现多个 Limbus Company 窗口，使用第一个匹配项")
        return matches[0] if matches else None

    def _discover_mumu_targets(self) -> list[DeviceTarget]:
        manager = self._find_mumu_manager()
        if not manager:
            return []

        info = self._query_mumu_info(manager, "all")
        targets = self._targets_from_mumu_info(info)
        if targets:
            return targets

        # Older MuMu builds may not implement ``info -v all``.  Preserve
        # compatibility by probing the configured instance, but only expose
        # it when MuMu confirms that the player actually exists.
        if info is None:
            configured = self._configured_mumu_instance()
            return self._targets_from_mumu_info(self._query_mumu_info(manager, str(configured)))
        return []

    @classmethod
    def _query_mumu_info(cls, manager: str, vmindex: str) -> dict[str, Any] | None:
        """Query MuMu player metadata without constructing a controller."""
        no_window_flag = getattr(subprocess, "CREATE_NO_WINDOW", 0x08000000)
        try:
            result = subprocess.run(
                [manager, "info", "-v", vmindex],
                capture_output=True,
                text=True,
                encoding="utf-8",
                errors="replace",
                timeout=5,
                creationflags=no_window_flag,
            )
        except (OSError, subprocess.SubprocessError) as error:
            log.debug("查询 MuMu 实例信息失败（%s）：%s", vmindex, error)
            return None

        output = result.stdout.strip()
        if not output:
            log.debug("查询 MuMu 实例信息返回空结果（%s）", vmindex)
            return None

        try:
            payload = json.loads("\n".join(line for line in output.splitlines() if "Active code page" not in line))
        except json.JSONDecodeError as error:
            log.debug("解析 MuMu 实例信息失败（%s）：%s", vmindex, error)
            return None

        if not isinstance(payload, dict):
            log.debug("MuMu 实例信息格式无效（%s）", vmindex)
            return None

        error_code = payload.get("errcode")
        try:
            has_error = error_code is not None and int(error_code) != 0
        except (TypeError, ValueError):
            has_error = True
        if has_error:
            log.debug(
                "MuMu 实例不可用（%s）：%s",
                vmindex,
                payload.get("errmsg", f"errcode={error_code}"),
            )
            return None
        return payload

    @classmethod
    def _targets_from_mumu_info(cls, info: Mapping[str, Any] | None) -> list[DeviceTarget]:
        if not info:
            return []

        records: list[tuple[str, Mapping[str, Any]]] = []
        # ``info -v all`` returns {"1": {...}, "2": {...}}, while a single
        # player query returns the player object itself on some versions.
        if "index" in info:
            records.append((str(info.get("index", "")), info))
        else:
            for key, value in info.items():
                if isinstance(value, Mapping):
                    records.append((str(key), value))

        targets: list[DeviceTarget] = []
        seen: set[int] = set()
        for key, record in records:
            record_error = record.get("errcode")
            try:
                if record_error is not None and int(record_error) != 0:
                    continue
            except (TypeError, ValueError):
                continue
            raw_index = record.get("index", key)
            try:
                instance = int(raw_index)
            except (TypeError, ValueError):
                continue
            if instance < 0 or instance in seen:
                continue
            seen.add(instance)

            host = record.get("adb_host_ip") or record.get("adb_host") or "127.0.0.1"
            raw_port = record.get("adb_port")
            endpoint = None
            try:
                port = int(raw_port)
                if 1 <= port <= 65535:
                    endpoint = f"{host}:{port}"
            except (TypeError, ValueError):
                port = cls._mumu_port(instance)

            if endpoint is None:
                port = cls._mumu_port(instance)
            state = record.get("player_state")
            detail = f"实例 {instance} · ADB {port}"
            if isinstance(state, str) and state:
                detail += f" · {state}"
            targets.append(
                cls._make_mumu_target(
                    instance,
                    endpoint=endpoint,
                    detail=detail,
                )
            )
        return targets

    def _discover_adb_targets(self) -> list[DeviceTarget]:
        try:
            from adbutils import adb

            devices = adb.device_list()
        except Exception as error:
            log.debug("扫描 ADB 设备失败：%s", error)
            return []

        result: list[DeviceTarget] = []
        for device in devices:
            serial = str(getattr(device, "serial", ""))
            if not serial:
                continue
            name, detail = self._inspect_adb_device(device)
            result.append(
                self._make_adb_target(
                    serial,
                    name=name,
                    detail=detail,
                )
            )
        return result

    @staticmethod
    def _inspect_adb_device(device: Any) -> tuple[str, str]:
        """识别 ADB 设备类型（真机型号 vs 第三方模拟器）。"""
        serial = str(getattr(device, "serial", ""))
        state = str(getattr(device, "state", "device"))
        if state != "device":
            return f"ADB 设备 {serial}", f"ADB · {serial} · {state}"

        is_network = ":" in serial
        conn_label = "Wi-Fi" if is_network else "USB"

        try:
            props_raw = device.shell("getprop")

            def get_prop(key: str) -> str:
                match = re.search(rf"\[{key}\]:\s*\[(.*?)\]", props_raw)
                return match.group(1).strip() if match else ""

            # 1. 检查各大知名模拟器特征
            if get_prop("ro.ld.version") or "leidian" in get_prop("ro.hardware").lower():
                return "雷电模拟器", f"ADB · {serial}"
            if get_prop("ro.nox.version") or "nox" in get_prop("ro.hardware").lower():
                return "夜神模拟器", f"ADB · {serial}"
            if get_prop("ro.microvirt.version") or "microvirt" in get_prop("ro.hardware").lower():
                return "逍遥模拟器", f"ADB · {serial}"
            if get_prop("ro.bluestacks.version") or get_prop("bst.instance.name"):
                return "蓝叠模拟器", f"ADB · {serial}"
            if "Subsystem for Android" in get_prop("ro.product.model"):
                return "微软 WSA", f"ADB · {serial}"

            # 2. 检查通用 QEMU / VirtualBox 模拟器
            is_qemu = get_prop("ro.kernel.qemu") == "1"
            hardware = get_prop("ro.hardware").lower()
            if is_qemu or hardware in ("goldfish", "ranchu", "vbox86", "ttvm_x86"):
                return "Android 模拟器", f"ADB · {serial}"

            # 3. 确认为手机真机：提取友好型号，不带“真机 · ”前缀
            market_name = get_prop("ro.product.marketname")
            brand = get_prop("ro.product.brand").strip()
            model = get_prop("ro.product.model").strip()

            if market_name:
                phone_name = market_name
            elif brand and model:
                if model.lower().startswith(brand.lower()):
                    phone_name = model
                else:
                    phone_name = f"{brand} {model}"
            else:
                phone_name = model or brand or "Android 设备"

            return phone_name, f"{conn_label} · {serial}"
        except Exception as error:
            log.debug("读取 ADB 设备属性失败（%s）：%s", serial, error)
            return f"ADB 设备 {serial}", f"ADB · {serial}"

    @staticmethod
    def _make_mumu_target(
        instance: int,
        *,
        name: str | None = None,
        endpoint: str | None = None,
        detail: str | None = None,
    ) -> DeviceTarget:
        port = DeviceManager._mumu_port(instance)
        endpoint = endpoint or f"127.0.0.1:{port}"
        return DeviceTarget(
            DeviceInfo(
                f"mumu:{instance}",
                name or f"MuMu 模拟器 #{instance}",
                detail or f"实例 {instance} · ADB {port}",
            ),
            "mumu",
            endpoint=endpoint,
            instance_number=instance,
        )

    @staticmethod
    def _make_adb_target(
        endpoint: str,
        *,
        name: str | None = None,
        detail: str | None = None,
    ) -> DeviceTarget:
        return DeviceTarget(
            DeviceInfo(
                f"adb:{endpoint}",
                name or f"ADB 设备 {endpoint}",
                detail or f"ADB · {endpoint}",
            ),
            "adb",
            endpoint=endpoint,
        )

    @staticmethod
    def _mumu_port(instance: int) -> int:
        return 16384 + instance * 32

    @staticmethod
    def _port_from_endpoint(endpoint: str) -> int:
        match = re.search(r":(\d+)$", endpoint)
        if not match:
            raise DeviceError(f"ADB 地址必须包含端口：{endpoint}")
        port = int(match.group(1))
        if not 1 <= port <= 65535:
            raise DeviceError(f"ADB 端口无效：{port}")
        return port

    def _configured_mumu_instance(self) -> int:
        value = cfg.get_value("mumu_instance_number", -1)
        try:
            value = int(value)
        except (TypeError, ValueError):
            value = -1
        if value >= 0:
            return value
        port = cfg.get_value("simulator_port", 16384)
        try:
            port = int(port)
        except (TypeError, ValueError):
            port = 16384
        if port >= 16384 and (port - 16384) % 32 == 0:
            return (port - 16384) // 32
        return 0

    @staticmethod
    def _find_mumu_manager() -> str | None:
        if os.name != "nt":
            return None
        try:
            import winreg

            names = (
                "MuMuPlayer-12.0",
                "MuMuPlayer",
                "MuMuPlayerGlobal-12.0",
                "MuMuPlayerGlobal",
                "YXArkNights-12.0",
            )
            for name in names:
                key_path = rf"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\{name}"
                try:
                    with winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE, key_path) as key:
                        display_icon = str(winreg.QueryValueEx(key, "DisplayIcon")[0]).strip('"')
                    install_path = os.path.dirname(display_icon)
                    manager = os.path.join(install_path, "MuMuManager.exe")
                    if os.path.isfile(manager):
                        return manager
                    shell_manager = os.path.join(os.path.dirname(install_path), "shell", "MuMuManager.exe")
                    if os.path.isfile(shell_manager):
                        return shell_manager
                except (FileNotFoundError, OSError):
                    continue
        except (ImportError, OSError):
            return None
        return None


_runner_runtime_lock = threading.RLock()
_runner_runtime: RunnerDeviceRuntime | None = None


def _set_runner_runtime(runtime: RunnerDeviceRuntime) -> None:
    global _runner_runtime
    with _runner_runtime_lock:
        _runner_runtime = runtime


def _clear_runner_runtime(runtime: RunnerDeviceRuntime) -> None:
    global _runner_runtime
    with _runner_runtime_lock:
        if _runner_runtime is runtime:
            _runner_runtime = None


def get_runner_runtime() -> RunnerDeviceRuntime | None:
    """Return the currently installed private Runner runtime, if any."""

    with _runner_runtime_lock:
        runtime = _runner_runtime
        # The registry lock is authoritative for this lookup.  Do not take
        # runtime._lock here because close() acquires that lock before the
        # registry lock when unregistering.
        if runtime is None or runtime._closed:
            return None
        return runtime


def get_runner_session() -> DeviceSession | None:
    """Return the Runner-owned session without consulting sidecar state."""

    runtime = get_runner_runtime()
    return None if runtime is None else runtime.session


def install_runner_session(runtime: RunnerDeviceRuntime) -> RunnerDeviceRuntime:
    """Install a previously constructed Runner runtime."""

    if not isinstance(runtime, RunnerDeviceRuntime):
        raise TypeError("runtime must be a RunnerDeviceRuntime")
    return runtime.install()


def _runner_policy_for_factory() -> Any:
    from module.automation.input_handlers.simulator.runner_policy import RunnerPolicy

    # Factory callers are already in the Runner process even when tests (or a
    # bootstrap shim) have not set AALC_RUNNER_MODE.  Preserve an explicit
    # launch opt-in from the environment while keeping the default fail-closed.
    env_policy = RunnerPolicy.from_env()
    if "AALC_ALLOW_EMULATOR_LAUNCH" in os.environ:
        return RunnerPolicy(runner_mode=True, allow_emulator_launch=env_policy.allow_emulator_launch)
    return RunnerPolicy(runner_mode=True)


def _reservation_value(reservation: Mapping[str, Any], *keys: str) -> Any:
    for key in keys:
        if key in reservation:
            return reservation[key]
    return None


def _normalize_runner_reservation(
    run_id: str,
    reservation: Mapping[str, Any] | None,
) -> dict[str, Any]:
    if reservation is None:
        return {"runId": run_id}
    if not isinstance(reservation, Mapping):
        raise DeviceError("CleanupReservation 必须是对象")
    reserved_run_id = _reservation_value(reservation, "runId", "run_id")
    if reserved_run_id is not None and str(reserved_run_id) != run_id:
        raise DeviceError("CleanupReservation runId 与 Runner runId 不匹配")

    normalized = {"runId": run_id}
    scid = _reservation_value(reservation, "scid", "reservedScrcpyScid", "reserved_scid")
    socket_name = _reservation_value(
        reservation,
        "socketName",
        "reservedSocketName",
        "reserved_socket_name",
    )
    forward_port = _reservation_value(
        reservation,
        "adbForwardPort",
        "reservedAdbForwardPort",
        "reserved_adb_forward_port",
        "forwardPort",
        "forward_port",
    )
    serial = _reservation_value(reservation, "serial", "deviceSerial", "device_serial")
    if scid is not None:
        normalized["scid"] = scid
    if socket_name is not None:
        normalized["socketName"] = socket_name
    if forward_port is not None:
        normalized["adbForwardPort"] = forward_port
    if serial is not None:
        normalized["serial"] = serial
    return normalized


def _create_runner_session(
    manager: DeviceManager,
    target: DeviceTarget,
    run_id: str,
    reservation: dict[str, Any],
    *,
    runner_policy: Any,
    remote_cleanup: Callable[[dict[str, object]], object] | None,
) -> DeviceSession:
    """Construct one controller from an authoritative target snapshot."""

    manager._set_target_runtime_config(target)
    if target.kind == "pc":
        if not manager._valid_hwnd(target.hwnd):
            raise DeviceError("Runner 目标窗口已失效，拒绝重新发现或切换设备")
        return DeviceSession(target)

    if target.kind == "mumu":
        if target.instance_number is None:
            raise DeviceError("Runner MuMu 目标缺少实例编号")
        from module.automation.input_handlers.simulator.mumu_control import MumuControl

        controller = MumuControl(
            instance_number=target.instance_number,
            auto_start=False,
            runner_policy=runner_policy,
        )
        session = DeviceSession(target, controller)
        can_attach = getattr(controller, "can_attach_without_launch", None)
        if callable(can_attach) and not can_attach():
            raise DeviceError("Runner 仅允许附加已运行的 MuMu 实例，禁止自动启动或重启")
        attach_existing = getattr(controller, "attach_existing", None)
        if not callable(attach_existing):
            raise DeviceError("Runner MuMu 控制器缺少 attach_existing 接口")
        attach_existing()
        return session

    if target.kind != "adb" or target.endpoint is None:
        raise DeviceError("Runner ADB 目标地址缺失")
    from module.automation.input_handlers.simulator.scrcpy_control import ScrcpyControl

    reservation_serial = reservation.get("serial")
    if reservation_serial is not None and str(reservation_serial) != target.endpoint:
        raise DeviceError("CleanupReservation serial 与 ADB 目标不匹配")
    controller = ScrcpyControl(
        endpoint=target.endpoint,
        run_id=run_id,
        scid=reservation.get("scid"),
        socket_name=reservation.get("socketName"),
        reserved_forward_port=reservation.get("adbForwardPort"),
        runner_policy=runner_policy,
        remote_cleanup=remote_cleanup,
    )
    return DeviceSession(target, controller)


def create_runner_runtime(
    target_snapshot: DeviceTarget | Mapping[str, Any],
    run_id: str,
    cleanup_reservation: Mapping[str, Any] | None = None,
    *,
    runner_policy: Any = None,
    remote_cleanup: Callable[[dict[str, object]], object] | None = None,
) -> RunnerDeviceRuntime:
    """Create/install a private runtime from serialized Runner inputs.

    The target snapshot is authoritative: this function never calls device
    discovery and never uses the sidecar process-wide manager.  MuMu is
    checked and attached with ``auto_start=False``; ADB targets are opened
    through Scrcpy with the supplied per-run reservation.
    """

    if not isinstance(run_id, str) or not run_id.strip():
        raise DeviceError("Runner runId 不能为空")
    if isinstance(target_snapshot, DeviceTarget):
        target = target_snapshot
    else:
        target = DeviceTarget.from_snapshot(target_snapshot)
    reservation = _normalize_runner_reservation(run_id, cleanup_reservation)
    policy = runner_policy or _runner_policy_for_factory()
    manager = DeviceManager()
    session: DeviceSession | None = None
    try:
        session = _create_runner_session(
            manager,
            target,
            run_id,
            reservation,
            runner_policy=policy,
            remote_cleanup=remote_cleanup,
        )
        actual_reservation = getattr(session.controller, "session_reservation", None)
        if callable(actual_reservation):
            actual_reservation = actual_reservation()
        if isinstance(actual_reservation, Mapping):
            # Controller-normalized identifiers (including generated scid and
            # forward port) are retained for the host/ledger without allowing
            # live controller objects into the serializable reservation.
            for key in ("runId", "scid", "socketName", "remoteSocketName", "forwardPort", "serial"):
                value = actual_reservation.get(key)
                if value is not None:
                    reservation[key] = value
        runtime = RunnerDeviceRuntime(manager, run_id, session, reservation)
        runtime.install()
        return runtime
    except Exception:
        if session is not None:
            try:
                manager._cleanup_session(session)
            except Exception:
                log.exception("Runner 设备运行时创建失败后的清理失败")
        try:
            manager._restore_runtime_config()
        except Exception:
            log.exception("Runner 设备运行时创建失败后的配置恢复失败")
        raise


_default_manager: DeviceManager | None = None


def get_device_manager() -> DeviceManager:
    """Return the manager for the current process execution context.

    Sidecar callers keep the historical singleton.  A Runner runtime is
    installed before legacy task modules are imported, so returning its fresh
    manager here makes older helpers that call ``get_device_manager()`` see the
    same private session instead of creating a second manager with no active
    device.  The registry is process-local and is cleared during bounded
    runtime cleanup, after which sidecar compatibility resumes.
    """
    global _default_manager
    runner_runtime = get_runner_runtime()
    if runner_runtime is not None:
        return runner_runtime.manager
    if _default_manager is None:
        _default_manager = DeviceManager()
    return _default_manager


def is_simulator_runtime() -> bool:
    """Return whether the selected runtime is an Android emulator.

    The legacy configuration remains the fallback for command-line callers,
    but a sidecar-selected session always wins over its persisted values.
    """
    session = get_runner_session() or get_device_manager().active_session
    if session is not None:
        return session.target.kind in ("mumu", "adb")
    return bool(cfg.get_value("simulator", False))


__all__ = [
    "DeviceError",
    "DeviceInfo",
    "DeviceLease",
    "DeviceManager",
    "DeviceSession",
    "DeviceTarget",
    "RunnerDeviceRuntime",
    "create_runner_runtime",
    "get_device_manager",
    "get_runner_runtime",
    "get_runner_session",
    "install_runner_session",
    "is_simulator_runtime",
]
