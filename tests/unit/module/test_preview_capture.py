from __future__ import annotations

import io
import threading
import time

from PIL import Image

from module.preview_capture import PREVIEW_INTERVAL, PreviewCapture, encode_screenshot_frame


def test_preview_default_interval_is_two_frames_per_second() -> None:
    assert PREVIEW_INTERVAL == 0.5


def test_encode_preview_frame_limits_width_and_keeps_jpeg_payload() -> None:
    payload = encode_screenshot_frame(
        Image.new("RGB", (1280, 720), (32, 64, 96)),
        "pc:limbus",
    )

    assert payload["instanceId"] == "pc:limbus"
    assert payload["width"] == 720
    assert payload["height"] == 405
    decoded = Image.open(io.BytesIO(bytes(payload["jpeg"])))
    assert decoded.format == "JPEG"
    assert decoded.size == (720, 405)


def test_preview_capture_publishes_frames_until_stopped() -> None:
    events: list[tuple[str, dict]] = []
    first_frame = threading.Event()

    def emit(event: str, payload: dict) -> None:
        events.append((event, payload))
        if event == "screenshot.frame":
            first_frame.set()

    preview = PreviewCapture(
        emit,
        capture=lambda: Image.new("RGB", (320, 180), (8, 16, 24)),
        interval=0.05,
    )

    assert preview.start("pc:limbus") is True
    assert first_frame.wait(1.0)
    assert preview.running
    assert preview.start("pc:limbus") is False

    preview.stop()
    time.sleep(0.08)

    assert not preview.running
    event_names = [event for event, _ in events]
    assert event_names[0] == "preview.status"
    assert events[0][1]["status"] == "starting"
    assert "screenshot.frame" in event_names
    assert events[-1] == ("preview.status", {"deviceId": "pc:limbus", "status": "stopped"})
