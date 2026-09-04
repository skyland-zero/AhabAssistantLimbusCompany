from __future__ import annotations

import socket
import struct
import threading
import time
from unittest.mock import Mock

import numpy as np

from module.automation.input_handlers import AbstractInput
from module.automation.input_handlers.simulator.scrcpy_control import (
    SCRCPY_MAX_PACKET_SIZE,
    SCRCPY_PACKET_FLAG_CONFIG,
    SCRCPY_PACKET_FLAG_KEY_FRAME,
    ScrcpyControl,
    _DecodedFrame,
)


def test_scrcpy_decoder_requests_rgb_frames() -> None:
    frame = Mock()
    expected = np.array([[[255, 0, 0], [0, 0, 255]]], dtype=np.uint8)
    frame.to_ndarray.return_value = expected

    # The decoder's format choice is the color-space boundary. Keep this test
    # close to the controller contract so a future bgr24 regression is visible.
    actual = ScrcpyControl._frame_to_rgb(frame)

    assert np.array_equal(actual, expected)
    frame.to_ndarray.assert_called_once_with(format="rgb24")


def test_scrcpy_v41_server_command_keeps_resolution_and_uses_configured_30_fps_and_8mbps() -> None:
    command = ScrcpyControl._build_server_shell_command("/data/local/tmp/scrcpy-server.jar")

    assert "Server 4.1" in command
    assert "video_codec=h264" in command
    assert "max_size=0" in command
    assert "max_fps=30" in command
    assert "video_bit_rate=8000000" in command
    assert "send_stream_meta=true" in command
    assert "downsize_on_error=false" in command


def test_scrcpy_v41_session_metadata_updates_resolution() -> None:
    header = struct.pack(">III", 0x80000001, 1920, 1080)

    assert ScrcpyControl._parse_session_meta(header) == (1920, 1080)


def test_scrcpy_v41_regular_packet_header_is_not_session_metadata() -> None:
    header = struct.pack(">QI", 123456, 42)

    assert ScrcpyControl._parse_session_meta(header) is None


def test_scrcpy_packet_header_extracts_config_keyframe_and_pts_flags() -> None:
    header = struct.pack(">QI", SCRCPY_PACKET_FLAG_KEY_FRAME | 987654, 42)

    assert ScrcpyControl._parse_packet_header(header) == (987654, False, True, 42)

    config_header = struct.pack(">QI", SCRCPY_PACKET_FLAG_CONFIG, 12)
    assert ScrcpyControl._parse_packet_header(config_header) == (None, True, False, 12)


def test_scrcpy_packet_header_rejects_zero_and_oversized_payloads() -> None:
    for size in (0, SCRCPY_MAX_PACKET_SIZE + 1):
        header = struct.pack(">QI", 1, size)
        try:
            ScrcpyControl._parse_packet_header(header)
        except RuntimeError:
            pass
        else:
            raise AssertionError(f"packet size {size} should be rejected")


def test_scrcpy_recv_buffer_survives_timeout_mid_payload() -> None:
    class PartialSocket:
        def __init__(self) -> None:
            self.calls = 0

        def recv(self, _size: int) -> bytes:
            self.calls += 1
            if self.calls == 1:
                return b"ab"
            if self.calls == 2:
                raise socket.timeout()
            return b"cd"

    control = object.__new__(ScrcpyControl)
    control._video_socket = PartialSocket()
    control._recv_buffer = bytearray()
    control._running = True
    control._metrics_lock = threading.Lock()
    control._metrics = {"packet_timeouts": 0}

    assert control._read_exact(4) == b"abcd"
    assert control._metrics["packet_timeouts"] == 1


