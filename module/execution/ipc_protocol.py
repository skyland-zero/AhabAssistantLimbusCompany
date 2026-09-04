"""The binary protocol used by a sidecar and one task Runner.

The transport is deliberately implemented against the tiny ``read``/``write``
file-like interface exposed by anonymous pipes.  Pipes and sockets are allowed to
return short reads/writes, therefore neither the reader nor writer assumes that a
single call transfers a complete frame.

This module only imports the standard library.  It is safe to import from the
bootstrap before the application's business modules are loaded.
"""

from __future__ import annotations

import json
import struct
import threading
from collections.abc import Mapping
from dataclasses import dataclass
from typing import Any, BinaryIO

PROTOCOL_VERSION = 1
MAX_TOTAL_LENGTH = 8 * 1024 * 1024
MAX_HEADER_LENGTH = 64 * 1024
MAX_PREVIEW_BINARY_LENGTH = 4 * 1024 * 1024
MAX_U32 = (1 << 32) - 1

_U32 = struct.Struct("!I")

# These sets are part of the contract.  Keeping them in one place lets both the
# bootstrap and the sidecar reject a typo before it can alter execution state.
COMMAND_TYPES = frozenset({"attached", "start", "setPaused", "stop", "finishAck", "shutdown"})
EVENT_TYPES = frozenset(
    {
        "hello",
        "ready",
        "status",
        "task.started",
        "task.completed",
        "mirror.progress",
        "mirror.floor",
        "preview.frame",
        "preview.status",
        "log.entry",
        "warning",
        "hdr.warning",
        "config.delta",
        "resource.created",
        "resource.released",
        "afterCompletion.requested",
        "app.focusRequested",
        "heartbeat",
        "finished",
        "error",
    }
)
MESSAGE_TYPES = COMMAND_TYPES | EVENT_TYPES


class ProtocolError(ValueError):
    """A frame or protocol message is not valid for Runner protocol 1."""

    code = "RUNNER_PROTOCOL_ERROR"


class FrameTooLarge(ProtocolError):
    """A frame exceeds the protocol's bounded memory budget."""


class FrameTruncated(ProtocolError):
    """The peer closed a pipe in the middle of a frame."""


class FrameEOF(ProtocolError):
    """The peer closed the frame stream between frames."""


class FrameIOError(ProtocolError):
    """The underlying stream could not be read or written."""


@dataclass(frozen=True, slots=True)
class Frame:
    """One decoded protocol frame.

    ``header`` is never mutated by the codec.  ``payload`` is immutable bytes so
    callers cannot accidentally change the length after validation.
    """

    header: dict[str, Any]
    payload: bytes = b""

    @property
    def type(self) -> str | None:
        value = self.header.get("type")
        return value if isinstance(value, str) else None

    @property
    def run_id(self) -> str | None:
        value = self.header.get("runId")
        return value if isinstance(value, str) else None

    @property
    def binary_length(self) -> int:
        return len(self.payload)

    def as_message(self) -> dict[str, Any]:
        """Return a shallow copy suitable for tests and structured logging."""

        message = dict(self.header)
        if self.payload:
            message["payload"] = self.payload
        return message


def _positive_or_zero_int(value: Any, field: str, *, allow_zero: bool = True) -> int:
    # bool is an int subclass, but accepting ``True`` for a byte count or sequence
    # number is an easy way to create ambiguous wire messages.
    if isinstance(value, bool) or not isinstance(value, int):
        raise ProtocolError(f"{field} must be an integer")
    if value < 0 or (not allow_zero and value == 0):
        raise ProtocolError(f"{field} must be non-negative")
    if value > MAX_U32:
        raise ProtocolError(f"{field} overflows u32")
    return value


def _as_bytes(payload: bytes | bytearray | memoryview) -> bytes:
    if isinstance(payload, bytes):
        return payload
    if isinstance(payload, (bytearray, memoryview)):
        return bytes(payload)
    raise TypeError("payload must be bytes-like")


