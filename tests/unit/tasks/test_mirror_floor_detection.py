from __future__ import annotations

from types import SimpleNamespace

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
