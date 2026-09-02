from __future__ import annotations

import random
import socket
import struct
import subprocess
import sys
import threading
import time
from pathlib import Path
from typing import TypeVar

import av
import numpy as np
from adbutils import AdbDevice, adb

from module.config import cfg
from module.logger import log

from .. import AbstractInput

T = TypeVar("T")

# Scrcpy protocol constants (v2.4)
SC_CONTROL_MSG_TYPE_INJECT_KEYCODE = 0
SC_CONTROL_MSG_TYPE_INJECT_TEXT = 1
SC_CONTROL_MSG_TYPE_INJECT_TOUCH_EVENT = 2
SC_CONTROL_MSG_TYPE_INJECT_SCROLL_EVENT = 3

AMOTION_EVENT_ACTION_DOWN = 0
AMOTION_EVENT_ACTION_UP = 1
AMOTION_EVENT_ACTION_MOVE = 2
AMOTION_EVENT_BUTTON_PRIMARY = 1

AKEY_EVENT_ACTION_DOWN = 0
AKEY_EVENT_ACTION_UP = 1

# POINTER_ID_GENERIC_FINGER = -2 (0xFFFFFFFFFFFFFFFE)
POINTER_ID_GENERIC_FINGER = 0xFFFFFFFFFFFFFFFE

ANDROID_KEY_MAP = {
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
    "esc": 4,  # Android Back key
    "back": 4,
    "home": 3,
    "menu": 82,
    "space": 62,
    "tab": 61,
    "shift": 59,
    "ctrl": 113,
    "alt": 57,
    "up": 19,
    "down": 20,
    "left": 21,
    "right": 22,
}


def _resolve_scrcpy_server_jar() -> Path:
    """Find the bundled scrcpy-server.jar path."""
    candidates = [
        Path(getattr(sys, "_MEIPASS", "")) / "assets" / "binary" / "scrcpy-server.jar",
        Path(__file__).resolve().parents[4] / "assets" / "binary" / "scrcpy-server.jar",
        Path("assets") / "binary" / "scrcpy-server.jar",
    ]
    for candidate in candidates:
        if candidate.is_file():
            return candidate.resolve()
    raise FileNotFoundError("未找到 scrcpy-server.jar 文件，请检查 assets/binary 目录")


def _recv_exact(sock: socket.socket | None, n: int) -> bytes:
    """从 Socket 中准确读取指定字节数（兼容 Windows Winsock，避免 settimeout 下使用 MSG_WAITALL 产生 WinError 10045）。"""
    if sock is None or n <= 0:
        return b""
    buf = bytearray()
    while len(buf) < n:
        chunk = sock.recv(n - len(buf))
        if not chunk:
            break
        buf.extend(chunk)
    return bytes(buf)


