import gc
import math
import random
import threading
import time
from dataclasses import dataclass
from typing import Any, List

import cv2
import numpy as np
import psutil
from PIL.Image import Image

from core.execution_control import check_cancelled, interruptible_sleep, wait_for_event
from utils.image_utils import ImageUtils
from utils.path_manager import path_manager
from utils.singletonmeta import SingletonMeta

from ..config import cfg
from ..logger import log
from ..ocr import ocr
from .input_handlers.input import AbstractInput
from .screenshot import ScreenShot

# ponytail: 交互门最长关闭时间。监控线程卡在持久弹窗上时,超时放行业务输入,
# 恢复 retry() 等流程的卡死看门狗(kill_game/restart_game)。
GATE_WAIT_TIMEOUT = 30.0


@dataclass(frozen=True)
class TextMatchResult:
    """Structured result for dict-based OCR target matches."""

    value: Any
    text: str
    position: list[float]


class Automation(metaclass=SingletonMeta):
    """自动化管理类，用于管理与游戏窗口有关的自动化操作"""

    def __init__(self, windows_title):
        self.windows_title = windows_title
        self.screenshot = None
        self.input_handler = AbstractInput()
        self._screenshot_lock = threading.RLock()
        self._latest_screenshot = None
        self._latest_screenshot_monotonic = 0.0
        # ``screenshot`` is the business thread's current frame.  Keep a
        # monotonically increasing id so OCR/feature caches can never cross a
        # physical screenshot boundary.
        self._frame_id = 0
        self._frame_dirty = True
        self._screenshot_array_cache = {}
        self._ocr_cache = {}
        self._input_lock = threading.RLock()
        self._interaction_gate = threading.Event()
        self._interaction_gate.set()

        self.init_input()

        self.img_cache = {}
        self._feature_frame_cache = {}
        self.last_screenshot_time = 0
        self.last_click_time = 0
        self.model = "clam"

    def init_input(self, *, session=None):
        """初始化输入处理器并绑定当前选中的运行时设备。

        The sidecar owns a ``DeviceSession`` in ``DeviceManager``. Resolve it
        before consulting the legacy class-level connection pointers so a
        stale pointer cannot redirect input to another device.
        """
        if self.input_handler:
            self.input_handler = None

        if session is None:
            try:
                from module.device_manager import get_device_manager

                session = get_device_manager().active_session
            except ImportError:
                # Keep import-time compatibility for the legacy application.
                session = None

        active_kind = getattr(getattr(session, "target", None), "kind", None)
        if active_kind in ("mumu", "adb"):
            controller = getattr(session, "controller", None)
            if controller is None:
                from module.device_manager import DeviceError

                raise DeviceError("当前设备会话缺少模拟器控制器")
            self.input_handler = controller
            if active_kind == "mumu":
                log.debug("使用选中 MuMu 模拟器输入模块")
            else:
                log.debug("使用选中 ADB 模拟器输入模块")
        elif active_kind == "pc":
            self._init_windows_input()
        elif cfg.simulator:
            # Legacy configuration-driven path used by the old UI/CLI.
            if cfg.simulator_type == 0:
                from .input_handlers.simulator.mumu_control import MumuControl

                log.debug("使用MuMu模拟器输入模块")
                if MumuControl.connection_device is not None:
                    self.input_handler = MumuControl.connection_device
            else:
                from .input_handlers.simulator.simulator_control import SimulatorControl

                log.debug("使用基于PyMiniTouch的通用模拟器输入模块")
                self.input_handler = SimulatorControl.connection_device
        else:
            self._init_windows_input()

        if self.input_handler is None:
            if active_kind in ("mumu", "adb"):
                from module.device_manager import DeviceError

                raise DeviceError("当前选中设备没有可用的输入控制器")
            from .input_handlers.input import BackgroundInput

            self.input_handler = BackgroundInput()
        assert isinstance(self.input_handler, AbstractInput), "输入处理器必须是AbstractInput的实例"
        self.set_pause = self.input_handler.set_pause
        self.wait_pause = self.input_handler.wait_pause
        self.memory_protection = cfg.memory_protection

    def _init_windows_input(self) -> None:
        input_type = cfg.win_input_type
        if input_type == "background":
            from .input_handlers.input import BackgroundInput

            log.debug("使用后台点击模块")
            self.input_handler = BackgroundInput()
        elif input_type == "foreground":
            from .input_handlers.input import Input

            log.debug("使用前台点击模块")
            self.input_handler = Input()
        elif input_type == "window_move":
            from .input_handlers.input import WindowMoveInput

            log.debug("使用基于窗口移动的后台点击模块")
            self.input_handler = WindowMoveInput()

    def suspend_interactions(self) -> None:
        """暂时阻止业务线程继续点击。"""
        self._interaction_gate.clear()

    def resume_interactions(self) -> None:
        """恢复业务线程点击。"""
        self._interaction_gate.set()

    def reset_safety_locks(self) -> None:
        """线程被强制终止后换新锁,清除可能残留的持有状态。

        仅应在 my_script_task.terminate() 等硬杀路径调用:被杀线程持有的
        RLock 计数不会释放,不换锁则后续任务取锁永久阻塞。
        """
        self._input_lock = threading.RLock()
        self._screenshot_lock = threading.RLock()

    def _run_business_interaction(self, method_name: str, *args, **kwargs):
        """在交互门放行且取得输入锁后执行一次业务输入。

        交互门可能在等待输入锁期间被监控线程关闭，因此取得锁后需要再次确认。
        门连续关闭超过 GATE_WAIT_TIMEOUT 时视为监控卡在持久弹窗上，放行业务
        输入，让业务流程自身的卡死兜底(如 check_times)得以继续运行。
        """
        while True:
            check_cancelled()
            gate_open = wait_for_event(self._interaction_gate, GATE_WAIT_TIMEOUT)
            with self._input_lock:
                check_cancelled()
                if gate_open and self._interaction_gate.is_set():
                    method = getattr(self.input_handler, method_name)
                    self._mark_screenshot_dirty()
                    result = method(*args, **kwargs)
                    return result
                if not gate_open:
                    method = getattr(self.input_handler, method_name)
                    self._mark_screenshot_dirty()
                    result = method(*args, **kwargs)
                    return result
                # gate_open 但等待输入锁期间门被关闭:重新等待

    def mouse_click(self, x, y, times=1):
        return self._run_business_interaction("mouse_click", x, y, times=times)

    def mouse_click_blank(self, *args, **kwargs):
        return self._run_business_interaction("mouse_click_blank", *args, **kwargs)

    def mouse_move(self, *args, **kwargs):
        """移动鼠标而不点击，用于读取游戏内悬浮提示。"""

        return self._run_business_interaction("mouse_move", *args, **kwargs)

    def mouse_drag(self, *args, **kwargs):
        return self._run_business_interaction("mouse_drag", *args, **kwargs)

    def mouse_swipe_for_scroll(self, *args, **kwargs):
        return self._run_business_interaction("mouse_swipe_for_scroll", *args, **kwargs)

    def mouse_drag_down(self, *args, **kwargs):
        return self._run_business_interaction("mouse_drag_down", *args, **kwargs)

    def mouse_scroll(self, *args, **kwargs):
        return self._run_business_interaction("mouse_scroll", *args, **kwargs)

    def mouse_to_blank(self, *args, **kwargs):
        return self._run_business_interaction("mouse_to_blank", *args, **kwargs)

    def mouse_drag_link(self, *args, **kwargs):
        return self._run_business_interaction("mouse_drag_link", *args, **kwargs)

    def key_press(self, *args, **kwargs):
        return self._run_business_interaction("key_press", *args, **kwargs)

    def input_text(self, *args, **kwargs):
        return self._run_business_interaction("input_text", *args, **kwargs)

    def monitor_mouse_click(self, x, y, times=1):
        """由系统监控线程点击，不等待该监控线程设置的互斥门。"""
        self._mark_screenshot_dirty()
        with self._input_lock:
            result = self.input_handler.mouse_click(x, y, times=times)
        return result

    def _mark_screenshot_dirty(self) -> None:
        """Invalidate data derived from the current frame after an input action.

        The old ``self.screenshot`` is intentionally retained for callers that
        explicitly want to inspect the last frame.  Any subsequent screenshot
        request still captures a new frame, while monitor callers cannot reuse
        the old image through ``max_age``.
        """
        self._frame_dirty = True
        self._latest_screenshot_monotonic = 0.0
        getattr(self, "_screenshot_array_cache", {}).clear()
        getattr(self, "_ocr_cache", {}).clear()
        getattr(self, "_feature_frame_cache", {}).clear()

    def _set_business_screenshot(self, screenshot: Image) -> None:
        """Publish a newly captured business frame and invalidate old results."""
        self.screenshot = screenshot
        self._frame_id = getattr(self, "_frame_id", 0) + 1
        self._frame_dirty = False
        getattr(self, "_screenshot_array_cache", {}).clear()
        getattr(self, "_ocr_cache", {}).clear()
        getattr(self, "_feature_frame_cache", {}).clear()

    @staticmethod
    def _normalize_crop(crop):
        if crop is None:
            return None
        return tuple(round(float(value), 4) for value in crop)

    def _current_frame_key(self, screenshot=None):
        """Return a cache key that changes for both new frames and new objects."""
        if screenshot is None:
            screenshot = getattr(self, "screenshot", None)
            frame_id = getattr(self, "_frame_id", 0)
        else:
            frame_id = 0
        return frame_id, id(screenshot)

    def _get_screenshot_array(self, screenshot=None, *, gray=True):
        """Convert a PIL/ndarray screenshot once per frame and color mode."""
        source_is_current_frame = screenshot is None
        if screenshot is None:
            screenshot = self.screenshot
        cache = getattr(self, "_screenshot_array_cache", None)
        if cache is None:
            cache = self._screenshot_array_cache = {}
        frame_key = self._current_frame_key() if source_is_current_frame else self._current_frame_key(screenshot)
        cache_key = (frame_key, gray)
        if cache_key in cache:
            return cache[cache_key]

        image_array = np.asarray(screenshot)
        if gray:
            if image_array.ndim == 3:
                if image_array.shape[2] == 4:
                    image_array = cv2.cvtColor(image_array, cv2.COLOR_RGBA2GRAY)
                elif image_array.shape[2] == 3:
                    image_array = cv2.cvtColor(image_array, cv2.COLOR_RGB2GRAY)
                elif image_array.shape[2] == 1:
                    image_array = image_array[:, :, 0]
                else:
                    raise ValueError(f"不支持的图像通道数: {image_array.shape[2]}")
        elif image_array.ndim == 2:
            image_array = cv2.cvtColor(image_array, cv2.COLOR_GRAY2RGB)
        elif image_array.ndim == 3 and image_array.shape[2] == 4:
            image_array = image_array[:, :, :3]

        cache[cache_key] = image_array
        return image_array

    def _remember_screenshot(self, screenshot: Image | None) -> None:
        if screenshot is None:
            return
        self._latest_screenshot = screenshot
        self._latest_screenshot_monotonic = time.monotonic()

    def invalidate_screenshot_cache(self) -> None:
        """让监控线程在下一轮检查时获取新截图。"""
        with self._screenshot_lock:
            self._latest_screenshot_monotonic = 0.0
        self._mark_screenshot_dirty()

    def take_monitor_screenshot(
        self,
        gray: bool = True,
        max_age: float = 0.0,
        *,
        ensure_window_visible: bool = True,
    ) -> Image | None:
        """获取监控截图，优先复用业务线程的最近帧且不覆盖业务截图。"""
        with self._screenshot_lock:
            latest_screenshot = self._latest_screenshot
            if (
                latest_screenshot is not None
                and max_age > 0
                and time.monotonic() - self._latest_screenshot_monotonic <= max_age
                # 业务截图通常是灰度图。灰度图无法通过 convert("RGB")
                # 恢复颜色，因此彩色监控截图不能复用 L 模式缓存。
                and (gray or latest_screenshot.mode != "L")
            ):
                if gray and latest_screenshot.mode != "L":
                    return latest_screenshot.convert("L")
                return latest_screenshot

            screenshot = ScreenShot.take_screenshot(
                gray,
                ensure_window_visible=ensure_window_visible,
            )
            self._remember_screenshot(screenshot)
            return screenshot

    def check_pause(self) -> bool:
        """
        检查是否处于暂停状态

        Returns:
            bool: 是否处于暂停状态
        """
        return self.input_handler.is_pause

    def get_restore_time(self) -> float:
        """
        获取上一次结束暂停的时间
        Returns:
            float: 上一次结束暂停的时间
        """
        return self.input_handler.restore_time if self.input_handler.restore_time else 0

    def click_element(
        self,
        target,
        find_type="image",
        threshold=0.8,
        max_retries=1,
        take_screenshot=False,
        offset=True,
        action="click",
        times=1,
        dx=0,
        dy=0,
        model=None,
        my_crop=None,
        click=True,
        drag_time=None,
        interval=0.5,
    ):
        """查找并点击屏幕上的元素"""
        if model is None:
            model = self.model
        coordinates = self.find_element(
            target,
            find_type,
            threshold,
            max_retries,
            take_screenshot,
            model=model,
            my_crop=my_crop,
            additional_stack=1,
        )
        if coordinates:
            if click:
                return self.mouse_action_with_pos(
                    coordinates,
                    offset,
                    action,
                    times,
                    drag_time,
                    dx,
                    dy,
                    find_type,
                    interval,
                )
            return coordinates
        return False

    def calculate_click_position(self, coordinates, offset=True):
        """
        根据给定的坐标计算点击位置。
        参数:
        coordinates (tuple): 一个包含(x, y)坐标的元组，表示点击的位置。
        返回:
        tuple: 经过计算后的点击位置坐标。
        """
        # TODO:后续适配无需窗口设置模式
        x, y = coordinates
        screenshot = np.array(self.screenshot)
        if offset:
            x = max(0, min(screenshot.shape[1], x + random.randint(-10, 10)))
            y = max(0, min(screenshot.shape[0], y + random.randint(-10, 10)))
        return x, y

    def mouse_action_with_pos(
        self,
        coordinates,
        offset=True,
        action="click",
        times=1,
        drag_time=None,
        dx=0,
        dy=0,
        find_type=None,
        interval=0.5,
    ) -> bool:
        """
        在指定坐标上执行点击操作
        Args:
            coordinates: 坐标位置，用于计算点击位置
            offset: 是否使用偏移量计算点击位置，默认为True
            action: 鼠标操作类型，默认为"click"
            move_back: 是否在操作后将鼠标移动回原位置，默认为False
        Returns:
           bool (True) : 总是返回True表示操作执行完毕
        """
        if find_type == "image_with_multiple_targets" and len(coordinates) > 0:
            for c in coordinates:
                self.mouse_action_with_pos(
                    c,
                    offset=offset,
                    action=action,
                    times=times,
                    drag_time=drag_time,
                    dx=dx,
                    dy=dy,
                    find_type="image",
                    interval=1,
                )
            return True

        if cfg.mouse_action_interval and interval == 0.5:
            interval = cfg.mouse_action_interval

        if self.last_click_time == 0:
            self.last_click_time = time.time()
        wait_time = max(0, interval - (time.time() - self.last_click_time))
        interruptible_sleep(wait_time)

        # 计算传入的位置
        wait_for_event(self._interaction_gate, GATE_WAIT_TIMEOUT)
        x, y = self.calculate_click_position(coordinates, offset)

        # 定义鼠标操作映射
        action_map = {
            "click": self.mouse_click,
            "drag": self.mouse_drag,
            "drag_down": self.mouse_drag_down,
            "scroll": self.mouse_scroll,
        }
        # 根据操作类型执行相应的鼠标操作
        if action in action_map:
            if action == "click":
                self.mouse_click(x, y, times=times)
            elif action == "drag":
                self.mouse_drag(x, y, drag_time=drag_time, dx=dx, dy=dy)
            elif action == "drag_down":
                self.mouse_drag_down(x, y)
            elif action == "scroll":
                self.mouse_scroll()
            self.last_click_time = time.time()
        else:
            # 如果操作类型未知，抛出异常
            raise ValueError(f"未知的操作类型{action}")

        return True

    def take_screenshot(self, gray: bool = True) -> Image | None:
        """
        截取当前屏幕并返回图像对象。
        Args:
            gray (bool): 是否将图像转换为灰度图，默认为True。
        Returns:
            Image: 截取当前屏幕的图像对象
        """
        start_time = time.time()
        screenshot_interval_time = cfg.screenshot_interval if cfg.screenshot_interval else 0.85
        while True:
            try:
                check_cancelled()
                if time.time() - self.last_screenshot_time < screenshot_interval_time:
                    wait_time = max(
                        screenshot_interval_time - (time.time() - self.last_screenshot_time),
                        0,
                    )
                    interruptible_sleep(wait_time)

                with self._screenshot_lock:
                    result = ScreenShot.take_screenshot(gray)
                    self._remember_screenshot(result)
                if result:
                    with self._screenshot_lock:
                        self._set_business_screenshot(result)
                    self.last_screenshot_time = time.time()
                    return result
                else:
                    return None
            except Exception as e:
                # ``userStopError`` is an intentional control-flow signal;
                # never turn cancellation into a retry loop.
                check_cancelled()
                log.error(f"截图失败:{e}")
            interruptible_sleep(1)
            if time.time() - start_time > 60:
                log.error("截图超时，尝试重启游戏")
                import os

                from module.game_and_screen import screen

                try:
                    from module.device_manager import get_device_manager

                    selected_session = get_device_manager().active_session
                    if selected_session is not None and selected_session.target.kind in ("mumu", "adb"):
                        controller = selected_session.controller
                        if controller is not None:
                            controller.close_current_app()
                    else:
                        import win32process

                        _, pid = win32process.GetWindowThreadProcessId(screen.handle.hwnd)
                        os.system(f"taskkill /F /PID {pid}")
                except Exception:
                    pass
                from tasks.base.script_task_scheme import init_game

                init_game()
                start_time = time.time()

    def find_element(
        self,
        target,
        find_type="image",
        threshold=0.8,
        max_retries=1,
        take_screenshot=False,
        model=None,
        my_crop=None,
        min_dist=10,
        additional_stack=0,
        screenshot_image=None,
    ):
        """
        查找元素，并根据指定的查找类型执行不同的查找策略。
        Args:
            target: 查找目标，可以是图像路径或文字。(zh_cn)->en->share
            find_type: 查找类型，例如'image', 'text'等。
            threshold: 查找阈值，用于图像查找时的相似度匹配。
            max_retries: 最大重试次数。
            take_screenshot: 是否需要先截图。
            model: 查找的策略,'clam' 为在模板图片位置查找，'normal' 为模板图片位置扩大范围查找，'aggressive' 为全截屏区域查找
            my_crop: 用于限制图像或OCR识别范围的裁剪区域
            min_dist: 多目标图像查找时的NMS最小距离。
            additional_stack: 用于日志堆栈层级调整
            screenshot_image: 可选的外部截图帧；提供后不会改写业务线程当前帧。
        Returns:
            查找到的元素位置，或者在图像计数查找时返回计数。
        """
        if model is None:
            model = self.model
        # 如果不需要截图，则重试次数设置为1
        max_retries = 1 if not take_screenshot else max_retries
        for i in range(max_retries):
            if take_screenshot and screenshot_image is None:
                # 截图并根据裁剪参数获取截图结果
                while self.take_screenshot() is None:
                    continue
            # 根据查找类型执行不同的查找策略
            if find_type in ["image", "text"]:
                center = None
                if find_type in ["image"]:
                    # 使用图像查找方法查找元素
                    center = self.find_image_element(
                        target,
                        threshold,
                        model=model,
                        my_crop=my_crop,
                        additional_stack=additional_stack,
                        screenshot_image=screenshot_image,
                    )
                elif find_type == "text":
                    # 使用文本查找方法查找元素
                    center = self.find_text_element(
                        target,
                        my_crop,
                        additional_stack=additional_stack,
                        screenshot_image=screenshot_image,
                    )
                if center:
                    return center
            elif find_type in ["feature"]:
                return self.find_feature_element(
                    target,
                    my_crop,
                    additional_stack=additional_stack,
                    screenshot_image=screenshot_image,
                )
            elif find_type in ["image_with_multiple_targets"]:
                # 使用多目标图像查找方法查找元素
                return self.find_image_with_multiple_targets(
                    target,
                    threshold,
                    my_crop=my_crop,
                    min_dist=min_dist,
                    additional_stack=additional_stack,
                    screenshot_image=screenshot_image,
                )
            else:
                raise ValueError("错误的类型")

            if i < max_retries - 1:
                interruptible_sleep(1)  # 在重试前等待一定时间
        return None

    def find_image_with_multiple_targets(
        self,
        target: str,
        threshold,
        my_crop=None,
        min_dist=10,
        additional_stack=0,
        screenshot_image=None,
    ) -> List:
        """
        在当前截图中查找多个目标图像的位置
        """
        try:
            template, bbox = self._load_active_template(target)
            if template is None:
                raise ValueError("读取图片失败")
            screenshot = self._get_screenshot_array(screenshot_image, gray=True)
            crop_offset = (0, 0)
            if my_crop:
                crop_offset = (int(round(my_crop[0])), int(round(my_crop[1])))
                screenshot = ImageUtils.crop(screenshot, my_crop)
            matches = ImageUtils.match_template_with_multiple_targets(
                screenshot, template, threshold, min_dist=min_dist
            )
            if crop_offset != (0, 0):
                matches = [(x + crop_offset[0], y + crop_offset[1]) for x, y in matches]
            if len(matches) == 0:
                log.debug(f"未找到任何目标图像{target}", stacklevel=additional_stack + 3)
                return []
            else:
                log.debug(
                    f"找到{len(matches)}个目标：{matches}",
                    stacklevel=additional_stack + 3,
                )
                return matches
        except Exception as e:
            log.error(f"寻找图片出错:{e}")
            return []

    def find_str_in_text(self, target, ocr_dict):
        """
        返回目标文本的坐标
        """
        for text in ocr_dict.keys():
            if target.lower() in text.lower():
                log.debug(f"识别到目标：{text},坐标为：{ocr_dict[text]}")
                return ocr_dict[text]
            # 去除空格后再匹配，解决OCR识别结果带空格的问题（如 "HongLu" vs "Hong Lu"）
            if target.replace(" ", "").lower() in text.replace(" ", "").lower():
                log.debug(f"识别到目标（去空格匹配）：{text},坐标为：{ocr_dict[text]}")
                return ocr_dict[text]
        return False

    def _run_ocr_for_text(self, my_crop=None, only_text=False, additional_stack=0, screenshot_image=None):
        """Run OCR once for a frame/crop and reuse both text and coordinates."""
        frame_key = self._current_frame_key(screenshot_image)
        cache_key = (frame_key, self._normalize_crop(my_crop))
        ocr_cache = getattr(self, "_ocr_cache", None)
        if ocr_cache is None:
            ocr_cache = self._ocr_cache = {}

        cached = ocr_cache.get(cache_key)
        if cached is None:
            source_image = self.screenshot if screenshot_image is None else screenshot_image
            if my_crop is not None:
                if hasattr(source_image, "crop"):
                    cropped_image = source_image.crop(my_crop)
                else:
                    cropped_image = ImageUtils.crop(np.asarray(source_image), my_crop)
                ocr_result = ocr.run(cropped_image)
            else:
                ocr_result = ocr.run(source_image)

            ocr_texts = getattr(ocr_result, "txts", None)
            ocr_boxes = getattr(ocr_result, "boxes", None)
            ocr_text_list = list(ocr_texts) if ocr_texts is not None else []
            ocr_position_list = []
            for box in ocr_boxes if ocr_boxes is not None else []:
                x = (box[0][0] + box[2][0]) / 2
                y = (box[0][1] + box[2][1]) / 2
                ocr_position_list.append([x, y])
            ocr_dict = {text: position for text, position in zip(ocr_text_list, ocr_position_list)}
            cached = (ocr_dict, ocr_text_list)
            ocr_cache[cache_key] = cached

            if ocr_dict:
                log.debug(f"识别到文本及其坐标：{ocr_dict}", stacklevel=additional_stack + 3)

        ocr_dict, ocr_text_list = cached
        if only_text:
            return ocr_text_list or False
        return ocr_dict

    def _find_target_in_ocr_dict(self, target, ocr_dict, all_text=False):
        if ocr_dict == {}:
            return False
        if isinstance(target, str):
            return self.find_str_in_text(target, ocr_dict)
        elif isinstance(target, list):
            if all_text:
                for key in target:
                    if self.find_str_in_text(str(key), ocr_dict) is False:
                        return False
                return True
            for key in target:
                if result := self.find_str_in_text(str(key), ocr_dict):
                    return result
            return False
        elif isinstance(target, dict):
            for key, value in target.items():
                if position := self.find_str_in_text(str(key), ocr_dict):
                    return TextMatchResult(value=value, text=str(key), position=position)
            return None
        return False

    def find_language_text(
        self,
        zh_text,
        en_text,
        my_crop=None,
        all_text=False,
        additional_stack=0,
    ):
        """
        按当前语言状态查找中英文文本，并在语言未知时用命中结果同步语言。

        该方法只执行一次 OCR，然后在同一份 OCR 结果中匹配文本：
        - 当前语言为 zh_cn 时，只匹配 zh_text。
        - 当前语言为 en 时，只匹配 en_text。
        - 当前语言未知时，先匹配 zh_text；中文命中则同步语言为 zh_cn。
        - 中文未命中时再匹配 en_text；英文命中则同步语言为 en，并移除 zh_cn 图片路径。

        Args:
            zh_text: 中文目标文本，支持 str、list、dict，规则同 find_text_element。
            en_text: 英文目标文本，支持 str、list、dict，规则同 find_text_element。
            my_crop: OCR 裁剪区域，格式为 (x1, y1, x2, y2)；为 None 时识别整张截图。
            all_text: 当目标文本为 list 时，是否要求列表内所有关键词全部命中。
            additional_stack: 日志 stacklevel 补偿，用于让日志定位到业务调用处。

        Returns:
            文本命中结果，返回格式同 find_text_element；未命中返回 False。
        """
        ocr_dict = self._run_ocr_for_text(my_crop=my_crop, additional_stack=additional_stack)
        if ocr_dict == {}:
            return False

        if path_manager.current_language == "zh_cn":
            return self._find_target_in_ocr_dict(zh_text, ocr_dict, all_text=all_text)
        if path_manager.current_language == "en":
            return self._find_target_in_ocr_dict(en_text, ocr_dict, all_text=all_text)

        zh_result = self._find_target_in_ocr_dict(zh_text, ocr_dict, all_text=all_text)
        if zh_result is not False and zh_result is not None:
            path_manager.set_language("zh_cn", log_stacklevel=additional_stack + 4)
            return zh_result

        en_result = self._find_target_in_ocr_dict(en_text, ocr_dict, all_text=all_text)
        if en_result is not False and en_result is not None:
            path_manager.set_language("en", log_stacklevel=additional_stack + 4)
            if path_manager.eliminate_zh_cn_paths():
                self.clear_img_cache()
            return en_result

        return False

    def find_text_element(
        self,
        target,
        my_crop=None,
        all_text=False,
        only_text=False,
        additional_stack=0,
        screenshot_image=None,
    ):
        """
        寻找文本元素所在的坐标位置。

        str/list 目标返回坐标；dict 目标返回 TextMatchResult。
        """
        ocr_result = self._run_ocr_for_text(
            my_crop=my_crop,
            only_text=only_text,
            additional_stack=additional_stack,
            screenshot_image=screenshot_image,
        )
        if only_text:
            return ocr_result
        return self._find_target_in_ocr_dict(target, ocr_result, all_text=all_text)

    def get_text_from_screenshot(self, my_crop=None):
        """
        从屏幕截图中提取文字
        """
        result = self._run_ocr_for_text(my_crop=my_crop, only_text=True)
        return result if result is not False else []

    def find_feature_element(
        self,
        target,
        pic_crop=None,
        min_matches=8,
        additional_stack=0,
        screenshot_image=None,
    ):
        """
        寻找特征元素所在的坐标位置
        """
        try:
            template, template_features = self._load_feature_template(target)
            screenshot = self._get_screenshot_array(screenshot_image, gray=True)
            if cfg.set_win_size < 1440:
                screenshot = cv2.resize(
                    screenshot,
                    None,
                    fx=1440 / cfg.set_win_size,
                    fy=1440 / cfg.set_win_size,
                    interpolation=cv2.INTER_AREA,
                )
            elif cfg.set_win_size > 1440:
                screenshot = cv2.resize(
                    screenshot,
                    None,
                    fx=cfg.set_win_size / 1440,
                    fy=cfg.set_win_size / 1440,
                    interpolation=cv2.INTER_AREA,
                )
            if pic_crop:
                if cfg.set_win_size < 1440:
                    pic_crop = [int(i * 1440 / cfg.set_win_size) for i in pic_crop]
                elif cfg.set_win_size > 1440:
                    pic_crop = [int(i * cfg.set_win_size / 1440) for i in pic_crop]
                screenshot = ImageUtils.crop(screenshot, pic_crop)

            target_cache_key = (
                self._current_frame_key(screenshot_image),
                self._normalize_crop(pic_crop),
                screenshot.shape,
            )
            feature_frame_cache = getattr(self, "_feature_frame_cache", None)
            if feature_frame_cache is None:
                feature_frame_cache = self._feature_frame_cache = {}
            target_features = feature_frame_cache.get(target_cache_key)
            if target_features is None:
                target_features = ImageUtils.feature_descriptors(screenshot)
                feature_frame_cache[target_cache_key] = target_features

            result, num_matches = ImageUtils.feature_matching(
                template,
                screenshot,
                min_matches,
                template_features=template_features,
                target_features=target_features,
            )
            log.debug(
                f"匹配目标特征图片：{target.replace('./assets/images/', '')}结果{result}, 找到 {num_matches} 个匹配点",
                stacklevel=additional_stack + 3,
            )
            return result
        except Exception as e:
            error_message = str(e)
            if "cv::flann" in error_message:
                pass
            else:
                log.error(f"匹配图片特征失败:{e}")
            return None

    def clear_img_cache(self) -> None:
        """清除图片缓存"""
        self.img_cache.clear()
        getattr(self, "_feature_frame_cache", {}).clear()
        gc.collect()  # 强制垃圾回收，清理内存
        log.debug("图片缓存已清除", stacklevel=2)

    def _load_template_for_path(self, target: str, target_path: str, cacheable: bool):
        cache_key = ("template", target, target_path, cfg.set_win_size)
        if cacheable and cache_key in self.img_cache:
            cached = self.img_cache[cache_key]
            return cached["template"], cached["bbox"]

        template = ImageUtils.load_from_specific_path(target, target_path)
        if template is None:
            return None, None
        if target.endswith("assets.png"):
            bbox = ImageUtils.get_bbox(template)
            template = ImageUtils.crop(template, bbox)
        else:
            bbox = None
        if cacheable:
            self.img_cache[cache_key] = {"template": template, "bbox": bbox}
        return template, bbox

    def _load_active_template(self, target: str):
        """Load a multi-target template once for the active resource state."""
        cache_key = (
            "multiple_template",
            target,
            cfg.set_win_size,
            tuple(path_manager.pic_path),
            path_manager.current_theme,
            path_manager.current_language,
        )
        cached = self.img_cache.get(cache_key)
        if cached is not None:
            return cached["template"], cached["bbox"]

        template = ImageUtils.load_image(target)
        if template is None:
            return None, None
        bbox = None
        if target.endswith("assets.png"):
            bbox = ImageUtils.get_bbox(template)
            template = ImageUtils.crop(template, bbox)
        self.img_cache[cache_key] = {"template": template, "bbox": bbox}
        return template, bbox

    def _load_feature_template(self, target: str):
        """Load and precompute ORB descriptors for a feature template."""
        cache_key = (
            "feature_template",
            target,
            cfg.set_win_size,
            tuple(path_manager.pic_path),
            path_manager.current_theme,
            path_manager.current_language,
        )
        cached = self.img_cache.get(cache_key)
        if cached is not None:
            return cached["template"], cached["features"]

        template = ImageUtils.load_image(target, resize=False)
        if template is None:
            return None, None
        features = ImageUtils.feature_descriptors(template)
        self.img_cache[cache_key] = {"template": template, "features": features}
        return template, features

    @staticmethod
    def _is_valid_match(match_val, threshold) -> bool:
        return (
            isinstance(match_val, (int, float, np.integer, np.floating))
            and not math.isinf(match_val)
            and match_val >= threshold
        )

    MATCH_GAP = 0.15

    def _update_path_state_from_match_results(self, results, additional_stack: int = 0) -> None:
        dark_results = [result for result in results if path_manager.is_path_dark(result["path"])]
        default_results = [result for result in results if path_manager.is_path_default(result["path"])]
        zh_cn_results = [result for result in results if path_manager.is_path_zh_cn(result["path"])]
        en_results = [result for result in results if result["path"].endswith("/en")]
        share_results = [result for result in results if result["path"].endswith("/share")]

        dark_matched = any(result["matched"] for result in dark_results)
        default_matched = any(result["matched"] for result in default_results)

        path_changed = False
        if dark_matched and not default_matched:
            path_manager.set_theme("dark", log_stacklevel=additional_stack + 4)
        elif default_matched and dark_results and not dark_matched:
            path_manager.set_theme("default", log_stacklevel=additional_stack + 4)
            path_changed = path_manager.eliminate_dark_paths() or path_changed
        elif dark_matched and default_matched:
            best_dark = max(r["matchVal"] for r in dark_results if r["matched"])
            best_default = max(r["matchVal"] for r in default_results if r["matched"])
            if best_default - best_dark > self.MATCH_GAP:
                path_manager.set_theme("default", log_stacklevel=additional_stack + 4)
                path_changed = path_manager.eliminate_dark_paths() or path_changed
            elif best_dark - best_default > self.MATCH_GAP:
                path_manager.set_theme("dark", log_stacklevel=additional_stack + 4)

        zh_cn_matched = any(result["matched"] for result in zh_cn_results)
        en_matched = any(result["matched"] for result in en_results)
        share_matched = any(result["matched"] for result in share_results)

        # share 路径是语言无关资源，不能单独决定语言为英文
        if zh_cn_matched and not en_matched:
            path_manager.set_language("zh_cn", log_stacklevel=additional_stack + 4)
        elif en_matched and not zh_cn_matched:
            path_manager.set_language("en", log_stacklevel=additional_stack + 4)
            path_changed = path_manager.eliminate_zh_cn_paths() or path_changed
        elif zh_cn_matched and en_matched:
            best_zh = max(r["matchVal"] for r in zh_cn_results if r["matched"])
            best_en = max(r["matchVal"] for r in en_results if r["matched"])
            if best_en - best_zh > self.MATCH_GAP:
                path_manager.set_language("en", log_stacklevel=additional_stack + 4)
                path_changed = path_manager.eliminate_zh_cn_paths() or path_changed
            elif best_zh - best_en > self.MATCH_GAP:
                path_manager.set_language("zh_cn", log_stacklevel=additional_stack + 4)
        elif share_matched:
            # 仅命中 share 时保持当前语言未知/不变，等待后续专属语言资源判定
            pass

        if path_changed:
            self.clear_img_cache()

    @staticmethod
    def _path_state_is_known() -> bool:
        return path_manager.current_theme is not None and path_manager.current_language is not None

    def find_image_element(
        self,
        target: str,
        threshold,
        cacheable=True,
        model="clam",
        my_crop=None,
        additional_stack=0,
        screenshot_image=None,
    ):
        """
        在当前截图中查找目标图像的位置
        """
        try:
            if self.memory_protection:
                memory = psutil.virtual_memory()
                # memory.percent 直接返回当前已使用的百分比 (0.0 到 100.0)
                current_percent = memory.percent
                if current_percent > 90:
                    log.debug(f"当前系统内存总占用率: {current_percent}%，释放图片缓存")
                    self.clear_img_cache()

            existing_paths = ImageUtils.existing_image_paths(target)
            if not existing_paths:
                log.error(f"未找到图片： {target} ")
                log.debug(f"无法加载图片: {target}", stacklevel=additional_stack + 3)
                return None

            screenshot = self._get_screenshot_array(screenshot_image, gray=True)
            if my_crop:
                screenshot = ImageUtils.crop(screenshot, my_crop)

            results = []
            for loaded_path in existing_paths:
                template, bbox = self._load_template_for_path(target, loaded_path, cacheable)
                if template is None:
                    continue
                center, matchVal = ImageUtils.match_template(screenshot, template, bbox, model)
                matched = self._is_valid_match(matchVal, threshold)
                if 0.70 < matchVal < 0.90 and int(matchVal * 1000 + 1e-9) % 10 >= 5:
                    match_fmt = ".3f"
                else:
                    match_fmt = ".2f"
                log.debug(
                    f"目标图片：{target.replace('./assets/images/', '')}, 路径: {loaded_path}, 相似度：{matchVal:{match_fmt}}, 目标位置：{center}",
                    stacklevel=additional_stack + 3,
                )
                results.append(
                    {
                        "path": loaded_path,
                        "center": center,
                        "matched": matched,
                        "matchVal": matchVal,
                    }
                )
                if matched and self._path_state_is_known():
                    return center

            if not results:
                log.debug(f"无法加载图片: {target}", stacklevel=additional_stack + 3)
                return None

            self._update_path_state_from_match_results(results, additional_stack=additional_stack)
            for result in results:
                if result["matched"]:
                    return result["center"]
        except Exception as e:
            log.error(f"寻找图片失败:{e}")
        return None

    def get_screenshot_crop(self, crop):
        """
        获取指定区域的彩色截图
        """
        self.take_screenshot(False)
        screenshot = self._get_screenshot_array(gray=False)
        screenshot = screenshot[:, :, ::-1]
        screenshot = ImageUtils.crop(screenshot, crop)
        return screenshot
