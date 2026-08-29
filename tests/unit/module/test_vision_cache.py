from __future__ import annotations

import threading
from types import SimpleNamespace
from unittest.mock import Mock, patch

from PIL import Image

from module.automation.automation import Automation


def make_automation(screenshot: Image.Image) -> Automation:
    automation = object.__new__(Automation)
    automation.screenshot = screenshot
    automation._frame_id = 1
    automation._frame_dirty = False
    automation._screenshot_array_cache = {}
    automation._ocr_cache = {}
    automation._feature_frame_cache = {}
    automation._latest_screenshot_monotonic = 1.0
    automation._screenshot_lock = threading.RLock()
    automation._input_lock = threading.RLock()
    automation._interaction_gate = threading.Event()
    automation._interaction_gate.set()
    automation.img_cache = {}
    return automation


def test_ocr_is_reused_for_text_and_coordinate_queries_on_one_frame() -> None:
    automation = make_automation(Image.new("L", (20, 20), 128))
    result = SimpleNamespace(txts=["Ready"], boxes=[[[1, 2], [5, 2], [5, 6], [1, 6]]])

    with patch("module.automation.automation.ocr.run", return_value=result) as run:
        assert automation._run_ocr_for_text(only_text=True) == ["Ready"]
        assert automation._run_ocr_for_text() == {"Ready": [3.0, 4.0]}
        assert automation.get_text_from_screenshot() == ["Ready"]

    run.assert_called_once()


def test_new_business_frame_discards_ocr_cache() -> None:
    automation = make_automation(Image.new("L", (20, 20), 128))
    result = SimpleNamespace(txts=["Old"], boxes=[[[1, 2], [5, 2], [5, 6], [1, 6]]])
    new_frame = Image.new("L", (20, 20), 64)

    with patch("module.automation.automation.ocr.run", return_value=result) as run:
        assert automation.get_text_from_screenshot() == ["Old"]
        automation._set_business_screenshot(new_frame)
        assert automation.get_text_from_screenshot() == ["Old"]

    assert automation._frame_id == 2
    assert run.call_count == 2


def test_business_input_invalidates_monitor_and_derived_frame_caches() -> None:
    automation = make_automation(Image.new("L", (4, 4), 128))
    input_handler = Mock()
    input_handler.mouse_click.return_value = True
    automation.input_handler = input_handler
    automation._ocr_cache[(1, None)] = ({}, [])
    automation._feature_frame_cache["target"] = ([], None)
    automation._screenshot_array_cache["gray"] = object()

    assert automation._run_business_interaction("mouse_click", 1, 2, times=1) is True

    assert automation._frame_dirty is True
    assert automation._latest_screenshot_monotonic == 0.0
    assert automation._ocr_cache == {}
    assert automation._feature_frame_cache == {}
    assert automation._screenshot_array_cache == {}
    input_handler.mouse_click.assert_called_once_with(1, 2, times=1)


def test_feature_template_and_descriptors_are_loaded_once_until_cleared() -> None:
    automation = make_automation(Image.new("L", (4, 4), 128))
    template = Image.new("L", (4, 4), 255)
    descriptors = (["keypoint"], "descriptor")

    with (
        patch("module.automation.automation.ImageUtils.load_image", return_value=template) as load_image,
        patch("module.automation.automation.ImageUtils.feature_descriptors", return_value=descriptors) as prepare,
    ):
        assert automation._load_feature_template("marker.png") == (template, descriptors)
        assert automation._load_feature_template("marker.png") == (template, descriptors)
        load_image.assert_called_once_with("marker.png", resize=False)
        prepare.assert_called_once_with(template)

        automation.clear_img_cache()
        automation._load_feature_template("marker.png")

    assert load_image.call_count == 2
    assert prepare.call_count == 2
