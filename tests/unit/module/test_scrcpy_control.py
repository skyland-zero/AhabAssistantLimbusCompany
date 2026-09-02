from __future__ import annotations

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
