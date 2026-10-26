from __future__ import annotations

import threading
from types import SimpleNamespace
from unittest.mock import Mock, patch

import numpy as np
from PIL import Image

from module.automation.automation import Automation


def make_automation(screenshot: Image.Image) -> Automation:
    automation = object.__new__(Automation)
    automation.screenshot = screenshot
    automation._frame_id = 1
    automation._frame_dirty = False
    automation._screenshot_array_cache = {}
    automation._ocr_cache = {}
    automation._match_result_cache = {}
    automation._feature_frame_cache = {}
    automation._latest_screenshot_monotonic = 1.0
    automation._screenshot_lock = threading.RLock()
    automation._input_lock = threading.RLock()
    automation._interaction_gate = threading.Event()
    automation._interaction_gate.set()
    automation.img_cache = {}
    automation.memory_protection = False
    automation.model = "clam"
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
    automation._match_result_cache[("marker",)] = object()
    automation._feature_frame_cache["target"] = ([], None)
    automation._screenshot_array_cache["gray"] = object()

    assert automation._run_business_interaction("mouse_click", 1, 2, times=1) is True

    assert automation._frame_dirty is True
    assert automation._latest_screenshot_monotonic == 0.0
    assert automation._ocr_cache == {}
    assert automation._match_result_cache == {}
    assert automation._feature_frame_cache == {}
    assert automation._screenshot_array_cache == {}
    input_handler.mouse_click.assert_called_once_with(1, 2, times=1)


def test_scrcpy_business_input_waits_for_a_new_decoder_frame() -> None:
    automation = make_automation(Image.new("L", (4, 4), 128))
    input_handler = Mock()
    input_handler.frame_seq = 7
    input_handler.mouse_click.return_value = True
    input_handler.wait_for_next_frame.return_value = True
    automation.input_handler = input_handler

    assert automation._run_business_interaction("mouse_click", 1, 2, times=1) is True

    input_handler.touch_consumer.assert_called_once_with()
    input_handler.wait_for_next_frame.assert_called_once()
    assert input_handler.wait_for_next_frame.call_args.args == (7,)
    assert input_handler.wait_for_next_frame.call_args.kwargs["timeout"] == 1.0


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


def test_image_match_result_is_reused_for_a_known_frame() -> None:
    automation = make_automation(Image.new("L", (20, 20), 128))
    template = Image.new("L", (4, 4), 255)

    with (
        patch.object(automation, "_path_state_is_known", return_value=True),
        patch("module.automation.automation.ImageUtils.existing_image_paths", return_value=["/marker"]),
        patch("module.automation.automation.ImageUtils.load_from_specific_path", return_value=template),
        patch("module.automation.automation.ImageUtils.match_template", return_value=((3, 4), 0.95)) as match,
    ):
        assert automation.find_image_element("marker.png", threshold=0.8) == (3, 4)
        assert automation.find_image_element("marker.png", threshold=0.8) == (3, 4)

    match.assert_called_once()


def test_negative_image_match_is_cached_but_invalidated_by_a_new_frame() -> None:
    automation = make_automation(Image.new("L", (20, 20), 128))
    template = Image.new("L", (4, 4), 255)

    with (
        patch.object(automation, "_path_state_is_known", return_value=True),
        patch("module.automation.automation.ImageUtils.existing_image_paths", return_value=["/marker"]),
        patch("module.automation.automation.ImageUtils.load_from_specific_path", return_value=template),
        patch("module.automation.automation.ImageUtils.match_template", return_value=((0, 0), 0.2)) as match,
    ):
        assert automation.find_image_element("marker.png", threshold=0.8) is None
        assert automation.find_image_element("marker.png", threshold=0.8) is None
        automation._set_business_screenshot(Image.new("L", (20, 20), 64))
        assert automation.find_image_element("marker.png", threshold=0.8) is None

    assert match.call_count == 2


def test_take_screenshot_forces_matching_against_a_new_frame() -> None:
    automation = make_automation(Image.new("L", (20, 20), 128))
    template = Image.new("L", (4, 4), 255)
    screenshots = iter(
        [
            Image.new("L", (20, 20), 64),
            Image.new("L", (20, 20), 32),
        ]
    )

    def capture_new_frame():
        frame = next(screenshots)
        automation._set_business_screenshot(frame)
        return frame

    with (
        patch.object(automation, "_path_state_is_known", return_value=True),
        patch.object(automation, "take_screenshot", side_effect=capture_new_frame) as take_screenshot,
        patch("module.automation.automation.ImageUtils.existing_image_paths", return_value=["/marker"]),
        patch("module.automation.automation.ImageUtils.load_from_specific_path", return_value=template),
        patch("module.automation.automation.ImageUtils.match_template", return_value=((3, 4), 0.95)) as match,
    ):
        assert automation.find_element("marker.png", take_screenshot=True) == (3, 4)
        assert automation.find_element("marker.png", take_screenshot=True) == (3, 4)

    assert take_screenshot.call_count == 2
    assert match.call_count == 2


