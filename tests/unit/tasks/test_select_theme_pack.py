from module.mirror_routes import get_mirror_route
from tasks.mirror.select_theme_pack import (
    _theme_alias_matches,
    _theme_pack_aliases_for_floor,
    _theme_pack_card_point,
)


def test_numbered_theme_alias_does_not_match_a_longer_line_number() -> None:
    assert _theme_alias_matches("Line 1", "Line 1")
    assert not _theme_alias_matches("Line 10", "Line 1")
    assert _theme_alias_matches("Line 10", "Line 10")


def test_theme_pack_drag_starts_on_card_body_not_gift_info_icon() -> None:
    # The icon is at (2222, 451) in the 2560x1440 screenshot; the drag must
    # start on the card body to avoid opening the E.G.O Gift Info dialog.
    assert _theme_pack_card_point((2222, 451), 4 / 3) == (2122.0, 671.0)


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
