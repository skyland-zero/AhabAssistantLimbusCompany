from __future__ import annotations

import io

import pytest

from module.execution.ipc_protocol import (
    FrameCodec,
    FrameTooLarge,
    FrameTruncated,
    FrameWriter,
    ProtocolError,
    Sequence,
    decode_frame,
    encode_frame,
    make_event,
    read_frame,
)


class ShortReader:
    def __init__(self, data: bytes, chunk: int = 1) -> None:
        self.stream = io.BytesIO(data)
        self.chunk = chunk

    def read(self, size: int) -> bytes:
        return self.stream.read(min(size, self.chunk))


class ShortWriter:
    def __init__(self, chunk: int = 1) -> None:
        self.data = bytearray()
        self.chunk = chunk

    def write(self, data: bytes) -> int:
        count = min(len(data), self.chunk)
        self.data.extend(data[:count])
        return count

    def flush(self) -> None:
        return


def test_frame_codec_handles_short_reads_and_writes() -> None:
    header = {"type": "preview.frame", "protocol": 1, "runId": "run", "seq": 1}
    encoded = encode_frame(header, b"jpeg")
    writer = ShortWriter()
    FrameCodec().write(writer, header, b"jpeg")
    assert bytes(writer.data) == encoded
    decoded = read_frame(ShortReader(encoded))
    assert decoded is not None
    assert decoded.header["type"] == "preview.frame"
    assert decoded.payload == b"jpeg"


def test_frame_codec_rejects_mismatch_and_truncation() -> None:
    with pytest.raises(ProtocolError):
        encode_frame({"type": "hello", "protocol": 1, "runId": "run", "pid": 1, "binaryLength": 2}, b"x")
    encoded = encode_frame({"type": "hello", "protocol": 1, "runId": "run", "pid": 1}, b"")
    with pytest.raises(FrameTruncated):
        decode_frame(encoded[:-1])


def test_protocol_limits_and_seq_are_bounded() -> None:
    with pytest.raises(FrameTooLarge):
        encode_frame({"type": "hello", "protocol": 1, "runId": "run", "pid": 1}, b"x" * (8 * 1024 * 1024))
    sequence = Sequence((1 << 32) - 1)
    with pytest.raises(ProtocolError):
        sequence.next()
    assert make_event("heartbeat", "run", 1)["seq"] == 1


def test_event_writer_does_not_consume_seq_when_validation_fails() -> None:
    writer = ShortWriter(10000)
    event_writer = FrameWriter(writer, run_id="run")
    with pytest.raises(ProtocolError):
        event_writer.send_event("preview.frame", payload=b"x", binaryLength=2)
    assert event_writer.event_sequence.value == 0
    assert event_writer.send_event("heartbeat") == 1


def test_codec_read_honours_instance_frame_limit() -> None:
    header = {"type": "preview.status", "protocol": 1, "runId": "run", "seq": 1}
    encoded = encode_frame(header, b"x" * 16)
    with pytest.raises(FrameTooLarge):
        FrameCodec(max_total_length=16).read(io.BytesIO(encoded))
