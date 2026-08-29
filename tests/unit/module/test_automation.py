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
