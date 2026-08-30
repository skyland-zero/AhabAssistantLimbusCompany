"""Continuous, low-cost device preview capture for the GPUI sidecar.

The preview is deliberately separate from task screenshots.  It uses the
existing monitor screenshot path so PC windows, MuMu, and generic ADB devices
keep their established capture behavior, while avoiding the task screenshot
interval and disk writes.
"""

from __future__ import annotations

import io
import threading
import time
from collections.abc import Callable
from typing import Any

from module.logger import log

PREVIEW_INTERVAL = 2.0
PREVIEW_MAX_WIDTH = 540
PREVIEW_JPEG_QUALITY = 72

EventEmitter = Callable[[str, dict[str, Any]], Any]
CaptureFunction = Callable[[], Any]


def encode_screenshot_frame(
    image: Any,
    instance_id: str,
    *,
    max_width: int | None = PREVIEW_MAX_WIDTH,
    quality: int = PREVIEW_JPEG_QUALITY,
) -> dict[str, Any]:
    """Encode a PIL-like image into the sidecar screenshot event payload."""

    if image is None:
        raise ValueError("截图为空")
    if not getattr(image, "width", 0) or not getattr(image, "height", 0):
        raise ValueError("截图尺寸无效")

    if getattr(image, "mode", None) != "RGB":
        image = image.convert("RGB")
    if max_width is not None and max_width > 0 and image.width > max_width:
        height = max(1, round(image.height * max_width / image.width))
        resampling = getattr(getattr(image, "Resampling", None), "LANCZOS", None)
        if resampling is None:
            from PIL import Image

            resampling = Image.Resampling.LANCZOS
        image = image.resize((max_width, height), resampling)

    buffer = io.BytesIO()
    image.save(buffer, format="JPEG", quality=quality)
    return {
        "instanceId": instance_id,
        "jpeg": buffer.getvalue(),
        "width": image.width,
        "height": image.height,
    }


class PreviewCapture:
    """Capture and publish the latest frame while a device is connected."""

    def __init__(
        self,
        emit: EventEmitter,
        *,
        capture: CaptureFunction | None = None,
        interval: float = PREVIEW_INTERVAL,
        max_width: int | None = PREVIEW_MAX_WIDTH,
        quality: int = PREVIEW_JPEG_QUALITY,
    ) -> None:
        self._emit = emit
        self._capture = capture or self._capture_current_screen
        self._interval = max(0.05, float(interval))
        self._max_width = max_width
        self._quality = max(1, min(95, int(quality)))
        self._lock = threading.RLock()
        self._stop_event: threading.Event | None = None
        self._thread: threading.Thread | None = None
        self._device_id: str | None = None

    @property
    def running(self) -> bool:
        with self._lock:
            return self._thread is not None and self._thread.is_alive()

    def start(self, device_id: str) -> bool:
        """Start preview for ``device_id``; repeated starts are idempotent."""

        if not device_id:
            raise ValueError("预览设备 ID 不能为空")

        with self._lock:
            if (
                self._device_id == device_id
                and self._thread is not None
                and self._thread.is_alive()
            ):
                return False

        self.stop()
        stop_event = threading.Event()
        thread = threading.Thread(
            target=self._run,
            args=(device_id, stop_event),
            name="AALCPreviewCapture",
            daemon=True,
        )
        with self._lock:
            self._stop_event = stop_event
            self._thread = thread
            self._device_id = device_id
        self._emit_status(device_id, "starting")
        thread.start()
        return True

    def stop(self) -> bool:
        """Stop the active preview and wait briefly for an in-flight capture."""

        with self._lock:
            stop_event = self._stop_event
            thread = self._thread
            device_id = self._device_id
            self._stop_event = None
            self._thread = None
            self._device_id = None

        if stop_event is None:
            return False
        stop_event.set()
        if thread is not None and thread is not threading.current_thread():
            thread.join(timeout=max(1.0, self._interval * 2))
        if device_id:
            self._emit_status(device_id, "stopped")
        return True

    def close(self) -> None:
        self.stop()

    @staticmethod
    def _capture_current_screen() -> Any:
        from module.automation import auto

        # Reuse a fresh business frame while automation is active. When idle,
        # the monitor path captures independently at the preview rate.
        return auto.take_monitor_screenshot(
            gray=False,
            max_age=0.5,
            ensure_window_visible=False,
        )

    def _run(self, device_id: str, stop_event: threading.Event) -> None:
        status = "starting"
        while not stop_event.is_set():
            started = time.monotonic()
            try:
                image = self._capture()
                frame = encode_screenshot_frame(
                    image,
                    device_id,
                    max_width=self._max_width,
                    quality=self._quality,
                )
                self._emit("screenshot.frame", frame)
                if status != "running":
                    status = "running"
                    self._emit_status(device_id, status)
            except Exception as error:
                if status != "error":
                    status = "error"
                    self._emit_status(device_id, status, str(error))
                log.debug("实时预览截图失败：%s", error)

            elapsed = time.monotonic() - started
            stop_event.wait(max(0.0, self._interval - elapsed))

    def _emit_status(self, device_id: str, status: str, error: str | None = None) -> None:
        payload: dict[str, Any] = {
            "deviceId": device_id,
            "status": status,
        }
        if error:
            payload["error"] = error
        try:
            self._emit("preview.status", payload)
        except Exception:
            log.debug("发布实时预览状态失败", exc_info=True)


__all__ = [
    "PREVIEW_INTERVAL",
    "PREVIEW_JPEG_QUALITY",
    "PREVIEW_MAX_WIDTH",
    "PreviewCapture",
    "encode_screenshot_frame",
]
