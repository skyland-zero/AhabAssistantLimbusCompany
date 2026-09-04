"""Cooperative cancellation and pause control owned by one Runner.

The object is intentionally per-run.  It must not be reused by a second Runner:
the cancellation event, pause condition and all waiters are state belonging to one
task invocation only.  Every blocking helper is bounded and is woken immediately
by :meth:`request_stop`.
"""

from __future__ import annotations

import threading
import time
from collections.abc import Callable
from typing import Any


class ExecutionCancelled(RuntimeError):
    """Raised by a cooperative task checkpoint after stop was requested."""

    code = "EXECUTION_STOP_REQUESTED"


# A descriptive alias is convenient for task adapters and keeps callers from
# depending on the implementation's original exception name.
CancellationRequested = ExecutionCancelled


class ExecutionControl:
    """A single source of truth for cancellation and pause state.

    ``cancel_event`` is public on purpose: a few legacy adapters need to pass an
    event into an SDK call.  New code should prefer :meth:`checkpoint` and the
    interruptible wait helpers so a stop also wakes a paused task.
    """

    def __init__(self, *, wait_quantum: float = 0.1) -> None:
        if not isinstance(wait_quantum, (int, float)) or wait_quantum <= 0:
            raise ValueError("wait_quantum must be positive")
        self.cancel_event = threading.Event()
        self.condition = threading.Condition(threading.RLock())
        self._paused = False
        self._wait_quantum = float(wait_quantum)

    @property
    def paused(self) -> bool:
        with self.condition:
            return self._paused

    @property
    def cancelled(self) -> bool:
        return self.cancel_event.is_set()

    @property
    def stop_requested(self) -> bool:
        """Readable synonym used by status/reporting code."""

        return self.cancel_event.is_set()

    def set_paused(self, paused: bool) -> bool:
        """Set the explicit pause target and wake all waiters.

        Returns ``False`` when the requested value is already in effect.  Once a
        stop is requested, pausing is not allowed to reintroduce an uninterruptible
        wait; the effective state remains unpaused.
        """

        if not isinstance(paused, bool):
            raise TypeError("paused must be bool")
        with self.condition:
            target = paused and not self.cancel_event.is_set()
            changed = self._paused != target
            self._paused = target
            self.condition.notify_all()
            return changed

    def request_stop(self) -> bool:
        """Request cancellation, unpause, and wake every blocked waiter.

        The return value indicates whether this call changed the cancellation
        state.  Repeated stop commands are intentionally idempotent.
        """

        first_request = not self.cancel_event.is_set()
        self.cancel_event.set()
        with self.condition:
            self._paused = False
            self.condition.notify_all()
        return first_request

    def checkpoint(self) -> None:
        """Raise :class:`ExecutionCancelled` if this run has been stopped."""

        if self.cancel_event.is_set():
            raise ExecutionCancelled("execution stop requested")

    def check_cancelled(self) -> None:
        """Compatibility spelling for task code."""

        self.checkpoint()

    def wait_if_paused(self, *, timeout: float | None = None) -> bool:
        """Wait until resumed or stopped.

        The optional timeout is a *maximum total wait*, not a condition wait
        timeout.  ``True`` means the task may continue; a stop raises
        :class:`ExecutionCancelled`.  A timeout returns ``False`` while preserving
        the paused state, which lets callers implement periodic housekeeping.
        """

        deadline = None if timeout is None else time.monotonic() + max(0.0, float(timeout))
        while True:
            self.checkpoint()
            with self.condition:
                if not self._paused:
                    return True
                if deadline is not None:
                    remaining = deadline - time.monotonic()
                    if remaining <= 0:
                        return False
                    wait_for = min(self._wait_quantum, remaining)
                else:
                    wait_for = self._wait_quantum
                self.condition.wait(wait_for)

    def interruptible_sleep(self, seconds: float, *, poll_interval: float | None = None) -> None:
        """Sleep while responding promptly to pause/resume and stop.

        This method intentionally uses the condition rather than ``time.sleep``;
        ``request_stop`` can therefore wake it immediately even for a long delay.
        """

        if not isinstance(seconds, (int, float)):
            raise TypeError("seconds must be numeric")
        duration = max(0.0, float(seconds))
        quantum = self._wait_quantum if poll_interval is None else float(poll_interval)
        if quantum <= 0:
            raise ValueError("poll_interval must be positive")
        deadline = time.monotonic() + duration
        while True:
            self.checkpoint()
            self.wait_if_paused()
            self.checkpoint()
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                return
            with self.condition:
                # Re-check after acquiring the condition so a stop racing with
                # this call cannot be lost between checkpoint and wait.
                if self.cancel_event.is_set():
                    continue
                self.condition.wait(min(quantum, remaining))

    def run_checkpointed(self, callback: Callable[..., Any], *args: Any, **kwargs: Any) -> Any:
        """Run one small callback with a checkpoint on both sides."""

        self.checkpoint()
        result = callback(*args, **kwargs)
        self.checkpoint()
        return result

    def bind(self, target: Any) -> Any:
        """Bind this control to a newly-created controller/task adapter.

        Adapters in the codebase have used both ``set_execution_control`` and an
        ``execution_control`` attribute.  Supporting both keeps replacement of a
        controller from accidentally copying a stale boolean.  The target is
        returned for fluent construction.
        """

        setter = getattr(target, "set_execution_control", None)
        if callable(setter):
            setter(self)
        elif hasattr(target, "execution_control"):
            setattr(target, "execution_control", self)
        return target


__all__ = [
    "CancellationRequested",
    "ExecutionCancelled",
    "ExecutionControl",
]
