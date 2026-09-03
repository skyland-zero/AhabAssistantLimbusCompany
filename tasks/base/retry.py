import os
import platform
import time
from typing import Callable

import psutil
import win32process

from core.execution_control import check_cancelled, interruptible_sleep
from core.execution_control import interruptible_sleep as sleep
from module.automation import auto
from module.config import cfg
from module.game_and_screen import screen
from module.logger import log
from utils.utils import check_game_running

_last_title_screen_tap_time = 0.0
_last_simulator_alive_check_time = 0.0
# P0止血：重弹窗重检查限频状态（8次未命中后1/3帧+0.8s节流）
_last_heavy_retry_ts = 0.0
_heavy_retry_miss_streak = 0


def _active_session():
    from module.device_manager import get_device_manager

    return get_device_manager().active_session


def ensure_simulator_game_started() -> bool:
    """模拟器模式下确认游戏仍在前台，不在时尝试拉起游戏。"""
    global _last_simulator_alive_check_time

    selected_session = _active_session()
    if selected_session is not None:
        target_kind = selected_session.target.kind
        if target_kind not in ("mumu", "adb"):
            return False
        connection_device = selected_session.controller
        if connection_device is None:
            raise RuntimeError("当前设备会话缺少模拟器控制器")
    else:
        if not cfg.simulator:
            return False

        if cfg.simulator_type == 0:
            from module.automation.input_handlers.simulator.mumu_control import (
                MumuControl,
            )

            connection_device = MumuControl.connection_device
        else:
            from module.automation.input_handlers.simulator.simulator_control import (
                SimulatorControl,
            )

            connection_device = SimulatorControl.connection_device

    now = time.time()
    if now - _last_simulator_alive_check_time < 5:
        return False
    _last_simulator_alive_check_time = now

    if connection_device is None:
        return False

    if connection_device.check_game_alive():
        return False

    log.info("检测到游戏未运行或不在前台，尝试自动启动游戏")
    connection_device.start_game()
    sleep(3)
    return True


