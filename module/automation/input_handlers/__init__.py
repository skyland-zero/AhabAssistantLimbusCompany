from __future__ import annotations

import threading
import time

from module.logger import log


class AbstractInput:
    """输入接口类，定义输入方法的抽象接口

    Tips: 有特殊需求写在对应方法描述中
    """

    def __init__(self, cancel_event: threading.Event | None = None) -> None:
        # A Condition gives pause/resume/stop a wake-up path.  The old
        # implementation polled ``time.sleep(1)`` and could leave a stopped
        # task blocked for a full second (or longer when several waits were
        # stacked).
        self._pause_condition = threading.Condition(threading.RLock())
        self._is_pause = False
        self._cancel_event = cancel_event if cancel_event is not None else threading.Event()
        self.restore_time: float | None = None

    def _ensure_control_state(self) -> None:
        """Lazily initialize control fields for legacy ``__new__`` callers.

        A few integrations construct an input adapter with ``object.__new__``
        in order to test platform-specific methods without opening a device.
        Keeping this guard makes the new cancellation contract compatible with
        those adapters while normal construction still initializes eagerly.
        """

        if not hasattr(self, "_pause_condition"):
            self._pause_condition = threading.Condition(threading.RLock())
        if not hasattr(self, "_is_pause"):
            self._is_pause = False
        if not hasattr(self, "_cancel_event"):
            self._cancel_event = threading.Event()
        if not hasattr(self, "restore_time"):
            self.restore_time = None

    @property
    def is_pause(self) -> bool:
        """Compatibility view of the current pause state."""

        self._ensure_control_state()
        with self._pause_condition:
            return self._is_pause

    @is_pause.setter
    def is_pause(self, paused: bool) -> None:
        self.set_paused(paused)

    @property
    def paused(self) -> bool:
        """Explicit spelling used by newer execution-control callers."""

        return self.is_pause

    @property
    def cancel_event(self) -> threading.Event:
        """Event set when this adapter is requested to stop."""

        self._ensure_control_state()
        return self._cancel_event

    @cancel_event.setter
    def cancel_event(self, event: threading.Event) -> None:
        if not isinstance(event, threading.Event):
            raise TypeError("cancel_event must be a threading.Event")
        self._ensure_control_state()
        with self._pause_condition:
            self._cancel_event = event
            self._pause_condition.notify_all()

    @property
    def stop_event(self) -> threading.Event:
        """Alias for :attr:`cancel_event` used by some runner adapters."""

        return self.cancel_event

    def _external_cancelled(self) -> bool:
        """Observe the task-wide cancellation bridge when one is bound."""

        try:
            from core.execution_control import current_cancel_event

            event = current_cancel_event()
        except Exception:
            event = None
        return event is not None and event is not self.cancel_event and event.is_set()

    def cancellation_requested(self) -> bool:
        """Return whether either local or task-wide cancellation was requested."""

        return self.cancel_event.is_set() or self._external_cancelled()

    def set_paused(self, paused: bool) -> None:
        """Set an explicit pause target and wake waiters.

        ``set_pause`` remains available as a compatibility toggle, but all
        new control paths should use this idempotent method so duplicate
        pause/resume requests cannot accidentally invert one another.
        """

        if not isinstance(paused, bool):
            raise TypeError("paused must be a bool")
        self._ensure_control_state()
        with self._pause_condition:
            changed = self._is_pause != paused
            self._is_pause = paused
            self._pause_condition.notify_all()
        if changed:
            if paused:
                log.info("操作将在下一次点击时暂停")
            else:
                log.info("继续操作")

    def set_pause(self) -> None:
        """Toggle pause for legacy hotkeys.

        New code should call :meth:`set_paused` with an explicit target.
        """

        self.set_paused(not self.is_pause)

    def request_stop(self) -> None:
        """Request cooperative stop and wake both pause and sleep waiters."""

        self._ensure_control_state()
        self.cancel_event.set()
        # ``set_paused(False)`` performs the condition notification required
        # to release a thread currently blocked in ``wait_pause``.
        self.set_paused(False)

    def clear_stop(self) -> None:
        """Clear local cancellation before reusing an adapter for a new run."""

        self._ensure_control_state()
        self.cancel_event.clear()
        with self._pause_condition:
            self._pause_condition.notify_all()

    def reset_control(self) -> None:
        """Reset both cancellation and pause state for a fresh run."""

        self.clear_stop()
        self.set_paused(False)

    def wait_if_paused(self, timeout: float | None = None) -> bool:
        """Wait until resumed or cancelled, returning ``False`` on cancel.

        ``timeout`` is optional and primarily useful to callers that need a
        bounded checkpoint.  The internal polling interval is short enough to
        observe a task-wide cancellation event even when that event cannot
        directly notify this adapter's Condition.
        """

        self._ensure_control_state()
        deadline = None if timeout is None else time.monotonic() + max(0.0, float(timeout))
        announced = False
        while True:
            if self.cancellation_requested():
                return False
            with self._pause_condition:
                if not self._is_pause:
                    self.restore_time = time.time()
                    return not self.cancellation_requested()
                if not announced:
                    log.info("AALC 已暂停")
                    announced = True
                if deadline is None:
                    wait_for = 0.1
                else:
                    wait_for = min(0.1, max(0.0, deadline - time.monotonic()))
                    if wait_for <= 0:
                        return False
                self._pause_condition.wait(wait_for)
            self.restore_time = time.time()

    def wait_pause(self) -> bool:
        """Compatibility alias for :meth:`wait_if_paused`."""

        return self.wait_if_paused()

    def interruptible_sleep(self, seconds: float, *, poll_interval: float = 0.1) -> bool:
        """Sleep while remaining responsive to pause and cancellation.

        Returns ``True`` when the requested duration elapsed and ``False`` if
        cancellation interrupted the wait.  Pause state is honored so a
        gesture's settle delay cannot run ahead of a user pause request.
        """

        self._ensure_control_state()
        deadline = time.monotonic() + max(0.0, float(seconds))
        interval = max(0.01, float(poll_interval))
        while True:
            if self.cancellation_requested():
                return False
            if not self.wait_if_paused():
                return False
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                return True
            with self._pause_condition:
                # The Condition is used even while running so request_stop
                # wakes this sleep immediately instead of waiting for a
                # polling timer.
                self._pause_condition.wait(min(interval, remaining))

    def checkpoint(self) -> bool:
        """Return whether the adapter is still allowed to continue."""

        if self.cancellation_requested():
            return False
        return self.wait_if_paused()

    @staticmethod
    def _best_effort(callback, *args, **kwargs) -> None:
        """Run native cleanup without masking the original input error."""

        try:
            callback(*args, **kwargs)
        except Exception as error:  # pragma: no cover - native input boundary
            log.warning("输入释放失败（将由 CleanupLedger 继续补偿）：%s", error)

    _best_effort_release = _best_effort

    def mouse_click(self, x, y, times=1, move_back=False) -> bool:
        """在指定坐标上执行点击操作

        Args:
            x (int): x坐标
            y (int): y坐标
            times (int): 点击次数
            move_back (bool): 是否在点击后将鼠标移动回原位置
        Returns:
            bool (True) : 总是返回True表示操作执行完毕
        ---
        Extra:<br>
            输出日志: "点击位置:(x,y)"
        """
        raise InterruptedError(f"未实现的输入方法 {self.__class__.__name__}.mouse_click")

    def mouse_click_blank(self, coordinate=(1, 1), times=1, move_back=False) -> bool:
        """在空白位置点击鼠标
        Args:
            coordinate (tuple): 坐标元组 (x, y)
            times (int): 点击次数
            move_back (bool): 是否在点击后将鼠标移动回原位置
        Returns:
            bool (True) : 总是返回True表示操作执行完毕
        ---
        Extra:<br>
            输出日志: "点击（1，1）空白位置"
        """
        raise InterruptedError(f"未实现的输入方法 {self.__class__.__name__}.mouse_click_blank")

    def mouse_move(self, coordinate=(1, 1)) -> None:
        """将鼠标移动到指定坐标，不触发点击。"""

        raise InterruptedError(f"未实现的输入方法 {self.__class__.__name__}.mouse_move")

    # 普通拖拽会在终点停留后再抬起，用于确实需要“拖住”的场景。
    def mouse_drag(self, x, y, drag_time=0.1, dx=0, dy=0, move_back=True) -> None:
        """鼠标从指定位置拖动到另一个位置
        Args:
            x (int): 起始x坐标
            y (int): 起始y坐标
            drag_time (float): 拖动时间
            dx (int): x方向拖动距离
            dy (int): y方向拖动距离
            move_back (bool): 是否在拖动后将鼠标移动回原位置
        """
        raise InterruptedError(f"未实现的输入方法 {self.__class__.__name__}.mouse_drag")

    def mouse_swipe_for_scroll(self, x, y, duration=0.3, dx=0, dy=0, move_back=True) -> None:
        """各输入适配器必须实现的列表滚动手势。

        手势按下后快速脱离长按判定区域，并在到达终点后立即抬起。
        此处是接口占位；实际输入由具体适配器实现。
        """
        raise InterruptedError(f"输入适配器 {self.__class__.__name__} 未实现 mouse_swipe_for_scroll")

    def mouse_drag_down(self, x, y, reverse=1, move_back=True) -> None:
        """鼠标从指定位置向下拖动

        Args:
            x (int): x坐标
            y (int): y坐标
            reverse (int): 拖动方向，1表示向下，-1表示向上
            move_back (bool): 是否在拖动后将鼠标移动回原位置
        """
        raise InterruptedError(f"未实现的输入方法 {self.__class__.__name__}.mouse_drag_down")

    def mouse_drag_link(self, position: list, drag_time=0.1, move_back=False) -> None:
        """鼠标从指定位置拖动到指定位置
        Args:
            x (int): 起始x坐标
            y (int): 起始y坐标
            position (list): 目标位置列表
            drag_time (float): 拖动时间
        """
        raise InterruptedError(f"未实现的输入方法 {self.__class__.__name__}.mouse_drag_link")

    def mouse_scroll(self, direction: int = -3) -> bool:
        """
        进行鼠标滚动操作
        Args:
            direction (int): 滚动方向，正值表示拉近，负值表示缩小
        Returns:
            bool (True) : 表示是否支持该操作
        ---
        Extra:<br>
            如果`direction`为负数, 输出日志: "鼠标滚动滚轮，远离界面"<br>
            如果`direction`为正数, 输出日志: "鼠标滚动滚轮，拉近界面"
        """
        raise InterruptedError(f"未实现的输入方法 {self.__class__.__name__}.mouse_scroll")

    def mouse_to_blank(self, coordinate=(1, 1), move_back=False) -> None:
        """鼠标移动到空白位置，避免遮挡
        Args:
            coordinate (tuple): 坐标元组 (x, y)
            move_back (bool): 是否在移动后将鼠标移动回原位置
        ---
        Extra:<br>
            输出日志: "鼠标移动到空白，避免遮挡"
        """
        raise InterruptedError(f"未实现的输入方法 {self.__class__.__name__}.mouse_to_blank")

    def key_press(self, key: str) -> None:
        raise InterruptedError(f"未实现的输入方法 {self.__class__.__name__}.key_press")

    def input_text(self, text: str) -> None:
        raise InterruptedError(f"未实现的输入方法 {self.__class__.__name__}.input_text")
