import random
import re
from time import sleep
from typing import Callable, TypeVar

import cv2
import numpy as np
from adbutils import adb
from adbutils.errors import AdbError

from module.config import cfg
from module.logger import log

from .. import AbstractInput
from .runner_policy import RunnerDevicePolicyError, RunnerPolicy

T = TypeVar("T")

key_list = {
    "a": 29,
    "b": 30,
    "c": 31,
    "d": 32,
    "e": 33,
    "f": 34,
    "g": 35,
    "h": 36,
    "i": 37,
    "j": 38,
    "k": 39,
    "l": 40,
    "m": 41,
    "n": 42,
    "o": 43,
    "p": 44,
    "q": 45,
    "r": 46,
    "s": 47,
    "t": 48,
    "u": 49,
    "v": 50,
    "w": 51,
    "x": 52,
    "y": 53,
    "z": 54,
    "0": 7,
    "1": 8,
    "2": 9,
    "3": 10,
    "4": 11,
    "5": 12,
    "6": 13,
    "7": 14,
    "8": 15,
    "9": 16,
    "enter": 66,
    "esc": 111,
    "up": 19,
    "down": 20,
    "left": 21,
    "right": 22,
    "space": 62,
    "tab": 61,
    "shift": 59,
    "ctrl": 113,
    "alt": 57,
}


