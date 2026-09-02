from __future__ import annotations

import random
import socket
import struct
import subprocess
import sys
import threading
import time
import zlib
from pathlib import Path
from typing import TypeVar

import av
import cv2
import numpy as np
from adbutils import AdbDevice, adb

from module.config import cfg
from module.logger import log

from .. import AbstractInput
from . import insert_swipe

T = TypeVar("T")

# Scrcpy protocol constants (v4.1)
SCRCPY_VERSION = "4.1"
SCRCPY_VIDEO_CODEC = "h264"
SCRCPY_MAX_FPS = 15
SCRCPY_VIDEO_BIT_RATE = 8_000_000
SCRCPY_VIDEO_SESSION_META_FLAG = 1 << 31

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
        self._last_hash: int | None = None
        self._last_pil = None
        self._last_request_mono: float = 0.0

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

        shell_cmd = self._build_server_shell_command(remote_jar)

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

            # Scrcpy 4.1 先发送 4 字节 codec id，再发送 12 字节 session metadata。
            codec_bytes = _recv_exact(self._video_socket, 4)
            codec_id = codec_bytes.decode("utf-8", errors="ignore").strip("\x00")
            if codec_id != SCRCPY_VIDEO_CODEC:
                raise RuntimeError(f"Scrcpy 视频编码不受支持：{codec_id or '未知'}（需要 {SCRCPY_VIDEO_CODEC}）")

            session_meta = _recv_exact(self._video_socket, 12)
            initial_resolution = self._parse_session_meta(session_meta)
            if initial_resolution is None:
                raise RuntimeError("Scrcpy 4.1 握手失败：未收到初始视频 session metadata")
            self.resolution = initial_resolution

            log.info(
                "Scrcpy 连接成功：%s，编码：%s，分辨率：%dx%d，最大帧率：%d FPS，码率：%d bps",
                self.device_name or self.serial,
                codec_id,
                self.resolution[0],
                self.resolution[1],
                SCRCPY_MAX_FPS,
                SCRCPY_VIDEO_BIT_RATE,
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
    def _build_server_shell_command(remote_jar: str) -> str:
        """Build the Scrcpy 4.1 server command and keep stream settings explicit."""
        return (
            f"CLASSPATH={remote_jar} app_process / com.genymobile.scrcpy.Server "
            f"{SCRCPY_VERSION} log_level=info audio=false control=true tunnel_forward=true "
            f"video_codec={SCRCPY_VIDEO_CODEC} max_size=0 max_fps={SCRCPY_MAX_FPS} "
            f"video_bit_rate={SCRCPY_VIDEO_BIT_RATE} send_stream_meta=true stay_awake=true cleanup=false "
            "power_off_on_close=false downsize_on_error=false"
        )

    @staticmethod
    def _frame_to_rgb(frame) -> np.ndarray:
        """Convert a decoded PyAV frame to the controller's RGB contract."""
        return frame.to_ndarray(format="rgb24")

    @staticmethod
    def _parse_session_meta(header: bytes) -> tuple[int, int] | None:
        """Parse a Scrcpy 4.1 12-byte session metadata header."""
        if len(header) != 12:
            raise ValueError(f"Scrcpy session metadata 长度错误：{len(header)}")

        flags, width, height = struct.unpack(">III", header)
        if not flags & SCRCPY_VIDEO_SESSION_META_FLAG:
            return None
        if width <= 0 or height <= 0:
            raise ValueError(f"Scrcpy session metadata 分辨率无效：{width}x{height}")
        return width, height

    def _decode_loop(self) -> None:
        """后台持续读取 H.264 视频流并解码为 RGB 图像，单帧覆盖式更新。

        按需解码：若 0.5s 内无 screenshot() 请求则只收包不解码，静止期 15fps->~0帧，省 150ms/s。
        """
        codec = av.CodecContext.create("h264", "r")

        while self._running:
            try:
                # 普通 packet 为 8 字节 PTS/flags + 4 字节 Payload 大小；
                # Scrcpy 4.1 的 session metadata 也是 12 字节，但格式为 flags + width + height。
                header = _recv_exact(self._video_socket, 12)
                if len(header) < 12:
                    if not self._running:
                        break
                    continue

                session_resolution = self._parse_session_meta(header)
                if session_resolution is not None:
                    self.resolution = session_resolution
                    log.debug("Scrcpy 视频 session 已更新分辨率：%dx%d", *session_resolution)
                    continue

                _, size = struct.unpack(">QI", header)
                if size == 0:
                    continue

                raw_packet = _recv_exact(self._video_socket, size)
                if len(raw_packet) < size:
                    continue

                # 按需解码：0.5s 内无请求则丢包不解，省 CPU（0.2s 轮询下静止期几乎不解）
                if time.monotonic() - self._last_request_mono > 0.5 and self._first_frame_ready.is_set():
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
        控制器层指纹：::32 采样 6KB + zlib.crc32 0.02ms，命中则复用同一对象零拷；
        Automation 层另有 PIL 指纹去重避免清缓存。
        """
        self._last_request_mono = time.monotonic()
        if not self._first_frame_ready.wait(timeout=3.0):
            raise RuntimeError(f"Scrcpy 视频流等待超时 ({self.serial})，未能接收到画面帧")

        with self._frame_lock:
            if self._latest_frame is None:
                raise RuntimeError("当前无可用 Scrcpy 视频帧")
            frame = self._latest_frame
            try:
                h = zlib.crc32(np.ascontiguousarray(frame[::32, ::32])) ^ hash(frame.shape)
            except Exception:
                h = None
            if h is not None and h == self._last_hash and self._last_pil is not None:
                return self._last_pil
            # 零拷：直接复用 _latest_frame 对象（后续 decode 会替换为新对象，旧对象仍被调用方持有）
            self._last_hash = h
            self._last_pil = frame
            return frame

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
        """执行指定坐标点击。对齐 MuMu 的 down(0.015)+up(0.035) 时序。"""
        log.debug("Scrcpy 点击位置：(%d, %d)", x, y)
        for _ in range(times):
            self._send_control(self._build_touch_msg(AMOTION_EVENT_ACTION_DOWN, int(x), int(y)))
            time.sleep(0.015)
            self._send_control(self._build_touch_msg(AMOTION_EVENT_ACTION_UP, int(x), int(y)))
            time.sleep(0.035)
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
        """从 (x, y) 拖拽滑动至 (x+dx, y+dy)。对齐 MuMu 的 insert_swipe 贝塞尔+0.5s抬手。"""
        # 对齐 MuMu：不做 resolution 钳位，允许拖到负坐标/超界由系统裁剪，保持距离一致
        x, y, dx, dy = int(x), int(y), int(dx), int(dy)
        x2 = x + dx
        y2 = y + dy
        points = insert_swipe(p0=(x, y), p3=(x2, y2))
        # MuMu: down(x,y) -> 逐点 down -> sleep 0.5/0.3*drag_time -> up()
        self._send_control(self._build_touch_msg(AMOTION_EVENT_ACTION_DOWN, x, y))
        time.sleep(0.02)
        # 跳过首点（已 down），其余用 MOVE 模拟 MuMu 的连续 down
        for px, py in points[1:]:
            self._send_control(self._build_touch_msg(AMOTION_EVENT_ACTION_MOVE, int(px), int(py)))
            time.sleep(0.02)
        if drag_time * 0.3 > 0.5:
            time.sleep(drag_time * 0.3)
        else:
            time.sleep(0.5)
        self._send_control(self._build_touch_msg(AMOTION_EVENT_ACTION_UP, int(x2), int(y2)))
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
        """列表滚动手势。对齐 MuMu 的 speed=8/min_distance=1 +0.2s 停留。"""
        x, y, dx, dy = int(x), int(y), int(dx), int(dy)
        x2, y2 = x + dx, y + dy
        points = insert_swipe(p0=(x, y), p3=(x2, y2), speed=8, min_distance=1)
        self._send_control(self._build_touch_msg(AMOTION_EVENT_ACTION_DOWN, x, y))
        time.sleep(0.02)
        try:
            for px, py in points[1:]:
                self._send_control(self._build_touch_msg(AMOTION_EVENT_ACTION_MOVE, int(px), int(py)))
                time.sleep(0.02)
            time.sleep(0.20)
        finally:
            self._send_control(self._build_touch_msg(AMOTION_EVENT_ACTION_UP, int(x2), int(y2)))
        self.wait_pause()

    def mouse_drag_down(self, x: int, y: int, reverse: int = 1, move_back: bool = True) -> None:
        """向下/向上拖动手势。对齐 MuMu 的 swipe(duration=0.4)。"""
        scale = cfg.set_win_size / 1080
        x, y = int(x), int(y)
        x2 = x
        y2 = y + int(300 * scale * reverse)
        # MuMu 的 swipe 用 insert_swipe + duration/min_distance 节奏，这里直接复用 mouse_drag 的对齐逻辑
        self.mouse_drag(x, y, drag_time=0.4, dx=0, dy=int(300 * scale * reverse), move_back=move_back)

    def mouse_drag_link(self, position: list, drag_time: float = 0.1, move_back: bool = False) -> None:
        """按路径多点连续拖拽。对齐 MuMu 的分段 insert_swipe。"""
        if not position:
            return
        # MuMu: down(p0) -> 每段 insert_swipe -> 逐点 down -> sleep(drag_time/min_distance) -> 最终 sleep0.5 -> up
        start_x, start_y = int(position[0][0]), int(position[0][1])
        self._send_control(self._build_touch_msg(AMOTION_EVENT_ACTION_DOWN, start_x, start_y))
        time.sleep(0.02)
        p = (start_x, start_y)
        min_distance = 10
        for target in position[1:]:
            tx, ty = int(target[0]), int(target[1])
            points = insert_swipe(p0=p, p3=(tx, ty), min_distance=min_distance)
            for px, py in points[1:]:
                self._send_control(self._build_touch_msg(AMOTION_EVENT_ACTION_MOVE, int(px), int(py)))
                time.sleep(drag_time / min_distance if min_distance else 0.02)
            p = (tx, ty)
        time.sleep(0.5)
        last_x, last_y = int(position[-1][0]), int(position[-1][1])
        self._send_control(self._build_touch_msg(AMOTION_EVENT_ACTION_UP, last_x, last_y))
        time.sleep(0.05)
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
        # [临时测试] 直接返回 True，绕过所有可能阻塞的底层 ADB 查询
        return True

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

        # 5. 清理内存中保存的最新帧及指纹缓存
        with self._frame_lock:
            self._latest_frame = None
            self._last_hash = None
            self._last_pil = None

        if ScrcpyControl.connection_device is self:
            ScrcpyControl.connection_device = None

        log.info("Scrcpy 控制器已完全释放")