def _json_header(header: Mapping[str, Any]) -> bytes:
    if not isinstance(header, Mapping):
        raise ProtocolError("header must be a JSON object")
    # Convert to a regular dict first: this prevents a mutable custom Mapping from
    # changing between length calculation and serialization.
    copied = dict(header)
    try:
        encoded = json.dumps(
            copied,
            ensure_ascii=False,
            allow_nan=False,
            separators=(",", ":"),
            sort_keys=True,
        ).encode("utf-8")
    except (TypeError, ValueError, UnicodeEncodeError) as exc:
        raise ProtocolError(f"header is not valid JSON: {exc}") from exc
    if len(encoded) > MAX_HEADER_LENGTH:
        raise FrameTooLarge(f"header exceeds {MAX_HEADER_LENGTH} bytes")
    return encoded


def encode_frame(
    header: Mapping[str, Any],
    payload: bytes | bytearray | memoryview = b"",
    *,
    max_total_length: int = MAX_TOTAL_LENGTH,
) -> bytes:
    """Encode a frame using network byte order.

    ``binaryLength`` is filled in when omitted (handshake messages generally have
    no binary payload).  If the caller supplied it, it must exactly match the
    payload; this makes malformed frames impossible to create accidentally.
    """

    raw_payload = _as_bytes(payload)
    if isinstance(max_total_length, bool) or not isinstance(max_total_length, int) or max_total_length <= 0:
        raise ValueError("max_total_length must be positive")
    copied = dict(header) if isinstance(header, Mapping) else header
    if not isinstance(copied, dict):
        raise ProtocolError("header must be a JSON object")
    declared = copied.get("binaryLength", len(raw_payload))
    _positive_or_zero_int(declared, "binaryLength")
    if declared != len(raw_payload):
        raise ProtocolError(
            f"binaryLength={declared} does not match payload length {len(raw_payload)}"
        )
    if copied.get("type") == "preview.frame" and len(raw_payload) > MAX_PREVIEW_BINARY_LENGTH:
        raise FrameTooLarge(f"preview payload exceeds {MAX_PREVIEW_BINARY_LENGTH} bytes")
    # Keep the wire representation explicit even for hello/commands which do not
    # mention the field in their prose examples.
    copied["binaryLength"] = declared
    header_bytes = _json_header(copied)
    total_length = 4 + len(header_bytes) + len(raw_payload)
    if total_length > max_total_length or total_length > MAX_TOTAL_LENGTH:
        raise FrameTooLarge(f"frame exceeds {min(max_total_length, MAX_TOTAL_LENGTH)} bytes")
    if total_length > MAX_U32:
        raise FrameTooLarge("frame length overflows u32")
    return _U32.pack(total_length) + _U32.pack(len(header_bytes)) + header_bytes + raw_payload


def _decode_body(total_length: int, body: bytes) -> Frame:
    if total_length < 4:
        raise ProtocolError("totalLength must include headerLength")
    if len(body) != total_length:
        raise FrameTruncated("frame body is shorter than totalLength")
    header_length = _U32.unpack(body[:4])[0]
    if header_length > MAX_HEADER_LENGTH:
        raise FrameTooLarge(f"header exceeds {MAX_HEADER_LENGTH} bytes")
    if header_length > total_length - 4:
        raise ProtocolError("headerLength exceeds frame body")
    header_bytes = body[4 : 4 + header_length]
    payload = body[4 + header_length :]
    try:
        text = header_bytes.decode("utf-8", errors="strict")
    except UnicodeDecodeError as exc:
        raise ProtocolError("header is not valid UTF-8") from exc
    try:
        header = json.loads(text)
    except json.JSONDecodeError as exc:
        raise ProtocolError("header is not valid JSON") from exc
    if not isinstance(header, dict):
        raise ProtocolError("header JSON must be an object")
    declared = header.get("binaryLength", len(payload))
    _positive_or_zero_int(declared, "binaryLength")
    if declared != len(payload):
        raise ProtocolError(
            f"binaryLength={declared} does not match payload length {len(payload)}"
        )
    if header.get("type") == "preview.frame" and len(payload) > MAX_PREVIEW_BINARY_LENGTH:
        raise FrameTooLarge(f"preview payload exceeds {MAX_PREVIEW_BINARY_LENGTH} bytes")
    # A non-preview payload still cannot make totalLength exceed the global bound;
    # no separate arbitrary limit is needed here.
    return Frame(dict(header), bytes(payload))


