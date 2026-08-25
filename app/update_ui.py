"""更新检查的界面胶水层。

纯逻辑位于 module.update.checker（核心层）；本模块只负责把检查结果
渲染为 qfluentwidgets 弹窗/信息栏，并保持旧 check_update 调用签名不变，
调用方（base_combination、resource_sync_coordinator）无需感知拆分。
"""

from typing import Callable

from PySide6.QtCore import QT_TRANSLATE_NOOP, Qt
from qfluentwidgets import InfoBarPosition

from app.card.messagebox_custom import BaseInfoBar, MessageBoxUpdate
from app.event_bridge import connect_queued
from module.update.checker import UpdateStatus, UpdateThread, start_update_thread


def handle_update_status(
    self,
    update_thread: UpdateThread,
    status: UpdateStatus,
    *,
    show_success: bool = True,
    show_failure: bool = True,
    show_update_dialog: bool = True,
):
    """根据更新状态执行默认的界面提示逻辑。

    参数:
        self: 当前界面对象，用于挂载提示框和信息栏。
        update_thread: 刚完成检查的更新线程对象。
        status: 更新检查结果状态。
        show_success: 是否显示“当前已是最新版本”的提示。
        show_failure: 是否显示“检查更新失败”的提示。
        show_update_dialog: 是否显示“发现新版本”的更新弹窗。
    """
    if status == UpdateStatus.UPDATE_AVAILABLE:
        # 第一类：发现新版本时，按策略决定是否弹出更新确认框。
        if not show_update_dialog:
            return

        messages_box = MessageBoxUpdate(update_thread.title, update_thread.content, self.window())
        if messages_box.exec():
            # 如果用户确认更新，则从指定的URL下载更新资源
            assets_url = update_thread.get_assets_url()
            if assets_url:
                start_update_thread(assets_url)
    elif status == UpdateStatus.SUCCESS:
        # 第二类：已经是最新版本时，按策略显示成功提示。
        if not show_success:
            return

        BaseInfoBar.success(
            title=QT_TRANSLATE_NOOP("BaseInfoBar", "当前是最新版本(＾∀＾●)"),
            content="",
            orient=Qt.Horizontal,
            isClosable=True,
            position=InfoBarPosition.TOP,
            duration=1000,
            parent=self,
        )
    else:
        # 第三类：检查失败时，按策略展示失败原因。
        if not show_failure:
            return

        BaseInfoBar.warning(
            title=QT_TRANSLATE_NOOP("BaseInfoBar", "检测更新失败(╥﹏╥)"),
            content=update_thread.error_msg,
            orient=Qt.Horizontal,
            isClosable=True,
            position=InfoBarPosition.TOP,
            duration=5000,
            parent=self,
        )


def check_update(
    self,
    timeout=5,
    flag=False,
    on_finished: Callable[[UpdateStatus, UpdateThread], None] | None = None,
    *,
    show_success: bool = True,
    show_failure: bool = True,
    show_update_dialog: bool = True,
):
    """启动更新检查线程，并允许调用方监听完成结果。

    参数:
        self: 当前界面对象。
        timeout: 更新检查请求的超时时间（秒）。
        flag: 是否遵循配置项 check_update 来决定是否执行检查。
        on_finished: 可选回调；线程完成后会收到状态和线程对象。
        show_success: 是否显示“当前已是最新版本”的提示。
        show_failure: 是否显示“检查更新失败”的提示。
        show_update_dialog: 是否显示“发现新版本”的更新弹窗。
    """

    # 第一步：先创建当前这一次检查专属的线程实例，避免后续再次触发检查时覆盖回调引用。
    update_thread = UpdateThread(timeout, flag)

    def handler_update(status):
        """处理线程返回的更新状态，并在需要时回调调用方。

        参数:
            status: 更新线程发回的状态枚举。
        """
        # 第一步：先执行默认的界面提示逻辑。
        handle_update_status(
            self,
            update_thread,
            status,
            show_success=show_success,
            show_failure=show_failure,
            show_update_dialog=show_update_dialog,
        )
        # 第二步：再把结果交给调用方补充后续动作，例如资源同步门禁。
        if on_finished is not None:
            on_finished(status, update_thread)

    # 第二步：仍将线程实例缓存到界面对象，保持现有调用方通过 self.update_thread 访问的行为不变。
    self.update_thread = update_thread
    # 检查在后台线程执行；通过桥接器把结果封送回主线程再弹窗（保持旧 Qt 信号语义）。
    connect_queued(update_thread.updateSignal, handler_update)
    # 启动后台更新检查线程。
    update_thread.start()

