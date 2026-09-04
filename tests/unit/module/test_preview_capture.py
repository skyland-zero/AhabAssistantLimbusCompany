from __future__ import annotations

import io
import threading
import time

from PIL import Image

from module.preview_capture import PREVIEW_INTERVAL, PreviewCapture, encode_screenshot_frame


def test_preview_default_interval_is_about_two_seconds() -> None:
    assert PREVIEW_INTERVAL == 2.0


def test_encode_preview_frame_limits_width_and_keeps_jpeg_payload() -> None:
    payload = encode_screenshot_frame(
        Image.new("RGB", (1280, 720), (32, 64, 96)),
        "pc:limbus",
    )

    assert payload["instanceId"] == "pc:limbus"
    assert payload["width"] == 540
    assert payload["height"] == 304
    decoded = Image.open(io.BytesIO(bytes(payload["jpeg"])))
    assert decoded.format == "JPEG"
    assert decoded.size == (540, 304)


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
    assert events[0][1]["deviceId"] == "pc:limbus"
    assert events[0][1]["runId"] is None
    assert events[0][1]["generation"] == 1
    assert "screenshot.frame" in event_names
    frame = next(payload for event, payload in events if event == "screenshot.frame")
    assert frame["deviceId"] == "pc:limbus"
    assert frame["runId"] is None
    assert frame["generation"] == 1
    assert events[-1][0] == "preview.status"
    assert events[-1][1] == {
        "deviceId": "pc:limbus",
        "runId": None,
        "generation": 1,
        "status": "stopped",
    }


def test_preview_replaces_old_run_only_after_stopping_it() -> None:
    events: list[tuple[str, dict]] = []
    first_capture_started = threading.Event()
    release_first_capture = threading.Event()
    capture_count = 0

    def capture() -> Image.Image:
        nonlocal capture_count
        capture_count += 1
        if capture_count == 1:
            first_capture_started.set()
            release_first_capture.wait(1.0)
        return Image.new("RGB", (32, 18), (8, 16, 24))

    preview = PreviewCapture(lambda event, payload: events.append((event, payload)), capture=capture, interval=0.05)
    assert preview.start("pc:limbus", run_id="old-run")
    assert first_capture_started.wait(1.0)

    # The old worker is still inside capture.  start() must wait for its
    # proven stop before publishing the new run's starting status.
    old_stop_event = preview._stop_event
    assert old_stop_event is not None
    replacement_result: list[bool] = []
    replacement = threading.Thread(
        target=lambda: replacement_result.append(preview.start("pc:limbus", run_id="new-run")),
        daemon=True,
    )
    replacement.start()
    assert old_stop_event.wait(1.0)
    release_first_capture.set()
    replacement.join(1.0)
    assert not replacement.is_alive()
    assert replacement_result == [True]
    assert preview.run_id == "new-run"
    preview.stop()

    statuses = [payload for event, payload in events if event == "preview.status"]
    lifecycle_statuses = [payload for payload in statuses if payload["status"] in {"starting", "stopped"}]
    assert [payload["status"] for payload in lifecycle_statuses] == ["starting", "stopped", "starting", "stopped"]
    assert lifecycle_statuses[0]["runId"] == "old-run"
    assert lifecycle_statuses[1]["runId"] == "old-run"
    assert lifecycle_statuses[2]["runId"] == "new-run"
    assert lifecycle_statuses[3]["runId"] == "new-run"
    assert lifecycle_statuses[1]["generation"] < lifecycle_statuses[2]["generation"]


def test_preview_discards_late_old_run_frame_and_status_by_generation() -> None:
    events: list[tuple[str, dict]] = []
    preview = PreviewCapture(
        lambda event, payload: events.append((event, payload)),
        capture=lambda: Image.new("RGB", (32, 18), (8, 16, 24)),
        interval=0.05,
    )
    assert preview.start("pc:limbus", run_id="old-run")
    old_generation = preview.generation
    old_stop_event = preview._stop_event
    assert old_stop_event is not None
    old_thread = preview._thread
    assert old_thread is not None

    assert preview.stop_and_wait(run_id="old-run", generation=old_generation)
    assert preview.start("pc:limbus", run_id="new-run")
    new_generation = preview.generation

    stale_frame = {"deviceId": "pc:limbus", "runId": "old-run", "generation": old_generation}
    assert not preview._emit_if_current_worker(
        "screenshot.frame",
        stale_frame,
        stop_event=old_stop_event,
        run_id="old-run",
        generation=old_generation,
    )
    assert not preview._emit_status_if_current(
        "pc:limbus",
        "running",
        stop_event=old_stop_event,
        run_id="old-run",
        generation=old_generation,
    )

    preview.stop()
    assert old_thread is not preview._thread
    assert new_generation > old_generation
    new_start_index = next(
        index
        for index, (event, payload) in enumerate(events)
        if event == "preview.status" and payload.get("runId") == "new-run" and payload.get("status") == "starting"
    )
    assert all(
        not (event == "preview.status" and payload.get("runId") == "old-run")
        for event, payload in events[new_start_index + 1 :]
    )


def test_preview_run_id_and_generation_are_optional_for_legacy_start() -> None:
    events: list[tuple[str, dict]] = []
    preview = PreviewCapture(
        lambda event, payload: events.append((event, payload)),
        capture=lambda: Image.new("RGB", (16, 9)),
        interval=0.05,
    )

    assert preview.start("pc:limbus")
    assert preview.run_id is None
    assert preview.generation >= 1
    preview.stop()

    payloads = [payload for _event, payload in events]
    assert payloads
    assert all(payload["deviceId"] == "pc:limbus" for payload in payloads)
    assert all(payload["runId"] is None for payload in payloads)
    assert all(isinstance(payload["generation"], int) for payload in payloads)


def test_preview_start_callback_cannot_orphan_a_worker_after_reentrant_stop() -> None:
    events: list[tuple[str, dict]] = []
    preview: PreviewCapture

    def emit(event: str, payload: dict) -> None:
        events.append((event, payload))
        if event == "preview.status" and payload.get("status") == "starting":
            assert preview.stop_and_wait(
                run_id=payload["runId"],
                generation=payload["generation"],
            )

    preview = PreviewCapture(
        emit,
        capture=lambda: Image.new("RGB", (16, 9)),
        interval=0.05,
    )

    assert preview.start("pc:limbus", run_id="reentrant-run") is False
    assert not preview.running
    assert preview._thread is None
    assert not any(event == "screenshot.frame" for event, _payload in events)
