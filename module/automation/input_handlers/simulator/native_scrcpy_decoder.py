"""Native H.264 decoder adapter used by the Scrcpy controller.

The video protocol remains implemented in Python.  This module only bridges
the packet-level decoder to a small Rust DLL, which dynamically loads the
Scrcpy-style, Windows FFmpeg runtime from ``assets/binary/scrcpy-ffmpeg``.
"""

from __future__ import annotations

import ctypes
import os
import sys
from dataclasses import dataclass
from pathlib import Path

import numpy as np


class NativeScrcpyDecoderError(RuntimeError):
    """Raised when the native Scrcpy decoder cannot be initialized or used."""


class _FrameInfo(ctypes.Structure):
    _fields_ = [
        ("width", ctypes.c_uint32),
        ("height", ctypes.c_uint32),
        ("y", ctypes.c_void_p),
        ("y_len", ctypes.c_size_t),
        ("u", ctypes.c_void_p),
        ("u_len", ctypes.c_size_t),
        ("v", ctypes.c_void_p),
        ("v_len", ctypes.c_size_t),
        ("y_stride", ctypes.c_uint32),
        ("uv_stride", ctypes.c_uint32),
    ]


def _runtime_directories() -> list[Path]:
    roots: list[Path] = []
    meipass = getattr(sys, "_MEIPASS", None)
    if meipass:
        roots.append(Path(meipass))
    try:
        roots.append(Path(sys.executable).resolve().parent)
    except OSError:
        pass

    # module/.../simulator/native_scrcpy_decoder.py -> project root
    source_root = Path(__file__).resolve().parents[4]
    roots.extend((source_root, Path.cwd()))

    candidates: list[Path] = []
    for root in roots:
        candidates.extend(
            (
                root / "assets" / "binary" / "scrcpy-ffmpeg",
                root / "assets" / "binary",
                root / "native" / "scrcpy_decoder" / "target" / "release",
            )
        )

    unique: list[Path] = []
    seen: set[str] = set()
    for candidate in candidates:
        key = os.path.normcase(os.path.abspath(candidate))
        if key not in seen:
            seen.add(key)
            unique.append(candidate)
    return unique


def _find_runtime_directory() -> Path:
    for directory in _runtime_directories():
        if (
            (directory / "scrcpy_decoder.dll").is_file()
            and (directory / "avcodec-62.dll").is_file()
            and (directory / "avutil-60.dll").is_file()
        ):
            return directory
    searched = ", ".join(str(path) for path in _runtime_directories())
    raise NativeScrcpyDecoderError(f"未找到 Scrcpy native decoder 运行时，已搜索：{searched}")


def _load_library(directory: Path) -> ctypes.CDLL:
    try:
        # The Rust bridge loads FFmpeg itself by absolute path.  Keeping the
        # directory registered also makes dependent DLL lookup predictable on
        # Windows versions with stricter DLL search rules.
        add_dll_directory = getattr(os, "add_dll_directory", None)
        if add_dll_directory is not None:
            add_dll_directory(str(directory))
        loader = getattr(ctypes, "WinDLL", ctypes.CDLL)
        return loader(str(directory / "scrcpy_decoder.dll"))
    except OSError as error:
        raise NativeScrcpyDecoderError(f"加载 Scrcpy native decoder 失败：{directory}") from error


def _configure_library(library: ctypes.CDLL) -> None:
    library.scrcpy_decoder_create.argtypes = [ctypes.c_wchar_p, ctypes.c_uint32, ctypes.c_uint32]
    library.scrcpy_decoder_create.restype = ctypes.c_void_p

    library.scrcpy_decoder_push.argtypes = [
        ctypes.c_void_p,
        ctypes.POINTER(ctypes.c_uint8),
        ctypes.c_size_t,
        ctypes.c_int,
    ]
    library.scrcpy_decoder_push.restype = ctypes.c_int

    library.scrcpy_decoder_receive.argtypes = [ctypes.c_void_p, ctypes.POINTER(_FrameInfo)]
    library.scrcpy_decoder_receive.restype = ctypes.c_int

    library.scrcpy_decoder_reset.argtypes = [ctypes.c_void_p]
    library.scrcpy_decoder_reset.restype = None

    library.scrcpy_decoder_destroy.argtypes = [ctypes.c_void_p]
    library.scrcpy_decoder_destroy.restype = None

    library.scrcpy_decoder_last_error.argtypes = [ctypes.c_void_p, ctypes.c_size_t]
    library.scrcpy_decoder_last_error.restype = ctypes.c_size_t


