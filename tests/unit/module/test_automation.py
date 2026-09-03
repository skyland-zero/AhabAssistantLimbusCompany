from __future__ import annotations

import threading
import time
from unittest.mock import patch

from PIL import Image

from module.automation.automation import Automation
from module.automation.screenshot import ScreenShot


def make_automation_with_cached_screenshot(screenshot: Image.Image) -> Automation:
    automation = object.__new__(Automation)
    automation._screenshot_lock = threading.RLock()
    automation._latest_screenshot = screenshot
    automation._latest_screenshot_monotonic = time.monotonic()
    return automation


def test_monitor_screenshot_reuses_fresh_color_cache() -> None:
    cached = Image.new("RGB", (2, 2), (12, 34, 56))
    automation = make_automation_with_cached_screenshot(cached)

    with patch.object(ScreenShot, "take_screenshot") as take_screenshot:
        result = automation.take_monitor_screenshot(gray=False, max_age=0.5)

    assert result is cached
    take_screenshot.assert_not_called()


def test_monitor_screenshot_refreshes_fresh_grayscale_cache_for_color() -> None:
    cached = Image.new("L", (2, 2), 128)
    fresh = Image.new("RGB", (2, 2), (12, 34, 56))
    automation = make_automation_with_cached_screenshot(cached)

    with patch.object(ScreenShot, "take_screenshot", return_value=fresh) as take_screenshot:
        result = automation.take_monitor_screenshot(gray=False, max_age=0.5)

    assert result is fresh
    take_screenshot.assert_called_once_with(False, ensure_window_visible=True)


def test_run_ocr_for_text_applies_my_crop_offset() -> None:
    automation = object.__new__(Automation)
    automation.screenshot = Image.new("RGB", (1000, 1000), (0, 0, 0))
    automation._frame_id = 1
    automation._ocr_cache = {}

    class FakeOcrResult:
        txts = ["TEAMS#1"]
        boxes = [[[10.0, 20.0], [50.0, 20.0], [50.0, 40.0], [10.0, 40.0]]]

    with patch("module.automation.automation.ocr.run", return_value=FakeOcrResult()):
        ocr_dict = automation._run_ocr_for_text(my_crop=(100, 200, 300, 400))

    assert "TEAMS#1" in ocr_dict
    pos = ocr_dict["TEAMS#1"]
    # 原始中心为 (30.0, 30.0)，叠加裁剪偏移 (100, 200) 后应为 (130.0, 230.0)
    assert pos == [130.0, 230.0]

