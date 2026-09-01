"""WxPusher SPT delivery with a bounded, best-effort background queue.

The public service exposes notification-shaped operations instead of binding
the execution layer to WxPusher.  A future channel can implement the same
``NotificationSender`` protocol and reuse the queue and formatting helpers.
"""

from __future__ import annotations

import queue
import threading
import time
from collections.abc import Mapping
from dataclasses import dataclass
from typing import Any, Callable, Protocol

import requests

from module.config.redaction import redact_text
from module.logger import log

WXPUSHER_SIMPLE_PUSH_URL = "https://wxpusher.zjiecode.com/api/send/message/simple-push"
WXPUSHER_SUCCESS_CODE = 1000
WXPUSHER_MAX_SPT = 10
WXPUSHER_MIN_INTERVAL = 0.5  # approximately two requests per second


class NotificationSender(Protocol):
    """Transport abstraction retained for future notification channels."""

    def send_message(self, spt: str, content: str, summary: str) -> Mapping[str, Any]: ...


class WxPusherError(RuntimeError):
    """A safe-to-display WxPusher error that never contains the SPT."""


def normalize_spt(value: Any, *, required: bool = True) -> str:
    """Validate and normalize one personal SPT without echoing its value."""

    if not isinstance(value, str):
        raise WxPusherError("SPT 必须是字符串")
    normalized = value.strip()
    if not normalized:
        if required:
            raise WxPusherError("SPT 未配置")
        return ""
    if len(normalized) <= 4 or not normalized.startswith("SPT_"):
        raise WxPusherError("SPT 格式无效")
    return normalized


class WxPusherClient:
    """Synchronous WxPusher API client used by the queue worker and test RPC."""

    def __init__(
        self,
        *,
        post: Callable[..., Any] | None = None,
        timeout: float = 10.0,
        max_attempts: int = 3,
        retry_backoff: float = 0.5,
        sleeper: Callable[[float], None] = time.sleep,
    ) -> None:
        if max_attempts < 1:
            raise ValueError("max_attempts must be positive")
        self._post = post or requests.post
        self.timeout = timeout
        self.max_attempts = max_attempts
        self.retry_backoff = max(0.0, retry_backoff)
        self._sleep = sleeper

    def send_message(self, spt: str, content: str, summary: str) -> Mapping[str, Any]:
        normalized_spt = normalize_spt(spt)
        if not isinstance(content, str) or not content.strip():
            raise WxPusherError("通知内容不能为空")
        if not isinstance(summary, str):
            raise WxPusherError("通知摘要必须是字符串")

        payload = {
            "content": content,
            "summary": summary.strip()[:100],
            "contentType": 1,
            "spt": normalized_spt,
        }
        last_error: WxPusherError | None = None
        for attempt in range(self.max_attempts):
            try:
                response = self._post(WXPUSHER_SIMPLE_PUSH_URL, json=payload, timeout=self.timeout)
                status_code = int(getattr(response, "status_code", 200))
                if status_code >= 400:
                    last_error = WxPusherError(self._http_error_message(status_code))
                    if self._retryable_status(status_code) and attempt + 1 < self.max_attempts:
                        self._sleep(self.retry_backoff * (2**attempt))
                        continue
                    raise last_error

                try:
                    result = response.json()
                except (TypeError, ValueError, AttributeError) as error:
                    raise WxPusherError("WxPusher 响应格式错误") from error
                if not isinstance(result, Mapping):
                    raise WxPusherError("WxPusher 响应格式错误")
                try:
                    code = int(result.get("code", -1))
                except TypeError, ValueError:
                    code = -1
                if code != WXPUSHER_SUCCESS_CODE:
                    raise WxPusherError("WxPusher API 返回错误")
                return dict(result)
            except WxPusherError:
                raise
            except requests.exceptions.Timeout as error:
                last_error = WxPusherError("WxPusher 请求超时")
                if attempt + 1 < self.max_attempts:
                    self._sleep(self.retry_backoff * (2**attempt))
                    continue
                raise last_error from error
            except requests.exceptions.RequestException as error:
                last_error = WxPusherError("WxPusher 网络请求失败")
                if attempt + 1 < self.max_attempts:
                    self._sleep(self.retry_backoff * (2**attempt))
                    continue
                raise last_error from error
            except (OSError, RuntimeError) as error:
                # Small HTTP test doubles and platform transports may expose
                # ordinary runtime errors rather than requests exceptions.
                last_error = WxPusherError("WxPusher 网络请求失败")
                if attempt + 1 < self.max_attempts:
                    self._sleep(self.retry_backoff * (2**attempt))
                    continue
                raise last_error from error
            except Exception as error:
                last_error = WxPusherError("WxPusher 请求失败")
                if attempt + 1 < self.max_attempts:
                    self._sleep(self.retry_backoff * (2**attempt))
                    continue
                raise last_error from error

        raise last_error or WxPusherError("WxPusher 请求失败")

    @staticmethod
    def _retryable_status(status_code: int) -> bool:
        return status_code == 429 or status_code >= 500

    @staticmethod
    def _http_error_message(status_code: int) -> str:
        if status_code == 429:
            return "WxPusher 请求过于频繁"
        if status_code >= 500:
            return f"WxPusher 服务暂时不可用（状态码 {status_code}）"
        return f"WxPusher 请求失败（状态码 {status_code}）"


@dataclass(frozen=True)
class NotificationMessage:
    spt: str
    content: str
    summary: str


