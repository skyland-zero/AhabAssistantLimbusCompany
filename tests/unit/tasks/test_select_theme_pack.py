from module.mirror_routes import get_mirror_route
from tasks.mirror import select_theme_pack as select_theme_pack_module
from tasks.mirror.select_theme_pack import (
    _THEME_PACK_GIFT_INFO_DISMISS_LIMIT,
    _dismiss_gift_info_dialog,
    _theme_alias_matches,
    _theme_pack_aliases_for_floor,
    _theme_pack_card_point,
)


def test_numbered_theme_alias_does_not_match_a_longer_line_number() -> None:
    assert _theme_alias_matches("Line 1", "Line 1")
    assert not _theme_alias_matches("Line 10", "Line 1")
    assert _theme_alias_matches("Line 10", "Line 10")


def test_theme_pack_drag_starts_on_card_body_not_gift_info_icon() -> None:
    # The icon is at (2222, 451) in the 2560x1440 screenshot.  The live-tested
    # successful pull point is (2182, 698); a point further left still opens
    # the E.G.O Gift Info dialog.
    assert _theme_pack_card_point((2222, 451), 4 / 3) == (2182.0, 451 + 185 * 4 / 3)


def test_hatred_and_despair_preference_is_limited_to_floors_three_and_four() -> None:
    route = get_mirror_route("hos_ryoshu_solo_route")

    floor_three = _theme_pack_aliases_for_floor(
        3,
        route,
        prefer_hatred_and_despair=True,
    )
    floor_two = _theme_pack_aliases_for_floor(
        2,
        route,
        prefer_hatred_and_despair=True,
    )

    assert floor_three[:4] == ("Hatred", "绝望", "Hatred and Despair", "憎恶与绝望")
    assert "Hatred" not in floor_two


def test_dismiss_gift_info_dialog_clicks_the_confirm_button(monkeypatch) -> None:
    # ``E.G.O Gift Info`` is modal: its check-mark Confirm button reuses the
    # existing ``ego_gift_get_confirm_assets.png`` template, which scored 0.987
    # on a real 2560x1440 capture with the dialog open and 0.60 with it closed.
    calls: list[dict] = []

    def fake_click(target, **kwargs):
        calls.append({"target": target, **kwargs})
        return True

    monkeypatch.setattr(select_theme_pack_module.auto, "click_element", fake_click)

    assert _dismiss_gift_info_dialog() is True
    assert len(calls) == 1
    assert calls[0]["target"] == "mirror/road_in_mir/ego_gift_get_confirm_assets.png"
    # A stricter threshold keeps the probe from firing on unrelated screens.
    assert calls[0]["threshold"] >= 0.9


def test_dismiss_gift_info_dialog_reports_absent_dialog(monkeypatch) -> None:
    monkeypatch.setattr(
        select_theme_pack_module.auto,
        "click_element",
        lambda *_args, **_kwargs: False,
    )

    assert _dismiss_gift_info_dialog(take_screenshot=True) is False


def test_gift_info_dismissal_limit_is_bounded() -> None:
    # The retry budget must stay finite so a persistently reopening dialog ends
    # in a logged failure instead of an endless pack-selection loop.
    assert 1 <= _THEME_PACK_GIFT_INFO_DISMISS_LIMIT <= 50
