"""Bounded, single-writer command queue for one Runner IPC channel.

The sidecar may receive pause/stop/finish requests from different threads.  A
single writer owns command sequence allocation and physical pipe writes so
frames cannot interleave and command sequence numbers describe wire order.
Queue admission is deliberately non-blocking: a full queue reports a
structured ``IPC_BACKPRESSURE`` error instead of making an RPC handler wait on
an unhealthy child process.
"""

from __future__ import annotations

import heapq
import threading
from collections.abc import Callable, Mapping
from dataclasses import dataclass, field
from typing import Any

from .ipc_protocol import Sequence, make_command


class CommandQueueError(RuntimeError):
    """A command could not be admitted or written by the single writer."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(message)
        self.code = str(code)
        self.message = str(message)

    def as_dict(self) -> dict[str, str]:
        return {"code": self.code, "message": self.message}


class CommandQueueTicket:
    """Completion handle for one admitted command.

    Most control callers only need admission and never wait on the ticket.  The
    bootstrap handshake uses ``wait`` to ensure attached/start reached the wire
    before it permits an early stop command.
    """

    def __init__(self) -> None:
        self._done = threading.Event()
        self.sequence: int | None = None
        self.error: CommandQueueError | None = None

    @property
    def completed(self) -> bool:
        return self._done.is_set()

    def wait(self, timeout: float | None = None) -> int:
        if not self._done.wait(timeout):
            raise TimeoutError("Runner command writer did not complete in time")
        if self.error is not None:
            raise self.error
        if self.sequence is None:
            raise CommandQueueError("IPC_WRITE_FAILED", "Runner command completed without a sequence")
        return self.sequence

    def _succeed(self, sequence: int) -> None:
        self.sequence = int(sequence)
        self._done.set()

    def _fail(self, error: CommandQueueError) -> None:
        self.error = error
        self._done.set()


@dataclass(order=True, slots=True)
class _PendingCommand:
    priority: int
    order: int
    message_type: str = field(compare=False)
    fields: dict[str, Any] = field(compare=False)
    payload: bytes = field(compare=False)
    ticket: CommandQueueTicket = field(compare=False)


class CommandQueue:
    """A bounded priority queue drained by exactly one writer thread."""

    CRITICAL_PRIORITY = 0
    HANDSHAKE_PRIORITY = 10
    NORMAL_PRIORITY = 20
    CRITICAL_TYPES = frozenset({"stop", "finishAck"})
    HANDSHAKE_TYPES = frozenset({"attached", "start"})

    def __init__(
        self,
        send: Callable[[Mapping[str, Any], bytes], Any],
        *,
        run_id: str,
        sequence: Sequence | None = None,
        maxsize: int = 64,
        name: str | None = None,
        on_error: Callable[[CommandQueueError], Any] | None = None,
    ) -> None:
        if not callable(send):
            raise TypeError("send must be callable")
        if not isinstance(run_id, str) or not run_id:
            raise ValueError("run_id must be a non-empty string")
        if isinstance(maxsize, bool) or not isinstance(maxsize, int) or maxsize <= 0:
            raise ValueError("maxsize must be a positive integer")
        self.send = send
        self.run_id = run_id
        self.sequence = sequence or Sequence()
        self.maxsize = maxsize
        if on_error is not None and not callable(on_error):
            raise TypeError("on_error must be callable")
        self.on_error = on_error
        self._condition = threading.Condition(threading.RLock())
        self._items: list[_PendingCommand] = []
        self._next_order = 0
        self._pending_pause: _PendingCommand | None = None
        self._closed = False
        self._terminal_error: CommandQueueError | None = None
        self._thread = threading.Thread(target=self._run, name=name or f"runner-command-writer-{run_id}", daemon=True)
        self._thread.start()

    @property
    def thread(self) -> threading.Thread:
        return self._thread

    @property
    def closed(self) -> bool:
        with self._condition:
            return self._closed

    @property
    def terminal_error(self) -> CommandQueueError | None:
        with self._condition:
            return self._terminal_error

    def qsize(self) -> int:
        with self._condition:
            return len(self._items)

    def enqueue(
        self,
        message_type: str,
        *,
        fields: Mapping[str, Any] | None = None,
        payload: bytes = b"",
        priority: int | None = None,
    ) -> CommandQueueTicket:
        if not isinstance(message_type, str) or not message_type:
            raise ValueError("message_type must be a non-empty string")
        command_fields = dict(fields or {})
        if not isinstance(payload, (bytes, bytearray, memoryview)):
            raise TypeError("payload must be bytes-like")
        if not isinstance(payload, bytes):
            payload = bytes(payload)
        command_priority = self._priority(message_type) if priority is None else int(priority)
        with self._condition:
            if self._closed:
                raise self._closed_error_locked()
            # Pause/resume is state, not a log.  Keep exactly one queued
            # setPaused item and replace its value with the newest request.
            if message_type == "setPaused" and self._pending_pause is not None:
                previous = self._pending_pause
                ticket = CommandQueueTicket()
                previous.ticket._fail(CommandQueueError("IPC_COMMAND_SUPERSEDED", "setPaused was superseded"))
                previous.fields = command_fields
                previous.payload = payload
                previous.ticket = ticket
                return ticket
            if len(self._items) >= self.maxsize:
                raise CommandQueueError(
                    "IPC_BACKPRESSURE",
                    f"Runner command queue is full (capacity={self.maxsize})",
                )
            ticket = CommandQueueTicket()
            self._next_order += 1
            item = _PendingCommand(
                command_priority,
                self._next_order,
                message_type,
                command_fields,
                payload,
                ticket,
            )
            heapq.heappush(self._items, item)
            if message_type == "setPaused":
                self._pending_pause = item
            self._condition.notify()
            return ticket

    def close(self, timeout: float | None = 0.5) -> bool:
        """Reject queued work and stop the writer, returning thread liveness."""

        pending: list[_PendingCommand]
        with self._condition:
            if not self._closed:
                self._closed = True
                pending = list(self._items)
                self._items.clear()
                self._pending_pause = None
                self._condition.notify_all()
            else:
                pending = []
        error = CommandQueueError("IPC_CLOSED", "Runner command queue is closed")
        for item in pending:
            item.ticket._fail(error)
        if timeout is not None and threading.current_thread() is not self._thread:
            self._thread.join(max(0.0, float(timeout)))
        return not self._thread.is_alive()

    def _priority(self, message_type: str) -> int:
        if message_type in self.CRITICAL_TYPES:
            return self.CRITICAL_PRIORITY
        if message_type in self.HANDSHAKE_TYPES:
            return self.HANDSHAKE_PRIORITY
        return self.NORMAL_PRIORITY

    def _closed_error_locked(self) -> CommandQueueError:
        return self._terminal_error or CommandQueueError("IPC_CLOSED", "Runner command queue is closed")

    def _run(self) -> None:
        while True:
            with self._condition:
                while not self._items and not self._closed:
                    self._condition.wait()
                if self._closed and not self._items:
                    return
                item = heapq.heappop(self._items)
                if self._pending_pause is item:
                    self._pending_pause = None
            try:
                # next_for validates the complete header before committing the
                # sequence.  Once a command is selected, its sequence belongs to
                # the wire attempt even when the pipe fails part-way through.
                header = self.sequence.next_for(
                    lambda candidate: make_command(item.message_type, self.run_id, candidate, **item.fields)
                )
                self.send(header, item.payload)
            except BaseException as exc:
                error = exc if isinstance(exc, CommandQueueError) else CommandQueueError("IPC_WRITE_FAILED", str(exc))
                item.ticket._fail(error)
                self._stop_with_error(error)
                return
            item.ticket._succeed(header["commandSeq"])

    def _stop_with_error(self, error: CommandQueueError) -> None:
        callback: Callable[[CommandQueueError], Any] | None
        with self._condition:
            if self._terminal_error is not None:
                return
            self._terminal_error = error
            self._closed = True
            pending = list(self._items)
            self._items.clear()
            self._pending_pause = None
            callback = self.on_error
            self._condition.notify_all()
        for item in pending:
            item.ticket._fail(error)
        if callback is not None:
            try:
                callback(error)
            except BaseException:
                # Error reporting must not resurrect or strand the writer.  The
                # supervisor has its own fail-closed fallback when this callback
                # cannot run (for example during interpreter shutdown).
                pass


__all__ = ["CommandQueue", "CommandQueueError", "CommandQueueTicket"]