def decode_frame(
    data: bytes | bytearray | memoryview,
    *,
    allow_trailing: bool = False,
    max_total_length: int = MAX_TOTAL_LENGTH,
) -> Frame:
    """Decode one complete frame from a bytes object."""

    raw = _as_bytes(data)
    if len(raw) < 4:
        raise FrameTruncated("missing totalLength")
    total_length = _U32.unpack(raw[:4])[0]
    if isinstance(max_total_length, bool) or not isinstance(max_total_length, int) or max_total_length <= 0:
        raise ValueError("max_total_length must be positive")
    if total_length > min(max_total_length, MAX_TOTAL_LENGTH):
        raise FrameTooLarge(f"frame exceeds {min(max_total_length, MAX_TOTAL_LENGTH)} bytes")
    frame_size = 4 + total_length
    if len(raw) < frame_size:
        raise FrameTruncated("frame body is truncated")
    if not allow_trailing and len(raw) != frame_size:
        raise ProtocolError("trailing bytes after frame")
    return _decode_body(total_length, raw[4:frame_size])


def read_exact(stream: BinaryIO, size: int) -> bytes:
    """Read exactly ``size`` bytes, tolerating arbitrary short reads."""

    if size < 0:
        raise ValueError("size must be non-negative")
    chunks: list[bytes] = []
    remaining = size
    while remaining:
        try:
            chunk = stream.read(remaining)
        except (OSError, ValueError) as exc:
            raise FrameIOError(f"read failed: {exc}") from exc
        if chunk is None:
            raise FrameIOError("stream returned None from read")
        if not isinstance(chunk, (bytes, bytearray, memoryview)):
            raise FrameIOError("stream returned a non-bytes value")
        chunk = bytes(chunk)
        if len(chunk) > remaining:
            raise FrameIOError("stream returned more bytes than requested")
        if not chunk:
            raise FrameTruncated(f"expected {remaining} more bytes")
        chunks.append(chunk)
        remaining -= len(chunk)
    return b"".join(chunks)


def write_all(stream: BinaryIO, data: bytes | bytearray | memoryview) -> None:
    """Write every byte, tolerating arbitrary short writes."""

    raw = _as_bytes(data)
    offset = 0
    while offset < len(raw):
        try:
            written = stream.write(raw[offset:])
        except (OSError, ValueError) as exc:
            raise FrameIOError(f"write failed: {exc}") from exc
        # A few file-like adapters return None while consuming the complete
        # buffer.  Treat that convention as a full write; a zero integer is never
        # progress and is therefore an error.
        if written is None:
            written = len(raw) - offset
        if isinstance(written, bool) or not isinstance(written, int):
            raise FrameIOError("stream returned a non-integer write count")
        if written <= 0 or written > len(raw) - offset:
            raise FrameIOError("stream made no progress while writing")
        offset += written
    flush = getattr(stream, "flush", None)
    if callable(flush):
        try:
            flush()
        except (OSError, ValueError) as exc:
            raise FrameIOError(f"flush failed: {exc}") from exc


