"""Capture a native Windows client area at a deterministic logical size.

This intentionally captures the client area rather than the desktop so GPUI
screenshots can be compared with the Tauri/WebView2 client viewport. The
script expects Pillow and pywin32, both already used by this repository's
Windows automation tooling.
"""

from __future__ import annotations

import argparse
import ctypes
import json
import time
from pathlib import Path

import pyautogui
import win32con
import win32gui


def make_process_dpi_aware() -> None:
    """Make Win32 coordinates and screenshots use physical pixels."""

    try:
        # PROCESS_PER_MONITOR_DPI_AWARE = 2; this fallback works on older
        # Windows versions where SetProcessDpiAwarenessContext is unavailable.
        ctypes.windll.shcore.SetProcessDpiAwareness(2)
    except (AttributeError, OSError):
        try:
            ctypes.windll.user32.SetProcessDPIAware()
        except (AttributeError, OSError):
            pass


def wait_for_window(title: str, timeout: float) -> int:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        hwnd = win32gui.FindWindow(None, title)
        if hwnd and win32gui.IsWindowVisible(hwnd):
            return hwnd
        time.sleep(0.1)
    raise TimeoutError(f"window not found within {timeout:.1f}s: {title!r}")


def client_size(hwnd: int) -> tuple[int, int]:
    _left, _top, right, bottom = win32gui.GetClientRect(hwnd)
    return right, bottom


def dpi_for_window(hwnd: int) -> int:
    try:
        return int(ctypes.windll.user32.GetDpiForWindow(hwnd)) or 96
    except (AttributeError, OSError):
        return 96


def resize_client(hwnd: int, logical_width: int, logical_height: int) -> tuple[int, int]:
    current_client_width, current_client_height = client_size(hwnd)
    left, top, right, bottom = win32gui.GetWindowRect(hwnd)
    frame_width = (right - left) - current_client_width
    frame_height = (bottom - top) - current_client_height
    dpi = dpi_for_window(hwnd)
    scale = dpi / 96.0
    physical_client_width = round(logical_width * scale)
    physical_client_height = round(logical_height * scale)
    win32gui.SetWindowPos(
        hwnd,
        0,
        left,
        top,
        physical_client_width + frame_width,
        physical_client_height + frame_height,
        win32con.SWP_NOZORDER | win32con.SWP_NOACTIVATE,
    )
    return physical_client_width, physical_client_height


def capture_client(hwnd: int, output: Path) -> dict[str, object]:
    left, top = win32gui.ClientToScreen(hwnd, (0, 0))
    width, height = client_size(hwnd)
    image = pyautogui.screenshot(region=(left, top, width, height))
    output.parent.mkdir(parents=True, exist_ok=True)
    image.save(output)
    return {
        "window": win32gui.GetWindowText(hwnd),
        "hwnd": hwnd,
        "clientOrigin": [left, top],
        "physicalClient": [width, height],
        "dpi": dpi_for_window(hwnd),
        "path": output.as_posix(),
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--title", default="Ahab Assistant · Limbus Company")
    parser.add_argument("--output", type=Path)
    parser.add_argument("--logical-width", type=int, required=True)
    parser.add_argument("--logical-height", type=int, required=True)
    parser.add_argument("--timeout", type=float, default=30.0)
    parser.add_argument("--settle-ms", type=int, default=800)
    parser.add_argument(
        "--resize-only",
        action="store_true",
        help="resize and validate the client area without writing a screenshot",
    )
    args = parser.parse_args()

    make_process_dpi_aware()
    hwnd = wait_for_window(args.title, args.timeout)
    win32gui.SetForegroundWindow(hwnd)
    expected_width, expected_height = resize_client(
        hwnd, args.logical_width, args.logical_height
    )
    time.sleep(max(args.settle_ms, 0) / 1000.0)
    actual_width, actual_height = client_size(hwnd)
    if (actual_width, actual_height) != (expected_width, expected_height):
        raise RuntimeError(
            "client resize did not settle: "
            f"expected {expected_width}x{expected_height}, "
            f"got {actual_width}x{actual_height}"
        )

    if args.resize_only:
        print(
            json.dumps(
                {
                    "window": win32gui.GetWindowText(hwnd),
                    "hwnd": hwnd,
                    "physicalClient": [actual_width, actual_height],
                    "dpi": dpi_for_window(hwnd),
                    "logicalClient": [args.logical_width, args.logical_height],
                },
                ensure_ascii=False,
            )
        )
        return
    if args.output is None:
        parser.error("--output is required unless --resize-only is set")

    result = capture_client(hwnd, args.output)
    result["logicalClient"] = [args.logical_width, args.logical_height]
    print(json.dumps(result, ensure_ascii=False))


if __name__ == "__main__":
    main()
