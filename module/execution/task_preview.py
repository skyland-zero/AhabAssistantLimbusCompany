"""Runner-side live preview for the duration of one execution.

The sidecar deliberately quiesces its own :class:`module.preview_capture.PreviewCapture`
before it hands the device to a Runner, and a device lease rejects every
sidecar-owned device write.  Without a Runner producer the GPUI live screen
therefore stayed empty for the whole run even though protocol 1 already
reserves ``preview.frame`` / ``preview.status`` for exactly this purpose.

This module owns that producer.  It is intentionally side-effect free towards
task state: a failed capture publishes one ``preview.status=error`` and the run
continues.  Frames reuse the established monitor screenshot path (PC window,
MuMu, ADB/scrcpy) and the sidecar's own JPEG sizing so both producers are
indistinguishable to the UI.
"""

from __future__ import annotations

import threading
import time
from collections.abc import Callable
from typing import Any

from module.logger import log
from module.preview_capture import encode_screenshot_frame

# The sidecar preview and the Runner preview never run at the same time, so the
# cadence and payload budget stay identical to keep switching seamless.
PREVIEW_INTERVAL = 2.0
PREVIEW_MAX_WIDTH = 540
PREVIEW_JPEG_QUALITY = 72
STATUS_STARTING = "starting"
STATUS_RUNNING = "running"
STATUS_ERROR = "error"
STATUS_STOPPED = "stopped"

EmitFrame = Callable[[dict[str, Any], bytes], Any]
EmitStatus = Callable[[dict[str, Any]], Any]


