"""把 core.events 事件封送回 Qt GUI 主线程的桥接器。

core.events.Event 的处理器在发射者线程被同步调用；而后台线程（脚本线程、
下载线程、快捷键监听线程等）发射的事件最终要更新 QWidget。本模块提供一个
转发器：在发射线程只做一次 ``Signal(object)`` 发射，借助 Qt 的自动排队
连接把真正处理派发到接收者所属的主线程——与旧 QThread 信号的语义一致。
"""

from __future__ import annotations

from typing import Any, Callable

from PySide6.QtCore import QObject, Signal

from core.events import Event


class _QueuedForwarder(QObject):
    """持有 Qt 排队连接的转发对象。

    forward 在发射线程被 core Event 调用，仅打包参数并发信号；
    _dispatch 由 Qt 调度到本对象所属线程（主线程）后执行真正的槽函数。
    """

    _fired = Signal(object)

    def __init__(self, slot: Callable[..., Any]):
        super().__init__()
        self._slot = slot
        # AutoConnection：同线程直呼，跨线程自动排队到主线程。
        self._fired.connect(self._dispatch)

    def forward(self, *args: Any, **kwargs: Any) -> None:
        self._fired.emit((args, kwargs))

    def _dispatch(self, payload: object) -> None:
        args, kwargs = payload  # type: ignore[misc]
        self._slot(*args, **kwargs)


def connect_queued(event: Event, slot: Callable[..., Any]) -> _QueuedForwarder:
    """订阅事件并把槽函数调度回主线程执行。

    返回转发器实例；调用方需持有引用防止被垃圾回收后断开连接。
    """
    forwarder = _QueuedForwarder(slot)
    event.connect(forwarder.forward)
    return forwarder
