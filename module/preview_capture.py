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
        # Serialize lifecycle transitions.  In particular, a new start must
        # never install a worker while an older stop is still waiting for its
        # capture call to return.
        self._lifecycle_lock = threading.RLock()
        self._stop_event: threading.Event | None = None
        self._thread: threading.Thread | None = None
        self._device_id: str | None = None
        self._run_id: str | None = None
        self._generation = 0

    @property
    def running(self) -> bool:
        with self._lock:
            return self._thread is not None and self._thread.is_alive()

    @property
    def generation(self) -> int:
        """Current preview generation.

        A generation changes for every start/stop transition.  Consumers that
        attach their own asynchronous callbacks can retain this value and
        discard callbacks for an older generation before publishing them.
        """

        with self._lock:
            return self._generation

    def is_generation_current(self, generation: int) -> bool:
        """Return whether ``generation`` is still the active preview token."""

        with self._lock:
            return generation == self._generation and self._thread is not None and self._thread.is_alive()

    @property
    def run_id(self) -> str | None:
        """Run identity attached to the active preview session, if any."""

        with self._lock:
            return self._run_id

    @staticmethod
    def _normalize_run_id(run_id: str | None) -> str | None:
        if run_id is None:
            return None
        if not isinstance(run_id, str) or not run_id:
            raise ValueError("预览 run_id 必须是非空字符串或 None")
        return run_id

    @staticmethod
    def _normalize_generation(generation: int | None) -> int:
        if generation is None:
            return 0
        if isinstance(generation, bool):
            raise ValueError("预览 generation 必须是非负整数")
        try:
            value = int(generation)
        except (TypeError, ValueError, OverflowError) as error:
            raise ValueError("预览 generation 必须是非负整数") from error
        if value < 0:
            raise ValueError("预览 generation 必须是非负整数")
        return value

    def start(
        self,
        device_id: str,
        *,
        run_id: str | None = None,
        generation: int | None = None,
    ) -> bool:
        """Start a run-scoped preview; repeated starts of one session are idempotent.

        ``run_id`` is optional for the ordinary sidecar preview.  A new run
        identity on the same device is still a new session and therefore
        stops the old worker before installing its generation.
        """

        if not device_id:
            raise ValueError("预览设备 ID 不能为空")
        normalized_run_id = self._normalize_run_id(run_id)
        requested_generation = self._normalize_generation(generation)

        with self._lifecycle_lock:
            with self._lock:
                if (
                    self._device_id == device_id
                    and self._run_id == normalized_run_id
                    and self._thread is not None
                    and self._thread.is_alive()
                ):
                    return False

            # A failed stop is deliberately fail-closed.  Starting a second
            # worker would let stale frames from the first worker race the
            # new device and would make quiescence impossible to prove.
            self.stop_and_wait()
            with self._lock:
                if self._thread is not None:
                    return False
                self._generation = max(self._generation + 1, requested_generation)
                active_generation = self._generation
                stop_event = threading.Event()
                thread = threading.Thread(
                    target=self._run,
                    args=(device_id, normalized_run_id, stop_event, active_generation),
                    name="AALCPreviewCapture",
                    daemon=True,
                )
                self._stop_event = stop_event
                self._thread = thread
                self._device_id = device_id
                self._run_id = normalized_run_id
            self._emit_status(
                device_id,
                "starting",
                run_id=normalized_run_id,
                generation=active_generation,
            )
            try:
                # ``emit`` is user/application code and may synchronously
                # request stop (or even a replacement start).  Do not start
                # a thread whose lifecycle references were cleared by that
                # callback; such an orphan could capture after a newer
                # generation and ``start()`` would falsely report success.
                with self._lock:
                    if self._thread is not thread or self._stop_event is not stop_event or stop_event.is_set():
                        return False
                thread.start()
            except Exception:
                # Restore the invariant that a published worker is either
                # running or has a retained stop/thread reference.
                with self._lock:
                    if self._thread is thread:
                        self._thread = None
                        self._stop_event = None
                        self._device_id = None
                        self._run_id = None
                raise
            return True

    def stop(
        self,
        *,
        deadline: float | None = None,
        run_id: str | None = None,
        generation: int | None = None,
    ) -> bool:
        """Stop the active preview, retaining legacy best-effort semantics."""

        return self.stop_and_wait(deadline, run_id=run_id, generation=generation)

    def stop_and_wait(
        self,
        deadline: float | None = None,
        *,
        run_id: str | None = None,
        generation: int | None = None,
    ) -> bool:
        """Request stop and prove that the worker has exited.

        ``deadline`` is an absolute ``time.monotonic()`` deadline.  When it is
        omitted, the historical bounded wait (at least one second) is used.
        On timeout the thread and stop-event references intentionally remain
        published so a later call can retry the join; returning ``False`` is
        therefore a reliable quiescence failure rather than a best-effort
        indication.  References are cleared only after ``join`` confirms the
        worker is no longer alive.

        Optional identity arguments make a delayed stop fail closed instead of
        stopping a newer session.  Omitting them preserves the old wildcard
        stop behavior used by sidecar shutdown paths.
        """

        normalized_run_id = self._normalize_run_id(run_id)
        normalized_generation = None if generation is None else self._normalize_generation(generation)

        with self._lifecycle_lock:
            with self._lock:
                stop_event = self._stop_event
                thread = self._thread
                device_id = self._device_id
                active_run_id = self._run_id
                generation = self._generation

            # No worker is already a proven quiescent state.  Treat the
            # idempotent stop as successful; callers such as execution.start
            # must not report a preview timeout merely because preview was
            # never started.  A partially published lifecycle is different:
            # retain the fail-closed result until its owner can reconcile the
            # missing reference.
            if stop_event is None and thread is None:
                return True
            if stop_event is None or thread is None:
                return False

            if normalized_run_id is not None and active_run_id != normalized_run_id:
                return False
            if normalized_generation is not None and generation != normalized_generation:
                return False

            stop_event.set()
            if thread is threading.current_thread():
                # A worker cannot join itself.  Keep all references for its
                # eventual owner to prove exit.
                return False

            # ``start()`` publishes the worker references before emitting the
            # initial status.  A synchronous status listener can therefore
            # stop a thread that has not been started yet.  ``join`` raises
            # for that state; it is already provably quiescent, so clear the
            # matching references and complete the same generation transition
            # as a normally joined worker.
            if thread.ident is None:
                with self._lock:
                    if self._thread is not thread or self._stop_event is not stop_event:
                        return False
                    self._thread = None
                    self._stop_event = None
                    self._device_id = None
                    self._run_id = None
                    self._generation = max(self._generation + 1, generation + 1)
                if device_id:
                    self._emit_status(
                        device_id,
                        "stopped",
                        run_id=active_run_id,
                        generation=generation,
                    )
                return True

            if deadline is None:
                join_timeout = max(1.0, self._interval * 2)
            else:
                join_timeout = max(0.0, float(deadline) - time.monotonic())
            thread.join(timeout=join_timeout)
            if thread.is_alive():
                return False

            with self._lock:
                # A lifecycle lock normally makes this identity check
                # sufficient; retain it as a guard for defensive callers that
                # manipulate internals in tests or shutdown hooks.
                if self._thread is thread and self._stop_event is stop_event:
                    self._thread = None
                    self._stop_event = None
                    self._device_id = None
                    self._run_id = None
                else:
                    return False

            # Invalidate callbacks before publishing the stopped status.  The
            # worker performs the same generation check after every capture,
            # so a late frame cannot appear after this point.
            with self._lock:
                self._generation = max(self._generation + 1, generation + 1)
            if device_id:
                self._emit_status(
                    device_id,
                    "stopped",
                    run_id=active_run_id,
                    generation=generation,
                )
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

    def _run(
        self,
        device_id: str,
        run_id: str | None,
        stop_event: threading.Event,
        generation: int,
    ) -> None:
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
                frame.update(self._session_payload(device_id, run_id, generation))
                # Capture/encoding may run concurrently with stop().  Never
                # publish a frame or status after the worker lost ownership of
                # its generation.
                if not self._emit_if_current_worker(
                    "screenshot.frame",
                    frame,
                    run_id=run_id,
                    stop_event=stop_event,
                    generation=generation,
                ):
                    return
                if status != "running":
                    status = "running"
                    if not self._emit_status_if_current(
                        device_id,
                        status,
                        stop_event=stop_event,
                        run_id=run_id,
                        generation=generation,
                    ):
                        return
            except Exception as error:
                if not self._is_current_worker(stop_event, generation, run_id):
                    return
                if status != "error":
                    status = "error"
                    if not self._emit_status_if_current(
                        device_id,
                        status,
                        str(error),
                        stop_event=stop_event,
                        run_id=run_id,
                        generation=generation,
                    ):
                        return
                log.debug("实时预览截图失败：%s", error)

            elapsed = time.monotonic() - started
            stop_event.wait(max(0.0, self._interval - elapsed))

    def _is_current_worker(
        self,
        stop_event: threading.Event,
        generation: int,
        run_id: str | None = None,
    ) -> bool:
        with self._lock:
            return (
                not stop_event.is_set()
                and self._stop_event is stop_event
                and self._thread is threading.current_thread()
                and self._generation == generation
                and self._run_id == run_id
            )

    def _emit_if_current_worker(
        self,
        event: str,
        payload: dict[str, Any],
        *,
        stop_event: threading.Event,
        generation: int,
        run_id: str | None = None,
    ) -> bool:
        """Publish a worker event while holding the lifecycle identity lock."""

        with self._lock:
            if not (
                not stop_event.is_set()
                and self._stop_event is stop_event
                and self._thread is threading.current_thread()
                and self._generation == generation
                and self._run_id == run_id
            ):
                return False
            self._emit(event, payload)
        return True

    def _emit_status_if_current(
        self,
        device_id: str,
        status: str,
        error: str | None = None,
        *,
        stop_event: threading.Event,
        generation: int,
        run_id: str | None = None,
    ) -> bool:
        payload: dict[str, Any] = {
            "deviceId": device_id,
            "runId": run_id,
            "generation": generation,
            "status": status,
        }
        if error:
            payload["error"] = error
        with self._lock:
            if not (
                not stop_event.is_set()
                and self._stop_event is stop_event
                and self._thread is threading.current_thread()
                and self._generation == generation
                and self._run_id == run_id
            ):
                return False
            try:
                self._emit("preview.status", payload)
            except Exception:
                log.debug("发布实时预览状态失败", exc_info=True)
        return True

    @staticmethod
    def _session_payload(device_id: str, run_id: str | None, generation: int) -> dict[str, Any]:
        return {
            "deviceId": device_id,
            "runId": run_id,
            "generation": generation,
        }

    def _emit_status(
        self,
        device_id: str,
        status: str,
        error: str | None = None,
        *,
        run_id: str | None = None,
        generation: int | None = None,
    ) -> None:
        if generation is None:
            with self._lock:
                generation = self._generation
        payload: dict[str, Any] = {
            **self._session_payload(device_id, run_id, generation),
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
