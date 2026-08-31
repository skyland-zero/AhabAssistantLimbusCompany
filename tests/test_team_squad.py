from __future__ import annotations

import pytest

from core.team_squad import ordered_sinner_indices, validate_pseudo_solo_selection


def test_ordered_sinner_indices_supports_partial_team_with_zero_slots() -> None:
    chosen = [1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 0]
    order = [2, 8, 4, 1, 0, 5, 7, 9, 3, 10, 6, 0]

    assert ordered_sinner_indices(order, chosen) == [3, 0, 8, 2, 5, 10, 6, 1, 7, 9]


def test_ordered_sinner_indices_appends_unranked_selected_sinners() -> None:
    chosen = [1, 1, 1, 0] + [0] * 8
    order = [2, 0, 1, 0] + [0] * 8

    assert ordered_sinner_indices(order, chosen) == [2, 0, 1]


def test_validate_pseudo_solo_selection_requires_ryoshu_and_two_family_members() -> None:
    with pytest.raises(ValueError, match="必须包含良秀"):
        validate_pseudo_solo_selection([1, 1, 1] + [0] * 9)

    with pytest.raises(ValueError, match="至少需要"):
        validate_pseudo_solo_selection([0, 0, 0, 1] + [0] * 8)

    validate_pseudo_solo_selection([1, 1, 0, 1] + [0] * 8)
