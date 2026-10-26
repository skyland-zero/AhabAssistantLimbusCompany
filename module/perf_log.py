"""Independent, machine-readable performance log.

The normal application log is kept for task progress and user-facing
diagnostics.  This module writes a separate JSONL file so that repeated
per-frame measurements can be collected without flooding ``debugLog.log``.

The log is intentionally dependency-light: it only uses the standard library
plus ``concurrent_log_handler`` (already used by the main logger).  Callers
should treat every method as best-effort; instrumentation must never break
automation.
"""

from __future__ import annotations

import atexit
import json
import logging
import os
import sys
import threading
import time
from contextlib import contextmanager
from pathlib import Path
from typing import Any, Iterator

from concurrent_log_handler import ConcurrentRotatingFileHandler

_DEFAULT_PATH = Path("./logs/perfLog.log")
_TRUTHY = {"1", "true", "yes", "on"}
_MAX_BYTES = 10 * 1024 * 1024
_BACKUP_COUNT = 5


def _env_enabled() -> bool:
    raw = os.environ.get("AALC_PERF_LOG")
    if raw is None:
        return True
    return raw.strip().lower() in _TRUTHY


_INTERNAL_CALLER_FILES = {
    "module/perf_log.py",
    "module/automation/automation.py",
    "core/execution_control.py",
    "tasks/base/retry.py",
}


def _format_caller(skip: int) -> str:
    try:
        frame = sys._getframe(skip)
        for _ in range(16):
            filename = frame.f_code.co_filename
            try:
                path = Path(os.path.relpath(filename, Path.cwd())).as_posix()
            except ValueError:
                path = Path(filename).name
            if path not in _INTERNAL_CALLER_FILES:
                return f"{path}:{frame.f_lineno}"
            frame = frame.f_back
            if frame is None:
                break
        return path
    except Exception:
        return ""