def read_frame(stream: BinaryIO, *, max_total_length: int = MAX_TOTAL_LENGTH) -> Frame | None:
    """Read a frame from a stream.

    ``None`` denotes clean EOF between frames.  EOF after one to three length
    bytes, or after a declared body, is a ``FrameTruncated`` protocol error.
    """

    if isinstance(max_total_length, bool) or not isinstance(max_total_length, int) or max_total_length <= 0:
        raise ValueError("max_total_length must be positive")
    limit = min(max_total_length, MAX_TOTAL_LENGTH)
    try:
        prefix = stream.read(4)
    except (OSError, ValueError) as exc:
        raise FrameIOError(f"read failed: {exc}") from exc
    if prefix is None:
        raise FrameIOError("stream returned None from read")
    prefix = bytes(prefix)
    if not prefix:
        return None
    # A pipe is allowed to return fewer than four bytes even when the peer is
    # healthy.  Complete the length prefix before treating EOF as truncation.
    if len(prefix) < 4:
        try:
            prefix += read_exact(stream, 4 - len(prefix))
        except FrameTruncated as exc:
            raise FrameTruncated("totalLength is truncated") from exc
    elif len(prefix) > 4:
        raise FrameIOError("stream returned more bytes than requested")
    total_length = _U32.unpack(prefix)[0]
    if total_length > limit:
        raise FrameTooLarge(f"frame exceeds {limit} bytes")
    body = read_exact(stream, total_length)
    return _decode_body(total_length, body)


def write_frame(
    stream: BinaryIO,
    header: Mapping[str, Any],
    payload: bytes | bytearray | memoryview = b"",
    *,
    lock: threading.Lock | threading.RLock | None = None,
) -> None:
    """Encode and write one frame atomically with respect to ``lock``."""

    encoded = encode_frame(header, payload)
    if lock is None:
        write_all(stream, encoded)
    else:
        with lock:
            write_all(stream, encoded)


def validate_message(
    header: Mapping[str, Any],
    *,
    direction: str | None = None,
    expected_run_id: str | None = None,
    expected_protocol: int = PROTOCOL_VERSION,
) -> dict[str, Any]:
    """Validate a decoded message header and return a defensive copy.

    ``direction`` can be ``"command"`` or ``"event"``.  Omitting it is useful
    for the hello frame, while still rejecting unknown message types.
    """

    if not isinstance(header, Mapping):
        raise ProtocolError("message header must be a JSON object")
    message = dict(header)
    message_type = message.get("type")
    if not isinstance(message_type, str) or message_type not in MESSAGE_TYPES:
        raise ProtocolError(f"unknown message type: {message_type!r}")
    protocol = message.get("protocol")
    if isinstance(protocol, bool) or not isinstance(protocol, int) or protocol != expected_protocol:
        raise ProtocolError(f"unsupported protocol: {protocol!r}")
    run_id = message.get("runId")
    if not isinstance(run_id, str) or not run_id:
        raise ProtocolError("runId must be a non-empty string")
    if expected_run_id is not None and run_id != expected_run_id:
        raise ProtocolError("runId does not match this Runner")
    if direction == "command" and message_type not in COMMAND_TYPES:
        raise ProtocolError(f"{message_type} is not a command")
    if direction == "event" and message_type not in EVENT_TYPES:
        raise ProtocolError(f"{message_type} is not an event")

    if message_type == "hello":
        pid = message.get("pid")
        if isinstance(pid, bool) or not isinstance(pid, int) or pid <= 0:
            raise ProtocolError("hello.pid must be a positive integer")
    elif message_type in COMMAND_TYPES:
        command_seq = _positive_or_zero_int(message.get("commandSeq"), "commandSeq", allow_zero=False)
        if command_seq <= 0:
            raise ProtocolError("commandSeq must be positive")
        if message_type == "start":
            if not isinstance(message.get("spec"), Mapping):
                raise ProtocolError("start.spec must be an object")
        elif message_type == "setPaused" and not isinstance(message.get("paused"), bool):
            raise ProtocolError("setPaused.paused must be boolean")
        elif message_type == "stop":
            requested_by = message.get("requestedBy")
            if requested_by not in {"user", "shutdown", "watchdog"}:
                raise ProtocolError("stop.requestedBy is invalid")
        elif message_type == "finishAck":
            _positive_or_zero_int(message.get("finalSeq"), "finalSeq")
    else:
        if message_type != "hello":
            seq = _positive_or_zero_int(message.get("seq"), "seq", allow_zero=False)
            if seq <= 0:
                raise ProtocolError("event seq must be positive")
            if "binaryLength" not in message:
                raise ProtocolError("event requires binaryLength")
        if "binaryLength" in message:
            _positive_or_zero_int(message["binaryLength"], "binaryLength")
        if message_type == "status" and message.get("status") not in {"running", "paused", "stopping"}:
            raise ProtocolError("status.status is invalid")
        if message_type == "finished":
            if message.get("outcome") not in {"completed", "stopped", "failed", "crashed"}:
                raise ProtocolError("finished.outcome is invalid")
            if not isinstance(message.get("forced", False), bool):
                raise ProtocolError("finished.forced must be boolean")
            if message.get("deviceDisposition", "restore") not in {
                "restore",
                "game_closed",
                "emulator_closed",
            }:
                raise ProtocolError("finished.deviceDisposition is invalid")
        if message_type == "preview.frame":
            binary_length = message.get("binaryLength")
            if binary_length is None:
                raise ProtocolError("preview.frame requires binaryLength")
            if binary_length > MAX_PREVIEW_BINARY_LENGTH:
                raise FrameTooLarge("preview.frame payload exceeds limit")
    return message


