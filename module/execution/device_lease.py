"""Transactional device lease guard for Runner-owned control.

This first-batch implementation intentionally does not know how a specific MuMu or
scrcpy controller works.  Integrations inject quiesce/restore callbacks; the
manager still enforces the important invariants and generation checks at the
sidecar boundary.
"""

from __future__ import annotations

import threading
import time
from collections.abc import Callable, Mapping
from dataclasses import dataclass, field
from enum import StrEnum
from typing import Any


class LeaseState(StrEnum):
    NONE = "none"
    ACQUIRING = "acquiring"
    RUNNER = "runner"
    RESTORING = "restoring"


class DeviceLeaseError(RuntimeError):
    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = code
        self.message = message

    def as_dict(self) -> dict[str, str]:
        return {"code": self.code, "message": self.message}


@dataclass(frozen=True, slots=True)
class DeviceLease:
    run_id: str
    generation: int
    target: Mapping[str, Any]
    preview_was_enabled: bool = False
    original_window_state: Mapping[str, Any] = field(default_factory=dict)
    reserved_scrcpy_scid: str | None = None
    reserved_socket_name: str | None = None
    reserved_adb_forward_port: int | None = None
    acquired_at: float = field(default_factory=time.monotonic)

    def to_dict(self) -> dict[str, Any]:
        return {
            "runId": self.run_id,
            "generation": self.generation,
            "target": dict(self.target),
            "previewWasEnabled": self.preview_was_enabled,
            "originalWindowState": dict(self.original_window_state),
            "reservedScrcpyScid": self.reserved_scrcpy_scid,
            "reservedSocketName": self.reserved_socket_name,
            "reservedAdbForwardPort": self.reserved_adb_forward_port,
            "acquiredAt": self.acquired_at,
        }