class ScrcpyControl(AbstractInput):
    """基于 scrcpy-server 的高性能 Android 真机与第三方模拟器控制器。

    具备极低延迟 H.264 视频流解码（单帧覆盖，零内存堆积）以及 Control Socket 毫秒级原生触控/按键注入。
    """

    connection_device: ScrcpyControl | None = None

    def __init__(self, endpoint: str | None = None) -> None:
        super().__init__()
        self.endpoint = endpoint
        self.serial = endpoint
        self.device: AdbDevice | None = None
        self.game_package_name = "com.ProjectMoon.LimbusCompany"

        self._running = False
        self._video_socket: socket.socket | None = None
        self._control_socket: socket.socket | None = None
        self._control_lock = threading.Lock()
        self._forward_port: int | None = None
        self._server_proc: subprocess.Popen | None = None
        self._decode_thread: threading.Thread | None = None

        self._latest_frame: np.ndarray | None = None
        self._frame_lock = threading.Lock()
        self._first_frame_ready = threading.Event()

        self.resolution: tuple[int, int] = (1920, 1080)
        self.device_name: str = ""

        self._start()
        ScrcpyControl.connection_device = self

    @classmethod
    def clean_connect(cls) -> None:
        if cls.connection_device is not None:
            try:
                cls.connection_device.stop()
            except Exception as e:
                log.debug("清理 Scrcpy 连接失败：%s", e)
            cls.connection_device = None

    def _ensure_device_connected(self) -> AdbDevice:
        """Connect and resolve the target AdbDevice."""
        if not self.serial:
            devices = adb.device_list()
            if not devices:
                raise RuntimeError("未发现任何已连接的 ADB 设备")
            self.device = devices[0]
            self.serial = self.device.serial
            return self.device

        if ":" in self.serial:
            # Network device
            try:
                adb.connect(self.serial)
            except Exception as error:
                log.debug("ADB 连接网络端点失败（%s）：%s", self.serial, error)

        self.device = adb.device(self.serial)
        return self.device

    def _start(self) -> None:
        log.info("正在初始化 Scrcpy 连接：%s", self.serial or "默认设备")
        self._ensure_device_connected()
        assert self.device is not None

        jar_path = _resolve_scrcpy_server_jar()
        remote_jar = "/data/local/tmp/scrcpy-server.jar"

        # 1. 推送 scrcpy-server.jar 到设备
        try:
            self.device.sync.push(str(jar_path), remote_jar)
        except Exception as error:
            raise RuntimeError(f"推送 scrcpy-server.jar 到设备失败：{error}") from error

        # 2. 建立本地端口映射
        try:
            self._forward_port = self.device.forward_port("localabstract:scrcpy")
        except Exception as error:
            raise RuntimeError(f"创建 ADB 端口映射失败：{error}") from error

        # 3. 启动设备端 scrcpy-server 守护进程
        adb_bin = "adb"
        try:
            import adbutils

            adb_bin = adbutils.adb_path()
        except Exception:
            pass

        shell_cmd = (
            f"CLASSPATH={remote_jar} app_process / com.genymobile.scrcpy.Server "
            "2.4 log_level=info audio=false control=true tunnel_forward=true "
            "max_size=1920 video_bit_rate=8000000 stay_awake=true cleanup=false "
            "power_off_on_close=false downsize_on_error=true"
        )

        server_args = [
            adb_bin,
            "-s",
            self.device.serial,
            "shell",
            shell_cmd,
        ]

        no_window = getattr(subprocess, "CREATE_NO_WINDOW", 0x08000000)
        self._server_proc = subprocess.Popen(
            server_args,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            creationflags=no_window,
        )

        time.sleep(0.2)

        # 4. 连接 Video Socket 与 Control Socket（重试直至 abstract socket 就绪）
        try:
            connected = False
            last_conn_err: Exception | None = None
            for _ in range(25):
                try:
                    self._video_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                    self._video_socket.settimeout(2.0)
                    self._video_socket.connect(("127.0.0.1", self._forward_port))
                    dummy = _recv_exact(self._video_socket, 1)
                    if dummy == b"\x00":
                        connected = True
                        break
                    self._video_socket.close()
                    self._video_socket = None
                except Exception as e:
                    last_conn_err = e
                    if self._video_socket:
                        try:
                            self._video_socket.close()
                        except Exception:
                            pass
                        self._video_socket = None
                time.sleep(0.1)

            if not connected:
                raise ConnectionError(f"Scrcpy 握手失败：未收到服务端就绪信号（{last_conn_err}）")

            self._control_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
            self._control_socket.settimeout(3.0)
            self._control_socket.connect(("127.0.0.1", self._forward_port))

            # 读取设备名称（64 字节）
            dev_name_bytes = _recv_exact(self._video_socket, 64)
            self.device_name = dev_name_bytes.decode("utf-8", errors="ignore").rstrip("\x00")

            # 读取视频编码格式（4 字节）与屏幕宽高（8 字节）
            codec_bytes = _recv_exact(self._video_socket, 4)
            codec_id = codec_bytes.decode("utf-8", errors="ignore").strip("\x00") or "h264"

            res_bytes = _recv_exact(self._video_socket, 8)
            if len(res_bytes) == 8:
                width, height = struct.unpack(">II", res_bytes)
                self.resolution = (width, height)
            log.info(
                "Scrcpy 连接成功：%s，编码：%s，分辨率：%dx%d",
                self.device_name or self.serial,
                codec_id,
                self.resolution[0],
                self.resolution[1],
            )
        except Exception as error:
            self.stop()
            raise RuntimeError(f"Scrcpy Socket 通信建立失败：{error}") from error

        # 5. 启动后台单帧解码线程
        self._running = True
        self._video_socket.settimeout(2.0)
        self._decode_thread = threading.Thread(
            target=self._decode_loop,
            name=f"ScrcpyDecode-{self.serial}",
            daemon=True,
        )
        self._decode_thread.start()

        # 等待首帧接收与解码成功才算连接完成（超时未就绪直接视为连接失败）
        if not self._first_frame_ready.wait(timeout=5.0):
            self.stop()
            raise ConnectionError(f"Scrcpy 连接失败：未能在 5 秒内获取到设备首帧画面（{self.serial}）")

        log.info(
            "Scrcpy 首帧获取成功，设备连接就绪：%s (%dx%d)",
            self.device_name or self.serial,
            self.resolution[0],
            self.resolution[1],
        )

    @staticmethod
    def _frame_to_rgb(frame) -> np.ndarray:
        """Convert a decoded PyAV frame to the controller's RGB contract."""
        return frame.to_ndarray(format="rgb24")

    def _decode_loop(self) -> None:
        """后台持续读取 H.264 视频流并解码为 RGB 图像，单帧覆盖式更新。"""
        codec = av.CodecContext.create("h264", "r")

        while self._running:
            try:
                # 读取 12 字节 Packet 头部：8 字节 PTS + 4 字节 Payload 大小
                header = _recv_exact(self._video_socket, 12)
                if len(header) < 12:
                    if not self._running:
                        break
                    continue

                _, size = struct.unpack(">QI", header)
                if size == 0:
                    continue

                raw_packet = _recv_exact(self._video_socket, size)
                if len(raw_packet) < size:
                    continue

                packets = codec.parse(raw_packet)
                if not packets:
                    continue

                for packet in packets:
                    frames = codec.decode(packet)
                    for frame in frames:
                        # Scrcpy/MuMu 控制器的截图契约统一为 RGB。PIL、OCR、模板
                        # 灰度化以及 GPUI 预览都按 RGB 解释这个数组。
                        rgb_image = self._frame_to_rgb(frame)
                        with self._frame_lock:
                            self._latest_frame = rgb_image
                        if not self._first_frame_ready.is_set():
                            self._first_frame_ready.set()

            except (socket.timeout, socket.error):
                if not self._running:
                    break
                continue
            except Exception as error:
                if self._running:
                    log.debug("Scrcpy 视频解码异常：%s", error)
                time.sleep(0.01)

    def screenshot(self) -> np.ndarray:
        """获取当前最新画面（RGB 格式 ndarray，与 MuMu 图像管线一致）。

        严格等待 Scrcpy 视频流首帧就绪，不降级至 slow adb screencap。
        """
        if not self._first_frame_ready.wait(timeout=3.0):
            raise RuntimeError(f"Scrcpy 视频流等待超时 ({self.serial})，未能接收到画面帧")

        with self._frame_lock:
            if self._latest_frame is None:
                raise RuntimeError("当前无可用 Scrcpy 视频帧")
            return self._latest_frame.copy()

    def _send_control(self, payload: bytes) -> None:
        """线程安全地向 Control Socket 发送二进制指令。"""
        if not self._control_socket or not self._running:
            return
        with self._control_lock:
            try:
                self._control_socket.sendall(payload)
            except Exception as error:
                log.debug("发送 Scrcpy 控制指令失败：%s", error)

    def _build_touch_msg(self, action: int, x: int, y: int) -> bytes:
        """构建 SC_CONTROL_MSG_TYPE_INJECT_TOUCH_EVENT 二进制消息 (32 字节)。"""
        width, height = self.resolution
        pressure = 0xFFFF if action in (AMOTION_EVENT_ACTION_DOWN, AMOTION_EVENT_ACTION_MOVE) else 0x0000
        btn = AMOTION_EVENT_BUTTON_PRIMARY if action in (AMOTION_EVENT_ACTION_DOWN, AMOTION_EVENT_ACTION_MOVE) else 0

        return struct.pack(
            ">BBQiiHHHii",
            SC_CONTROL_MSG_TYPE_INJECT_TOUCH_EVENT,
            action,
            POINTER_ID_GENERIC_FINGER,
            int(x),
            int(y),
            int(width),
            int(height),
            pressure,
            btn,
            btn,
        )

    def mouse_click(self, x: int, y: int, times: int = 1, move_back: bool = False) -> bool:
        """执行指定坐标点击。"""
        log.debug("Scrcpy 点击位置：(%d, %d)", x, y)
        for _ in range(times):
            self._send_control(self._build_touch_msg(AMOTION_EVENT_ACTION_DOWN, x, y))
            time.sleep(0.04)
            self._send_control(self._build_touch_msg(AMOTION_EVENT_ACTION_UP, x, y))
            if times > 1:
                time.sleep(0.05)
        self.wait_pause()
        return True

    def mouse_click_blank(self, coordinate: tuple[int, int] = (1, 1), times: int = 1) -> bool:
        """在空白位置执行点击。"""
        log.debug("Scrcpy 点击空白位置")
        x = coordinate[0] + random.randint(0, 10)
        y = coordinate[1] + random.randint(0, 10)
        return self.mouse_click(x, y, times=times)

    def mouse_move(self, coordinate: tuple[int, int] = (1, 1)) -> None:
        """移动位置（发送 MOVE 事件）。"""
        self._send_control(self._build_touch_msg(AMOTION_EVENT_ACTION_MOVE, coordinate[0], coordinate[1]))

    def mouse_to_blank(self, coordinate: tuple[int, int] = (1, 1), move_back: bool = False) -> None:
        """触控屏设备无需移动鼠标避开遮挡（空操作）。"""
        return

    def mouse_drag(
        self,
        x: int,
        y: int,
        drag_time: float = 0.2,
        dx: int = 0,
        dy: int = 0,
        move_back: bool = True,
    ) -> None:
        """从 (x, y) 拖拽滑动至 (x+dx, y+dy)。"""
        end_x = max(0, min(self.resolution[0], x + dx))
        end_y = max(0, min(self.resolution[1], y + dy))

        steps = max(5, int(drag_time * 60))
        interval = max(0.005, drag_time / steps)

        self._send_control(self._build_touch_msg(AMOTION_EVENT_ACTION_DOWN, x, y))
        time.sleep(0.02)

        for i in range(1, steps + 1):
            cur_x = int(x + (end_x - x) * (i / steps))
            cur_y = int(y + (end_y - y) * (i / steps))
            self._send_control(self._build_touch_msg(AMOTION_EVENT_ACTION_MOVE, cur_x, cur_y))
            time.sleep(interval)

        self._send_control(self._build_touch_msg(AMOTION_EVENT_ACTION_UP, end_x, end_y))
        self.wait_pause()

    def mouse_swipe_for_scroll(
        self,
        x: int,
        y: int,
        duration: float = 0.3,
        dx: int = 0,
        dy: int = 0,
        move_back: bool = True,
    ) -> None:
        """列表滚动手势。"""
        self.mouse_drag(x, y, drag_time=duration, dx=dx, dy=dy, move_back=move_back)

    def mouse_drag_down(self, x: int, y: int, reverse: int = 1, move_back: bool = True) -> None:
        """向下/向上拖动手势。"""
        scale = cfg.set_win_size / 1080
        self.mouse_drag(x, y, drag_time=0.4, dx=0, dy=int(300 * scale * reverse), move_back=move_back)

    def mouse_drag_link(self, position: list, drag_time: float = 0.1, move_back: bool = False) -> None:
        """按路径多点连续拖拽。"""
        if not position:
            return
        start_x, start_y = position[0]
        self._send_control(self._build_touch_msg(AMOTION_EVENT_ACTION_DOWN, start_x, start_y))
        time.sleep(0.02)

        for target in position[1:]:
            tx, ty = target
            self._send_control(self._build_touch_msg(AMOTION_EVENT_ACTION_MOVE, tx, ty))
            time.sleep(drag_time)

        last_x, last_y = position[-1]
        self._send_control(self._build_touch_msg(AMOTION_EVENT_ACTION_UP, last_x, last_y))
        self.wait_pause()

    def mouse_scroll(self, direction: int = -3) -> bool:
        """鼠标滚轮事件注入。"""
        cx, cy = self.resolution[0] // 2, self.resolution[1] // 2
        vscroll = 0x7FFF if direction > 0 else 0x8000
        msg = struct.pack(
            ">BiiHHhhi",
            SC_CONTROL_MSG_TYPE_INJECT_SCROLL_EVENT,
            cx,
            cy,
            self.resolution[0],
            self.resolution[1],
            0,
            vscroll,
            0,
        )
        self._send_control(msg)
        return True

    def key_press(self, key: str) -> None:
        """模拟按键按下与抬起。"""
        keycode = ANDROID_KEY_MAP.get(key.lower())
        if keycode is None:
            log.warning("未知按键：%s，忽略按键注入", key)
            return

        down_msg = struct.pack(">BBiII", SC_CONTROL_MSG_TYPE_INJECT_KEYCODE, AKEY_EVENT_ACTION_DOWN, keycode, 0, 0)
        up_msg = struct.pack(">BBiII", SC_CONTROL_MSG_TYPE_INJECT_KEYCODE, AKEY_EVENT_ACTION_UP, keycode, 0, 0)
        self._send_control(down_msg)
        time.sleep(0.03)
        self._send_control(up_msg)

    def input_text(self, text: str) -> None:
        """注入文本。"""
        if not text:
            return
        text_bytes = text.encode("utf-8")
        msg = struct.pack(">BI", SC_CONTROL_MSG_TYPE_INJECT_TEXT, len(text_bytes)) + text_bytes
        self._send_control(msg)

    def start_game(self) -> None:
        """启动游戏 App。"""
        if self.device is None:
            self._ensure_device_connected()
        assert self.device is not None
        log.info("正在启动游戏：%s", self.game_package_name)
        self.device.app_start(self.game_package_name)

    def stop_game(self) -> None:
        """停止游戏 App。"""
        if self.device is None:
            self._ensure_device_connected()
        assert self.device is not None
        log.info("正在停止游戏：%s", self.game_package_name)
        self.device.app_stop(self.game_package_name)

    def close_current_app(self) -> None:
        """关闭当前游戏（生命周期接口对齐）。"""
        self.stop_game()

    def get_current_package(self) -> str:
        """获取当前前台应用包名。"""
        if self.device is None:
            self._ensure_device_connected()
        if self.device is None:
            return ""
        for attempt in range(3):
            try:
                app_info = self.device.app_current()
                current_package = getattr(app_info, "package", "") or ""
                log.debug("当前应用包名: %s", current_package)
                return current_package
            except Exception as error:
                log.debug("获取当前应用包名错误 (%d/3): %s", attempt + 1, error)
                if attempt < 2:
                    time.sleep(0.5)
        return ""

    def check_game_alive(self) -> bool:
        """检查游戏是否存活且处于前台（支持悬浮窗/画中画/无障碍层共存）。"""
        if self.device is None:
            self._ensure_device_connected()
        if self.device is None:
            return False

        # 1. 快速检查 app_current
        try:
            app_info = self.device.app_current()
            current_package = getattr(app_info, "package", "") or ""
            if current_package == self.game_package_name:
                return True
        except Exception:
            pass

        # 2. 检查游戏进程是否存活
        try:
            pid_output = (self.device.shell(["pidof", self.game_package_name]) or "").strip()
            if not pid_output:
                return False
        except Exception:
            try:
                ps_output = self.device.shell(f"ps -ef | grep {self.game_package_name} | grep -v grep") or ""
                if self.game_package_name not in ps_output:
                    return False
            except Exception:
                return False

        # 3. 进程存活时，排查是否被悬浮窗/画中画/辅助工具（如 ChatGPT、系统助手等）抢占焦点
        try:
            # 检查 WindowManager 的顶层窗口/全屏活动
            focus_output = self.device.shell("dumpsys window | grep -E 'mFocusedApp|mCurrentFocus|mTopFullscreenOpaqueWindowState'") or ""
            if self.game_package_name in focus_output:
                return True

            # 检查前台活动栈顶
            activity_output = self.device.shell("dumpsys activity top") or ""
            if self.game_package_name in activity_output:
                return True
        except Exception as error:
            log.debug("检查前台活动栈异常：%s", error)

        return False

    def adb_disconnect(self) -> None:
        """断开控制器（生命周期接口对齐）。"""
        self.stop()

    def stop(self) -> None:
        """彻底停止 Scrcpy 客户端并完全释放所有线程与 Socket 资源。"""
        self._running = False
        self._first_frame_ready.clear()

        # 1. 唤醒并等待解码线程退出
        if self._video_socket:
            try:
                self._video_socket.shutdown(socket.SHUT_RDWR)
                self._video_socket.close()
            except Exception:
                pass
            self._video_socket = None

        if self._decode_thread and self._decode_thread.is_alive():
            self._decode_thread.join(timeout=1.0)
            self._decode_thread = None

        # 2. 关闭控制 Socket
        if self._control_socket:
            try:
                self._control_socket.shutdown(socket.SHUT_RDWR)
                self._control_socket.close()
            except Exception:
                pass
            self._control_socket = None

        # 3. 移除端口映射
        if self.device and self._forward_port:
            try:
                self.device.forward_remove(f"tcp:{self._forward_port}")
            except Exception:
                pass
            self._forward_port = None

        # 4. 终止设备端 server 进程
        if self._server_proc:
            try:
                self._server_proc.terminate()
                self._server_proc.wait(timeout=1.0)
            except Exception:
                pass
            self._server_proc = None

        # 5. 清理内存中保存的最新帧
        with self._frame_lock:
            self._latest_frame = None

        if ScrcpyControl.connection_device is self:
            ScrcpyControl.connection_device = None

        log.info("Scrcpy 控制器已完全释放")
