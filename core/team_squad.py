"""Pure helpers for validating and ordering configured sinner teams.

The legacy YAML format stores ``sinner_order`` as a 12-item array where the
index identifies a sinner and the value is that sinner's 1-based position in
the selected formation.  Zero means that the sinner is not selected.
"""

from __future__ import annotations

from collections.abc import Sequence

SINNER_COUNT = 12
RYOSHU_INDEX = 3  # sinner id 4 in the legacy configuration arrays
MIN_PSEUDO_SOLO_TEAM_SIZE = 3


def _selected_indices(chosen_sinners: Sequence[int] | None) -> list[int]:
    if chosen_sinners is None:
        return []
    return [index for index, selected in enumerate(chosen_sinners[:SINNER_COUNT]) if bool(selected)]


def ordered_sinner_indices(
    sinner_order: Sequence[int] | None,
    chosen_sinners: Sequence[int] | None = None,
) -> list[int]:
    """Return selected sinner roster indexes in formation order.

    ``sinner_order`` is allowed to be incomplete or contain invalid entries.
    Valid ranked entries are emitted first; selected sinners without a valid
    unique rank are appended in fixed roster order.  This makes old partial
    teams (for example a valid 10-sinner team with two zero slots) usable
    without changing the persisted representation.
    """

    raw_order = list(sinner_order or [])[:SINNER_COUNT]
    if chosen_sinners is None:
        selected = [
            index
            for index, position in enumerate(raw_order)
            if isinstance(position, int) and not isinstance(position, bool) and 1 <= position <= SINNER_COUNT
        ]
    else:
        selected = _selected_indices(chosen_sinners)

    selected_count = len(selected)
    if selected_count == 0:
        return []

    ranked: dict[int, int] = {}
    for index in selected:
        position = raw_order[index] if index < len(raw_order) else 0
        if (
            isinstance(position, int)
            and not isinstance(position, bool)
            and 1 <= position <= selected_count
            and position not in ranked
        ):
            ranked[position] = index

    ordered = [ranked[position] for position in range(1, selected_count + 1) if position in ranked]
    ordered.extend(index for index in selected if index not in ordered)
    return ordered


def validate_pseudo_solo_selection(chosen_sinners: Sequence[int] | None) -> None:
    """Validate the minimum roster needed by the pseudo-solo mode."""

    selected = _selected_indices(chosen_sinners)
    if RYOSHU_INDEX not in selected:
        raise ValueError("小指良伪单通队伍必须包含良秀")
    if len(selected) < MIN_PSEUDO_SOLO_TEAM_SIZE:
        raise ValueError(f"小指良伪单通队伍至少需要良秀和另外{MIN_PSEUDO_SOLO_TEAM_SIZE - 1}名人格")