def test_scrcpy_frame_conversion_is_cached_per_sequence_and_mode() -> None:
    control = object.__new__(ScrcpyControl)
    control._metrics_lock = threading.Lock()
    control._metrics = {"gray_convert_time_ns": 0, "rgb_convert_time_ns": 0}
    video_frame = Mock()
    gray = np.array([[1, 2]], dtype=np.uint8)
    rgb = np.array([[[1, 2, 3], [4, 5, 6]]], dtype=np.uint8)
    decoded = _DecodedFrame(7, 123, 1.0, video_frame)

    control._frame_to_luma = Mock(return_value=gray)
    control._frame_to_rgb = Mock(return_value=rgb)

    assert control._frame_to_snapshot(decoded, "luma").image is gray
    assert control._frame_to_snapshot(decoded, "luma").image is gray
    assert control._frame_to_snapshot(decoded, "rgb").image is rgb
    assert control._frame_to_snapshot(decoded, "rgb").image is rgb
    control._frame_to_luma.assert_called_once_with(video_frame)
    control._frame_to_rgb.assert_called_once_with(video_frame)


def test_scrcpy_swipe_points_are_bounded_and_keep_endpoints() -> None:
    points = [(index, index) for index in range(200)]

    limited = ScrcpyControl._limit_swipe_points(points, duration=2.0)

    assert len(limited) <= 48
    assert limited[0] == points[0]
    assert limited[-1] == points[-1]


def test_scrcpy_start_command_is_argv_and_rejects_injection() -> None:
    command = ScrcpyControl._build_server_command_args(
        "/data/local/tmp/scrcpy-server.jar",
        scid="0x12",
        socket_name="scrcpy_00000012",
    )

    assert isinstance(command, list)
    assert command[0] == "CLASSPATH=/data/local/tmp/scrcpy-server.jar"
    assert "scid=0x12" in command
    assert all(isinstance(argument, str) for argument in command)

    for remote_jar in (
        "/data/local/tmp/scrcpy-server.jar;id",
        "/data/local/tmp/$(id).jar",
        "/data/local/tmp/../other.jar",
    ):
        try:
            ScrcpyControl._build_server_command_args(remote_jar)
        except ValueError:
            pass
        else:
            raise AssertionError(f"unsafe remote jar {remote_jar!r} was accepted")

    for serial in ("device;id", "device name", "device&whoami"):
        try:
            ScrcpyControl._validate_adb_serial(serial)
        except ValueError:
            pass
        else:
            raise AssertionError(f"unsafe ADB serial {serial!r} was accepted")


def test_scrcpy_runner_attach_rejects_an_invalid_serial() -> None:
    try:
        ScrcpyControl._validate_adb_serial("127.0.0.1:5555;id")
    except ValueError:
        pass
    else:
        raise AssertionError("ADB serial injection was accepted")


def test_scrcpy_first_frame_wait_is_woken_by_stop_request() -> None:
    control = object.__new__(ScrcpyControl)
    AbstractInput.__init__(control)
    control._first_frame_ready = threading.Event()
    result: list[bool] = []

    worker = threading.Thread(target=lambda: result.append(control._wait_for_first_frame(30)))
    worker.start()
    time.sleep(0.02)
    control.request_stop()
    worker.join(timeout=1)

    assert not worker.is_alive()
    assert result == [False]


class _FakeControlSocket:
    def __init__(self) -> None:
        self.messages: list[bytes] = []

    def sendall(self, payload: bytes) -> None:
        self.messages.append(payload)


def test_scrcpy_drag_releases_touch_when_wait_is_cancelled(monkeypatch) -> None:
    control = object.__new__(ScrcpyControl)
    AbstractInput.__init__(control)
    control._control_socket = _FakeControlSocket()
    control._control_lock = threading.Lock()
    control._running = True
    control.resolution = (100, 100)
    monkeypatch.setattr(control, "interruptible_sleep", lambda _seconds: False)

    control.mouse_drag(10, 20, dx=5, dy=5)

    actions = [message[1] for message in control._control_socket.messages]
    assert actions == [0, 1]


def test_scrcpy_key_press_releases_key_when_wait_is_cancelled(monkeypatch) -> None:
    control = object.__new__(ScrcpyControl)
    AbstractInput.__init__(control)
    control._control_socket = _FakeControlSocket()
    control._control_lock = threading.Lock()
    control._running = True
    control.resolution = (100, 100)
    monkeypatch.setattr(control, "interruptible_sleep", lambda _seconds: False)

    control.key_press("a")

    actions = [message[1] for message in control._control_socket.messages]
    assert actions == [0, 1]
