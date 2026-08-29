"""Fork-owned cooperative cancellation bridge for legacy automation code."""

from __future__ import annotations

import threading
import time
from collections.abc import Callable

_lock = threading.RLock()
_cancel_event: threading.Event | None = None


def bind_cancel_event(event: threading.Event | None) -> None:
    global _cancel_event
    with _lock:
        _cancel_event = event


def cancellation_requested() -> bool:
    with _lock:
        event = _cancel_event
    return event is not None and event.is_set()


def current_cancel_event() -> threading.Event | None:
    """Return the currently bound event for legacy thread adapters."""

    with _lock:
        return _cancel_event


def request_cancellation() -> bool:
    with _lock:
        event = _cancel_event
    if event is None:
        return False
    event.set()
    return True


def check_cancelled() -> None:
    if cancellation_requested():
        # Import lazily to keep this core bridge independent during startup.
        from module.my_error.my_error import userStopError

        raise userStopError("用户已请求停止任务")


def interruptible_sleep(seconds: float, *, poll_interval: float = 0.1) -> None:
    deadline = time.monotonic() + max(0.0, seconds)
    while True:
        check_cancelled()
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            return
        with _lock:
            event = _cancel_event
        delay = min(poll_interval, remaining)
        if event is None:
            time.sleep(delay)
        elif event.wait(delay):
            check_cancelled()


def wait_for_event(event: threading.Event, timeout: float, *, poll_interval: float = 0.1) -> bool:
    deadline = time.monotonic() + max(0.0, timeout)
    while True:
        check_cancelled()
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            return event.is_set()
        if event.wait(min(poll_interval, remaining)):
            return True


def cancellable_call(call: Callable[[], object]) -> object:
    check_cancelled()
    result = call()
    check_cancelled()
    return result


__all__ = [
    "bind_cancel_event",
    "cancellable_call",
    "cancellation_requested",
    "check_cancelled",
    "current_cancel_event",
    "interruptible_sleep",
    "request_cancellation",
    "wait_for_event",
]
