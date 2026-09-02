from __future__ import annotations

import math
import random
import socket
import struct
import subprocess
import sys
import threading
import time
from dataclasses import dataclass, field
from pathlib import Path

import numpy as np
from adbutils import AdbDevice, adb

from module.config import cfg
from module.logger import log

from .. import AbstractInput
from . import insert_swipe
from .native_scrcpy_decoder import NativeScrcpyDecoder, NativeVideoFrame

# Scrcpy protocol constants (v4.1)
SCRCPY_VERSION = "4.1"
SCRCPY_VIDEO_CODEC = "h264"
SCRCPY_MAX_FPS = 15
SCRCPY_VIDEO_BIT_RATE = 8_000_000
SCRCPY_VIDEO_SESSION_META_FLAG = 1 << 31
# Scrcpy stores two packet flags in the most significant bits of the PTS
# field.  Keep these separate from the session-metadata flag above: session
# headers are identified by the first byte of the 12-byte header.
SCRCPY_PACKET_FLAG_CONFIG = 1 << 62
SCRCPY_PACKET_FLAG_KEY_FRAME = 1 << 61
SCRCPY_PACKET_PTS_MASK = SCRCPY_PACKET_FLAG_KEY_FRAME - 1
SCRCPY_MAX_PACKET_SIZE = 16 * 1024 * 1024

# A recent screenshot/input request keeps the decoder in ACTIVE mode.  A
# short QUIET period still feeds every packet to H.264 so reference frames
# remain valid; only long idle periods enter SUSPENDED mode.
SCRCPY_ACTIVE_WINDOW = 0.5
SCRCPY_SUSPEND_AFTER = 3.0
SCRCPY_RESYNC_TIMEOUT = 3.0
SCRCPY_SOCKET_TIMEOUT = 0.25

# Input timing defaults.  The path is sampled at a bounded frequency and
# each event is scheduled against the gesture deadline, avoiding drift.
SCRCPY_GESTURE_HZ = 60.0
SCRCPY_MAX_MOVE_EVENTS = 48
SCRCPY_GESTURE_SETTLE = 0.05

SCRCPY_FRAME_SEQ_INFO_KEY = "scrcpy_frame_seq"
SCRCPY_FRAME_PTS_INFO_KEY = "scrcpy_frame_pts"
SCRCPY_FRAME_DECODED_AT_INFO_KEY = "scrcpy_decoded_at"

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


@dataclass
class FrameSnapshot:
    """A decoded frame plus its timing metadata.

    ``image`` is populated by :meth:`ScrcpyControl.snapshot` and
    :meth:`ScrcpyControl.wait_next_frame`.  The lightweight
    ``wait_for_next_frame`` helper intentionally waits for the same frame
    without forcing a pixel-format conversion.
    """

    seq: int
    pts: int | None
    decoded_at: float
    image: np.ndarray


@dataclass
class _DecodedFrame:
    seq: int
    pts: int | None
    decoded_at: float
    video_frame: object
    luma_cache: np.ndarray | None = None
    rgb_cache: np.ndarray | None = None
    conversion_lock: threading.Lock = field(default_factory=threading.Lock, repr=False)


# Keep a descriptive name available for diagnostics/tests while retaining the
# private implementation name used by the controller internals.
DecodedFrame = _DecodedFrame


