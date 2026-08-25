"""系统通知模块（Windows Toast）。

核心层实现：不依赖 PySide6 与 UI 层。
文案翻译通过 core.i18n 的可插拔钩子完成，
“关闭 AALC”按钮通过 core.events.mediator.kill_signal 通知宿主程序退出。
"""

from .toast import (
    APPID,
    APPNAME,
    ICONPATH,
    TemplateToast,
    send_toast,
    unregister_toast,
)

__all__ = [
    "APPID",
    "APPNAME",
    "ICONPATH",
    "TemplateToast",
    "send_toast",
    "unregister_toast",
]
