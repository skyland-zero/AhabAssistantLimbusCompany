from __future__ import annotations

import struct
from unittest.mock import Mock

import numpy as np

from module.automation.input_handlers.simulator.scrcpy_control import ScrcpyControl


def test_scrcpy_decoder_requests_rgb_frames() -> None:
    frame = Mock()
    expected = np.array([[[255, 0, 0], [0, 0, 255]]], dtype=np.uint8)
    frame.to_ndarray.return_value = expected

    # The decoder's format choice is the color-space boundary. Keep this test
    # close to the controller contract so a future bgr24 regression is visible.
    actual = ScrcpyControl._frame_to_rgb(frame)

    assert np.array_equal(actual, expected)
    frame.to_ndarray.assert_called_once_with(format="rgb24")


def test_scrcpy_v41_server_command_uses_exact_size_30_fps_and_30mbps() -> None:
    command = ScrcpyControl._build_server_shell_command("/data/local/tmp/scrcpy-server.jar")

    assert "Server 4.1" in command
    assert "video_codec=h264" in command
    assert "max_size=0" in command
    assert "max_fps=15" in command
    assert "video_bit_rate=8000000" in command
    assert "send_stream_meta=true" in command
    assert "downsize_on_error=false" in command


def test_scrcpy_v41_session_metadata_updates_resolution() -> None:
    header = struct.pack(">III", 0x80000001, 1920, 1080)

    assert ScrcpyControl._parse_session_meta(header) == (1920, 1080)


def test_scrcpy_v41_regular_packet_header_is_not_session_metadata() -> None:
    header = struct.pack(">QI", 123456, 42)

    assert ScrcpyControl._parse_session_meta(header) is None
