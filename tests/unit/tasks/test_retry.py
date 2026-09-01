from __future__ import annotations

from types import SimpleNamespace
from unittest.mock import Mock

import tasks.base.retry as retry_module


def test_wait_for_ui_state_reuses_a_clean_current_frame(monkeypatch) -> None:
    current_auto = SimpleNamespace(
        screenshot=object(),
        _frame_dirty=False,
        take_screenshot=Mock(),
    )
    monkeypatch.setattr(retry_module, "auto", current_auto)
    monkeypatch.setattr(retry_module, "check_cancelled", lambda: None)

    assert retry_module.wait_for_ui_state(lambda: True, timeout=1, screenshot_ready=True) is True
    current_auto.take_screenshot.assert_not_called()


def test_wait_for_ui_state_captures_when_the_current_frame_is_dirty(monkeypatch) -> None:
    current_auto = SimpleNamespace(
        screenshot=object(),
        _frame_dirty=True,
        take_screenshot=Mock(return_value=object()),
    )
    monkeypatch.setattr(retry_module, "auto", current_auto)
    monkeypatch.setattr(retry_module, "check_cancelled", lambda: None)

    assert retry_module.wait_for_ui_state(lambda: True, timeout=1, screenshot_ready=True) is True
    current_auto.take_screenshot.assert_called_once_with()


def test_wait_for_ui_state_returns_false_at_the_original_timeout(monkeypatch) -> None:
    current_auto = SimpleNamespace(
        screenshot=object(),
        _frame_dirty=False,
        take_screenshot=Mock(),
    )
    monkeypatch.setattr(retry_module, "auto", current_auto)
    monkeypatch.setattr(retry_module, "check_cancelled", lambda: None)

    assert retry_module.wait_for_ui_state(lambda: False, timeout=0, screenshot_ready=True) is False
    current_auto.take_screenshot.assert_not_called()


def test_retry_only_reuses_a_frame_when_explicitly_requested(monkeypatch) -> None:
    current_auto = SimpleNamespace(
        screenshot=object(),
        _frame_dirty=False,
        take_screenshot=Mock(return_value=object()),
        get_restore_time=Mock(return_value=0),
        find_element=Mock(return_value=None),
        click_element=Mock(return_value=False),
    )
    current_screen = SimpleNamespace(handle=SimpleNamespace(hwnd=1))
    session = SimpleNamespace(target=SimpleNamespace(kind="pc"))

    monkeypatch.setattr(retry_module, "auto", current_auto)
    monkeypatch.setattr(retry_module, "screen", current_screen)
    monkeypatch.setattr(retry_module, "_active_session", lambda: session)
    monkeypatch.setattr(retry_module, "ensure_simulator_game_started", lambda: False)
    monkeypatch.setattr(retry_module, "check_times", lambda *_args, **_kwargs: False)

    assert retry_module.retry(screenshot_ready=True) is None
    current_auto.take_screenshot.assert_not_called()

    retry_module.retry()
    current_auto.take_screenshot.assert_called_once_with()