class PerfLog:
    """Write structured performance events to a dedicated rotating JSONL file."""

    def __init__(
        self,
        path: str | os.PathLike[str] | None = None,
        *,
        enabled: bool | None = None,
        run_id: str | None = None,
        max_bytes: int = _MAX_BYTES,
        backup_count: int = _BACKUP_COUNT,
    ) -> None:
        configured_path = os.environ.get("AALC_PERF_LOG_PATH")
        self.path = Path(path or configured_path or _DEFAULT_PATH)
        self.enabled = _env_enabled() if enabled is None else bool(enabled)
        self.run_id = run_id or f"{time.strftime('%Y%m%d-%H%M%S')}-{os.getpid()}"
        self.max_bytes = max_bytes
        self.backup_count = backup_count

        self._local = threading.local()
        self._logger: logging.Logger | None = None
        self._handler: logging.Handler | None = None
        self._closed = False
        self._run_started = False
        self._created_perf = time.perf_counter()

        if self.enabled:
            self._setup_logger()
        atexit.register(self.close)

    # ------------------------------------------------------------------
    # logger lifecycle
    # ------------------------------------------------------------------
    def _setup_logger(self) -> None:
        try:
            self.path.parent.mkdir(parents=True, exist_ok=True)
            logger = logging.getLogger(f"AALC.PERF.{id(self)}")
            logger.setLevel(logging.INFO)
            logger.propagate = False
            handler = ConcurrentRotatingFileHandler(
                str(self.path),
                maxBytes=self.max_bytes,
                backupCount=self.backup_count,
                encoding="utf-8",
            )
            handler.setFormatter(logging.Formatter("%(message)s"))
            logger.addHandler(handler)
            self._logger = logger
            self._handler = handler
        except Exception:
            # Performance logging is strictly optional.
            self.enabled = False
            self._logger = None
            self._handler = None

    def close(self) -> None:
        if self._closed:
            return
        if self._run_started:
            try:
                self.record(
                    "run_end",
                    duration_ms=round((time.perf_counter() - self._created_perf) * 1000.0, 3),
                )
            except Exception:
                pass
        self._closed = True
        handler = self._handler
        logger = self._logger
        self._handler = None
        self._logger = None
        if handler is not None:
            try:
                handler.close()
            except Exception:
                pass
        if logger is not None and handler is not None:
            try:
                logger.removeHandler(handler)
            except Exception:
                pass

    # ------------------------------------------------------------------
    # context helpers
    # ------------------------------------------------------------------
    def caller(self, skip: int = 1) -> str:
        """Return ``file:line`` for the business caller above this method.

        Instrumentation-heavy internal frames (automation, retry and the
        performance logger itself) are skipped so the report points at the
        task code that triggered the measurement.
        """

        return _format_caller(skip + 1)

    def current_task(self) -> str:
        stack = getattr(self._local, "task_stack", None)
        return stack[-1]["name"] if stack else ""

    def current_frame(self) -> int | None:
        return getattr(self._local, "frame", None)

    def set_frame(self, frame_id: int | None) -> None:
        if not self.enabled:
            return
        self._local.frame = frame_id

    def push_task(self, name: str, **fields: Any) -> None:
        if not self.enabled:
            return
        stack = getattr(self._local, "task_stack", None)
        if stack is None:
            stack = self._local.task_stack = []
        parent = stack[-1]["name"] if stack else ""
        self.record("task_start", name=name, parent=parent, **fields)
        stack.append({"name": name, "start": time.perf_counter(), "parent": parent})

    def pop_task(self, status: str = "ok", **fields: Any) -> float:
        if not self.enabled:
            return 0.0
        stack = getattr(self._local, "task_stack", None)
        if not stack:
            return 0.0
        entry = stack.pop()
        duration_ms = (time.perf_counter() - entry["start"]) * 1000.0
        self.record(
            "task_end",
            name=entry["name"],
            parent=entry["parent"],
            duration_ms=round(duration_ms, 3),
            status=status,
            **fields,
        )
        return duration_ms

    @contextmanager
    def task(self, name: str, **fields: Any) -> Iterator[None]:
        self.push_task(name, **fields)
        try:
            yield
        except Exception:
            self.pop_task(status="error")
            raise
        else:
            self.pop_task(status="ok")

    # ------------------------------------------------------------------
    # event helpers
    # ------------------------------------------------------------------
    def screenshot(
        self,
        *,
        frame: int | None,
        caller: str = "",
        start_ts: float | None = None,
        start_perf: float | None = None,
        wait_ms: float = 0.0,
        capture_ms: float = 0.0,
        total_ms: float = 0.0,
        **fields: Any,
    ) -> None:
        if not self.enabled:
            return
        previous = getattr(self._local, "last_screenshot_start", None)
        gap_ms = fields.pop("gap_ms", None)
        if gap_ms is None and previous is not None and start_perf is not None:
            gap_ms = (start_perf - previous) * 1000.0
        if start_perf is not None:
            self._local.last_screenshot_start = start_perf
        self.set_frame(frame)
        self.record(
            "screenshot",
            frame=frame,
            caller=caller,
            start_ts=start_ts,
            gap_ms=round(float(gap_ms), 3) if gap_ms is not None else None,
            wait_ms=round(float(wait_ms), 3),
            capture_ms=round(float(capture_ms), 3),
            total_ms=round(float(total_ms), 3),
            duration_ms=round(float(total_ms), 3),
            **fields,
        )

    def match(self, **fields: Any) -> None:
        self.record("match", **fields)

    def multi_match(self, **fields: Any) -> None:
        self.record("multi_match", **fields)

    def ocr(self, **fields: Any) -> None:
        self.record("ocr", **fields)

    def ocr_engine(self, **fields: Any) -> None:
        self.record("ocr_engine", **fields)

    def ocr_match(self, **fields: Any) -> None:
        self.record("ocr_match", **fields)

    def retry(self, **fields: Any) -> None:
        self.record("retry", **fields)

    def sleep(self, **fields: Any) -> None:
        fields.setdefault("duration_ms", fields.get("actual_ms", 0.0))
        self.record("sleep", **fields)

    def action(self, **fields: Any) -> None:
        self.record("action", **fields)

    def summary(self, data: dict[str, Any], **fields: Any) -> None:
        self.record("vision_summary", data=data, **fields)

    # ------------------------------------------------------------------
    # generic writer
    # ------------------------------------------------------------------
    def record(self, event: str, **fields: Any) -> None:
        if not self.enabled or self._closed:
            return
        try:
            self._ensure_run_start()
            payload = self._base_payload(event)
            payload.update(fields)
            self._write(payload)
        except Exception:
            # Instrumentation must never interfere with automation.
            pass

    def _base_payload(self, event: str) -> dict[str, Any]:
        payload: dict[str, Any] = {
            "ts": time.time(),
            "run": self.run_id,
            "pid": os.getpid(),
            "thread": threading.current_thread().name,
            "event": event,
        }
        task = self.current_task()
        if task:
            payload["task"] = task
        frame = self.current_frame()
        if frame is not None:
            payload["frame"] = frame
        return payload

    def _write(self, payload: dict[str, Any]) -> None:
        logger = self._logger
        if logger is None:
            return
        line = json.dumps(payload, ensure_ascii=False, separators=(",", ":"), default=str)
        logger.info("%s", line)

    def _ensure_run_start(self) -> None:
        if self._run_started:
            return
        self._run_started = True
        payload = self._base_payload("run_start")
        payload.update(self._run_config())
        self._write(payload)

    @staticmethod
    def _run_config() -> dict[str, Any]:
        data: dict[str, Any] = {
            "cwd": str(Path.cwd()),
            "python": sys.version.split()[0],
        }
        try:
            from module.config import cfg

            for key in (
                "set_win_size",
                "screenshot_interval",
                "mouse_action_interval",
                "enable_template_blur",
                "simulator",
                "simulator_type",
                "background_click",
                "win_input_type",
                "language_in_program",
            ):
                data[key] = getattr(cfg, key, None)
        except Exception:
            pass
        try:
            from utils.image_utils import ImageUtils

            data["coarse"] = {
                "min_area": ImageUtils._COARSE_MIN_AREA,
                "min_template_side": ImageUtils._COARSE_MIN_TEMPLATE_SIDE,
                "min_scaled_side": ImageUtils._COARSE_MIN_SCALED_SIDE,
                "min_score": ImageUtils._COARSE_MIN_SCORE,
                "max_scale": ImageUtils._COARSE_MAX_SCALE,
                "max_candidates": ImageUtils._COARSE_MAX_CANDIDATES,
                "max_multi_candidates": ImageUtils._COARSE_MAX_MULTI_CANDIDATES,
            }
        except Exception:
            pass
        return data


perf_log = PerfLog()