def test_multiple_target_match_returns_a_copy_of_the_cached_list() -> None:
    automation = make_automation(Image.new("L", (20, 20), 128))
    template = Image.new("L", (4, 4), 255)
    matches = [(3, 4), (8, 9)]

    with (
        patch("module.automation.automation.ImageUtils.load_image", return_value=template),
        patch(
            "module.automation.automation.ImageUtils.match_template_with_multiple_targets",
            return_value=list(matches),
        ) as match,
    ):
        first = automation.find_image_with_multiple_targets("gifts.png", threshold=0.8)
        first.pop()
        second = automation.find_image_with_multiple_targets("gifts.png", threshold=0.8)

    assert second == matches
    match.assert_called_once()


def _ocr_result(txts, centers, scores=None):
    boxes = []
    for x, y in centers:
        boxes.append([[x - 5, y - 5], [x + 5, y - 5], [x + 5, y + 5], [x - 5, y + 5]])
    payload = {"txts": txts, "boxes": boxes}
    if scores is not None:
        payload["scores"] = scores
    return SimpleNamespace(**payload)


def test_ocr_matching_prefers_exact_text_and_keeps_duplicate_entries() -> None:
    automation = make_automation(Image.new("L", (200, 200), 128))
    result = _ocr_result(["10", "1", "1"], [(105, 10), (10, 20), (30, 20)], scores=(0.99, 0.90, 0.95))

    with patch("module.automation.automation.ocr.run", return_value=result):
        assert automation.find_text_element("1") == [30.0, 20.0]
        assert automation.find_text_element("10") == [105.0, 10.0]


def test_ocr_matching_rejects_longer_numeric_strings() -> None:
    automation = make_automation(Image.new("L", (200, 200), 128))
    automation._last_ocr_entries = [
        ("1000", [10.0, 10.0], (0, 0, 20, 20), 0.99),
        ("100%", [30.0, 30.0], (20, 20, 40, 40), 0.90),
    ]

    assert automation.find_str_in_text("100", {"1000": [10.0, 10.0], "100%": [30.0, 30.0]}) == [30.0, 30.0]

    automation._last_ocr_entries = [("10", [10.0, 10.0], (0, 0, 20, 20), 0.99)]
    assert automation.find_str_in_text("1", {"10": [10.0, 10.0]}) is False


def test_ocr_matching_normalizes_full_width_and_team_separator() -> None:
    automation = make_automation(Image.new("L", (200, 200), 128))
    automation._last_ocr_entries = [("ＴＥＡＭＳ　＃５", [12.0, 34.0], (0, 0, 20, 20), 0.9)]

    assert automation.find_str_in_text("TEAMS #5", {"ＴＥＡＭＳ　＃５": [12.0, 34.0]}) == [12.0, 34.0]


def test_empty_ocr_target_never_matches_the_first_box() -> None:
    automation = make_automation(Image.new("L", (200, 200), 128))
    automation._last_ocr_entries = [("anything", [1.0, 2.0], (0, 0, 1, 1), 1.0)]

    assert automation.find_str_in_text("", {"anything": [1.0, 2.0]}) is False


def test_assets_bbox_is_not_reapplied_after_my_crop() -> None:
    rng = np.random.default_rng(1)
    template = rng.integers(0, 255, (20, 20), dtype=np.uint8)
    frame = np.zeros((200, 200), dtype=np.uint8)
    frame[100:120, 100:120] = template
    automation = make_automation(Image.fromarray(frame))

    with (
        patch.object(automation, "_path_state_is_known", return_value=True),
        patch.object(automation, "_match_cache_state_key", return_value=("state",)),
        patch("module.automation.automation.ImageUtils.existing_image_paths", return_value=["fake"]),
        patch.object(
            automation,
            "_load_template_for_path",
            return_value=(template, (100, 100, 120, 120)),
        ),
    ):
        # The absolute asset bbox starts outside this crop; using it as a
        # crop-relative region used to make the match fail.
        assert automation.find_image_element(
            "fake_assets.png",
            threshold=0.8,
            my_crop=(80, 80, 140, 140),
        ) == (110, 110)