def make_command(message_type: str, run_id: str, command_seq: int, **fields: Any) -> dict[str, Any]:
    """Build and validate a command header."""

    header = {
        "type": message_type,
        "protocol": PROTOCOL_VERSION,
        "runId": run_id,
        "commandSeq": command_seq,
        **fields,
    }
    return validate_message(header, direction="command", expected_run_id=run_id)


def make_event(message_type: str, run_id: str, seq: int, **fields: Any) -> dict[str, Any]:
    """Build and validate an event header."""

    header = {
        "type": message_type,
        "protocol": PROTOCOL_VERSION,
        "runId": run_id,
        "seq": seq,
        "binaryLength": 0,
        **fields,
    }
    return validate_message(header, direction="event", expected_run_id=run_id)


class Sequence:
    """Thread-safe positive sequence allocator.

    Event writers should call ``reserve`` only after preparing the complete event
    header.  A failed validation does not consume a sequence number when callers
    use ``next_for`` below.
    """

    def __init__(self, initial: int = 0) -> None:
        _positive_or_zero_int(initial, "initial")
        self._value = initial
        self._lock = threading.Lock()

    @property
    def value(self) -> int:
        with self._lock:
            return self._value

    def next(self) -> int:
        with self._lock:
            if self._value >= MAX_U32:
                raise ProtocolError("sequence overflows u32")
            self._value += 1
            return self._value

    def next_for(self, builder: Any) -> Any:
        """Build a value with the next number, committing only on success.

        ``builder`` receives the candidate sequence.  This small helper is useful
        for event writers: malformed low-priority events do not consume ``seq``.
        """

        with self._lock:
            if self._value >= MAX_U32:
                raise ProtocolError("sequence overflows u32")
            candidate = self._value + 1
            result = builder(candidate)
            self._value = candidate
            return result