def _native_error(library: ctypes.CDLL) -> str:
    buffer = ctypes.create_string_buffer(2048)
    library.scrcpy_decoder_last_error(buffer, len(buffer))
    message = buffer.value.decode("utf-8", errors="replace").strip()
    return message or "未知 native decoder 错误"


@dataclass(frozen=True)
class NativeVideoFrame:
    """Owned planar YUV420P frame returned by the native decoder."""

    width: int
    height: int
    y: bytes
    u: bytes
    v: bytes

    def to_ndarray(self, *, format: str) -> np.ndarray:
        normalized = format.lower()
        if normalized in {"gray", "gray8", "y8"}:
            return np.frombuffer(self.y, dtype=np.uint8).reshape((self.height, self.width))
        if normalized == "rgb24":
            return self._to_rgb24()
        raise ValueError(f"不支持的 native Scrcpy 图像格式：{format}")

    def _to_rgb24(self) -> np.ndarray:
        if self.width % 2 or self.height % 2:
            raise ValueError(f"native Scrcpy RGB 转换要求偶数分辨率：{self.width}x{self.height}")

        try:
            import cv2
        except ImportError as error:
            raise NativeScrcpyDecoderError("native Scrcpy 彩色截图需要 OpenCV") from error

        y_size = self.width * self.height
        chroma_size = (self.width // 2) * (self.height // 2)
        i420 = np.empty((self.height * 3 // 2, self.width), dtype=np.uint8)
        packed = i420.reshape(-1)
        packed[:y_size] = np.frombuffer(self.y, dtype=np.uint8)
        packed[y_size : y_size + chroma_size] = np.frombuffer(self.u, dtype=np.uint8)
        packed[y_size + chroma_size : y_size + chroma_size * 2] = np.frombuffer(self.v, dtype=np.uint8)
        return cv2.cvtColor(i420, cv2.COLOR_YUV2RGB_I420)


class NativeScrcpyDecoder:
    """Single-threaded H.264 packet decoder matching the Scrcpy stream."""

    def __init__(self, width: int = 0, height: int = 0) -> None:
        self._directory = _find_runtime_directory()
        self._library = _load_library(self._directory)
        _configure_library(self._library)
        self._handle = self._library.scrcpy_decoder_create(str(self._directory), max(0, width), max(0, height))
        if not self._handle:
            raise NativeScrcpyDecoderError(_native_error(self._library))

    def decode(self, payload: bytes, *, is_config: bool = False) -> list[NativeVideoFrame]:
        if not payload:
            raise NativeScrcpyDecoderError("Scrcpy H.264 packet 为空")
        if not self._handle:
            raise NativeScrcpyDecoderError("native Scrcpy decoder 已关闭")

        buffer_type = ctypes.c_uint8 * len(payload)
        buffer = buffer_type.from_buffer_copy(payload)
        result = self._library.scrcpy_decoder_push(self._handle, buffer, len(payload), int(is_config))
        if result < 0:
            raise NativeScrcpyDecoderError(_native_error(self._library))

        frames: list[NativeVideoFrame] = []
        while True:
            info = _FrameInfo()
            result = self._library.scrcpy_decoder_receive(self._handle, ctypes.byref(info))
            if result == 0:
                break
            if result < 0:
                raise NativeScrcpyDecoderError(_native_error(self._library))
            frames.append(
                NativeVideoFrame(
                    width=int(info.width),
                    height=int(info.height),
                    y=ctypes.string_at(info.y, info.y_len),
                    u=ctypes.string_at(info.u, info.u_len),
                    v=ctypes.string_at(info.v, info.v_len),
                )
            )
        return frames

    def reset(self) -> None:
        if self._handle:
            self._library.scrcpy_decoder_reset(self._handle)

    def close(self) -> None:
        handle = self._handle
        if handle:
            self._handle = None
            self._library.scrcpy_decoder_destroy(handle)

    def __enter__(self) -> NativeScrcpyDecoder:
        return self

    def __exit__(self, _exc_type, _exc_value, _traceback) -> None:
        self.close()

    def __del__(self) -> None:
        try:
            self.close()
        except Exception:
            pass
