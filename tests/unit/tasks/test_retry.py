from __future__ import annotations

from types import SimpleNamespace
from unittest.mock import Mock

import pytest

import module.device_manager as device_manager_module
import tasks.base.retry as retry_module
from module.device_manager import DeviceError


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


def test_runner_retry_rejects_legacy_global_controller_fallback(monkeypatch) -> None:
    monkeypatch.setattr(retry_module, "_runner_mode_enabled", lambda: True)
    monkeypatch.setattr(device_manager_module, "get_device_manager", lambda: SimpleNamespace(active_session=None))

    with pytest.raises(DeviceError, match="私有设备 session"):
        retry_module._active_session()


def _patch_stall_config(monkeypatch, value) -> None:
    monkeypatch.setattr(
        retry_module,
        "cfg",
        SimpleNamespace(get_value=lambda *_args, **_kwargs: value),
    )


def test_get_task_stall_timeout_uses_defaults_and_a_safe_floor(monkeypatch) -> None:
    _patch_stall_config(monkeypatch, None)
    assert retry_module.get_task_stall_timeout() == retry_module.DEFAULT_TASK_STALL_TIMEOUT

    _patch_stall_config(monkeypatch, 10)
    assert retry_module.get_task_stall_timeout() == retry_module.MIN_TASK_STALL_TIMEOUT

    _patch_stall_config(monkeypatch, 31)
    assert retry_module.get_task_stall_timeout() == 31

    _patch_stall_config(monkeypatch, "not-a-number")
    assert retry_module.get_task_stall_timeout() == retry_module.DEFAULT_TASK_STALL_TIMEOUT


def test_check_times_uses_the_configured_stall_timeout(monkeypatch) -> None:
    _patch_stall_config(monkeypatch, 31)
    monkeypatch.setattr(retry_module, "time", SimpleNamespace(time=lambda: 100.0))
    kill_game = Mock()
    restart_game = Mock()
    monkeypatch.setattr(retry_module, "kill_game", kill_game)
    monkeypatch.setattr(retry_module, "restart_game", restart_game)

    assert retry_module.check_times(90.0, logs=False) is False
    kill_game.assert_not_called()

    assert retry_module.check_times(0.0, logs=False) is True
    kill_game.assert_called_once_with()
    restart_game.assert_called_once_with()


def test_check_times_explicit_timeout_overrides_the_config(monkeypatch) -> None:
    _patch_stall_config(monkeypatch, 31)
    monkeypatch.setattr(retry_module, "time", SimpleNamespace(time=lambda: 100.0))
    monkeypatch.setattr(retry_module, "kill_game", Mock())
    monkeypatch.setattr(retry_module, "restart_game", Mock())

    assert retry_module.check_times(0.0, timeout=200, logs=False) is False
    retry_module.kill_game.assert_not_called()
