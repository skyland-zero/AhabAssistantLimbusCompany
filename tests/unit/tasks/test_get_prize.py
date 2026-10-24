from __future__ import annotations

from types import SimpleNamespace

import tasks.daily.get_prize as get_prize


def test_mail_fallback_position_offsets_below_the_mail_icon(monkeypatch) -> None:
    monkeypatch.setattr(get_prize.auto, "find_element", lambda *_args, **_kwargs: (100, 200))
    monkeypatch.setattr(get_prize, "cfg", SimpleNamespace(set_win_size=1440))

    assert get_prize._mail_fallback_position() == (100, 400)


def test_mail_fallback_position_scales_with_window_size(monkeypatch) -> None:
    monkeypatch.setattr(get_prize.auto, "find_element", lambda *_args, **_kwargs: (100, 200))
    monkeypatch.setattr(get_prize, "cfg", SimpleNamespace(set_win_size=720))

    assert get_prize._mail_fallback_position() == (100, 300)


def test_mail_fallback_position_returns_none_without_the_mail_icon(monkeypatch) -> None:
    monkeypatch.setattr(get_prize.auto, "find_element", lambda *_args, **_kwargs: None)
    monkeypatch.setattr(get_prize, "cfg", SimpleNamespace(set_win_size=1440))

    assert get_prize._mail_fallback_position() is None