class DeviceLeaseManager:
    """Owns one sidecar↔Runner device capability at a time."""

    def __init__(
        self,
        *,
        active_tool_probe: Callable[[], bool] | None = None,
        quiesce_callback: Callable[..., Any] | None = None,
        restore_callback: Callable[..., Any] | None = None,
        generation_start: int = 0,
    ) -> None:
        if generation_start < 0:
            raise ValueError("generation_start must be non-negative")
        self._condition = threading.Condition(threading.RLock())
        self._state = LeaseState.NONE
        self._generation = int(generation_start)
        self._lease: DeviceLease | None = None
        self._desired_preview_enabled: bool | None = None
        self.active_tool_probe = active_tool_probe
        self.quiesce_callback = quiesce_callback
        self.restore_callback = restore_callback

    @property
    def state(self) -> LeaseState:
        with self._condition:
            return self._state

    @property
    def lease(self) -> DeviceLease | None:
        with self._condition:
            return self._lease

    @property
    def generation(self) -> int:
        with self._condition:
            return self._generation

    @property
    def desired_preview_enabled(self) -> bool | None:
        with self._condition:
            return self._desired_preview_enabled

    def snapshot(self) -> dict[str, Any]:
        with self._condition:
            return {
                "state": self._state.value,
                "generation": self._generation,
                "lease": None if self._lease is None else self._lease.to_dict(),
                "desiredPreviewEnabled": self._desired_preview_enabled,
            }

    def acquire(
        self,
        run_id: str,
        target: Mapping[str, Any],
        *,
        preview_was_enabled: bool = False,
        original_window_state: Mapping[str, Any] | None = None,
        reserved_scrcpy_scid: str | None = None,
        reserved_socket_name: str | None = None,
        reserved_adb_forward_port: int | None = None,
        timeout: float | None = None,
    ) -> DeviceLease:
        if not run_id:
            raise ValueError("run_id is required")
        if not isinstance(target, Mapping):
            raise TypeError("target must be a mapping")
        with self._condition:
            if self._state is not LeaseState.NONE or self._lease is not None:
                raise DeviceLeaseError("EXECUTION_BUSY", "device lease is already active")
            if self.active_tool_probe is not None and self.active_tool_probe():
                raise DeviceLeaseError("DEVICE_TOOL_ACTIVE", "a device tool is active")
            self._generation += 1
            generation = self._generation
            self._state = LeaseState.ACQUIRING
            self._condition.notify_all()
        lease = DeviceLease(
            run_id=str(run_id),
            generation=generation,
            target=dict(target),
            preview_was_enabled=bool(preview_was_enabled),
            original_window_state=dict(original_window_state or {}),
            reserved_scrcpy_scid=reserved_scrcpy_scid,
            reserved_socket_name=reserved_socket_name,
            reserved_adb_forward_port=reserved_adb_forward_port,
        )
        try:
            if self.quiesce_callback is not None:
                self._invoke_callback(self.quiesce_callback, lease, timeout=timeout)
        except TimeoutError as exc:
            self._reset_after_acquire_failure()
            raise DeviceLeaseError("DEVICE_QUIESCE_TIMEOUT", str(exc)) from exc
        except DeviceLeaseError:
            self._reset_after_acquire_failure()
            raise
        except Exception as exc:
            self._reset_after_acquire_failure()
            raise DeviceLeaseError("DEVICE_QUIESCE_TIMEOUT", str(exc)) from exc
        with self._condition:
            # A release cannot race this assignment without holding the same
            # condition; generation remains valid for all Runner callbacks.
            self._lease = lease
            self._state = LeaseState.RUNNER
            self._condition.notify_all()
            return lease

    def _reset_after_acquire_failure(self) -> None:
        with self._condition:
            self._state = LeaseState.NONE
            self._lease = None
            self._condition.notify_all()

    def release(
        self,
        run_id: str,
        generation: int,
        *,
        disposition: str = "restore",
        timeout: float | None = None,
    ) -> str:
        with self._condition:
            lease = self._lease
            if lease is None or lease.run_id != run_id or lease.generation != generation:
                raise DeviceLeaseError("STALE_RUN", "device lease generation is stale")
            if self._state is LeaseState.RESTORING:
                return "restoring"
            self._state = LeaseState.RESTORING
            self._condition.notify_all()
        restore_state = "restored"
        try:
            if self.restore_callback is not None:
                result = self._invoke_callback(self.restore_callback, lease, disposition=disposition, timeout=timeout)
                if result in {"restored", "disconnected", "failed"}:
                    restore_state = str(result)
                elif isinstance(result, Mapping) and result.get("deviceRestore") in {
                    "restored",
                    "disconnected",
                    "failed",
                }:
                    restore_state = str(result["deviceRestore"])
        except Exception as exc:
            restore_state = "failed"
            with self._condition:
                self._state = LeaseState.NONE
                self._lease = None
                self._condition.notify_all()
            raise DeviceLeaseError("DEVICE_RESTORE_FAILED", str(exc)) from exc
        with self._condition:
            self._state = LeaseState.NONE
            self._lease = None
            self._condition.notify_all()
        return restore_state

    @staticmethod
    def _invoke_callback(callback: Callable[..., Any], lease: DeviceLease, **kwargs: Any) -> Any:
        try:
            return callback(lease, **kwargs)
        except TypeError:
            try:
                return callback(lease)
            except TypeError:
                return callback()

    def set_desired_preview_enabled(self, enabled: bool) -> bool:
        if not isinstance(enabled, bool):
            raise TypeError("enabled must be bool")
        with self._condition:
            changed = self._desired_preview_enabled != enabled
            self._desired_preview_enabled = enabled
            return changed

    def is_current(self, run_id: str, generation: int) -> bool:
        with self._condition:
            return bool(
                self._lease is not None
                and self._state is LeaseState.RUNNER
                and self._lease.run_id == run_id
                and self._lease.generation == generation
            )

    def assert_current(self, run_id: str, generation: int) -> DeviceLease:
        with self._condition:
            lease = self._lease
            if (
                lease is None
                or self._state is not LeaseState.RUNNER
                or lease.run_id != run_id
                or lease.generation != generation
            ):
                raise DeviceLeaseError("STALE_RUN", "device lease generation is stale")
            return lease

    def assert_sidecar_available(self) -> None:
        with self._condition:
            if self._state in {LeaseState.ACQUIRING, LeaseState.RUNNER, LeaseState.RESTORING}:
                raise DeviceLeaseError("DEVICE_LEASED", "Runner currently owns the device")

    def wait_for_state(self, state: LeaseState | str, timeout: float = 20.0) -> LeaseState:
        expected = LeaseState(state)
        deadline = time.monotonic() + max(0.0, float(timeout))
        with self._condition:
            while self._state is not expected:
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    return self._state
                self._condition.wait(remaining)
            return self._state


__all__ = ["DeviceLease", "DeviceLeaseError", "DeviceLeaseManager", "LeaseState"]
