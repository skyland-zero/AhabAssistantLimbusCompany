"""Shared E.G.O Gift Search identifiers and validation helpers.

The Mirror UI stores selected gifts as strings.  Keep the user-facing preset
stable while retaining compatibility with the coordinate format used by the
automation task (``system_level_row_col``).
"""

from __future__ import annotations

import re
from collections.abc import Iterable
from dataclasses import dataclass

MAX_OBSERVE_EGO_GIFTS = 3

SPIDERWEB_ENTANGLED_IN_RED = "spiderweb_entangled_in_red"
SPIDERWEB_ENTANGLED_IN_RED_NAME = "赤红纠缠的蜘蛛巢"
SPIDERWEB_ENTANGLED_IN_RED_ASSET = "mirror/road_to_mir/observe_ego_gift/general_gift/spiderweb_entangled_in_red.png"

# The historical gift asset was ``general_gift_3_32.png``.  The current
# selector uses eight columns, so item 32 is row 4, column 8.
SPIDERWEB_ENTANGLED_IN_RED_SELECTION = "general_3_4_8"

_OBSERVE_SYSTEMS = frozenset(
    {
        "burn",
        "bleed",
        "tremor",
        "rupture",
        "poise",
        "sinking",
        "charge",
        "slash",
        "pierce",
        "blunt",
        "general",
    }
)
_COORDINATE_PATTERN = re.compile(r"^(?P<system>[a-z]+)_(?P<level>\d+)_(?P<row>\d+)_(?P<col>\d+)$")


@dataclass(frozen=True, slots=True)
class ObserveEgoGiftTarget:
    """Resolved target used by the Mirror automation task."""

    key: str
    system: str
    level: int
    row: int
    col: int
    asset: str | None = None

    @property
    def coordinate(self) -> str:
        return f"{self.system}_{self.level}_{self.row}_{self.col}"


_SPIDERWEB_TARGET = ObserveEgoGiftTarget(
    key=SPIDERWEB_ENTANGLED_IN_RED,
    system="general",
    level=3,
    row=4,
    col=8,
    asset=SPIDERWEB_ENTANGLED_IN_RED_ASSET,
)

_ALIASES = {
    SPIDERWEB_ENTANGLED_IN_RED.casefold(): SPIDERWEB_ENTANGLED_IN_RED,
    SPIDERWEB_ENTANGLED_IN_RED_NAME.casefold(): SPIDERWEB_ENTANGLED_IN_RED,
    "赤紅糾纏的蜘蛛巢".casefold(): SPIDERWEB_ENTANGLED_IN_RED,
    "spiderweb entangled in red": SPIDERWEB_ENTANGLED_IN_RED,
    "general_gift_3_32.png": SPIDERWEB_ENTANGLED_IN_RED,
}


def _clean(value: str) -> str:
    return " ".join(value.strip().split())


def resolve_observe_ego_gift(value: str) -> ObserveEgoGiftTarget | None:
    """Resolve a preset name or a valid legacy coordinate string."""

    if not isinstance(value, str):
        return None
    value = _clean(value)
    if not value:
        return None

    alias = _ALIASES.get(value.casefold())
    if alias == SPIDERWEB_ENTANGLED_IN_RED:
        return _SPIDERWEB_TARGET

    match = _COORDINATE_PATTERN.fullmatch(value.casefold())
    if match is None:
        return None
    system = match.group("system")
    level = int(match.group("level"))
    row = int(match.group("row"))
    col = int(match.group("col"))
    if system not in _OBSERVE_SYSTEMS or not 1 <= level <= 3 or not 1 <= row <= 10 or not 1 <= col <= 8:
        return None
    return ObserveEgoGiftTarget(
        key=f"{system}_{level}_{row}_{col}",
        system=system,
        level=level,
        row=row,
        col=col,
    )


def canonical_observe_ego_gift(value: str) -> str | None:
    """Return the stable stored value for a preset or coordinate."""

    target = resolve_observe_ego_gift(value)
    if target is None:
        return None
    return target.key


def normalize_observe_ego_gifts(values: Iterable[str]) -> list[str]:
    """Normalize, deduplicate, and cap configured Gift Search targets."""

    normalized: list[str] = []
    for value in values:
        canonical = canonical_observe_ego_gift(value)
        if canonical is None or canonical in normalized:
            continue
        normalized.append(canonical)
        if len(normalized) >= MAX_OBSERVE_EGO_GIFTS:
            break
    return normalized