class TaskPreview:
    """Publish the latest device frame while one Runner run owns the device.

    ``capture`` returns a PIL-like image.  It defaults to the shared automation
    monitor path, which already prefers a fresh business frame and never moves
    the PC game window.  Injection keeps the thread testable without touching a
    real device.
    """

    def __init__(
        self,
        *,
        run_id: str,
        device_id: str,
        emit_frame: EmitFrame,
        emit_status: EmitStatus,
        capture: Callable[[], Any] | None = None,
        interval: float = PREVIEW_INTERVAL,
        max_width: int | None = PREVIEW_MAX_WIDTH,
        quality: int = PREVIEW_JPEG_QUALITY,
    ) -> None:
        if not isinstance(run_id, str) or not run_id:
            raise ValueError("预览 runId 不能为空")
        if not isinstance(device_id, str) or not device_id:
            raise ValueError("预览 deviceId 不能为空")
        if not callable(emit_frame) or not callable(emit_status):
            raise TypeError("预览发布回调必须可调用")
        self.run_id = run_id
        self.device_id = device_id
        self._emit_frame = emit_frame
        self._emit_status = emit_status
        self._capture = capture or self._capture_current_screen
        self._interval = max(0.05, float(interval))
        self._max_width = max_width
        self._quality = max(1, min(95, int(quality)))
        self._lock = threading.RLock()
        self._stop_event: threading.Event | None = None
        self._thread: threading.Thread | None = None
        self._enabled = True
        self._generation = 0

    @property
    def running(self) -> bool:
        with self._lock:
            thread = self._thread
            return thread is not None and thread.is_alive()

    @property
    def enabled(self) -> bool:
        with self._lock:
            return self._enabled

    @property
    def generation(self) -> int:
        with self._lock:
            return self._generation

    def start(self) -> bool:
        """Start the capture thread; a repeated start is idempotent."""

        with self._lock:
            if self._thread is not None and self._thread.is_alive():
                return False
            stop_event = threading.Event()
            self._generation += 1
            generation = self._generation
            thread = threading.Thread(
                target=self._run,
                args=(stop_event, generation),
                name=f"AALCTaskPreview-{self.run_id}",
                daemon=True,
            )
            self._stop_event = stop_event
            self._thread = thread
        self._publish_status(STATUS_STARTING if self.enabled else STATUS_STOPPED, generation=generation)
        thread.start()
        return True

    def set_enabled(self, enabled: bool) -> bool:
        """Record the user's preview preference without touching the device.

        The worker keeps running so a re-enable can resume immediately, but it
        skips capture and transmission while disabled.  This mirrors the GPUI
        gating: hiding the panel must not cost a screenshot every interval.
        """

        if not isinstance(enabled, bool):
            raise TypeError("enabled 必须是 bool")
        with self._lock:
            if self._enabled == enabled:
                return False
            self._enabled = enabled
        if not enabled:
            self._publish_status(STATUS_STOPPED)
        return True

    def stop(self, deadline: float | None = None, *, notify: bool = True) -> bool:
        """Stop and prove the worker exited before the device is cleaned up.

        The capture call may be inside a driver operation, so a bounded join is
        used.  ``False`` means "not proven stopped" and the caller (the host's
        finalization) reports it instead of silently racing controller cleanup.

        ``notify=False`` suppresses the trailing ``preview.status=stopped``.
        A run that is ending must not publish a status that makes the UI drop
        the last frame: the authoritative ``execution.status=idle`` already
        clears the surface, and the sidecar preview replaces it.
        """

        with self._lock:
            stop_event = self._stop_event
            thread = self._thread
            generation = self._generation
        if stop_event is None or thread is None:
            return True
        stop_event.set()
        if thread is threading.current_thread():
            return False
        if deadline is None:
            # A capture may be inside a driver call; allow the same order of
            # magnitude as the sidecar preview join plus one scrcpy snapshot.
            timeout = max(2.0, self._interval * 2.5)
        else:
            timeout = max(0.0, float(deadline) - time.monotonic())
        thread.join(timeout=timeout)
        if thread.is_alive():
            return False
        with self._lock:
            if self._thread is thread:
                self._thread = None
                self._stop_event = None
                self._generation += 1
        if notify:
            self._publish_status(STATUS_STOPPED, generation=generation)
        return True

    def close(self) -> None:
        self.stop()

    @staticmethod
    def _capture_current_screen() -> Any:
        from module.automation import auto

        # Same contract as the sidecar preview: reuse a fresh business frame
        # while the task is working, otherwise capture at the preview rate.
        return auto.take_monitor_screenshot(
            gray=False,
            max_age=0.5,
            ensure_window_visible=False,
        )

    def _is_current(self, stop_event: threading.Event, generation: int) -> bool:
        with self._lock:
            return (
                not stop_event.is_set()
                and self._stop_event is stop_event
                and self._generation == generation
            )

    def _run(self, stop_event: threading.Event, generation: int) -> None:
        status = STATUS_STARTING
        while not stop_event.is_set():
            started = time.monotonic()
            if self.enabled:
                try:
                    image = self._capture()
                    frame = encode_screenshot_frame(
                        image,
                        self.device_id,
                        max_width=self._max_width,
                        quality=self._quality,
                    )
                    jpeg = frame.pop("jpeg")
                    payload = {
                        **frame,
                        "deviceId": self.device_id,
                        "runId": self.run_id,
                        "generation": generation,
                    }
                    if not self._is_current(stop_event, generation):
                        return
                    self._emit_frame(payload, jpeg)
                    if status != STATUS_RUNNING:
                        status = STATUS_RUNNING
                        self._publish_status(STATUS_RUNNING, generation=generation)
                except Exception as error:
                    if not self._is_current(stop_event, generation):
                        return
                    if status != STATUS_ERROR:
                        status = STATUS_ERROR
                        self._publish_status(
                            STATUS_ERROR,
                            generation=generation,
                            error=str(error) or type(error).__name__,
                        )
                    log.debug("Runner 实时预览截图失败：%s", error)
            elapsed = time.monotonic() - started
            stop_event.wait(max(0.0, self._interval - elapsed))

    def _publish_status(
        self,
        status: str,
        *,
        generation: int | None = None,
        error: str | None = None,
    ) -> None:
        if generation is None:
            generation = self.generation
        payload: dict[str, Any] = {
            "deviceId": self.device_id,
            "runId": self.run_id,
            "generation": generation,
            "status": status,
        }
        if error:
            payload["error"] = error
        try:
            self._emit_status(payload)
        except Exception:
            log.debug("发布 Runner 实时预览状态失败", exc_info=True)


__all__ = [
    "PREVIEW_INTERVAL",
    "PREVIEW_JPEG_QUALITY",
    "PREVIEW_MAX_WIDTH",
    "STATUS_ERROR",
    "STATUS_RUNNING",
    "STATUS_STARTING",
    "STATUS_STOPPED",
    "TaskPreview",
]