class ScrcpyProtocolError(RuntimeError):
    """Raised when the byte stream cannot be safely interpreted."""


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

        # The mailbox holds exactly one owned native YUV frame.  Pixel buffers
        # are materialized only when a consumer asks for a specific format.
        self._latest_frame: _DecodedFrame | None = None
        self._frame_lock = threading.Lock()
        self._frame_condition = threading.Condition(self._frame_lock)
        self._first_frame_ready = threading.Event()
        self._frame_seq = 0
        self._decode_state = "ACTIVE"
        self._state_lock = threading.Lock()
        self._decoder_needs_keyframe = True
        self._decoder_has_config = False
        self._resync_started_mono: float | None = None
        self._resync_attempts = 0
        self._last_config_packet: bytes | None = None
        self._last_config_pts: int | None = None
        self._codec = None
        self._stream_eof = threading.Event()
        self._stream_error: str | None = None
        self._recv_buffer = bytearray()
        self._restart_lock = threading.RLock()
        self._restart_thread: threading.Thread | None = None
        self._restart_cancel = threading.Event()
        self._last_request_mono: float = 0.0

        self._metrics_lock = threading.Lock()
        self._metrics: dict[str, int | float] = {
            "packets_received": 0,
            "packet_bytes": 0,
            "decoded_frames": 0,
            "published_frame_seq": 0,
            "decode_time_ns": 0,
            "gray_convert_time_ns": 0,
            "rgb_convert_time_ns": 0,
            "screenshot_requests": 0,
            "frame_age_sum_ms": 0.0,
            "frame_age_max_ms": 0.0,
            "input_to_next_frame_count": 0,
            "input_to_next_frame_sum_ms": 0.0,
            "input_to_next_frame_max_ms": 0.0,
            "packet_timeouts": 0,
            "decoder_errors": 0,
            "resync_count": 0,
            "resync_timeouts": 0,
        }
        self._metrics_last_log_mono = time.monotonic()

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

    def _recv_exact(self, n: int) -> bytes:
        """Read exactly ``n`` bytes while retaining partial TCP data.

        Socket timeouts are treated as a poll boundary while the decoder is
        running.  Crucially, bytes already received stay in
        ``_recv_buffer``; a timeout while reading a payload can therefore
        resume the same payload instead of interpreting its tail as a new
        packet header.
        """

        if n <= 0:
            return b""
        sock = self._video_socket
        if sock is None:
            raise EOFError("Scrcpy 视频 Socket 已关闭")

        while len(self._recv_buffer) < n:
            try:
                chunk = sock.recv(n - len(self._recv_buffer))
            except socket.timeout:
                self._record_metric("packet_timeouts")
                if not self._running:
                    raise
                continue
            if not chunk:
                raise EOFError("Scrcpy 视频 Socket 已到达 EOF")
            self._recv_buffer.extend(chunk)

        result = bytes(self._recv_buffer[:n])
        del self._recv_buffer[:n]
        return result

    def _read_exact(self, n: int) -> bytes:
        """Compatibility spelling for the persistent-buffer reader."""
        return self._recv_exact(n)

    def _record_metric(self, name: str, value: int | float = 1) -> None:
        with self._metrics_lock:
            self._metrics[name] = self._metrics.get(name, 0) + value

    def _set_metric(self, name: str, value: int | float) -> None:
        with self._metrics_lock:
            self._metrics[name] = value

    def get_metrics(self) -> dict[str, int | float | str]:
        """Return a point-in-time copy of Scrcpy performance counters."""

        with self._metrics_lock:
            metrics: dict[str, int | float | str] = dict(self._metrics)
        metrics["decode_time_ms"] = float(metrics.pop("decode_time_ns", 0)) / 1_000_000
        metrics["gray_convert_time_ms"] = float(metrics.pop("gray_convert_time_ns", 0)) / 1_000_000
        metrics["rgb_convert_time_ms"] = float(metrics.pop("rgb_convert_time_ns", 0)) / 1_000_000
        with self._state_lock:
            metrics["decode_state"] = self._decode_state
        return metrics

    def _maybe_log_metrics(self, *, force: bool = False) -> None:
        now = time.monotonic()
        if not force and now - self._metrics_last_log_mono < 10.0:
            return
        self._metrics_last_log_mono = now
        metrics = self.get_metrics()
        log.debug(
            "Scrcpy 链路统计：state=%s packets=%d bytes=%d decoded=%d published=%d "
            "decode=%.1fms gray=%.1fms rgb=%.1fms screenshots=%d age_max=%.1fms "
            "input_max=%.1fms timeouts=%d decoder_errors=%d resync=%d",
            metrics["decode_state"],
            metrics["packets_received"],
            metrics["packet_bytes"],
            metrics["decoded_frames"],
            metrics["published_frame_seq"],
            metrics["decode_time_ms"],
            metrics["gray_convert_time_ms"],
            metrics["rgb_convert_time_ms"],
            metrics["screenshot_requests"],
            metrics["frame_age_max_ms"],
            metrics["input_to_next_frame_max_ms"],
            metrics["packet_timeouts"],
            metrics["decoder_errors"],
            metrics["resync_count"],
        )

    def _touch_consumer(self) -> None:
        with self._state_lock:
            self._last_request_mono = time.monotonic()

    def touch_consumer(self) -> None:
        """Keep decoding active while an input action is in progress."""

        self._touch_consumer()

    @property
    def frame_seq(self) -> int:
        """The latest monotonically increasing decoded-frame sequence."""

        with self._frame_lock:
            return self._frame_seq

    @property
    def decode_state(self) -> str:
        with self._state_lock:
            return self._decode_state

    @staticmethod
    def _config_int(key: str, default: int, minimum: int, maximum: int) -> int:
        try:
            value = cfg.get_value(key, default)
        except Exception:
            value = default
        if isinstance(value, bool):
            value = default
        try:
            return max(minimum, min(maximum, int(value)))
        except (TypeError, ValueError):
            return default

    @classmethod
    def _video_settings(cls) -> tuple[int, int]:
        max_fps = cls._config_int("scrcpy_max_fps", SCRCPY_MAX_FPS, 1, 60)
        video_bit_rate = cls._config_int("scrcpy_video_bit_rate", SCRCPY_VIDEO_BIT_RATE, 100_000, 100_000_000)
        return max_fps, video_bit_rate

    @staticmethod
    def _gesture_settle_duration() -> float:
        try:
            value = cfg.get_value("scrcpy_gesture_settle", SCRCPY_GESTURE_SETTLE)
        except Exception:
            value = SCRCPY_GESTURE_SETTLE
        if isinstance(value, bool):
            return SCRCPY_GESTURE_SETTLE
        try:
            return max(0.0, min(1.0, float(value)))
        except (TypeError, ValueError):
            return SCRCPY_GESTURE_SETTLE

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
            self._recv_buffer.clear()
            for _ in range(25):
                try:
                    self._video_socket = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
                    self._video_socket.settimeout(2.0)
                    self._video_socket.connect(("127.0.0.1", self._forward_port))
                    dummy = self._recv_exact(1)
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
            try:
                self._control_socket.setsockopt(socket.IPPROTO_TCP, socket.TCP_NODELAY, 1)
            except OSError as error:
                log.debug("设置 Scrcpy Control Socket TCP_NODELAY 失败：%s", error)

            # 读取设备名称（64 字节）
            dev_name_bytes = self._recv_exact(64)
            self.device_name = dev_name_bytes.decode("utf-8", errors="ignore").rstrip("\x00")

            # Scrcpy 4.1 先发送 4 字节 codec id，再发送 12 字节 session metadata。
            codec_bytes = self._recv_exact(4)
            codec_id = codec_bytes.decode("utf-8", errors="ignore").strip("\x00")
            if codec_id != SCRCPY_VIDEO_CODEC:
                raise RuntimeError(f"Scrcpy 视频编码不受支持：{codec_id or '未知'}（需要 {SCRCPY_VIDEO_CODEC}）")

            session_meta = self._recv_exact(12)
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
                *self._video_settings(),
            )
        except Exception as error:
            self.stop()
            raise RuntimeError(f"Scrcpy Socket 通信建立失败：{error}") from error

        # 5. 启动后台单帧解码线程
        self._running = True
        self._stream_eof.clear()
        self._stream_error = None
        with self._state_lock:
            self._last_request_mono = time.monotonic()
            self._decode_state = "ACTIVE"
            self._decoder_needs_keyframe = True
            self._decoder_has_config = False
            self._resync_started_mono = time.monotonic()
            self._resync_attempts = 0
        self._video_socket.settimeout(SCRCPY_SOCKET_TIMEOUT)
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

    @classmethod
    def _build_server_shell_command(cls, remote_jar: str) -> str:
        """Build the Scrcpy 4.1 server command with explicit stream settings."""
        max_fps, video_bit_rate = cls._video_settings()
        return (
            f"CLASSPATH={remote_jar} app_process / com.genymobile.scrcpy.Server "
            f"{SCRCPY_VERSION} log_level=info audio=false control=true tunnel_forward=true "
            f"video_codec={SCRCPY_VIDEO_CODEC} max_size=0 max_fps={max_fps} "
            f"video_bit_rate={video_bit_rate} video_codec_options=i-frame-interval:int=1 "
            f"send_stream_meta=true stay_awake=true cleanup=false "
            "power_off_on_close=false downsize_on_error=false"
        )

    @staticmethod
    def _frame_to_rgb(frame) -> np.ndarray:
        """Convert a decoded frame to the controller's RGB contract."""
        return frame.to_ndarray(format="rgb24")

    @staticmethod
    def _frame_to_luma(frame) -> np.ndarray:
        """Return the Y/luma plane without going through RGB.

        The native decoder returns an owned, tightly packed Y plane.  The
        PyAV-style plane fallback is kept for lightweight test doubles and
        third-party frame adapters.
        """

        if isinstance(frame, NativeVideoFrame):
            return frame.to_ndarray(format="gray")

        try:
            plane = frame.planes[0]
            width = int(frame.width)
            height = int(frame.height)
            line_size = int(plane.line_size)
            if width <= 0 or height <= 0 or line_size < width:
                raise ValueError("无效的 Y 平面尺寸")
            plane_buffer = memoryview(plane)
            if len(plane_buffer) < line_size * height:
                raise ValueError("Y 平面缓冲区长度不足")
            luma = np.ndarray(
                shape=(height, width),
                dtype=np.uint8,
                buffer=plane_buffer,
                strides=(line_size, 1),
            )
            return luma
        except (AttributeError, IndexError, TypeError, ValueError):
            return frame.to_ndarray(format="gray")

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

    @staticmethod
    def _parse_packet_header(header: bytes) -> tuple[int | None, bool, bool, int]:
        """Return ``(pts, is_config, is_key_frame, payload_size)``."""
        if len(header) != 12:
            raise ScrcpyProtocolError(f"Scrcpy 视频 packet header 长度错误：{len(header)}")
        pts_flags, size = struct.unpack(">QI", header)
        if size <= 0:
            raise ScrcpyProtocolError("Scrcpy 视频 packet size 为 0")
        if size > SCRCPY_MAX_PACKET_SIZE:
            raise ScrcpyProtocolError(
                f"Scrcpy 视频 packet size 超过上限：{size} > {SCRCPY_MAX_PACKET_SIZE}"
            )
        is_config = bool(pts_flags & SCRCPY_PACKET_FLAG_CONFIG)
        is_key_frame = bool(pts_flags & SCRCPY_PACKET_FLAG_KEY_FRAME)
        pts = None if is_config else pts_flags & SCRCPY_PACKET_PTS_MASK
        return pts, is_config, is_key_frame, size

    def _create_decoder(self) -> NativeScrcpyDecoder:
        """Create the Scrcpy-style native low-delay H.264 decoder."""
        return NativeScrcpyDecoder(*self.resolution)

    def _set_decoder_resync(self, *, reset_attempts: bool = False) -> None:
        with self._state_lock:
            self._decoder_needs_keyframe = True
            self._decoder_has_config = False
            self._resync_started_mono = time.monotonic()
            if reset_attempts:
                self._resync_attempts = 0
        self._record_metric("resync_count")

    def _prime_decoder_config(self, codec) -> None:
        """Feed the most recent config packet into a recreated decoder."""
        config_packet = self._last_config_packet
        if not config_packet:
            return
        try:
            started = time.perf_counter_ns()
            codec.decode(config_packet, is_config=True)
            self._record_metric("decode_time_ns", time.perf_counter_ns() - started)
            with self._state_lock:
                self._decoder_has_config = True
        except Exception as error:
            self._record_metric("decoder_errors")
            log.debug("重建 Scrcpy decoder 注入 config 失败：%s", error)

    def _frame_to_snapshot(self, decoded: _DecodedFrame, mode: str) -> FrameSnapshot:
        if mode not in ("luma", "rgb"):
            raise ValueError(f"不支持的 Scrcpy 图像模式：{mode}")

        with decoded.conversion_lock:
            if mode == "rgb":
                if decoded.rgb_cache is None:
                    started = time.perf_counter_ns()
                    decoded.rgb_cache = self._frame_to_rgb(decoded.video_frame)
                    self._record_metric("rgb_convert_time_ns", time.perf_counter_ns() - started)
                image = decoded.rgb_cache
            else:
                if decoded.luma_cache is None:
                    started = time.perf_counter_ns()
                    try:
                        use_luma = bool(cfg.get_value("scrcpy_use_luma_gray", True))
                    except Exception:
                        use_luma = True
                    decoded.luma_cache = (
                        self._frame_to_luma(decoded.video_frame)
                        if use_luma
                        else decoded.video_frame.to_ndarray(format="gray")
                    )
                    self._record_metric("gray_convert_time_ns", time.perf_counter_ns() - started)
                image = decoded.luma_cache
        assert image is not None
        return FrameSnapshot(decoded.seq, decoded.pts, decoded.decoded_at, image)

    def _publish_frame(self, video_frame, pts: int | None, seq: int | None = None) -> None:
        if seq is None:
            seq = self._count_decoded_frame()
        decoded_at = time.monotonic()
        with self._frame_condition:
            decoded = _DecodedFrame(seq, pts, decoded_at, video_frame)
            self._latest_frame = decoded
            self._first_frame_ready.set()
            self._frame_condition.notify_all()
        self._set_metric("published_frame_seq", decoded.seq)

    def _count_decoded_frame(self) -> int:
        with self._frame_lock:
            self._frame_seq += 1
            return self._frame_seq

    def _desired_decode_state(self, now: float | None = None) -> str:
        now = time.monotonic() if now is None else now
        if not self._first_frame_ready.is_set():
            return "ACTIVE"
        with self._state_lock:
            idle_for = now - self._last_request_mono
        if idle_for <= SCRCPY_ACTIVE_WINDOW:
            return "ACTIVE"
        if idle_for <= SCRCPY_SUSPEND_AFTER:
            return "QUIET"
        return "SUSPENDED"

    def _refresh_decode_state(self) -> tuple[str, bool]:
        desired = self._desired_decode_state()
        with self._state_lock:
            previous = self._decode_state
            if previous == desired:
                return desired, False
            self._decode_state = desired
            if desired == "SUSPENDED":
                self._decoder_needs_keyframe = True
                self._decoder_has_config = False
                self._resync_started_mono = None
            elif previous == "SUSPENDED":
                self._decoder_needs_keyframe = True
                self._decoder_has_config = False
                self._resync_started_mono = time.monotonic()
                self._resync_attempts = 0
        log.debug("Scrcpy 解码模式切换：%s -> %s", previous, desired)
        if desired == "SUSPENDED":
            return desired, True
        if previous == "SUSPENDED":
            self._record_metric("resync_count")
        return desired, True

    def _resync_timed_out(self) -> bool:
        with self._state_lock:
            started = self._resync_started_mono
            needs_keyframe = self._decoder_needs_keyframe
        return bool(needs_keyframe and started is not None and time.monotonic() - started > SCRCPY_RESYNC_TIMEOUT)

    def _reset_decoder_after_resync_timeout(self, codec):
        with self._state_lock:
            self._resync_attempts += 1
            attempts = self._resync_attempts
        self._record_metric("resync_timeouts")
        if attempts > 1:
            self._stream_error = "Scrcpy 视频流等待关键帧超时"
            log.error("Scrcpy 视频流等待关键帧超时，准备重启视频会话")
            self._schedule_video_session_restart(self._stream_error)
            self._running = False
            return None

        log.warning("Scrcpy 视频流等待关键帧超时，重建 decoder（第 %d 次）", attempts)
        try:
            codec = self._create_decoder()
            self._codec = codec
            self._set_decoder_resync()
            self._prime_decoder_config(codec)
            return codec
        except Exception as error:
            self._record_metric("decoder_errors")
            self._stream_error = f"Scrcpy decoder 重建失败：{error}"
            self._running = False
            return None

    def _schedule_video_session_restart(self, reason: str) -> None:
        """Restart the complete Scrcpy session after decoder resync fails."""
        with self._restart_lock:
            if self._restart_thread is not None and self._restart_thread.is_alive():
                return
            self._restart_cancel.clear()
            thread = threading.Thread(
                target=self._restart_video_session,
                args=(reason, self._restart_cancel),
                name=f"ScrcpyRestart-{self.serial}",
                daemon=True,
            )
            self._restart_thread = thread
            thread.start()

    def _restart_video_session(self, reason: str, cancel: threading.Event) -> None:
        log.warning("Scrcpy 视频会话重启：%s", reason)
        try:
            self.stop(cancel_restart=False)
            # Serialize the cancellation check with an external stop.  This
            # prevents a user/device disconnect racing with _start() and
            # accidentally bringing the session back up after shutdown.
            with self._restart_lock:
                if cancel.is_set():
                    return
                self._start()
            ScrcpyControl.connection_device = self
            log.info("Scrcpy 视频会话重启成功：%s", self.serial)
        except Exception as error:
            self._stream_error = f"Scrcpy 视频会话重启失败：{error}"
            log.error("Scrcpy 视频会话重启失败：%s", error)
        finally:
            with self._restart_lock:
                if self._restart_thread is threading.current_thread():
                    self._restart_thread = None

    def _decode_loop(self) -> None:
        """Read packets continuously and adapt decode work to consumers."""
        codec = self._create_decoder()
        self._codec = codec
        self._set_decoder_resync(reset_attempts=True)

        try:
            while self._running:
                try:
                    header = self._recv_exact(12)
                    state, _ = self._refresh_decode_state()

                    if header[0] & 0x80:
                        try:
                            session_resolution = self._parse_session_meta(header)
                        except ValueError as error:
                            raise ScrcpyProtocolError(str(error)) from error
                        if session_resolution is None:
                            raise ScrcpyProtocolError("Scrcpy session header 标志非法")
                        if session_resolution != self.resolution:
                            self.resolution = session_resolution
                            self._last_config_packet = None
                            self._set_decoder_resync(reset_attempts=True)
                            codec = None
                            self._codec = None
                        log.debug("Scrcpy 视频 session 已更新分辨率：%dx%d", *session_resolution)
                        continue

                    pts, is_config, is_key_frame, size = self._parse_packet_header(header)
                    raw_packet = self._recv_exact(size)
                    self._record_metric("packets_received")
                    self._record_metric("packet_bytes", size)

                    # SUSPENDED drains complete encoded packets but never
                    # enters the decoder.  The next active request recreates
                    # the decoder and waits for config + keyframe.
                    if state == "SUSPENDED":
                        codec = None
                        self._codec = None
                        self._maybe_log_metrics()
                        continue

                    if codec is None:
                        codec = self._create_decoder()
                        self._codec = codec
                        self._set_decoder_resync(reset_attempts=True)
                        self._prime_decoder_config(codec)

                    if is_config:
                        self._last_config_packet = bytes(raw_packet)
                        self._last_config_pts = pts

                    started = time.perf_counter_ns()
                    try:
                        frames = codec.decode(raw_packet, is_config=is_config)
                    except Exception as error:
                        self._record_metric("decoder_errors")
                        log.debug("Scrcpy 视频 decoder 异常：%s", error)
                        codec = self._create_decoder()
                        self._codec = codec
                        self._set_decoder_resync()
                        self._prime_decoder_config(codec)
                        continue
                    self._record_metric("decode_time_ns", time.perf_counter_ns() - started)

                    if is_config:
                        with self._state_lock:
                            self._decoder_has_config = True
                    with self._state_lock:
                        has_config = self._decoder_has_config
                    if is_key_frame and has_config:
                        with self._state_lock:
                            self._decoder_needs_keyframe = False
                            self._resync_started_mono = None
                        log.debug("Scrcpy 视频已接收关键帧，完成重同步")

                    for frame in frames:
                        self._record_metric("decoded_frames")
                        seq = self._count_decoded_frame()
                        with self._state_lock:
                            needs_keyframe = self._decoder_needs_keyframe
                            has_config = self._decoder_has_config
                        if state != "ACTIVE":
                            continue
                        if needs_keyframe:
                            if not has_config:
                                continue
                            with self._state_lock:
                                self._decoder_needs_keyframe = False
                                self._resync_started_mono = None
                            log.debug("Scrcpy 视频已完成关键帧重同步：seq=%d", seq)
                        self._publish_frame(frame, pts, seq)

                    if self._resync_timed_out():
                        codec = self._reset_decoder_after_resync_timeout(codec)
                    self._maybe_log_metrics()
                except ScrcpyProtocolError as error:
                    self._stream_error = str(error)
                    self._record_metric("decoder_errors")
                    log.error("Scrcpy 视频协议错误：%s", error)
                    self._running = False
                    break
                except EOFError as error:
                    self._stream_eof.set()
                    self._stream_error = str(error)
                    if self._running:
                        log.debug("Scrcpy 视频流结束：%s", error)
                    self._running = False
                    break
                except (socket.timeout, socket.error, OSError) as error:
                    if not self._running:
                        break
                    self._stream_error = str(error)
                    log.debug("Scrcpy 视频 Socket 异常：%s", error)
                    self._running = False
                    break
                except Exception as error:
                    if self._running:
                        self._record_metric("decoder_errors")
                        log.debug("Scrcpy 视频解码异常：%s", error)
                        try:
                            codec = self._create_decoder()
                            self._codec = codec
                            self._set_decoder_resync()
                            self._prime_decoder_config(codec)
                        except Exception:
                            self._running = False
                            break
                        time.sleep(0.01)
        finally:
            self._codec = None
            with self._frame_condition:
                self._frame_condition.notify_all()
            self._maybe_log_metrics(force=True)

    def screenshot(self) -> np.ndarray:
        """Compatibility wrapper returning the current RGB ndarray."""
        return self.snapshot(mode="rgb").image

    def _wait_for_decoded_frame(self, after_seq: int | None, timeout: float) -> _DecodedFrame | None:
        deadline = time.monotonic() + max(0.0, float(timeout))
        with self._frame_condition:
            while True:
                current = self._latest_frame
                if current is not None and (after_seq is None or current.seq > after_seq):
                    return current
                if not self._running:
                    return None
                remaining = deadline - time.monotonic()
                if remaining <= 0:
                    return None
                self._frame_condition.wait(min(remaining, SCRCPY_SOCKET_TIMEOUT))

    def snapshot(
        self,
        *,
        mode: str = "luma",
        after_seq: int | None = None,
        timeout: float = 3.0,
    ) -> FrameSnapshot:
        """Return a current or explicitly newer frame in the requested mode."""
        self._touch_consumer()
        self._record_metric("screenshot_requests")

        effective_after_seq = after_seq
        with self._state_lock:
            must_resync = self._decode_state == "SUSPENDED" or self._decoder_needs_keyframe
        if effective_after_seq is None and must_resync:
            with self._frame_lock:
                effective_after_seq = self._latest_frame.seq if self._latest_frame is not None else self._frame_seq

        decoded = self._wait_for_decoded_frame(effective_after_seq, timeout)
        if decoded is None:
            error = self._stream_error or f"Scrcpy 视频流等待超时 ({self.serial})"
            raise RuntimeError(error)

        snapshot = self._frame_to_snapshot(decoded, mode)
        age_ms = max(0.0, (time.monotonic() - decoded.decoded_at) * 1000)
        self._record_metric("frame_age_sum_ms", age_ms)
        with self._metrics_lock:
            self._metrics["frame_age_max_ms"] = max(float(self._metrics["frame_age_max_ms"]), age_ms)
        return snapshot

    def wait_next_frame(
        self,
        after_seq: int,
        timeout: float = 3.0,
        *,
        mode: str = "luma",
        started_at: float | None = None,
    ) -> FrameSnapshot | None:
        """Wait for a published frame whose sequence is greater than ``after_seq``."""
        self._touch_consumer()
        decoded = self._wait_for_decoded_frame(after_seq, timeout)
        if decoded is None:
            return None
        if started_at is not None:
            latency_ms = max(0.0, (time.monotonic() - started_at) * 1000)
            self._record_metric("input_to_next_frame_count")
            self._record_metric("input_to_next_frame_sum_ms", latency_ms)
            with self._metrics_lock:
                self._metrics["input_to_next_frame_max_ms"] = max(
                    float(self._metrics["input_to_next_frame_max_ms"]), latency_ms
                )
        return self._frame_to_snapshot(decoded, mode)

    def wait_for_next_frame(
        self,
        after_seq: int,
        timeout: float = 1.0,
        *,
        started_at: float | None = None,
    ) -> bool:
        """Wait for a newer frame without allocating a gray/RGB image."""
        self._touch_consumer()
        decoded = self._wait_for_decoded_frame(after_seq, timeout)
        if decoded is None:
            return False
        if started_at is not None:
            latency_ms = max(0.0, (time.monotonic() - started_at) * 1000)
            self._record_metric("input_to_next_frame_count")
            self._record_metric("input_to_next_frame_sum_ms", latency_ms)
            with self._metrics_lock:
                self._metrics["input_to_next_frame_max_ms"] = max(
                    float(self._metrics["input_to_next_frame_max_ms"]), latency_ms
                )
        return True

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

    @staticmethod
    def _limit_swipe_points(points, duration: float) -> list[tuple[int, int]]:
        """Bound MOVE count while preserving the generated curve endpoints."""
        normalized = [(int(point[0]), int(point[1])) for point in points]
        if len(normalized) < 2:
            return normalized

        duration = max(0.0, float(duration))
        target_count = max(2, min(SCRCPY_MAX_MOVE_EVENTS, math.ceil(duration * SCRCPY_GESTURE_HZ) + 1))
        target_count = min(target_count, len(normalized))
        if target_count >= len(normalized):
            return normalized

        sampled: list[tuple[int, int]] = []
        for index in np.linspace(0, len(normalized) - 1, target_count):
            point = normalized[int(round(float(index)))]
            if not sampled or point != sampled[-1]:
                sampled.append(point)
        if sampled[0] != normalized[0]:
            sampled.insert(0, normalized[0])
        if sampled[-1] != normalized[-1]:
            sampled.append(normalized[-1])
        return sampled

    @staticmethod
    def _sleep_until(deadline: float) -> None:
        remaining = deadline - time.monotonic()
        if remaining > 0:
            time.sleep(remaining)

    def _send_timed_moves(self, points, duration: float) -> None:
        points = self._limit_swipe_points(points, duration)
        if len(points) < 2:
            return
        duration = max(0.0, float(duration))
        started = time.monotonic()
        denominator = len(points) - 1
        for index, (px, py) in enumerate(points[1:], start=1):
            self._sleep_until(started + duration * index / denominator)
            self._send_control(self._build_touch_msg(AMOTION_EVENT_ACTION_MOVE, px, py))
        self._sleep_until(started + duration)

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
        self._send_control(self._build_touch_msg(AMOTION_EVENT_ACTION_DOWN, x, y))
        time.sleep(0.02)
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
        """向下/向上拖动手势。对齐 MuMu.swipe(duration=0.4, min_distance=10)。"""
        scale = cfg.set_win_size / 1080
        x, y = int(x), int(y)
        x2 = x
        y2 = y + int(300 * scale * reverse)
        points = insert_swipe(p0=(x, y), p3=(x2, y2), min_distance=10)
        # MuMu.swipe: for point in points: down(*point); sleep(duration/min_distance) -> 0.04/点 +0.2+0.05
        self._send_control(self._build_touch_msg(AMOTION_EVENT_ACTION_DOWN, x, y))
        time.sleep(0.4 / 10)
        for px, py in points[1:]:
            self._send_control(self._build_touch_msg(AMOTION_EVENT_ACTION_MOVE, int(px), int(py)))
            time.sleep(0.4 / 10)
        time.sleep(0.2)
        self._send_control(self._build_touch_msg(AMOTION_EVENT_ACTION_UP, int(x2), int(y2)))
        time.sleep(0.05)
        self.wait_pause()

    def mouse_drag_link(self, position: list, drag_time: float = 0.1, move_back: bool = False) -> None:
        """按路径多点连续拖拽。对齐 MuMu 的分段 insert_swipe。"""
        if not position:
            return
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
        if self.device is None:
            try:
                self._ensure_device_connected()
            except Exception as error:
                log.debug("检查 Scrcpy 游戏状态时连接设备失败：%s", error)
                return False
        assert self.device is not None
        try:
            pid_output = self.device.shell(["pidof", self.game_package_name])
            if not str(pid_output or "").strip():
                return False

            current_package = self.get_current_package()
            if current_package == self.game_package_name:
                return True

            # A floating overlay can be reported as the current package even
            # while the game window remains focused/visible.
            window_output = self.device.shell("dumpsys window windows")
            return self.game_package_name in str(window_output or "")
        except Exception as error:
            log.debug("检查 Scrcpy 游戏状态失败：%s", error)
            return False

    def adb_disconnect(self) -> None:
        """断开控制器（生命周期接口对齐）。"""
        self.stop()

    def stop(self, *, cancel_restart: bool = True) -> None:
        """彻底停止 Scrcpy 客户端并完全释放所有线程与 Socket 资源。"""
        if cancel_restart:
            # Keep cancellation and the restart worker's start transition
            # atomic with respect to one another.  RLock permits _start's
            # failure cleanup to call stop() on the same worker thread.
            with self._restart_lock:
                self._restart_cancel.set()
        self._running = False
        self._first_frame_ready.clear()
        self._stream_eof.set()

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

        # 5. 清理内存中保存的最新帧及接收/解码状态
        with self._frame_condition:
            self._latest_frame = None
            self._recv_buffer.clear()
            self._codec = None
            self._frame_condition.notify_all()
        with self._state_lock:
            self._decode_state = "SUSPENDED"
            self._decoder_needs_keyframe = True
            self._decoder_has_config = False
            self._resync_started_mono = None
        self._last_config_packet = None
        self._last_config_pts = None

        if ScrcpyControl.connection_device is self:
            ScrcpyControl.connection_device = None

        log.info("Scrcpy 控制器已完全释放")