def click_title_screen_safely() -> None:
    """标题页点击入口，避开账号、清缓存和中间弹窗区域。"""
    global _last_title_screen_tap_time
    selected_session = _active_session()
    selected_is_simulator = selected_session is not None and selected_session.target.kind in ("mumu", "adb")
    if (selected_session is None and not cfg.simulator) or (selected_session is not None and not selected_is_simulator):
        auto.mouse_click_blank()
        return

    now = time.time()
    if now - _last_title_screen_tap_time < 15:
        return
    _last_title_screen_tap_time = now

    height = int(cfg.set_win_size or 1080)
    width = int(height * 16 / 9)
    tap_points = ((0.86, 0.80), (0.74, 0.83), (0.91, 0.58))
    index = int(now // 10) % len(tap_points)
    x_ratio, y_ratio = tap_points[index]
    auto.mouse_click(int(width * x_ratio), int(height * y_ratio))


def kill_game():
    """关闭游戏"""
    selected_session = _active_session()
    if selected_session is not None:
        if selected_session.target.kind in ("mumu", "adb"):
            connection_device = selected_session.controller
            if connection_device is None:
                raise RuntimeError("当前设备会话缺少模拟器控制器")
            connection_device.close_current_app()
            return
    elif cfg.simulator:
        if cfg.simulator_type == 0:
            from module.automation.input_handlers.simulator.mumu_control import (
                MumuControl,
            )

            connection_device = MumuControl.connection_device
        else:
            from module.automation.input_handlers.simulator.simulator_control import (
                SimulatorControl,
            )

            connection_device = SimulatorControl.connection_device
        if connection_device is None:
            raise RuntimeError("未连接模拟器，无法关闭游戏")
        connection_device.close_current_app()
        return
    if platform.system() == "Windows":
        from module.game_and_screen import screen

        _, pid = win32process.GetWindowThreadProcessId(screen.handle.hwnd)
        os.system(f"taskkill /F /PID {pid}")
    sleep(10)
    wait_start = time.time()
    while True:
        game_running = False
        for proc in psutil.process_iter(["name"]):
            try:
                # 获取进程的可执行文件名（如 "notepad.exe"）
                proc_name = proc.info["name"]
                # 仅当遍历后找不到任何游戏进程时，才认为游戏已退出
                if proc_name and cfg.game_process_name.lower() in proc_name.lower():
                    game_running = True
                    break
            except (psutil.NoSuchProcess, psutil.AccessDenied, psutil.ZombieProcess):
                # 忽略已终止、无权限或僵尸进程
                continue
        if not game_running:
            break
        if time.time() - wait_start > 30:
            log.warning("等待游戏进程退出超时(30s)，继续后续流程")
            break
        sleep(1)


def check_times(start_time, timeout=90, logs=True):
    """检查是否卡死超时，若是则尝试关闭重启游戏"""
    now_time = time.time()
    if logs and int(now_time - start_time) > 9 and int(now_time - start_time) % 10 == 0:
        log.info(
            f"初始时间为{time.strftime('%H:%M:%S', time.localtime(start_time))}，此刻时间为{time.strftime('%H:%M:%S', time.localtime(now_time))}，已卡死{int(now_time - start_time)}秒"
        )
        sleep(1)
    if now_time - start_time > timeout:
        log.info(f"已卡死超过{timeout}秒，尝试关闭重启游戏")
        kill_game()
        restart_game()
        return True
    else:
        return False


def _current_frame_is_reusable() -> bool:
    return auto.screenshot is not None and not getattr(auto, "_frame_dirty", True)


def wait_for_ui_state(
    check: Callable[[], bool],
    timeout: float,
    *,
    screenshot_ready: bool = False,
) -> bool:
    """Poll a post-action UI state without waiting longer than ``timeout``.

    The first check can reuse a freshly captured business frame.  Every later
    check captures a new frame, so callers never make a transition decision
    from an input-invalidated screenshot.
    """

    deadline = time.monotonic() + max(0.0, float(timeout))
    reuse_frame = screenshot_ready
    while True:
        check_cancelled()
        if reuse_frame and _current_frame_is_reusable():
            reuse_frame = False
        else:
            if auto.take_screenshot() is None:
                if time.monotonic() >= deadline:
                    return False
                interruptible_sleep(min(0.2, max(0.0, deadline - time.monotonic())))
                continue
            reuse_frame = False

        if check():
            return True

        remaining = deadline - time.monotonic()
        if remaining <= 0:
            return False
        interval = cfg.screenshot_interval or 0.2
        interruptible_sleep(min(max(float(interval), 0.01), remaining))


def _retry_dialog_crop() -> tuple[int, int, int, int]:
    """获取居中重试弹窗的稍微放宽裁剪区域，避免全屏模板匹配。"""
    height = int(cfg.set_win_size or 1080)
    width = int(height * 16 / 9)
    return (
        int(width * 0.18),
        int(height * 0.15),
        int(width * 0.82),
        int(height * 0.85),
    )


def retry(*, screenshot_ready: bool = False):
    """重试连接。

    默认情况下 retry 内循环始终刷新截图，避免复用旧帧导致误判。
    调用方可以在刚完成截图且期间没有输入操作时传入
    ``screenshot_ready=True``，复用首帧以避免重复截图。
    """
    global _last_heavy_retry_ts, _heavy_retry_miss_streak
    start_time = time.time()
    selected_session = _active_session()
    is_windows = selected_session.target.kind == "pc" if selected_session is not None else not cfg.config.simulator
    if is_windows:
        saved_hwnd = screen.handle.hwnd
    reuse_frame = screenshot_ready
    while True:
        if ensure_simulator_game_started():
            start_time = time.time()
            reuse_frame = False
            continue
        if is_windows and screen.handle.hwnd != saved_hwnd:
            # 句柄发生变化则重置初始时间, 以免误判卡死
            saved_hwnd = screen.handle.hwnd
            start_time = time.time()
        if auto.get_restore_time() is not None:
            start_time = max(start_time, auto.get_restore_time())
        if check_times(start_time):
            return False
        if reuse_frame and _current_frame_is_reusable():
            reuse_frame = False
        else:
            if auto.take_screenshot() is None:
                continue
            reuse_frame = False
        if auto.find_element("base/connecting_assets.png"):
            reuse_frame = False
            continue
        # P0止血：重弹窗为稀有事件，连续未命中时限频，避免每帧230ms空转
        heavy_due = True
        if _heavy_retry_miss_streak >= 8:
            # 8次未命中后，仅3帧一次且距上次重检查>=0.8s才查，正常态每帧必查保证不漏弹窗
            if time.monotonic() - _last_heavy_retry_ts < 0.8:
                heavy_due = False
            elif _heavy_retry_miss_streak % 3 != 0:
                heavy_due = False
        if heavy_due:
            dialog_crop = _retry_dialog_crop()
            if position := auto.find_element("base/retry_countdown.png", my_crop=dialog_crop):
                sleep(5)
                auto.mouse_click(position[0], position[1], times=3)
                reuse_frame = False
                _heavy_retry_miss_streak = 0
                _last_heavy_retry_ts = time.monotonic()
                continue
            if auto.click_element("base/retry.png", threshold=0.9, my_crop=dialog_crop):
                auto.mouse_to_blank()
                reuse_frame = False
                _heavy_retry_miss_streak = 0
                _last_heavy_retry_ts = time.monotonic()
                continue
            # 原三重OR冗余：仅保留try_again -> click retry，避免重复查countdown/retry
            if auto.find_element("base/try_again.png", my_crop=dialog_crop):
                auto.click_element("base/retry.png", threshold=0.9, my_crop=dialog_crop)
                reuse_frame = False
                _heavy_retry_miss_streak = 0
                _last_heavy_retry_ts = time.monotonic()
                continue
            # 未命中：计入miss并刷新时间戳，下次节流以此刻为准
            _heavy_retry_miss_streak += 1
            _last_heavy_retry_ts = time.monotonic()
        else:
            # 节流跳过帧也计miss
            _heavy_retry_miss_streak += 1
        if auto.find_element("base/clear_all_caches_assets.png", model="clam"):
            if auto.click_element("base/update_confirm_assets.png"):
                reuse_frame = False
                continue
            click_title_screen_safely()
            reuse_frame = False
            continue
        if auto.click_element("base/only_option_assets.png", model="clam"):
            sleep(5)
            if not check_game_running():
                log.debug("检测到游戏未运行，调用 init_game() 重新初始化")
                from tasks.base.script_task_scheme import init_game

                init_game()
            reuse_frame = False
            continue
        break


def restart_game():
    """重启游戏"""
    from tasks.base.back_init_menu import back_init_menu
    from tasks.base.script_task_scheme import init_game

    init_game()
    sleep(3)
    back_init_menu()
