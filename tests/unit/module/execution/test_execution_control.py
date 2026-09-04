from __future__ import annotations

import threading
import time

import pytest

from module.execution.execution_control import ExecutionCancelled, ExecutionControl


def test_stop_unpauses_and_wakes_paused_waiter() -> None:
    control = ExecutionControl(wait_quantum=0.01)
    control.set_paused(True)
    result: list[str] = []

    def wait() -> None:
        try:
            control.wait_if_paused()
        except ExecutionCancelled:
            result.append("cancelled")

    thread = threading.Thread(target=wait)
    thread.start()
    time.sleep(0.03)
    control.request_stop()
    thread.join(1)
    assert not thread.is_alive()
    assert result == ["cancelled"]
    assert control.paused is False


def test_interruptible_sleep_is_woken_by_stop() -> None:
    control = ExecutionControl(wait_quantum=0.01)

    def wait() -> None:
        with pytest.raises(ExecutionCancelled):
            control.interruptible_sleep(10)

    thread = threading.Thread(target=wait)
    thread.start()
    time.sleep(0.03)
    start = time.monotonic()
    control.request_stop()
    thread.join(1)
    assert time.monotonic() - start < 0.5
    assert not thread.is_alive()


def test_stop_is_idempotent_and_checkpoint_raises() -> None:
    control = ExecutionControl()
    assert control.request_stop() is True
    assert control.request_stop() is False
    with pytest.raises(ExecutionCancelled):
        control.checkpoint()