class NotificationQueue:
    """FIFO daemon worker that rate-limits and isolates notification errors."""

    def __init__(
        self,
        sender: NotificationSender,
        *,
        min_interval: float = WXPUSHER_MIN_INTERVAL,
        clock: Callable[[], float] = time.monotonic,
        sleeper: Callable[[float], None] = time.sleep,
    ) -> None:
        self.sender = sender
        self.min_interval = max(0.0, min_interval)
        self._clock = clock
        self._sleep = sleeper
        self._items: queue.Queue[NotificationMessage] = queue.Queue()
        self._lock = threading.Lock()
        self._closed = False
        self._thread: threading.Thread | None = None

    def enqueue(self, message: NotificationMessage) -> bool:
        with self._lock:
            if self._closed:
                return False
            if self._thread is None:
                self._thread = threading.Thread(
                    target=self._run,
                    name="AALCNotificationQueue",
                    daemon=True,
                )
                self._thread.start()
            self._items.put(message)
        return True

    def flush(self, timeout: float = 5.0) -> bool:
        """Wait for currently queued items, primarily for deterministic tests."""

        deadline = self._clock() + max(0.0, timeout)
        while self._items.unfinished_tasks:
            if self._clock() >= deadline:
                return False
            time.sleep(min(0.01, max(0.0, deadline - self._clock())))
        return True

    def close(self, *, wait: bool = False, timeout: float = 1.0) -> None:
        with self._lock:
            self._closed = True
            thread = self._thread
        if wait and thread is not None and thread is not threading.current_thread():
            thread.join(timeout=max(0.0, timeout))

    def _run(self) -> None:
        last_sent: float | None = None
        while True:
            try:
                message = self._items.get(timeout=0.1)
            except queue.Empty:
                with self._lock:
                    if self._closed:
                        return
                continue
            try:
                with self._lock:
                    if self._closed:
                        continue
                if last_sent is not None:
                    delay = self.min_interval - (self._clock() - last_sent)
                    if delay > 0:
                        self._sleep(delay)
                with self._lock:
                    if self._closed:
                        continue
                try:
                    self.sender.send_message(message.spt, message.content, message.summary)
                except Exception as error:  # notification is strictly best effort
                    log.warning(
                        "WxPusher 通知发送失败：%s",
                        redact_text(error, (message.spt,)),
                    )
                finally:
                    last_sent = self._clock()
            finally:
                self._items.task_done()


def format_completion(kind: str, count: int) -> tuple[str, str]:
    labels = {"exp": "经验本", "thread": "纽本", "mirror": "镜牢"}
    label = labels.get(kind, "任务")
    amount = max(1, int(count))
    content = f"AALC 任务进度\n{label}完成 {amount} 次"
    return content, f"{label}完成 {amount} 次"


def format_final_summary(current_run: Mapping[str, Any]) -> tuple[str, str]:
    completed = current_run.get("completed", {})
    lines = [
        "AALC 任务已完成",
        f"经验本：{_count(completed, 'exp')} 次",
        f"纽本：{_count(completed, 'thread')} 次",
        f"镜牢：{_count(completed, 'mirror')} 次",
    ]
    return "\n".join(lines), "AALC 任务已完成"


def format_failure(error: Any) -> tuple[str, str]:
    message = redact_text(error)
    content = f"AALC 任务执行失败\n原因：{message}"
    return content, "AALC 任务执行失败"


def _count(values: Mapping[str, Any], key: str) -> int:
    try:
        return max(0, int(values.get(key, 0)))
    except TypeError, ValueError:
        return 0


class NotificationService:
    """Application-facing notification facade backed by the FIFO queue."""

    def __init__(
        self,
        sender: NotificationSender | None = None,
        *,
        queue_: NotificationQueue | None = None,
    ) -> None:
        self.sender = sender or WxPusherClient()
        self.queue = queue_ or NotificationQueue(self.sender)

    def send_test(self, spt: str) -> Mapping[str, Any]:
        normalized = normalize_spt(spt)
        return self.sender.send_message(
            normalized,
            "AALC WxPusher 测试通知\n如果你看到这条消息，通知配置已经生效。",
            "AALC WxPusher 测试通知",
        )

    def enqueue_completion(self, spt: str, kind: str, count: int) -> bool:
        content, summary = format_completion(kind, count)
        return self._enqueue(spt, content, summary)

    def enqueue_final(self, spt: str, current_run: Mapping[str, Any]) -> bool:
        content, summary = format_final_summary(current_run)
        return self._enqueue(spt, content, summary)

    def enqueue_failure(self, spt: str, error: Any) -> bool:
        content, summary = format_failure(redact_text(error, (spt,)))
        return self._enqueue(spt, content, summary)

    def _enqueue(self, spt: str, content: str, summary: str) -> bool:
        normalized = normalize_spt(spt, required=False)
        if not normalized:
            return False
        return self.queue.enqueue(NotificationMessage(normalized, content, summary))

    def close(self) -> None:
        # Shutdown is deliberately bounded; a forced application close may
        # drop messages still waiting in the FIFO, as documented for v1.
        self.queue.close(wait=False)


__all__ = [
    "NotificationMessage",
    "NotificationQueue",
    "NotificationSender",
    "NotificationService",
    "WXPUSHER_MAX_SPT",
    "WXPUSHER_MIN_INTERVAL",
    "WXPUSHER_SIMPLE_PUSH_URL",
    "WXPUSHER_SUCCESS_CODE",
    "WxPusherClient",
    "WxPusherError",
    "format_completion",
    "format_failure",
    "format_final_summary",
    "normalize_spt",
]
