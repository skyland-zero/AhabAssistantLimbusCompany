from __future__ import annotations

import threading

import pytest

from module.execution.command_queue import CommandQueue, CommandQueueError


def test_priority_writer_sends_finish_ack_before_handshake_and_pause() -> None:
    entered = threading.Event()
    release = threading.Event()
    sent: list[dict] = []

    def send(header, payload=b""):
        del payload
        if not entered.is_set():
            entered.set()
            assert release.wait(1.0)
        sent.append(dict(header))

    queue = CommandQueue(send, run_id="priority", maxsize=8)
    try:
        first = queue.enqueue("setPaused", fields={"paused": True})
        assert entered.wait(1.0)
        queue.enqueue("setPaused", fields={"paused": False})
        start = queue.enqueue("start", fields={"spec": {"taskId": "mirror"}})
        finish_ack = queue.enqueue("finishAck", fields={"finalSeq": 4})
        release.set()

        assert first.wait(1.0) == 1
        assert finish_ack.wait(1.0) == 2
        assert start.wait(1.0) == 3
        assert [item["type"] for item in sent] == ["setPaused", "finishAck", "start", "setPaused"]
        assert [item["commandSeq"] for item in sent] == [1, 2, 3, 4]
        assert sent[-1]["paused"] is False
    finally:
        queue.close(1.0)


def test_set_paused_flood_keeps_only_latest_queued_value() -> None:
    entered = threading.Event()
    release = threading.Event()
    sent: list[dict] = []

    def send(header, payload=b""):
        del payload
        if not entered.is_set():
            entered.set()
            assert release.wait(1.0)
        sent.append(dict(header))

    queue = CommandQueue(send, run_id="pause-flood", maxsize=2)
    try:
        first = queue.enqueue("setPaused", fields={"paused": False})
        assert entered.wait(1.0)
        latest = None
        for index in range(100):
            latest = queue.enqueue("setPaused", fields={"paused": index % 2 == 0})
        assert latest is not None
        assert queue.qsize() == 1
        release.set()
        first.wait(1.0)
        latest.wait(1.0)
        assert [item["type"] for item in sent] == ["setPaused", "setPaused"]
        assert sent[-1]["paused"] is (99 % 2 == 0)
    finally:
        queue.close(1.0)


def test_full_queue_rejects_critical_command_without_waiting() -> None:
    entered = threading.Event()
    release = threading.Event()

    def send(header, payload=b""):
        del header, payload
        entered.set()
        assert release.wait(1.0)

    queue = CommandQueue(send, run_id="full", maxsize=1)
    try:
        first = queue.enqueue("setPaused", fields={"paused": True})
        assert entered.wait(1.0)
        queue.enqueue("start", fields={"spec": {"taskId": "mirror"}})
        with pytest.raises(CommandQueueError) as raised:
            queue.enqueue("finishAck", fields={"finalSeq": 1})
        assert raised.value.code == "IPC_BACKPRESSURE"
        release.set()
        first.wait(1.0)
    finally:
        release.set()
        queue.close(1.0)


def test_close_rejects_new_commands_and_eventually_stops_blocked_writer() -> None:
    entered = threading.Event()
    release = threading.Event()

    def send(header, payload=b""):
        del header, payload
        entered.set()
        assert release.wait(1.0)

    queue = CommandQueue(send, run_id="close-race", maxsize=2)
    ticket = queue.enqueue("setPaused", fields={"paused": True})
    assert entered.wait(1.0)
    assert queue.close(0.01) is False
    with pytest.raises(CommandQueueError) as raised:
        queue.enqueue("stop", fields={"requestedBy": "shutdown"})
    assert raised.value.code == "IPC_CLOSED"
    release.set()
    assert ticket.wait(1.0) == 1
    assert queue.close(1.0) is True
    assert not queue.thread.is_alive()


def test_writer_error_notifies_supervisor_callback_once() -> None:
    errors: list[CommandQueueError] = []

    def send(header, payload=b""):
        del header, payload
        raise BrokenPipeError("closed")

    queue = CommandQueue(send, run_id="write-error", on_error=errors.append)
    ticket = queue.enqueue("stop", fields={"requestedBy": "user"})
    with pytest.raises(CommandQueueError) as raised:
        ticket.wait(1.0)
    assert raised.value.code == "IPC_WRITE_FAILED"
    assert [error.code for error in errors] == ["IPC_WRITE_FAILED"]
    assert queue.close(1.0) is True
