"""Regression tests for the Runner-side live preview producer.

Before this producer existed the sidecar quiesced its own capture before the
device lease and the lease rejected every sidecar device write, so the GPUI live
screen stayed empty for the whole run.  These tests pin the two halves of the
fix: the Runner publishes identity-tagged frames, and the adapter delivers them
on the GPUI ``screenshot.frame`` contract with the binary JPEG intact.
"""

from __future__ import annotations

import threading
import time

from PIL import Image

from module.execution.event_adapter import RunnerEventAdapter
from module.execution.task_preview import PREVIEW_INTERVAL, TaskPreview


def _image(size: tuple[int, int] = (320, 180)) -> Image.Image:
    return Image.new("RGB", size, (8, 16, 24))


def _collect() -> tuple[list[tuple[dict, bytes]], list[dict]]:
    frames: list[tuple[dict, bytes]] = []
    statuses: list[dict] = []
    return frames, statuses


def _preview(interval: float = 0.01) -> tuple[TaskPreview, list, list]:
    frames, statuses = _collect()
    preview = TaskPreview(
        run_id="run-1",
        device_id="pc:limbus",
        emit_frame=lambda payload, jpeg: frames.append((payload, jpeg)),
        emit_status=lambda payload: statuses.append(payload),
        capture=_image,
        interval=interval,
    )
    return preview, frames, statuses


def test_preview_publishes_identity_tagged_frames_until_stopped() -> None:
    preview, frames, statuses = _preview()
    first = threading.Event()

    def note_frame(payload: dict, jpeg: bytes) -> None:
        frames.append((payload, jpeg))
        first.set()

    preview._emit_frame = note_frame  # noqa: SLF001 - deliberate test seam

    assert preview.start() is True
    assert first.wait(2.0), "preview did not publish a frame"
    assert preview.running
    assert preview.start() is False, "a repeated start must be idempotent"

    assert preview.stop(deadline=time.monotonic() + 3.0) is True
    assert not preview.running

    # Every frame carries the identity GPUI uses to reject stale streams.
    for payload, jpeg in frames:
        assert payload["deviceId"] == "pc:limbus"
        assert payload["runId"] == "run-1"
        assert payload["generation"] == 1
        assert payload["width"] <= 540
        assert payload["height"] > 0
        assert isinstance(jpeg, bytes) and jpeg.startswith(b"\xff\xd8")

    assert [status["status"] for status in statuses][0] == "starting"
    assert "running" in [status["status"] for status in statuses]
    assert statuses[-1]["status"] == "stopped"


def test_disabled_preview_transmits_nothing_but_keeps_the_worker() -> None:
    captures: list[int] = []
    frames: list = []
    statuses: list = []

    def capture() -> Image.Image:
        captures.append(1)
        return _image()

    preview = TaskPreview(
        run_id="run-1",
        device_id="pc:limbus",
        emit_frame=lambda payload, jpeg: frames.append(payload),
        emit_status=lambda payload: statuses.append(payload),
        capture=capture,
        interval=0.01,
    )
    preview.set_enabled(False)
    assert preview.start() is True
    time.sleep(0.1)
    assert preview.running, "a disabled preview still owns a worker"
    assert frames == []
    assert captures == []
    assert statuses[-1]["status"] == "stopped"

    preview.set_enabled(True)
    deadline = time.monotonic() + 2.0
    while not frames and time.monotonic() < deadline:
        time.sleep(0.01)
    assert frames, "re-enabling must resume capture"
    preview.stop(deadline=time.monotonic() + 3.0)


def test_capture_failure_reports_error_once_and_keeps_running() -> None:
    frames: list = []
    statuses: list = []

    def capture() -> Image.Image:
        raise RuntimeError("device went away")

    preview = TaskPreview(
        run_id="run-1",
        device_id="pc:limbus",
        emit_frame=lambda payload, jpeg: frames.append(payload),
        emit_status=lambda payload: statuses.append(payload),
        capture=capture,
        interval=0.01,
    )
    preview.start()
    time.sleep(0.1)
    assert preview.running, "a capture failure must not kill the worker"
    errors = [status for status in statuses if status["status"] == "error"]
    assert len(errors) == 1, "error status is edge-triggered, not repeated"
    assert errors[0]["error"] == "device went away"
    preview.stop(deadline=time.monotonic() + 3.0)


def test_preview_defaults_match_the_sidecar_cadence() -> None:
    assert PREVIEW_INTERVAL == 2.0
    preview, _frames, _statuses = _preview()
    assert preview.enabled is True


def test_runner_preview_frame_becomes_the_gpui_screenshot_contract() -> None:
    received: list[tuple[str, dict]] = []
    adapter = RunnerEventAdapter(
        lambda name, payload: received.append((name, dict(payload))),
        run_id="run-1",
    )

    adapter.forward(
        {
            "type": "preview.frame",
            "runId": "run-1",
            "seq": 4,
            "deviceId": "pc:limbus",
            "instanceId": "pc:limbus",
            "generation": 2,
            "width": 540,
            "height": 304,
        },
        b"\xff\xd8\xff\xd9",
    )

    assert len(received) == 1
    name, payload = received[0]
    assert name == "screenshot.frame"
    # The image bytes must stay a separate binary field: the WebSocket layer
    # sends them as the frame body, not as base64 inside the JSON payload.
    assert payload["jpeg"] == b"\xff\xd8\xff\xd9"
    assert "payload" not in payload
    assert payload["deviceId"] == "pc:limbus"
    assert payload["generation"] == 2
    assert payload["width"] == 540


def test_runner_preview_status_forwards_the_sidecar_vocabulary() -> None:
    received: list[tuple[str, dict]] = []
    adapter = RunnerEventAdapter(
        lambda name, payload: received.append((name, dict(payload))),
        run_id="run-1",
    )

    adapter.forward(
        {
            "type": "preview.status",
            "runId": "run-1",
            "seq": 5,
            "deviceId": "pc:limbus",
            "generation": 2,
            "status": "running",
        }
    )

    assert received == [
        (
            "preview.status",
            {
                "runId": "run-1",
                "deviceId": "pc:limbus",
                "generation": 2,
                "status": "running",
            },
        )
    ]