class FrameCodec:
    """Object-oriented facade retained for callers that prefer a codec instance."""

    def __init__(self, *, max_total_length: int = MAX_TOTAL_LENGTH) -> None:
        if isinstance(max_total_length, bool) or not isinstance(max_total_length, int) or max_total_length <= 0:
            raise ValueError("max_total_length must be positive")
        self.max_total_length = min(max_total_length, MAX_TOTAL_LENGTH)

    def encode(self, header: Mapping[str, Any], payload: bytes | bytearray | memoryview = b"") -> bytes:
        return encode_frame(header, payload, max_total_length=self.max_total_length)

    def decode(self, data: bytes | bytearray | memoryview, *, allow_trailing: bool = False) -> Frame:
        return decode_frame(data, allow_trailing=allow_trailing, max_total_length=self.max_total_length)

    def read(self, stream: BinaryIO) -> Frame | None:
        return read_frame(stream, max_total_length=self.max_total_length)

    def write(
        self,
        stream: BinaryIO,
        header: Mapping[str, Any],
        payload: bytes | bytearray | memoryview = b"",
        *,
        lock: threading.Lock | threading.RLock | None = None,
    ) -> None:
        encoded = self.encode(header, payload)
        if lock is None:
            write_all(stream, encoded)
        else:
            with lock:
                write_all(stream, encoded)


class FrameWriter:
    """Single-writer helper for command or event pipes.

    ``send_event`` allocates ``seq`` while holding the writer lock and only after
    the frame has been serialized.  Thus two producer threads cannot interleave a
    frame or consume sequence numbers for events rejected by validation.
    """

    def __init__(
        self,
        stream: BinaryIO,
        *,
        run_id: str,
        codec: FrameCodec | None = None,
        event_sequence: Sequence | None = None,
        command_sequence: Sequence | None = None,
    ) -> None:
        self.stream = stream
        self.run_id = run_id
        self.codec = codec or FrameCodec()
        self.event_sequence = event_sequence or Sequence()
        self.command_sequence = command_sequence or Sequence()
        self._lock = threading.Lock()
        self._closed = False

    def close(self) -> None:
        with self._lock:
            self._closed = True

    def send(self, header: Mapping[str, Any], payload: bytes | bytearray | memoryview = b"") -> None:
        with self._lock:
            if self._closed:
                raise FrameIOError("frame writer is closed")
            encoded = self.codec.encode(header, payload)
            write_all(self.stream, encoded)

    def send_command(self, message_type: str, **fields: Any) -> int:
        with self._lock:
            if self._closed:
                raise FrameIOError("frame writer is closed")
            sequence = self.command_sequence.value + 1
            header = make_command(message_type, self.run_id, sequence, **fields)
            encoded = self.codec.encode(header)
            # Commit only after serialization succeeds.  The actual write is still
            # protected by the same lock and the command sequence is then visible
            # to the next command.
            self.command_sequence.next()
            write_all(self.stream, encoded)
            return sequence

    def send_event(self, message_type: str, payload: bytes = b"", **fields: Any) -> int:
        with self._lock:
            if self._closed:
                raise FrameIOError("frame writer is closed")

            fields = dict(fields)
            declared = fields.pop("binaryLength", len(payload))
            if declared != len(payload):
                raise ProtocolError("binaryLength does not match event payload")

            def build(candidate: int) -> bytes:
                header = make_event(
                    message_type,
                    self.run_id,
                    candidate,
                    binaryLength=len(payload),
                    **fields,
                )
                return self.codec.encode(header, payload)

            encoded = self.event_sequence.next_for(build)
            write_all(self.stream, encoded)
            return self.event_sequence.value


# Backwards-compatible, discoverable aliases for tests and integration code.
EventSequence = Sequence
CommandSequence = Sequence


__all__ = [
    "COMMAND_TYPES",
    "EVENT_TYPES",
    "Frame",
    "FrameCodec",
    "FrameEOF",
    "FrameIOError",
    "FrameTooLarge",
    "FrameTruncated",
    "MAX_HEADER_LENGTH",
    "MAX_PREVIEW_BINARY_LENGTH",
    "MAX_TOTAL_LENGTH",
    "PROTOCOL_VERSION",
    "ProtocolError",
    "Sequence",
    "CommandSequence",
    "EventSequence",
    "FrameWriter",
    "decode_frame",
    "encode_frame",
    "make_command",
    "make_event",
    "read_exact",
    "read_frame",
    "validate_message",
    "write_all",
    "write_frame",
]