class SimulatorControl(AbstractInput):
    """通用 ADB 模拟器降级控制器（基于 adb shell input）。"""

    connection_device: SimulatorControl | None = None

    @staticmethod
    def clean_connect() -> None:
        if SimulatorControl.connection_device is None:
            return
        try:
            SimulatorControl.connection_device.adb_disconnect()
        except Exception as e:
            log.debug("断开ADB连接失败: %s", e)
        SimulatorControl.connection_device = None

    def __init__(self, endpoint: str | None = None, *, runner_policy: RunnerPolicy | None = None) -> None:
        super().__init__()
        self._runner_policy = runner_policy or RunnerPolicy.from_env()
        self.is_pause = False
        self.restore_time = None
        self.simulator_device = None
        self.simulator_max_x = 1920
        self.simulator_max_y = 1080
        self._configured_endpoint = endpoint
        self.simulator_port = endpoint
        self.game_package_name = "com.ProjectMoon.LimbusCompany"

        self.get_simulator()

    def _policy(self) -> RunnerPolicy:
        """Return the policy, including for object.__new__ test doubles."""

        policy = getattr(self, "_runner_policy", None)
        if isinstance(policy, RunnerPolicy):
            return policy
        policy = RunnerPolicy.from_env()
        self._runner_policy = policy
        return policy

    @staticmethod
    def _is_recoverable_connection_error(error: Exception) -> bool:
        message = str(error)
        return isinstance(error, AdbError) or any(
            marker in message
            for marker in (
                "device",
                "not found",
                "offline",
                "closed",
                "WinError 10054",
                "sendall",
                "意外截图",
                "截图解码失败",
                "Broken pipe",
            )
        )

    def reconnect(self, reason: str) -> bool:
        if self._policy().forbid_emulator_launch:
            raise RunnerDevicePolicyError(
                f"Runner device connection recovery is disabled after {reason}; failing closed",
                action="reconnect",
            )
        if not bool(cfg.get_value("adb_reconnect_on_error", True)):
            return False

        log.warning("检测到模拟器连接失效，正在重建ADB连接: %s", reason)
        try:
            self.adb_disconnect()
        except Exception as e:
            log.debug("断开旧ADB连接失败: %s", e)

        self.simulator_device = None
        self.simulator_port = self._configured_endpoint
        SimulatorControl.connection_device = None
        sleep(1)
        self.get_simulator()
        return True

    def _call_with_reconnect(self, action: str, func: Callable[[], T]) -> T:
        try:
            return func()
        except Exception as e:
            if not self._is_recoverable_connection_error(e):
                raise
            if self._policy().forbid_emulator_launch:
                raise RunnerDevicePolicyError(
                    f"Runner device operation {action} lost its ADB connection; automatic recovery is disabled",
                    action=action,
                ) from e
            if not self.reconnect(f"{action}: {e}"):
                raise
            log.info("模拟器连接已重建，重试操作: %s", action)
            return func()

    def start_game(self) -> None:
        def _start_game():
            if self.simulator_device is None:
                self.get_simulator()
            self.simulator_device.app_start(self.game_package_name)

        try:
            self._call_with_reconnect("启动游戏", _start_game)
        except RunnerDevicePolicyError:
            raise
        except Exception as e:
            log.error("启动游戏失败，失败原因为%s", e)
            log.error("启动游戏失败，请确认是否安装了Limbus Company，五秒后将重新尝试启动")
            try:
                packages = self._call_with_reconnect("获取应用列表", lambda: self.simulator_device.list_packages())
                log.debug("获取到的应用列表列表：%s", packages)
            except Exception as e2:
                log.error("获取应用列表失败，失败原因为%s", e2)
            sleep(5)
            self._call_with_reconnect("启动游戏", _start_game)

    def adb_connect(self) -> None:
        if self._configured_endpoint:
            self.simulator_port = self._configured_endpoint
            if ":" not in self._configured_endpoint:
                return
            endpoint = self._configured_endpoint
        else:
            port = int(cfg.simulator_port)
            if port <= 0:
                raise RuntimeError("其他模拟器需要填写 ADB 端口，例如蓝叠/雷电常见为 5555")
            endpoint = f"127.0.0.1:{port}"
            self.simulator_port = endpoint
        last_error: Exception | None = None
        for _ in range(3):
            try:
                msg = adb.connect(endpoint)
                if "connected" in msg:
                    log.debug("成功连接至:%s,连接信息: %s", endpoint, msg)
                    return
                if "bad port" in msg:
                    log.error("连接失败，端口号%s不正确，可能是拼写错误或不规范", endpoint)
            except Exception as error:
                last_error = error
                if not self._policy().forbid_emulator_launch:
                    raise
        if self._policy().forbid_emulator_launch:
            raise RunnerDevicePolicyError(
                f"Runner could not connect to ADB endpoint {endpoint}; emulator recovery is forbidden",
                action="adb_connect",
            ) from last_error

    def adb_disconnect(self) -> None:
        if not self.simulator_port or ":" not in str(self.simulator_port):
            return
        try:
            for _ in range(3):
                msg = adb.disconnect(self.simulator_port)
                if "disconnected" in msg:
                    log.debug("成功断开连接于:%s,连接信息: %s", self.simulator_port, msg)
                    break
                elif "bad port" in msg:
                    log.error("断开连接失败，端口号%s不正确，可能是拼写错误或不规范", self.simulator_port)
        except Exception:
            pass

    def get_simulator(self):
        if self.simulator_device is not None:
            return self.simulator_device

        last_error: Exception | None = None
        for attempt in range(3):
            try:
                if self.simulator_port is None or self._configured_endpoint is not None:
                    self.adb_connect()

                self.simulator_device = adb.device(self.simulator_port)

                # 提取分辨率（如 1080x1920 或 1920x1080）
                size_output = self.simulator_device.shell(["wm", "size"])
                match = re.search(r"(\d+)x(\d+)", size_output)
                if match:
                    width = int(match.group(1))
                    height = int(match.group(2))
                    self.simulator_max_x = width
                    self.simulator_max_y = height

                SimulatorControl.connection_device = self
                log.debug("连接成功，已将模拟器实例记录至 SimulatorControl.connection_device")
                return self.simulator_device
            except AdbError as e:
                last_error = e
                log.error("获取模拟器设备失败，ADB 错误: %s，正在尝试重新连接 (%d/3)", e, attempt + 1)
                if self._policy().forbid_emulator_launch:
                    raise RunnerDevicePolicyError(
                        "Runner could not resolve the ADB device; automatic reconnect is disabled",
                        action="get_simulator",
                    ) from e
                try:
                    self.adb_disconnect()
                except Exception:
                    pass
                self.simulator_device = None
                self.simulator_port = None
                sleep(1)
            except Exception as e:
                last_error = e
                log.error("初始化模拟器时出现未知异常: %s", e)
                break

        if last_error is not None:
            raise RuntimeError(f"无法连接到模拟器设备: {last_error}") from last_error

        raise RuntimeError("无法连接到模拟器设备，原因未知")

    def screenshot(self):
        def _screenshot():
            if self.simulator_device is None:
                self.get_simulator()
            data = self.simulator_device.shell(["screencap", "-p"], stream=False, encoding=None)
            if len(data) < 500:
                raise RuntimeError(f"意外截图: {data}")
            image = np.frombuffer(data, np.uint8)
            image = cv2.imdecode(image, cv2.IMREAD_COLOR)
            if image is None:
                raise RuntimeError("截图解码失败")
            return image

        return self._call_with_reconnect("截图", _screenshot)

    def mouse_click(self, x: int, y: int, times: int = 1) -> bool:
        """在指定坐标上执行点击操作。"""
        if self.simulator_device is None:
            self.get_simulator()

        log.debug("点击位置:(%d,%d)", x, y)

        def _tap():
            for _ in range(times):
                self.simulator_device.shell(f"input tap {x} {y}")
            return True

        self._call_with_reconnect("点击", _tap)
        self.wait_pause()
        return True

    def mouse_drag_down(self, x: int, y: int, reverse: int = 1) -> None:
        """向下/向上拖动手势。"""
        if self.simulator_device is None:
            self.get_simulator()
        scale = cfg.set_win_size / 1080
        self.mouse_drag(x, y, 0.4, 0, int(300 * scale * reverse))

    def mouse_scroll(self, direction: int = -3) -> bool:
        return True

    def mouse_click_blank(self, coordinate: tuple[int, int] = (1, 1), times: int = 1) -> bool:
        """在空白位置点击鼠标。"""
        x = coordinate[0] + random.randint(0, 10)
        y = coordinate[1] + random.randint(0, 10)
        for _ in range(times):
            self.mouse_click(x, y)
        self.wait_pause()
        return True

    def mouse_to_blank(self, coordinate: tuple[int, int] = (1, 1), move_back: bool = False) -> None:
        return

    def key_press(self, key: str) -> None:
        """模拟键盘输入。"""
        if self.simulator_device is None:
            self.get_simulator()
        try:
            keycode = key_list.get(key.lower(), 66)
            cmd = f"input keyevent {keycode}"
            self._call_with_reconnect("按键", lambda: self.simulator_device.shell(cmd))
        except RunnerDevicePolicyError:
            raise
        except Exception as e:
            log.error("输入失败：%s", e)

    def input_text(self, text: str) -> None:
        """输入文本。"""
        if not text:
            log.warning("未提供要输入的文本")
            return
        if self.simulator_device is None:
            self.get_simulator()
        try:
            send_text = text.replace(" ", "%s")
            self._call_with_reconnect("输入文本", lambda: self.simulator_device.shell(["input", "text", send_text]))
        except RunnerDevicePolicyError:
            raise
        except Exception as e:
            log.error("输入文本失败：%s", e)

    def mouse_drag(self, x: int, y: int, drag_time: float = 0.2, dx: int = 0, dy: int = 0) -> None:
        """鼠标拖拽。"""
        if self.simulator_device is None:
            self.get_simulator()
        pos_x_2 = x + dx
        pos_y_2 = y + dy
        duration_ms = max(50, int(drag_time * 1000))

        def _drag():
            self.simulator_device.shell(f"input swipe {x} {y} {pos_x_2} {pos_y_2} {duration_ms}")

        self._call_with_reconnect("滑动", _drag)

    def mouse_swipe_for_scroll(
        self,
        x: int,
        y: int,
        duration: float = 0.3,
        dx: int = 0,
        dy: int = 0,
        move_back: bool = True,
    ) -> None:
        """快速滚动滑动。"""
        if self.simulator_device is None:
            self.get_simulator()
        end_x = x + dx
        end_y = y + dy
        duration_ms = max(50, int(duration * 1000))

        def _swipe():
            self.simulator_device.shell(f"input swipe {x} {y} {end_x} {end_y} {duration_ms}")

        self._call_with_reconnect("快速滑动", _swipe)

    def close_current_app(self) -> None:
        if self.simulator_device is None:
            self.get_simulator()
        self._call_with_reconnect("关闭游戏", lambda: self.simulator_device.app_stop(self.game_package_name))

    def check_game_alive(self) -> bool:
        """检查游戏是否存活。"""
        if self.simulator_device is None:
            self.get_simulator()
        package = self._call_with_reconnect("检查游戏存活", lambda: self.simulator_device.app_current().package)
        return package == self.game_package_name

    def mouse_drag_link(self, position: list, drag_time: float = 0.15, move_back: bool = False) -> None:
        """多点拉链折线滑动。"""
        if not position:
            return
        if self.simulator_device is None:
            self.get_simulator()
        start = position[0]
        end = position[-1]
        duration_ms = max(50, int(drag_time * 1000 * len(position)))

        self._call_with_reconnect(
            "链式滑动",
            lambda: self.simulator_device.shell(f"input swipe {start[0]} {start[1]} {end[0]} {end[1]} {duration_ms}"),
        )
