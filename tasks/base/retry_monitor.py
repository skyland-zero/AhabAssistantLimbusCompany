import threading
import time

import cv2
import numpy as np

from module.automation import auto
from module.logger import log
from utils.image_utils import ImageUtils


class RetryMonitor:
    """独立处理服务器重试弹窗，避免业务流程各自重复实现。"""

    RETRY_TEMPLATE = "base/retry.png"
    TEMPLATE_PATHS = ("default/en", "default/zh_cn")
    _LIFECYCLE_LOCK_TIMEOUT = 2.0
    _LIFECYCLE_LOCK_POLL = 0.1

    def __init__(
        self,
        poll_interval: float = 0.5,
        click_cooldown: float = 2.0,
        screenshot_max_age: float = 3.0,
    ):
        self.poll_interval = poll_interval
        self.click_cooldown = click_cooldown
        self.screenshot_max_age = screenshot_max_age
        self._stop_event = threading.Event()
        self._lifecycle_lock = threading.Lock()
        self._thread: threading.Thread | None = None
        self._templates: tuple[np.ndarray, ...] = ()
        self._last_click_time = 0.0
        self._handling_retry = False
        self._clear_frames = 0
        self._lifecycle_request = 0

    def _acquire_lifecycle_lock(self) -> bool:
        """Acquire lifecycle state without an unbounded lock wait."""

        deadline = time.monotonic() + self._LIFECYCLE_LOCK_TIMEOUT
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                return False
            if self._lifecycle_lock.acquire(timeout=min(self._LIFECYCLE_LOCK_POLL, remaining)):
                return True

    def start(self) -> None:
        """加载模板并启动监控线程；重复调用不会创建多个线程。"""
        self._lifecycle_request += 1
        request = self._lifecycle_request
        if not self._acquire_lifecycle_lock():
            log.warning("重试监控生命周期锁等待超时，跳过启动")
            return
        try:
            # A concurrent stop supersedes a start that was still waiting for
            # templates or the lifecycle lock; do not resurrect the worker.
            if request != self._lifecycle_request:
                return
            if self._thread is not None and self._thread.is_alive():
                return
            self._templates = self._load_templates()
            if not self._templates:
                log.warning("未能加载通用重试按钮模板，重试监控线程未启动")
                return
            self._last_click_time = 0.0
            self._handling_retry = False
            self._clear_frames = 0
            self._stop_event.clear()
            self._thread = threading.Thread(
                target=self._run,
                name="ServerRetryMonitor",
                daemon=True,
            )
            self._thread.start()
            log.debug("通用服务器重试监控线程已启动")
        finally:
            self._lifecycle_lock.release()

    def stop(self) -> None:
        """停止监控并确保业务点击门恢复。"""
        # Set the event before contending for bookkeeping lock so the worker
        # wakes immediately even if template loading still owns the lock.
        self._lifecycle_request += 1
        self._stop_event.set()
        acquired = self._acquire_lifecycle_lock()
        if not acquired:
            log.warning("重试监控生命周期锁等待超时，继续执行有界停止")
            thread = self._thread
        else:
            try:
                thread = self._thread
                self._thread = None
            finally:
                self._lifecycle_lock.release()
        if thread is not None and thread is not threading.current_thread():
            thread.join(timeout=2)
        self._handling_retry = False
        self._clear_frames = 0
        auto.resume_interactions()
        if thread is not None:
            log.debug("通用服务器重试监控线程已停止")

    def _load_templates(self) -> tuple[np.ndarray, ...]:
        templates = []
        for path in self.TEMPLATE_PATHS:
            template = ImageUtils.load_from_specific_path(self.RETRY_TEMPLATE, path)
            if template is not None:
                templates.append(template)
        return tuple(templates)

    @staticmethod
    def _to_gray_array(screenshot) -> np.ndarray:
        image = np.asarray(screenshot)
        if image.ndim == 3:
            image = cv2.cvtColor(image, cv2.COLOR_RGB2GRAY)
        return image

    def _find_retry_button(self, screenshot) -> tuple[int, int] | None:
        image = self._to_gray_array(screenshot)
        best_center = None
        best_score = 0.9
        h, w = image.shape[:2]
        crop_x1 = int(w * 0.18)
        crop_y1 = int(h * 0.15)
        crop_x2 = int(w * 0.82)
        crop_y2 = int(h * 0.85)
        cropped_image = image[crop_y1:crop_y2, crop_x1:crop_x2]
        for template in self._templates:
            match = ImageUtils.match_template(cropped_image, template, None, model="clam")
            if match is None:
                continue
            center, score = match
            if score >= best_score:
                best_center = (center[0] + crop_x1, center[1] + crop_y1)
                best_score = score
        return best_center

    def check_once(self, screenshot=None) -> bool:
        """检查一次重试弹窗，返回本轮是否执行了点击。"""
        if screenshot is None:
            screenshot = auto.take_monitor_screenshot(max_age=self.screenshot_max_age)
        if screenshot is None:
            return False
        if auto.check_pause() and not self._handling_retry:
            return False

        retry_position = self._find_retry_button(screenshot)
        if retry_position is None:
            if self._handling_retry:
                self._clear_frames += 1
                if self._clear_frames >= 2:
                    self._handling_retry = False
                    self._clear_frames = 0
                    auto.resume_interactions()
                    log.debug("服务器错误弹窗已消失，恢复业务点击")
            return False

        self._clear_frames = 0
        if not self._handling_retry:
            self._handling_retry = True
            auto.suspend_interactions()
            log.debug("服务器错误弹窗处理中，暂时阻止业务点击")

        if auto.check_pause():
            return False

        now = time.monotonic()
        if now - self._last_click_time < self.click_cooldown:
            return False

        auto.monitor_mouse_click(retry_position[0], retry_position[1])
        auto.invalidate_screenshot_cache()
        self._last_click_time = now
        log.warning(f"检测到服务器错误弹窗，监控线程已点击重试: {retry_position}")
        return True

    def _run(self) -> None:
        while not self._stop_event.wait(self.poll_interval):
            try:
                self.check_once()
            except Exception:
                log.exception("通用服务器重试监控线程处理异常")


retry_monitor = RetryMonitor()
