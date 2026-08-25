"""框架无关的事件总线。

用纯 Python 实现 Qt ``Signal`` 的常用接口（``connect`` / ``disconnect`` /
``emit``），使核心业务代码可以在不依赖 PySide6 的情况下发布和订阅事件。

与 Qt Signal 的语义差异：
- 处理器在**发射者所在线程**被同步调用，没有 Qt 的接收者线程亲和性。
  UI 层订阅可能由后台线程发射的事件时，必须使用 app.event_bridge.connect_queued
  将回调封送回 GUI 主线程。

为兼容历史调用点，单例实例沿用了 ``mediator`` 这一名称；
app 包会将其再次导出，UI 层代码无需感知来源变化。
"""

from __future__ import annotations

import logging
from threading import RLock
from typing import Any, Callable

logger = logging.getLogger("AALC")


class Event:
    """带 Qt Signal 风格接口的多播事件。

    用法::

        value_changed = Event(int)
        value_changed.connect(handler)   # 订阅
        value_changed.emit(42)           # 发布（同步调用所有处理器）
    """

    __slots__ = ("_name", "_lock", "_handlers")

    def __init__(self, name: str = "") -> None:
        self._name = name or self.__class__.__qualname__
        self._lock = RLock()
        self._handlers: list[Callable[..., Any]] = []

    @property
    def name(self) -> str:
        return self._name

    def connect(self, handler: Callable[..., Any]) -> Callable[..., Any]:
        """注册处理器；返回传入的处理器以便装饰器式使用。"""
        with self._lock:
            if handler not in self._handlers:
                self._handlers.append(handler)
        return handler

    def disconnect(self, handler: Callable[..., Any] | None = None) -> None:
        """注销处理器；handler 为 None 时清空全部处理器。"""
        with self._lock:
            if handler is None:
                self._handlers.clear()
                return
            try:
                self._handlers.remove(handler)
            except ValueError:
                logger.debug(f"事件 {self._name} 注销了未注册的处理器: {handler}")

    def emit(self, *args: Any, **kwargs: Any) -> None:
        """同步调用全部处理器。

        单个处理器的异常不会影响其他处理器，异常只记录日志。
        """
        with self._lock:
            handlers = list(self._handlers)
        for handler in handlers:
            try:
                handler(*args, **kwargs)
            except Exception:
                logger.exception(f"事件 {self._name} 的处理器 {getattr(handler, '__name__', handler)} 抛出异常")

    def __repr__(self) -> str:  # pragma: no cover - 调试辅助
        return f"<Event {self._name} handlers={len(self._handlers)}>"


class EventBus:
    """集中声明项目全部跨层事件的容器。

    字段名与旧 Mediator 的信号名一一对应，替换后调用点无需修改。
    """

    # ---- 页面导航 / 队伍设置（UI 层内部）----
    switch_page = Event("switch_page")
    switch_team_setting = Event("switch_team_setting")
    delete_team_setting = Event("delete_team_setting")
    team_setting = Event("team_setting")
    close_setting = Event("close_setting")
    refresh_teams_order = Event("refresh_teams_order")
    autodaily_setting = Event("autodaily_setting")
    sinner_be_selected = Event("sinner_be_selected")

    # ---- 脚本控制 ----
    link_start = Event("link_start")
    save_warning = Event("save_warning")
    tasks_warning = Event("tasks_warning")
    warning = Event("warning")
    finished_signal = Event("finished_signal")
    script_finished = Event("script_finished")
    kill_signal = Event("kill_signal")
    pause_resume = Event("pause_resume")
    request_focus = Event("request_focus")

    # ---- 更新下载进度 ----
    update_progress = Event("update_progress")
    download_complete = Event("download_complete")
    hdr_warning = Event("hdr_warning")

    # ---- 镜牢进度条 ----
    mirror_signal = Event("mirror_signal")  # (当前次数, 总次数)
    mirror_bar_kill_signal = Event("mirror_bar_kill_signal")

    # ---- 快捷键监听 ----
    hotkey_listener_stop_signal = Event("hotkey_listener_stop_signal")
    hotkey_listener_start_signal = Event("hotkey_listener_start_signal")


# 全局唯一事件总线。保留 mediator 名称以兼容既有调用点。
mediator = EventBus()
