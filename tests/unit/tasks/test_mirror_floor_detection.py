from __future__ import annotations

from types import SimpleNamespace

import numpy as np

import tasks.mirror.mirror as mirror_module
from tasks.mirror.mirror import MIRROR_FLOOR_DETECT_MAX_ATTEMPTS, Mirror


def _detection_state() -> Mirror:
    mirror = object.__new__(Mirror)
    mirror.get_floor_num = True
    mirror.floor_detect_attempts = 0
    return mirror


def test_floor_detection_stops_after_the_attempt_budget() -> None:
    mirror = _detection_state()

    assert mirror._should_detect_floor() is True
    mirror._record_floor_detection(False)
    assert mirror._should_detect_floor() is True
    mirror._record_floor_detection(False)
    assert mirror.floor_detect_attempts == MIRROR_FLOOR_DETECT_MAX_ATTEMPTS
    assert mirror._should_detect_floor() is False


def test_successful_floor_detection_stops_retries() -> None:
    mirror = _detection_state()

    mirror._record_floor_detection(True)

    assert mirror._should_detect_floor() is False
    mirror._reset_floor_detection()
    assert mirror._should_detect_floor() is True


def test_get_which_floor_returns_false_when_the_popup_is_missing(monkeypatch) -> None:
    mirror = object.__new__(Mirror)
    mirror.floor = 0
    mirror.get_floor_num = True
    mirror.resumed_from_existing_game = False
    mirror.plan_runtime = SimpleNamespace(
        progress_observed=True,
        floor_count=5,
        record_deviation=lambda _message: None,
    )
    monkeypatch.setattr(mirror, "_read_absolute_floor", lambda: None)
    monkeypatch.setattr(mirror_module.auto, "click_element", lambda *_args, **_kwargs: False)
    monkeypatch.setattr(mirror_module.auto, "find_element", lambda *_args, **_kwargs: None)
    monkeypatch.setattr(mirror_module, "sleep", lambda _seconds: None)

    assert mirror.get_which_floor() is False


def test_read_absolute_floor_stops_after_first_unanimous_attempt(monkeypatch) -> None:
    from module.automation.input_handlers.simulator.scrcpy_control import ScrcpyControl
    from module.automation.screenshot import ScreenShot

    screenshots: list[object] = []
    ocr_calls: list[object] = []

    monkeypatch.setattr(mirror_module.ImageUtils, "load_image", lambda _path: object())
    monkeypatch.setattr(mirror_module.ImageUtils, "get_bbox", lambda _image: (10, 10, 60, 60))
    monkeypatch.setattr(
        mirror_module.ImageUtils,
        "crop",
        lambda _image, _bbox: np.zeros((20, 20, 3), dtype=np.uint8),
    )

    def fake_take_screenshot(*, gray: bool = False):
        screenshots.append(gray)
        return np.zeros((100, 100, 3), dtype=np.uint8)

    monkeypatch.setattr(mirror_module.auto, "take_screenshot", fake_take_screenshot)
    monkeypatch.setattr(ScreenShot, "_runner_mode", staticmethod(lambda: False))
    monkeypatch.setattr(ScreenShot, "_active_session", staticmethod(lambda: None))
    monkeypatch.setattr(ScrcpyControl, "connection_device", None, raising=False)

    class FakeResult:
        txts = ("ExploringFloor2Test",)

    def fake_ocr_run(_image):
        ocr_calls.append(1)
        return FakeResult()

    monkeypatch.setattr(mirror_module.ocr, "run", fake_ocr_run)
    monkeypatch.setattr(mirror_module, "extract_mirror_floor", lambda _text: 2)
    monkeypatch.setattr(mirror_module, "sleep", lambda _seconds: None)

    mirror = object.__new__(Mirror)

    assert mirror._read_absolute_floor() == 2
    assert len(screenshots) == 1
    assert len(ocr_calls) == 2
