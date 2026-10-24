from __future__ import annotations

import threading
import time
from unittest.mock import patch

import numpy as np
from PIL import Image

from module.automation.automation import Automation
from module.automation.screenshot import ScreenShot
from utils.image_utils import ImageUtils


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



def _automation_with_frame(screenshot, *, source_seq=None, handler=None) -> Automation:
    automation = object.__new__(Automation)
    automation.screenshot = screenshot
    automation._source_frame_seq = source_seq
    automation.input_handler = handler
    return automation


def _scrcpy_shot(seq: int, age_seconds: float = 0.05) -> Image.Image:
    shot = Image.new("L", (4, 4), 128)
    shot.info["scrcpy_frame_seq"] = seq
    shot.info["scrcpy_decoded_at"] = time.monotonic() - age_seconds
    return shot


def test_current_frame_age_none_for_non_scrcpy_screenshot() -> None:
    automation = _automation_with_frame(Image.new("RGB", (4, 4), (0, 0, 0)))

    assert automation.current_frame_age() is None


def test_current_frame_age_returns_seconds_since_decode() -> None:
    automation = _automation_with_frame(_scrcpy_shot(seq=7, age_seconds=0.2))

    age = automation.current_frame_age()

    assert age is not None
    assert 0.1 <= age <= 5.0


def test_wait_for_fresh_frame_returns_true_on_new_frame() -> None:
    seen = {}

    class FakeHandler:
        def wait_for_next_frame(self, after_seq, timeout=1.0, *, started_at=None):
            seen["after_seq"] = after_seq
            return True

    automation = _automation_with_frame(_scrcpy_shot(seq=10), handler=FakeHandler())

    assert automation.wait_for_fresh_frame(timeout=1.0) is True
    assert seen["after_seq"] == 10


def test_wait_for_fresh_frame_supports_two_arg_controllers() -> None:
    class LegacyHandler:
        def wait_for_next_frame(self, after_seq, timeout):
            return True

    automation = _automation_with_frame(_scrcpy_shot(seq=3), handler=LegacyHandler())

    assert automation.wait_for_fresh_frame(timeout=1.0) is True


def test_wait_for_fresh_frame_returns_false_on_timeout() -> None:
    class StalledHandler:
        def wait_for_next_frame(self, after_seq, timeout=1.0, *, started_at=None):
            return False

    automation = _automation_with_frame(_scrcpy_shot(seq=3), handler=StalledHandler())

    assert automation.wait_for_fresh_frame(timeout=0.1) is False


def test_wait_for_fresh_frame_treats_stream_error_as_timeout() -> None:
    class BrokenHandler:
        def wait_for_next_frame(self, after_seq, timeout=1.0, *, started_at=None):
            raise RuntimeError("Scrcpy 视频流等待超时")

    automation = _automation_with_frame(_scrcpy_shot(seq=3), handler=BrokenHandler())

    assert automation.wait_for_fresh_frame(timeout=0.1) is False


def test_wait_for_fresh_frame_true_without_frame_seq_transport() -> None:
    automation = _automation_with_frame(Image.new("RGB", (4, 4), (0, 0, 0)), handler=object())

    assert automation.wait_for_fresh_frame(timeout=0.1) is True


def test_is_usable_image_rejects_empty_inputs() -> None:
    assert Automation._is_usable_image(None) is False
    assert Automation._is_usable_image(Image.new("RGB", (4, 4))) is False
    assert Automation._is_usable_image(np.zeros((0, 4, 3), dtype=np.uint8)) is False
    assert Automation._is_usable_image(np.zeros((4, 0, 3), dtype=np.uint8)) is False
    assert Automation._is_usable_image(np.zeros((4, 4, 3), dtype=np.uint8)) is True


def test_missing_feature_template_is_only_loaded_once(monkeypatch) -> None:
    automation = object.__new__(Automation)
    automation.img_cache = {}
    automation._unavailable_feature_templates = set()
    load_calls: list[str] = []

    def fake_load_image(target, **_kwargs):
        load_calls.append(target)
        return None

    monkeypatch.setattr(ImageUtils, "load_image", fake_load_image)

    assert automation._load_feature_template("missing.png") == (None, None)
    assert automation._load_feature_template("missing.png") == (None, None)
    assert load_calls == ["missing.png"]


def test_clear_img_cache_retries_missing_feature_templates(monkeypatch) -> None:
    automation = object.__new__(Automation)
    automation.img_cache = {}
    automation._unavailable_feature_templates = {"missing.png"}
    load_calls: list[str] = []

    def fake_load_image(target, **_kwargs):
        load_calls.append(target)
        return None

    monkeypatch.setattr(ImageUtils, "load_image", fake_load_image)

    automation.clear_img_cache()
    assert automation._unavailable_feature_templates == set()

    automation._load_feature_template("missing.png")
    assert load_calls == ["missing.png"]


def test_find_feature_element_returns_none_without_a_screenshot(monkeypatch) -> None:
    automation = object.__new__(Automation)
    automation.screenshot = None
    automation._frame_id = 0
    automation._match_result_cache = {}
    automation._unavailable_feature_templates = set()
    monkeypatch.setattr(automation, "_load_feature_template", lambda _target: (np.zeros((4, 4), dtype=np.uint8), None))

    assert automation.find_feature_element("anything.png") is None
