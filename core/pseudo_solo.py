"""Pseudo-solo battle state and conservative survivor observation.

The upstream battle runner only needs a state with ``remaining_turns`` and
``consume_turn``.  Keeping the dynamic policy behind that small interface
allows the fork to observe the battle screen without changing the upstream
battle loop.
"""

from __future__ import annotations

import re
import time
from collections.abc import Callable, Sequence
from dataclasses import dataclass
from enum import StrEnum
from typing import Protocol

import cv2
import numpy as np

from core.execution_control import check_cancelled, interruptible_sleep
from module.automation import auto
from module.config import cfg
from module.logger import log
from module.my_error.my_error import userStopError
from utils.image_utils import ImageUtils

# Kept as a compatibility constant for integrations that imported the old
# setting.  Battle-row sampling is configured separately below.
PSEUDO_SOLO_OBSERVATION_STABILITY = 1
# The team-page templates only cover the 10/11/12-sinner counts.  The
# existing 10-sinner template scores about 0.818 on the current 1440p UI, so
# keep the threshold below that while the OCR counter fallback handles the
# smaller counts.
TEAM_PAGE_LIVE_COUNT_THRESHOLD = 0.80
# The team-page live count is drawn at the right edge of the panel.  This
# crop is derived from the non-black bounding box of
# teams/10_sinner_live_assets.png (2366,1017)-(2421,1086 at 2560x1440) with a
# small margin so the 1-2 digit number is readable at other resolutions.
TEAM_PAGE_COUNT_CROP_RATIO = (0.90, 0.67, 0.97, 0.79)
# Extra room around the packaged counter template bbox.  The ratio crop above
# is only a fallback: it also covered neighbouring panel content, and OCR
# picked a ``3`` from that area while a single ally was alive.
TEAM_PAGE_COUNT_CROP_MARGIN = (70, 40)
# A roster count is a bare 1-2 digit number.  Glued text such as ``3in`` means
# the crop caught neighbouring UI, so that frame must not become a count.
TEAM_PAGE_COUNT_TEXT_PATTERN = re.compile(r"^\s*(\d{1,2})\s*$")
# The command row can remain hidden behind the battle-entry animation for
# roughly one second.  Keep sampling the same battle-entry window a little
# longer so a transient no-badge frame is not cached as UNKNOWN.
BATTLE_ROSTER_MAX_SAMPLES = 12
BATTLE_ROSTER_RETRY_MAX_SAMPLES = 4
BATTLE_ROSTER_MAX_ATTEMPTS = 5
BATTLE_ROSTER_REQUIRED_STABLE_SAMPLES = 2
BATTLE_ROSTER_SAMPLE_INTERVAL = 0.15
BATTLE_ROSTER_CENTER_TOLERANCE_RATIO = 0.01
BATTLE_OBSERVATION_RETRY_SECONDS = 2.5
BATTLE_OBSERVATION_MAX_ATTEMPTS = 5
BATTLE_PORTRAIT_Y_START_RATIO = 0.90
BATTLE_PORTRAIT_Y_END_RATIO = 0.995
BATTLE_PORTRAIT_X_START_RATIO = 0.18
BATTLE_PORTRAIT_X_END_RATIO = 0.90
BATTLE_PORTRAIT_BADGE_Y_START_RATIO = 0.93
BATTLE_PORTRAIT_BADGE_Y_END_RATIO = 0.997
BATTLE_PORTRAIT_INNER_Y_START_RATIO = 0.89
BATTLE_PORTRAIT_INNER_Y_END_RATIO = 0.94
BATTLE_GEAR_LEFT_CROP_RATIO = (0.15, 0.62, 0.42, 0.98)
BATTLE_GEAR_RIGHT_CROP_RATIO = (0.52, 0.62, 0.82, 0.98)
BATTLE_ANIMATION_CROP_RATIO = (0.20, 0.15, 0.80, 0.75)
BATTLE_ANIMATION_MIN_VALUE = 120
# At 1920x1080 a real level badge is about 40x41 px and has a component area
# of roughly 750-900 px.  Short UI effects can have a similar horizontal
# projection but are much shorter; validate the connected component instead
# of accepting projection width alone.
BATTLE_BADGE_MIN_HEIGHT_AT_1080 = 30
BATTLE_BADGE_MIN_WIDTH_AT_1080 = 28
BATTLE_BADGE_MIN_AREA_AT_1080 = 500
# The current 1440p battle UI draws the level badge as a blue circle (HSV hue
# around 105-113).  Older captures used an orange circle.  Try blue first and
# keep orange as a compatibility fallback so both UI versions remain usable.
BATTLE_BADGE_BLUE_HSV_LOW = (100, 80, 40)
BATTLE_BADGE_BLUE_HSV_HIGH = (118, 255, 255)
BATTLE_BADGE_ORANGE_HSV_LOW = (8, 120, 120)
BATTLE_BADGE_ORANGE_HSV_HIGH = (22, 255, 255)
BATTLE_BADGE_COLOR_RANGES: tuple[
    tuple[str, tuple[int, int, int], tuple[int, int, int]], ...
] = (
    ("blue", BATTLE_BADGE_BLUE_HSV_LOW, BATTLE_BADGE_BLUE_HSV_HIGH),
    ("orange", BATTLE_BADGE_ORANGE_HSV_LOW, BATTLE_BADGE_ORANGE_HSV_HIGH),
)


class PseudoSoloObservation(StrEnum):
    """Conservative observations used by the pseudo-solo policy."""

    UNKNOWN = "unknown"
    MULTIPLE_SURVIVORS = "multiple_survivors"
    SINGLE_SURVIVOR = "single_survivor"


def defense_turns_for_live_count(live_count: int) -> int:
    """Return the turns needed for every living ally except the solo unit."""

    return max(int(live_count) - 1, 0)


@dataclass(frozen=True, slots=True)
class SurvivorLedgerUpdate:
    """Outcome of one reading offered to :class:`SurvivorLedger`."""

    accepted: bool
    reason: str
    confirmed: int | None
    pending: int | None
    changed: bool


class SurvivorLedger:
    """Run-scoped living-ally count for rosters whose deaths are permanent.

    ``赤红纠缠的蜘蛛巢`` (E.G.O Gift Search target
    ``spiderweb_entangled_in_red``) permanently removes a dead sinner for the
    rest of the mirror run.  Under that gift the living count is monotonic:
    it can only fall, and a rise is always a mis-read rather than a revival.

    A value is confirmed after ``required_confirmations`` independent readings
    of the same number.  Once confirmed, the count survives every battle for
    the rest of the run, so the expensive multi-frame roster sampling only
    happens until the first confirmation.  An unreadable frame never clears a
    confirmed value, and a larger count is rejected instead of overriding it.
    """

    REASON_UNREADABLE = "unreadable"
    REASON_PENDING = "pending_confirmation"
    REASON_MATCH = "match"
    REASON_CONFIRMED = "confirmed"
    REASON_DECREASE_PENDING = "decrease_pending"
    REASON_DECREASE_CONFIRMED = "decrease_confirmed"
    REASON_DECREASE_NEEDS_AUTHORITY = "decrease_needs_authoritative_source"
    REASON_IMPOSSIBLE_INCREASE = "impossible_increase"

    def __init__(
        self,
        *,
        required_confirmations: int = 2,
        authoritative_sources: Sequence[str] = ("battle_row", "team_page_asset"),
    ) -> None:
        self._required_confirmations = max(1, int(required_confirmations))
        # Readers that may lower a confirmed count on their own.  The team page
        # offers two very different readers: a template match against
        # ``teams/{10,11,12}_sinner_live_assets.png`` (scores ~0.98) and a bare
        # OCR digit from a narrow counter crop.  The OCR digit repeatedly
        # reported ``3`` while a single ally was alive and then locked the
        # whole run, so only the template match counts as authoritative.
        self._authoritative_sources = frozenset(authoritative_sources)
        self._confirmed: int | None = None
        self._pending: int | None = None
        self._streak = 0
        # Confidence is tracked per reader class.  Mixing both into one streak
        # let stale OCR digits promote a single battle-row reading straight to
        # a confirmed value.
        self._authoritative_streak = 0
        self._source = ""

    @property
    def required_confirmations(self) -> int:
        return self._required_confirmations

    @property
    def confirmed(self) -> int | None:
        """Return the run-scoped living-ally count, if it is confirmed yet."""

        return self._confirmed

    @property
    def pending(self) -> int | None:
        """Return the value waiting for its second confirmation."""

        return self._pending

    @property
    def source(self) -> str:
        """Return which reader confirmed the current value."""

        return self._source

    def record(self, count: object, *, source: str) -> SurvivorLedgerUpdate:
        """Offer one reading and return what the ledger did with it."""

        normalized = self._normalize(count)
        if normalized is None:
            return self._update(
                accepted=False,
                reason=self.REASON_UNREADABLE,
                changed=False,
            )

        if self._confirmed is not None:
            if normalized == self._confirmed:
                self._pending = None
                self._streak = 0
                self._authoritative_streak = 0
                return self._update(accepted=True, reason=self.REASON_MATCH, changed=False)
            if normalized > self._confirmed:
                # Permanent deaths make an increase impossible; keeping the
                # lower confirmed value also keeps the defense budget correct.
                self._pending = None
                self._streak = 0
                self._authoritative_streak = 0
                return self._update(
                    accepted=False,
                    reason=self.REASON_IMPOSSIBLE_INCREASE,
                    changed=False,
                )

        if self._pending == normalized:
            self._streak += 1
        else:
            self._pending = normalized
            self._streak = 1
            self._authoritative_streak = 0
        if source in self._authoritative_sources:
            self._authoritative_streak += 1

        if self._streak < self._required_confirmations:
            reason = self.REASON_DECREASE_PENDING if self._confirmed is not None else self.REASON_PENDING
            return self._update(accepted=True, reason=reason, changed=False)
        if self._authoritative_streak < self._authoritative_confirmations():
            # Readings from a non-authoritative reader (the team-page OCR
            # counter) never lock a value on their own.  That counter reported
            # ``3`` while a single ally was alive and then held the whole run at
            # that number, so it may only corroborate the battle portrait row.
            return self._update(
                accepted=True,
                reason=(
                    self.REASON_DECREASE_NEEDS_AUTHORITY
                    if self._confirmed is not None
                    else self.REASON_PENDING
                ),
                changed=False,
            )

        previous = self._confirmed
        self._confirmed = normalized
        self._pending = None
        self._streak = 0
        self._authoritative_streak = 0
        self._source = source
        reason = self.REASON_DECREASE_CONFIRMED if previous is not None else self.REASON_CONFIRMED
        return self._update(accepted=True, reason=reason, changed=True)

    def _authoritative_confirmations(self) -> int:
        """Return how many authoritative readings a value needs on its own.

        Non-authoritative readings (the team-page OCR counter) may keep a
        pending value alive, but they never complete the confirmation window:
        a value is only locked once the configured number of readings all came
        from an authoritative reader.
        """

        return self._required_confirmations

    def reset(self) -> None:
        """Forget the run cache; used when the mirror run restarts."""

        self._confirmed = None
        self._pending = None
        self._streak = 0
        self._authoritative_streak = 0
        self._source = ""

    @staticmethod
    def _normalize(count: object) -> int | None:
        if count is None or isinstance(count, bool):
            return None
        try:
            value = int(count)  # type: ignore[arg-type]
        except (TypeError, ValueError):
            return None
        if value < 0:
            return None
        return value

    def _update(self, *, accepted: bool, reason: str, changed: bool) -> SurvivorLedgerUpdate:
        return SurvivorLedgerUpdate(
            accepted=accepted,
            reason=reason,
            confirmed=self._confirmed,
            pending=self._pending,
            changed=changed,
        )


def _battle_animation_overlay_detection(array: np.ndarray) -> tuple[bool, dict[str, object]]:
    """Detect the large blue transition/animation veil in the battle area.

    A battle animation can leave a few orange pixels from the command row
    visible.  Those pixels must not be treated as a complete roster.  The
    animation veil in the captured screen is a large connected blue region,
    unlike the small blue UI and character fragments in a normal command
    frame.
    """

    height, width = array.shape[:2]
    x1 = max(0, min(width, round(width * BATTLE_ANIMATION_CROP_RATIO[0])))
    y1 = max(0, min(height, round(height * BATTLE_ANIMATION_CROP_RATIO[1])))
    x2 = max(x1, min(width, round(width * BATTLE_ANIMATION_CROP_RATIO[2])))
    y2 = max(y1, min(height, round(height * BATTLE_ANIMATION_CROP_RATIO[3])))
    roi = array[y1:y2, x1:x2]
    diagnostics: dict[str, object] = {
        "roi": (x1, y1, x2, y2),
        "blue_pixels": 0,
        "blue_ratio": 0.0,
        "largest_component": 0,
        "largest_bbox": None,
        "detected": False,
    }
    if roi.size == 0:
        return False, diagnostics

    hsv = cv2.cvtColor(roi, cv2.COLOR_RGB2HSV)
    # Dark blue battle backgrounds and HUD elements are common in normal
    # command frames.  Only use the brighter blue veil/arrow pixels for the
    # animation decision; otherwise a dark stage can become one giant mask.
    blue_mask = cv2.inRange(
        hsv,
        np.array((90, 100, BATTLE_ANIMATION_MIN_VALUE), dtype=np.uint8),
        np.array((135, 255, 255), dtype=np.uint8),
    )
    blue_pixels = int((blue_mask > 0).sum())
    roi_area = int(blue_mask.shape[0] * blue_mask.shape[1])
    component_count, _, stats, _ = cv2.connectedComponentsWithStats(blue_mask, 8)
    largest_index = 1 + int(np.argmax(stats[1:, cv2.CC_STAT_AREA])) if component_count > 1 else None
    largest_component = int(stats[largest_index, cv2.CC_STAT_AREA]) if largest_index is not None else 0
    largest_bbox = (
        tuple(int(value) for value in stats[largest_index, :4]) if largest_index is not None else None
    )
    blue_ratio = blue_pixels / roi_area if roi_area else 0.0
    # The supplied animation frame has a large, bright connected component.
    # Require both a meaningful absolute size and area ratio so ordinary blue
    # skill effects remain below the threshold.
    detected = blue_ratio >= 0.05 and largest_component >= max(20_000, round(roi_area * 0.05))
    diagnostics.update(
        {
            "blue_pixels": blue_pixels,
            "blue_ratio": round(blue_ratio, 4),
            "largest_component": largest_component,
            "largest_bbox": largest_bbox,
            "detected": detected,
        }
    )
    return detected, diagnostics


def _battle_badge_color_candidates(
    preferred: str | None = None,
) -> list[tuple[str, tuple[int, int, int], tuple[int, int, int]]]:
    """Return the badge HSV ranges to try, newest UI colour first."""

    if preferred is None:
        return list(BATTLE_BADGE_COLOR_RANGES)
    for candidate in BATTLE_BADGE_COLOR_RANGES:
        if candidate[0] == preferred:
            return [candidate]
    raise ValueError(f"unsupported battle badge colour: {preferred}")


def _battle_badge_hsv_range(
    color: str,
) -> tuple[tuple[int, int, int], tuple[int, int, int]]:
    candidates = _battle_badge_color_candidates(color)
    _, lower, upper = candidates[0]
    return lower, upper


def _battle_portrait_slot_detection(
    array: np.ndarray,
    *,
    slot_centers: Sequence[float],
    max_count: int,
    badge_color: str | None = None,
) -> tuple[int | None, dict[str, object]]:
    """Classify every fixed command slot as live, empty, or unknown."""

    badge_color = badge_color or "blue"
    badge_lower, badge_upper = _battle_badge_hsv_range(badge_color)

    height, width = array.shape[:2]
    diagnostics: dict[str, object] = {
        "shape": tuple(int(value) for value in array.shape),
        "badge_color": badge_color,
        "slot_centers": [round(float(value), 2) for value in slot_centers],
        "slot_states": [],
        "reason": "unknown",
    }
    if not slot_centers:
        diagnostics["reason"] = "no_slots"
        return None, diagnostics
    if len(slot_centers) > 12:
        diagnostics["reason"] = "too_many_slots"
        return None, diagnostics

    animation_detected, animation_diagnostics = _battle_animation_overlay_detection(array)
    diagnostics["animation"] = animation_diagnostics
    if animation_detected:
        diagnostics["reason"] = "animation_overlay"
        return None, diagnostics

    scale = height / 1440
    hsv = cv2.cvtColor(array, cv2.COLOR_RGB2HSV)
    badge_mask = cv2.inRange(
        hsv,
        np.array(badge_lower, dtype=np.uint8),
        np.array(badge_upper, dtype=np.uint8),
    )
    live_indexes: list[int] = []
    unknown_indexes: list[int] = []
    slot_states: list[dict[str, object]] = []
    badge_y1 = max(0, min(height, round(height * BATTLE_PORTRAIT_BADGE_Y_START_RATIO)))
    badge_y2 = max(badge_y1, min(height, round(height * BATTLE_PORTRAIT_BADGE_Y_END_RATIO)))
    inner_y1 = max(0, min(height, round(height * BATTLE_PORTRAIT_INNER_Y_START_RATIO)))
    inner_y2 = max(inner_y1, min(height, round(height * BATTLE_PORTRAIT_INNER_Y_END_RATIO)))
    badge_half_width = max(12, round(39 * scale))
    inner_half_width = max(10, round(31 * scale))

    for index, center_x_value in enumerate(slot_centers):
        center_x = round(float(center_x_value))
        badge_x1 = max(0, center_x - badge_half_width)
        badge_x2 = min(width, center_x + badge_half_width + 1)
        badge = badge_mask[badge_y1:badge_y2, badge_x1:badge_x2]
        badge_pixels = int((badge > 0).sum())
        component_count, _, stats, _ = cv2.connectedComponentsWithStats(badge, 8)
        largest_badge = 0
        largest_badge_size = (0, 0)
        if component_count > 1:
            largest_index = 1 + int(np.argmax(stats[1:, cv2.CC_STAT_AREA]))
            largest_badge = int(stats[largest_index, cv2.CC_STAT_AREA])
            largest_badge_size = (
                int(stats[largest_index, cv2.CC_STAT_WIDTH]),
                int(stats[largest_index, cv2.CC_STAT_HEIGHT]),
            )

        inner_x1 = max(0, center_x - inner_half_width)
        inner_x2 = min(width, center_x + inner_half_width + 1)
        inner = hsv[inner_y1:inner_y2, inner_x1:inner_x2]
        blue = (
            (inner[:, :, 0] >= 90)
            & (inner[:, :, 0] <= 135)
            & (inner[:, :, 1] >= 80)
            & (inner[:, :, 2] >= 40)
        )
        cyan = (
            (inner[:, :, 0] >= 80)
            & (inner[:, :, 0] <= 115)
            & (inner[:, :, 1] >= 100)
            & (inner[:, :, 2] >= 70)
        )
        inner_area = max(1, inner.shape[0] * inner.shape[1])
        blue_ratio = float(blue.sum() / inner_area)
        placeholder_ratio = float(cyan.sum() / inner_area)
        avatar_v_std = float(inner[:, :, 2].std()) if inner.size else 0.0
        avatar_unique = int(np.unique(inner[:, :, 2]).size) if inner.size else 0

        if largest_badge >= 300 and min(largest_badge_size) >= max(12, round(28 * scale)):
            # The level badge may remain visible while an attack/skill effect
            # covers the avatar.  Require enough variation in the inner
            # portrait area before accepting LIVE; otherwise the whole slot
            # is UNKNOWN and the caller must use normal P.
            if avatar_v_std < 45.0 or avatar_unique < 32:
                state = "UNKNOWN"
                unknown_indexes.append(index)
            else:
                state = "LIVE"
                live_indexes.append(index)
        else:
            # Empty command slots contain the blue/cyan sword or reload
            # placeholder.  A live avatar with a missing badge does not pass
            # this strict placeholder test and remains UNKNOWN.
            if placeholder_ratio >= 0.14 and blue_ratio >= 0.30:
                state = "EMPTY"
            else:
                state = "UNKNOWN"
                unknown_indexes.append(index)

        slot_states.append(
            {
                "index": index,
                "center_x": round(float(center_x_value), 2),
                "state": state,
                "badge_pixels": badge_pixels,
                "largest_badge": largest_badge,
                "badge_size": largest_badge_size,
                "placeholder_ratio": round(placeholder_ratio, 4),
                "blue_ratio": round(blue_ratio, 4),
                "avatar_v_std": round(avatar_v_std, 2),
                "avatar_unique": avatar_unique,
            }
        )

    diagnostics["slot_states"] = slot_states
    diagnostics["live_indexes"] = live_indexes
    diagnostics["unknown_indexes"] = unknown_indexes
    if unknown_indexes:
        diagnostics["reason"] = "slot_unknown"
        return None, diagnostics
    if not live_indexes:
        diagnostics["reason"] = "no_live_slots"
        return None, diagnostics
    if len(live_indexes) > max(0, min(int(max_count), 12)):
        diagnostics["reason"] = "more_than_selected"
        return None, diagnostics
    diagnostics["reason"] = "ok"
    return len(live_indexes), diagnostics


def _scan_badge_runs(
    array: np.ndarray,
    hsv: np.ndarray,
    *,
    color_name: str,
    lower: tuple[int, int, int],
    upper: tuple[int, int, int],
    max_count: int,
    animation_diagnostics: dict[str, object] | None,
) -> tuple[int | None, dict[str, object]]:
    """Scan the bottom badge band for one badge colour."""

    height, width = array.shape[:2]
    diagnostics: dict[str, object] = {
        "shape": tuple(int(value) for value in array.shape),
        "badge_color": color_name,
        "roi": None,
        "badge_pixels": 0,
        # Keep the legacy key for log consumers that already parse it.
        "orange_pixels": 0,
        "candidate_runs": [],
        "accepted_runs": [],
        "reason": "unknown",
    }
    if animation_diagnostics is not None:
        diagnostics["animation"] = animation_diagnostics

    mask = cv2.inRange(
        hsv,
        np.array(lower, dtype=np.uint8),
        np.array(upper, dtype=np.uint8),
    )
    y1 = max(0, min(height, round(height * BATTLE_PORTRAIT_Y_START_RATIO)))
    y2 = max(y1, min(height, round(height * BATTLE_PORTRAIT_Y_END_RATIO)))
    x1 = max(0, min(width, round(width * BATTLE_PORTRAIT_X_START_RATIO)))
    x2 = max(x1, min(width, round(width * BATTLE_PORTRAIT_X_END_RATIO)))
    diagnostics["roi"] = (x1, y1, x2, y2)
    roi_mask = mask[y1:y2, x1:x2]
    diagnostics["badge_pixels"] = int((roi_mask > 0).sum())
    diagnostics["orange_pixels"] = diagnostics["badge_pixels"]
    projection = (roi_mask > 0).sum(axis=0)
    active_columns = projection >= 10
    badge_scale = height / 1080
    min_badge_height = max(16, round(BATTLE_BADGE_MIN_HEIGHT_AT_1080 * badge_scale))
    min_badge_width = max(18, round(BATTLE_BADGE_MIN_WIDTH_AT_1080 * badge_scale))
    min_badge_area = max(180, round(BATTLE_BADGE_MIN_AREA_AT_1080 * badge_scale**2))
    max_badge_side = max(32, round(65 * badge_scale))
    diagnostics["badge_validation_thresholds"] = {
        "min_component_width": min_badge_width,
        "min_component_height": min_badge_height,
        "min_component_area": min_badge_area,
        "max_component_side": max_badge_side,
        "component_aspect_min": 0.80,
        "component_aspect_max": 1.25,
    }

    candidate_runs: list[tuple[int, int, int, int]] = []
    accepted_runs: list[tuple[int, int, int, int]] = []
    badge_component_details: list[dict[str, object]] = []
    run_start: int | None = None
    for index, active in enumerate(np.r_[active_columns, False]):
        if active and run_start is None:
            run_start = index
            continue
        if active or run_start is None:
            continue

        run_end = index
        run_width = run_end - run_start
        run_area = int(projection[run_start:run_end].sum())
        run = (x1 + run_start, x1 + run_end, run_width, run_area)
        if len(candidate_runs) < 64:
            candidate_runs.append(run)

        run_mask = roi_mask[:, run_start:run_end]
        component_count, _, stats, _ = cv2.connectedComponentsWithStats(run_mask, 8)
        largest_component = 0
        largest_component_size = (0, 0)
        if component_count > 1:
            largest_index = 1 + int(np.argmax(stats[1:, cv2.CC_STAT_AREA]))
            largest_component = int(stats[largest_index, cv2.CC_STAT_AREA])
            largest_component_size = (
                int(stats[largest_index, cv2.CC_STAT_WIDTH]),
                int(stats[largest_index, cv2.CC_STAT_HEIGHT]),
            )

        # A visible level badge is approximately 40 px wide and 41 px high at
        # the reference screenshot scale and is roughly square.  A blue status
        # effect can merge several badges into one wide horizontal projection
        # run, so validate every connected component in the run instead of
        # accepting/rejecting the whole run from its largest component.
        valid_components: list[dict[str, object]] = []
        if component_count > 1:
            for component_index in range(1, component_count):
                comp_x, _, comp_w, comp_h, comp_area = (
                    int(value) for value in stats[component_index, :5]
                )
                if comp_w < min_badge_width or comp_h < min_badge_height:
                    continue
                if comp_area < min_badge_area:
                    continue
                if comp_w > max_badge_side or comp_h > max_badge_side:
                    continue
                aspect = comp_w / max(comp_h, 1)
                if not 0.80 <= aspect <= 1.25:
                    continue
                component_left = x1 + run_start + comp_x
                component_right = component_left + comp_w
                accepted_runs.append((component_left, component_right, comp_w, comp_area))
                valid_components.append(
                    {
                        "bbox": (component_left, component_right),
                        "size": (comp_w, comp_h),
                        "area": comp_area,
                        "aspect": round(aspect, 3),
                    }
                )

        shape_valid = bool(valid_components)
        if len(badge_component_details) < 64:
            badge_component_details.append(
                {
                    "run": run,
                    "largest_component": largest_component,
                    "component_size": largest_component_size,
                    "valid_components": valid_components[:8],
                    "shape_valid": shape_valid,
                }
            )
        run_start = None

    diagnostics["candidate_runs"] = candidate_runs
    diagnostics["candidate_run_total"] = len(candidate_runs)
    diagnostics["badge_component_details"] = badge_component_details
    diagnostics["accepted_runs"] = accepted_runs
    max_allowed = max(0, min(int(max_count), 12))
    if len(accepted_runs) < 1:
        diagnostics["reason"] = "no_badge"
        return None, diagnostics
    if len(accepted_runs) > max_allowed:
        diagnostics["reason"] = "more_than_selected"
        return None, diagnostics
    diagnostics["reason"] = "ok"
    return len(accepted_runs), diagnostics


def _battle_portrait_badge_detection(
    image: object,
    *,
    max_count: int = 12,
    slot_centers: Sequence[float] | None = None,
    badge_color: str | None = None,
) -> tuple[int | None, dict[str, object]]:
    """Return the portrait count and compact diagnostics for one frame.

    Blue level circles are used by the current 1440p UI; the orange range is
    kept as a compatibility fallback for older captures.  When ``badge_color``
    is supplied only that colour is scanned, which lets the observer retry the
    whole badge + avatar validation pipeline with the other colour.
    """

    diagnostics: dict[str, object] = {
        "shape": None,
        "roi": None,
        "badge_pixels": 0,
        "orange_pixels": 0,
        "candidate_runs": [],
        "accepted_runs": [],
        "reason": "unknown",
        "badge_color": badge_color,
        "badge_colors_tried": [],
    }

    try:
        array = np.asarray(image)
        diagnostics["shape"] = tuple(int(value) for value in array.shape)
        if array.ndim != 3 or array.shape[2] not in (3, 4):
            diagnostics["reason"] = "not_rgb"
            return None, diagnostics
        if array.shape[2] == 4:
            array = array[:, :, :3]
        height, width = array.shape[:2]
        if height < 1 or width < 1:
            diagnostics["reason"] = "empty_image"
            return None, diagnostics

        if slot_centers is not None:
            return _battle_portrait_slot_detection(
                array,
                slot_centers=slot_centers,
                max_count=max_count,
                badge_color=badge_color,
            )

        animation_detected, animation_diagnostics = _battle_animation_overlay_detection(array)
        diagnostics["animation"] = animation_diagnostics
        if animation_detected:
            diagnostics["reason"] = "animation_overlay"
            return None, diagnostics

        hsv = cv2.cvtColor(array, cv2.COLOR_RGB2HSV)
        first_diagnostics: dict[str, object] | None = None
        best_diagnostics: dict[str, object] | None = None
        for color_name, lower, upper in _battle_badge_color_candidates(badge_color):
            diagnostics["badge_colors_tried"].append(color_name)
            count, attempt_diagnostics = _scan_badge_runs(
                array,
                hsv,
                color_name=color_name,
                lower=lower,
                upper=upper,
                max_count=max_count,
                animation_diagnostics=animation_diagnostics,
            )
            if first_diagnostics is None:
                first_diagnostics = attempt_diagnostics
            if count is not None:
                attempt_diagnostics["badge_colors_tried"] = list(diagnostics["badge_colors_tried"])
                attempt_diagnostics["badge_fallback"] = color_name != diagnostics["badge_colors_tried"][0]
                return count, attempt_diagnostics
            if best_diagnostics is None or len(attempt_diagnostics.get("accepted_runs", [])) > len(
                best_diagnostics.get("accepted_runs", [])
            ):
                best_diagnostics = attempt_diagnostics

        result = best_diagnostics or first_diagnostics or diagnostics
        result["badge_colors_tried"] = list(diagnostics["badge_colors_tried"])
        result["badge_fallback"] = len(diagnostics["badge_colors_tried"]) > 1
        result["badge_color"] = None
        return None, result
    except (TypeError, ValueError, cv2.error) as error:
        diagnostics["reason"] = f"exception:{type(error).__name__}"
        return None, diagnostics


def battle_portrait_badge_count(
    image: object,
    *,
    max_count: int = 12,
    slot_centers: Sequence[float] | None = None,
) -> int | None:
    """Count the visible living battle portraits from their level badges.

    A defeated sinner no longer has a bottom portrait, including its level
    badge.  The badge is a repeated, localized-independent visual element, so
    counting it avoids OCR and avoids using the decorative gear anchors that
    can also match skill icons or battle effects.

    ``image`` must be an RGB image (PIL images and the arrays returned by
    ``Automation._get_screenshot_array(gray=False)`` both satisfy this).
    Unknown/partial frames return ``None`` so the caller can use normal
    automatic P without guessing a survivor count.
    """

    count, _ = _battle_portrait_badge_detection(image, max_count=max_count, slot_centers=slot_centers)
    return count


def _battle_visible_portrait_detection(
    image: object,
    *,
    max_count: int = 12,
) -> tuple[int | None, dict[str, object]]:
    """Count and validate the portraits that are actually visible in a frame.

    The number of command slots is not the number of deployed combatants: it
    can change with the roster and it can also be misread when a gear anchor
    is covered by an effect.  First locate the level badges globally, then
    validate the avatar area around each badge.  This makes the badge runs
    themselves the dynamic slot list and keeps an obscured avatar as UNKNOWN
    instead of silently counting it as alive.

    The current UI uses blue level circles; orange is retried as a whole
    badge + avatar pipeline when the blue scan produces no usable roster.
    Retrying at this level (rather than only inside the badge scan) prevents
    an old UI's blue/cyan empty-slot placeholder from being accepted as a
    living portrait.
    """

    array = np.asarray(image)
    diagnostics: dict[str, object] = {
        "shape": tuple(int(value) for value in array.shape),
        "detection_source": "visible_badges",
        "badge_attempts": [],
        "reason": "unknown",
    }
    if array.ndim != 3 or array.shape[2] not in (3, 4):
        diagnostics["reason"] = "not_rgb"
        return None, diagnostics
    if array.shape[2] == 4:
        array = array[:, :, :3]

    first_failure: dict[str, object] | None = None
    for color_name, _, _ in _battle_badge_color_candidates():
        badge_count, badge_diagnostics = _battle_portrait_badge_detection(
            array,
            max_count=max_count,
            badge_color=color_name,
        )
        diagnostics["badge_attempts"].append(
            {
                "badge_color": color_name,
                "badge_count": badge_count,
                "badge_reason": badge_diagnostics.get("reason"),
            }
        )
        if badge_count is None:
            if first_failure is None:
                first_failure = badge_diagnostics
            continue

        accepted_runs = badge_diagnostics.get("accepted_runs", [])
        centers: list[float] = []
        if isinstance(accepted_runs, (list, tuple)):
            for run in accepted_runs:
                if not isinstance(run, (list, tuple)) or len(run) < 2:
                    continue
                try:
                    centers.append((float(run[0]) + float(run[1]) - 1.0) / 2.0)
                except (TypeError, ValueError):
                    continue

        merged = dict(badge_diagnostics)
        merged["badge_color"] = color_name
        merged["badge_fallback"] = color_name != "blue"
        merged["visible_badge_centers"] = [round(value, 2) for value in centers]
        merged["detection_source"] = "visible_badges"
        merged["badge_attempts"] = list(diagnostics["badge_attempts"])

        live_count, slot_diagnostics = _battle_portrait_slot_detection(
            array,
            slot_centers=centers,
            max_count=max_count,
            badge_color=color_name,
        )
        # Preserve the global scan details in the same top-level diagnostics
        # used by the GUI log while adding the per-avatar validation details.
        for key in ("slot_centers", "slot_states", "live_indexes", "unknown_indexes", "animation", "reason"):
            if key in slot_diagnostics:
                merged[key] = slot_diagnostics[key]
        merged["badge_validation"] = slot_diagnostics
        if live_count is not None:
            return live_count, merged
        if first_failure is None:
            first_failure = merged

    failure = first_failure or diagnostics
    failure["badge_attempts"] = list(diagnostics["badge_attempts"])
    return None, failure


class _DefenseState(Protocol):
    remaining_turns: int

    def consume_turn(self) -> None: ...


class PseudoSoloDefenseState:
    """Decide whether pseudo-solo must keep every visible ally on defense.

    ``remaining_turns`` is retained for compatibility with the legacy state
    wrapper and configuration.  The battle runner uses :meth:`should_defend`
    instead: a reliable multiple-survivor observation is the only condition
    that requests the special all-defense round.  A single survivor or an
    unreadable/occluded frame follows the normal automatic-P path.
    """

    def __init__(
        self,
        base_state: _DefenseState,
        observer: Callable[[], PseudoSoloObservation],
    ) -> None:
        self._base_state = base_state
        self._observer = observer
        # The configured value is only a compatibility fallback.  A reliable
        # team-page count replaces it with ``live_count - 1`` for each new
        # roster state.
        self._fallback_turn_limit = max(0, base_state.remaining_turns)
        self._stop_confirmed = False
        self._battle_observed = False
        self._battle_observation = PseudoSoloObservation.UNKNOWN
        self._battle_live_count: int | None = None
        self._observation_attempts = 0
        self._last_observation_at = 0.0
        self._last_team_page_observation: tuple[int, int] | None = None
        self._last_live_count: int | None = None

    @property
    def remaining_turns(self) -> int:
        return max(0, self._base_state.remaining_turns)

    def begin_battle(self) -> None:
        """Start one battle-scoped survivor observation and defense budget."""

        self._battle_observed = False
        self._battle_observation = PseudoSoloObservation.UNKNOWN
        self._battle_live_count = None
        self._observation_attempts = 0
        self._last_observation_at = 0.0
        self._stop_confirmed = False
        self._base_state.remaining_turns = 0
        begin_observation = getattr(self._observer, "begin_battle", None)
        if callable(begin_observation):
            begin_observation()
        cached_count = self._run_scoped_live_count()
        if cached_count is not None:
            # Seed the budget from the run cache, but leave the battle
            # unobserved: ``should_defend`` still verifies the number once
            # through the battle portrait row before any defense is submitted.
            self._apply_run_live_count(cached_count)
            self._battle_observed = False
            log.info(
                "伪单通本场战斗复用整局人数缓存（待战斗行复核）："
                f"存活{cached_count}，守备预算{self.remaining_turns}回合"
            )
            return
        log.debug(
            "伪单通开始本场战斗识别周期："
            f"UNKNOWN时最多重试{BATTLE_OBSERVATION_MAX_ATTEMPTS}次"
        )

    def consume_turn(self) -> None:
        if self.remaining_turns > 0:
            self._base_state.consume_turn()

    def _run_scoped_live_count(self) -> int | None:
        """Return the run-scoped count when the observer supports one."""

        return getattr(self._observer, "run_live_count", None)

    def _apply_run_live_count(self, live_count: int) -> None:
        """Adopt a run-scoped count as this battle's observation and budget."""

        count = max(0, int(live_count))
        multiple = count > 1
        self._battle_observed = True
        self._battle_observation = (
            PseudoSoloObservation.MULTIPLE_SURVIVORS if multiple else PseudoSoloObservation.SINGLE_SURVIVOR
        )
        self._battle_live_count = count
        if multiple:
            self._base_state.remaining_turns = max(0, defense_turns_for_live_count(count))
            self._stop_confirmed = False
        else:
            self._base_state.remaining_turns = 0
            self._stop_confirmed = True

    def should_defend(self) -> bool:
        """Return whether pseudo-solo should submit an all-defense round.

        The first call in a battle captures the roster and converts it to the
        exact defense budget (living count minus one).  If that first window
        lands during the battle-entry animation the result is UNKNOWN; the
        observer is retried a few times at a bounded cadence instead of caching
        UNKNOWN for the whole battle.  Once a reliable result is found it is
        fixed for the rest of the battle.
        """

        now = time.monotonic()
        cached_count = self._run_scoped_live_count()
        if cached_count is not None:
            if not self._battle_observed:
                # Verify the cached count once per battle through the battle
                # portrait row.  Skipping this made a wrong cached value
                # impossible to correct, and the team-page OCR counter has
                # already locked the run at ``3`` while a single ally was
                # alive.  The battle row is the only authoritative reader, so
                # a confirmed drop here also lowers the defense budget.
                self._observation_attempts += 1
                self._last_observation_at = now
                self._observer()
                recomputed = self._run_scoped_live_count()
                if recomputed is not None and recomputed != cached_count:
                    log.info(
                        "伪单通整局人数缓存被战斗行读数修正："
                        f"{cached_count}->{recomputed}，守备预算将按新人数重算"
                    )
                    cached_count = recomputed
                self._battle_observed = True
                self._apply_run_live_count(cached_count)
                log.info(
                    "伪单通整局人数缓存生效："
                    f"存活{cached_count}，守备预算{self.remaining_turns}回合"
                )
            elif self._battle_live_count != cached_count:
                self._apply_run_live_count(cached_count)
            observation = self._battle_observation
            observed_count = self._battle_live_count
            observed_now = False
        else:
            first_attempt = not self._battle_observed
            retry_due = (
                self._battle_observed
                and self._battle_observation == PseudoSoloObservation.UNKNOWN
                and self._observation_attempts < BATTLE_OBSERVATION_MAX_ATTEMPTS
                and now - self._last_observation_at >= BATTLE_OBSERVATION_RETRY_SECONDS
            )
            observed_now = first_attempt or retry_due
            if not observed_now:
                observation = self._battle_observation
                observed_count = self._battle_live_count
        if observed_now:
            observation = self._observer()
            observed_count = getattr(self._observer, "last_battle_live_count", None)
            if observed_count is None:
                observed_count = getattr(self._observer, "battle_live_count", None)
            self._observation_attempts += 1
            self._last_observation_at = now
            self._battle_observed = True
            self._battle_observation = observation
            self._battle_live_count = observed_count

            if observation == PseudoSoloObservation.MULTIPLE_SURVIVORS:
                required_turns = (
                    defense_turns_for_live_count(observed_count)
                    if observed_count is not None
                    else self._fallback_turn_limit
                )
                self._base_state.remaining_turns = max(0, required_turns)
                self._stop_confirmed = False
                log.info(
                    "伪单通本场战斗人数识别完成："
                    f"存活{observed_count if observed_count is not None else '多人'}，"
                    f"计算守备{self._base_state.remaining_turns}回合"
                )
            elif observation == PseudoSoloObservation.SINGLE_SURVIVOR:
                self._base_state.remaining_turns = 0
                self._stop_confirmed = True
                log.info("伪单通本场战斗识别到仅剩1名人格，不执行守备")
            else:
                self._base_state.remaining_turns = 0
                if self._observation_attempts >= BATTLE_OBSERVATION_MAX_ATTEMPTS:
                    log.info(
                        "伪单通本场战斗人数识别为UNKNOWN"
                        f"（已重试{self._observation_attempts}次），本场按普通P处理"
                    )
                else:
                    log.info(
                        "伪单通本场战斗人数识别为UNKNOWN，"
                        f"{BATTLE_OBSERVATION_RETRY_SECONDS:.1f}s后重试"
                    )
        else:
            observation = self._battle_observation
            observed_count = self._battle_live_count

        selected_count = getattr(self._observer, "selected_count", None)
        should_defend = observation == PseudoSoloObservation.MULTIPLE_SURVIVORS and self.remaining_turns > 0

        observation_name = getattr(observation, "value", str(observation))
        log.debug(
            "伪单通守备决策: "
            f"observation={observation_name} battle_live_count={observed_count} "
            f"selected_count={selected_count} stop_confirmed={self._stop_confirmed} "
            f"battle_observed={self._battle_observed} remaining_turns={self.remaining_turns} "
            f"attempts={self._observation_attempts} should_defend={should_defend}"
        )
        if observation == PseudoSoloObservation.UNKNOWN:
            if self._observation_attempts >= BATTLE_OBSERVATION_MAX_ATTEMPTS:
                log.debug("伪单通复用已耗尽重试的UNKNOWN结果，本回合按普通P处理")
            elif observed_now:
                log.debug("伪单通本轮UNKNOWN，等待下一次重试")
            else:
                log.debug("伪单通复用本场UNKNOWN人数结果，等待重试窗口")
        return should_defend

    @property
    def defense_cycle_complete(self) -> bool:
        """Return whether the current defense cycle has reached its stop point."""

        return self._stop_confirmed or self._base_state.remaining_turns <= 0

    @property
    def live_count(self) -> int | None:
        """Return the best known living-ally count.

        A confirmed run-scoped count wins over the per-floor team-page value
        because permanent deaths make it authoritative for the rest of the run.
        """

        cached = self._run_scoped_live_count()
        if cached is not None:
            return cached
        return self._last_live_count

    @property
    def last_battle_live_count(self) -> int | None:
        """Return the last battle-row count used for the current decision."""

        cached = self._run_scoped_live_count()
        if self._battle_observed:
            return self._battle_live_count
        if cached is not None:
            return cached
        return getattr(self._observer, "last_battle_live_count", None)

    @property
    def last_observation(self) -> PseudoSoloObservation:
        """Return the observer's last battle observation."""

        if self._battle_observed:
            return self._battle_observation
        return getattr(self._observer, "last_observation", PseudoSoloObservation.UNKNOWN)

    @property
    def battle_observation_pending(self) -> bool:
        """Return whether this battle still needs its first row observation.

        A run-scoped cache still needs one verification frame per battle: the
        battle loop waits for the command row before observing, so the cheap
        single-frame check lands on a complete frame instead of the entry
        animation.
        """

        return not self._battle_observed

    def observe_team_page(self, floor: int) -> bool:
        """Synchronize the defense budget with the visible live roster.

        The team-selection page is the authoritative place to count living
        allies.  For every newly observed roster state, the pseudo-solo budget
        is exactly the number of living teammates besides Ryoshu.  Repeated
        frames with the same count do not reset a partially consumed budget.
        If the count cannot be read, the configured compatibility fallback is
        left untouched.

        When the observer keeps a run-scoped ledger (permanent deaths after
        ``赤红纠缠的蜘蛛巢``), only an actual count change restarts observation;
        a new floor with the same count keeps the cached roster.
        """

        read_live_count = getattr(self._observer, "read_team_page_live_count", None)
        if not callable(read_live_count):
            log.debug(f"伪单通队伍页同步跳过: floor={floor} observer不支持队伍页人数读取")
            return False
        live_count = read_live_count()
        if live_count is None:
            log.debug(
                "伪单通队伍页同步未得到可靠人数: "
                f"floor={floor} selected_count={getattr(self._observer, 'selected_count', None)} "
                f"remaining_turns={self._base_state.remaining_turns}"
            )
            return False

        live_count = max(0, int(live_count))
        observation = (floor, live_count)
        repeated_observation = self._last_team_page_observation == observation
        previous_observation = self._last_team_page_observation
        floor_changed = previous_observation is not None and previous_observation[0] != floor
        self._last_team_page_observation = observation
        self._last_live_count = live_count
        cached_count = self._run_scoped_live_count()
        if cached_count is not None:
            # The ledger already folded this reading in, so a repeated frame is
            # still worth re-checking: a confirmed drop must reach the budget.
            return self._adopt_team_page_run_count(floor, live_count, cached_count)
        if repeated_observation:
            return False

        if live_count <= 1:
            changed = floor_changed or self._base_state.remaining_turns > 0 or not self._stop_confirmed
            self._base_state.remaining_turns = 0
            self._stop_confirmed = True
            self._battle_observed = False
            self._battle_observation = PseudoSoloObservation.UNKNOWN
            self._battle_live_count = None
            reset_observer = getattr(self._observer, "reset", None)
            if changed and callable(reset_observer):
                reset_observer()
            if changed:
                log.info(f"第{floor}层队伍页确认仅剩{live_count}名己方人格，停止连续防御")
            return changed

        required_turns = defense_turns_for_live_count(live_count)
        changed = floor_changed or self._base_state.remaining_turns != required_turns or self._stop_confirmed
        self._base_state.remaining_turns = required_turns
        self._stop_confirmed = False
        self._battle_observed = False
        self._battle_observation = PseudoSoloObservation.UNKNOWN
        self._battle_live_count = None
        reset_observer = getattr(self._observer, "reset", None)
        if callable(reset_observer):
            reset_observer()
        if changed:
            log.info(f"第{floor}层队伍页检测到{live_count}名存活己方人格，伪单通连续防御调整为{required_turns}回合")
        return changed

    def _adopt_team_page_run_count(self, floor: int, live_count: int, cached_count: int) -> bool:
        """Fold a team-page reading into the run-scoped ledger.

        The ledger owns the decision: a matching count changes nothing, a lower
        count still needs its second confirmation, and a higher count is
        rejected as a mis-read because deaths are permanent.
        """

        if live_count != cached_count:
            log.debug(
                "伪单通队伍页人数与整局缓存不一致，保留缓存: "
                f"floor={floor} team_page={live_count} cached={cached_count}"
            )
            return False
        required_turns = defense_turns_for_live_count(cached_count)
        changed = (
            self._base_state.remaining_turns != required_turns
            or self._stop_confirmed
            or not self._battle_observed
        )
        self._apply_run_live_count(cached_count)
        if not changed:
            return False
        log.info(
            f"第{floor}层队伍页确认整局人数缓存：存活{cached_count}名己方人格，"
            f"守备预算{required_turns}回合"
        )
        return True

    def reset_for_run(self) -> None:
        """Restore the initial cycle when the mirror run is restarted."""

        self._base_state.remaining_turns = self._fallback_turn_limit
        self._stop_confirmed = False
        self._battle_observed = False
        self._battle_observation = PseudoSoloObservation.UNKNOWN
        self._battle_live_count = None
        self._last_team_page_observation = None
        self._last_live_count = None
        reset_observer = getattr(self._observer, "reset_run", None)
        if not callable(reset_observer):
            reset_observer = getattr(self._observer, "reset", None)
        if callable(reset_observer):
            reset_observer()


class BattleRosterObserver:
    """Read the number of currently visible ally battle portraits.

    The lower battle row removes a dead sinner's portrait and its level badge.
    Counting those repeated badges gives the number of visible living
    combatants without OCR or a localized death label.  The visible badge
    positions are used as the slot list, so the detector does not assume
    there are seven (or any other fixed number of) deployed combatants.  A
    missing or partial portrait row returns ``None`` so the pseudo-solo policy
    can send the frame through normal automatic P instead of guessing.
    """

    def __init__(
        self,
        selected_sinners: Sequence[int],
        *,
        selected_count: int | None = None,
        no_revive: bool = False,
    ) -> None:
        selected = list(selected_sinners[:12])
        self.selected_count = selected_count if selected_count is not None else sum(bool(value) for value in selected)
        # ``no_revive`` mirrors ``赤红纠缠的蜘蛛巢``: dead allies stay dead for the
        # whole run, so one confirmed count stays authoritative until the run
        # restarts instead of being re-sampled once per battle.
        self.no_revive = bool(no_revive)
        self._ledger: SurvivorLedger | None = (
            SurvivorLedger(required_confirmations=BATTLE_ROSTER_REQUIRED_STABLE_SAMPLES)
            if self.no_revive
            else None
        )
        log.debug(
            f"伪单通人数观察器初始化: selected_count={self.selected_count} no_revive={self.no_revive}"
        )
        self._last_frame_token: tuple[object, int] | None = None
        self._last_observation = PseudoSoloObservation.UNKNOWN
        self._last_battle_frame_token: tuple[object, int] | None = None
        self._battle_detection_attempted = False
        self._battle_detection_attempts = 0
        self._last_battle_live_count: int | None = None
        self._last_battle_verification_reading: int | None = None
        self._last_battle_diagnostics: dict[str, object] = {}
        self._last_battle_slot_states: list[dict[str, object]] = []
        self._last_team_page_frame_token: tuple[object, int] | None = None
        self._last_team_page_live_count: int | None = None

    @property
    def run_live_count(self) -> int | None:
        """Return the confirmed run-scoped living-ally count, if any."""

        return self._ledger.confirmed if self._ledger is not None else None

    def enable_run_cache(self) -> None:
        """Start tracking one confirmed count for the rest of the run."""

        if self._ledger is not None:
            return
        self._ledger = SurvivorLedger(required_confirmations=BATTLE_ROSTER_REQUIRED_STABLE_SAMPLES)
        self.no_revive = True
        log.info("伪单通已启用整局存活人数缓存（队友阵亡后不复活）")

    def _record_run_count(self, count: int | None, *, source: str) -> None:
        """Offer one reading to the run ledger and log transitions."""

        if self._ledger is None:
            return
        update = self._ledger.record(count, source=source)
        if update.changed:
            log.info(
                "伪单通整局人数缓存确认: "
                f"live_count={update.confirmed} source={source} reason={update.reason}"
            )
        elif update.reason == SurvivorLedger.REASON_IMPOSSIBLE_INCREASE:
            log.warning(
                "伪单通整局人数缓存拒绝上升读数（队友阵亡后不复活）: "
                f"reading={count} cached={update.confirmed} source={source}"
            )
        elif not update.accepted and update.reason != SurvivorLedger.REASON_UNREADABLE:
            log.debug(
                "伪单通整局人数缓存忽略读数: "
                f"reading={count} cached={update.confirmed} source={source} reason={update.reason}"
            )

    def __call__(self) -> PseudoSoloObservation:
        frame_token = self._frame_token()
        if frame_token == self._last_frame_token:
            log.debug(
                "伪单通战斗人数观察复用缓存: "
                f"frame_id={frame_token[0]} screenshot_id={frame_token[1]} "
                f"live_count={self._last_battle_live_count} observation={self._last_observation.value}"
            )
            return self._last_observation
        self._last_frame_token = frame_token
        self._last_observation = self._read_current_frame()
        log.debug(
            "伪单通战斗人数观察结果: "
            f"frame_id={frame_token[0]} screenshot_id={frame_token[1]} "
            f"live_count={self._last_battle_live_count} result={self._last_observation.value}"
        )
        return self._last_observation

    def reset(self) -> None:
        """Forget battle and team-page observations for a new defense cycle."""

        self.begin_battle()
        self._last_team_page_frame_token = None
        self._last_team_page_live_count = None

    def reset_run(self) -> None:
        """Forget everything, including the run-scoped survivor cache."""

        self.reset()
        if self._ledger is not None:
            self._ledger.reset()

    def begin_battle(self) -> None:
        """Clear the battle cache before entering one new battle."""

        self._last_frame_token = None
        self._last_observation = PseudoSoloObservation.UNKNOWN
        self._last_battle_frame_token = None
        self._battle_detection_attempted = False
        self._battle_detection_attempts = 0
        self._last_battle_live_count = None
        self._last_battle_verification_reading = None
        self._last_battle_diagnostics = {}
        self._last_battle_slot_states = []

    def read_battle_live_count(self) -> int | None:
        """Read the number of visible living combatants in the battle row.

        Dead personalities no longer have a portrait or level badge.  Only the
        badges actually found in the frame become candidate slots;
        no command-slot count is inferred from gear anchors.  A
        partial/animated portrait is UNKNOWN rather than being silently
        omitted.  The battle is still observed only once, but that one
        observation is a short sequence of fresh frames.  This avoids caching
        the first transition frame as UNKNOWN while keeping the result fixed
        for the rest of the battle.
        """

        frame_token = self._frame_token()
        max_count = min(max(self.selected_count, 0), 12)
        frame_id, screenshot_id = frame_token
        run_count = self.run_live_count
        if run_count is not None:
            # A confirmed run-scoped count still needs one authoritative
            # battle-row reading per battle.  The team-page OCR counter has
            # locked the run at a wrong value (``3`` while a single ally was
            # alive), so the row reading is kept and exposed to the caller as
            # the authoritative correction.
            verified = self._verify_run_count_once()
            self._last_battle_verification_reading = verified
            self._last_battle_live_count = run_count
            log.debug(
                "伪单通战斗人数复用整局缓存: "
                f"frame_id={frame_id} screenshot_id={screenshot_id} "
                f"live_count={run_count} reading={verified}"
            )
            return run_count
        if self._battle_detection_attempted:
            if self._last_battle_live_count is not None:
                log.debug(
                    "伪单通战斗人数识别复用已锁定结果: "
                    f"frame_id={frame_id} screenshot_id={screenshot_id} "
                    f"live_count={self._last_battle_live_count}"
                )
                return self._last_battle_live_count
            if self._battle_detection_attempts >= BATTLE_ROSTER_MAX_ATTEMPTS:
                log.debug(
                    "伪单通战斗人数识别已达到重试上限: "
                    f"frame_id={frame_id} screenshot_id={screenshot_id} "
                    f"attempts={self._battle_detection_attempts} "
                    f"reason={self._last_battle_diagnostics.get('reason')}"
                )
                return None
            log.debug(
                "伪单通战斗人数识别上次为UNKNOWN，开始重试: "
                f"frame_id={frame_id} screenshot_id={screenshot_id} "
                f"attempts={self._battle_detection_attempts + 1}/{BATTLE_ROSTER_MAX_ATTEMPTS}"
            )
        elif frame_token == self._last_battle_frame_token:
            return self._last_battle_live_count

        self._last_battle_frame_token = frame_token
        self._last_battle_live_count = None
        self._last_battle_diagnostics = {}
        self._last_battle_slot_states = []
        self._battle_detection_attempted = True
        self._battle_detection_attempts += 1
        sample_limit = (
            BATTLE_ROSTER_MAX_SAMPLES
            if self._battle_detection_attempts == 1
            else BATTLE_ROSTER_RETRY_MAX_SAMPLES
        )
        if max_count < 1:
            log.debug(
                "伪单通战斗人数识别跳过: "
                f"frame_id={frame_id} screenshot_id={screenshot_id} selected_count={self.selected_count}"
            )
            return None

        def image_description(image: object) -> str:
            if image is None:
                return "none"
            mode = getattr(image, "mode", None)
            size = getattr(image, "size", None)
            shape = getattr(image, "shape", None)
            if shape is not None:
                try:
                    shape = tuple(int(value) for value in shape)
                except (TypeError, ValueError):
                    shape = str(shape)
            return f"type={type(image).__name__},mode={mode},size={size},shape={shape}"

        log.info(
            "伪单通战斗人数识别开始稳定采样: "
            f"selected_count={self.selected_count} max_samples={sample_limit} "
            f"attempt={self._battle_detection_attempts}/{BATTLE_ROSTER_MAX_ATTEMPTS} "
            f"required_stable={BATTLE_ROSTER_REQUIRED_STABLE_SAMPLES} "
            f"interval={BATTLE_ROSTER_SAMPLE_INTERVAL:.2f}s"
        )

        sampling_started = time.monotonic()
        sample_summaries: list[dict[str, object]] = []
        last_diagnostics: dict[str, object] = {}
        previous_count: int | None = None
        previous_centers: list[float] = []
        stable_samples = 0
        valid_samples = 0
        locked_count: int | None = None
        locked_diagnostics: dict[str, object] = {}
        use_fresh_monitor_frames = getattr(getattr(auto, "screenshot", None), "mode", None) == "L"

        for sample_index in range(max(1, int(sample_limit))):
            # 协作式取消：停止请求必须立即上抛，而不是跑完 MAX_SAMPLES
            # 轮截图 + CV 才退出（supervised runner 的 stop 优雅窗口只有约 3s）。
            check_cancelled()
            if sample_index > 0 and use_fresh_monitor_frames:
                interruptible_sleep(BATTLE_ROSTER_SAMPLE_INTERVAL)

            screenshot, screenshot_source, original_screenshot = self._capture_battle_color_frame(sample_index)
            live_count, diagnostics = _battle_visible_portrait_detection(
                screenshot,
                max_count=max_count,
            )
            last_diagnostics = diagnostics

            centers = self._battle_diagnostic_centers(diagnostics)
            if live_count is not None:
                valid_samples += 1
                if self._same_battle_sample(
                    previous_count,
                    previous_centers,
                    live_count,
                    centers,
                    diagnostics,
                ):
                    stable_samples += 1
                else:
                    stable_samples = 1
                previous_count = live_count
                previous_centers = centers
            else:
                stable_samples = 0
                previous_count = None
                previous_centers = []

            animation = diagnostics.get("animation")
            animation_detected = animation.get("detected") if isinstance(animation, dict) else None
            sample_summary = {
                "sample": sample_index + 1,
                "frame_id": getattr(auto, "_frame_id", None),
                "screenshot_id": id(screenshot),
                "source": screenshot_source,
                "result": live_count,
                "reason": diagnostics.get("reason"),
                "animation_detected": animation_detected,
                "centers": [round(value, 2) for value in centers],
                "stable_run": stable_samples,
            }
            sample_summaries.append(sample_summary)
            log.debug(
                "伪单通战斗人数识别采样详情: "
                f"sample={sample_index + 1}/{max(1, int(sample_limit))} "
                f"frame_id={getattr(auto, '_frame_id', None)} screenshot_id={id(screenshot)} "
                f"selected_count={self.selected_count} source={screenshot_source} "
                f"original={image_description(original_screenshot)} image={image_description(screenshot)} "
                f"result={live_count} reason={diagnostics.get('reason')} "
                f"stable_run={stable_samples} animation={diagnostics.get('animation')} "
                f"slot_states={diagnostics.get('slot_states')} "
                f"visible_badge_centers={diagnostics.get('visible_badge_centers')} "
                f"badge_color={diagnostics.get('badge_color')} "
                f"badge_colors_tried={diagnostics.get('badge_colors_tried')} "
                f"badge_fallback={diagnostics.get('badge_fallback')} "
                f"roi={diagnostics.get('roi')} orange_pixels={diagnostics.get('orange_pixels')} "
                f"candidate_runs={diagnostics.get('candidate_runs')} "
                f"badge_thresholds={diagnostics.get('badge_validation_thresholds')} "
                f"badge_components={diagnostics.get('badge_component_details')} "
                f"accepted_runs={diagnostics.get('accepted_runs')}"
            )

            if live_count is not None and stable_samples >= max(1, int(BATTLE_ROSTER_REQUIRED_STABLE_SAMPLES)):
                locked_count = live_count
                locked_diagnostics = diagnostics
                break

        diagnostics = dict(locked_diagnostics or last_diagnostics)
        diagnostics.setdefault("detection_source", "visible_badges")
        diagnostics["sample_count"] = len(sample_summaries)
        diagnostics["valid_sample_count"] = valid_samples
        diagnostics["stable_sample_count"] = stable_samples if locked_count is not None else 0
        diagnostics["sample_results"] = sample_summaries
        diagnostics["lock_reason"] = "stable_samples" if locked_count is not None else "no_stable_result"
        diagnostics["elapsed_ms"] = round((time.monotonic() - sampling_started) * 1000, 1)
        self._last_battle_diagnostics = diagnostics
        self._last_battle_slot_states = list(diagnostics.get("slot_states", []))

        log.info(
            "伪单通战斗人数识别锁定: "
            f"result={locked_count} lock_reason={diagnostics['lock_reason']} "
            f"samples={len(sample_summaries)} valid_samples={valid_samples} "
            f"stable_samples={diagnostics['stable_sample_count']} elapsed_ms={diagnostics['elapsed_ms']} "
            f"sample_results={sample_summaries}"
        )
        if locked_count is None:
            log.info(
                "伪单通战斗界面人数识别为UNKNOWN（"
                f"reason={diagnostics.get('reason')}），本回合不守备，按普通P处理"
            )
            return None

        self._last_battle_live_count = locked_count
        self._record_run_count(locked_count, source="battle_row")
        observation_name = "single_survivor" if locked_count == 1 else "multiple_survivors"
        log.info(f"伪单通战斗界面识别到{locked_count}名存活人格，状态={observation_name}")
        return locked_count

    def _verify_run_count_once(self) -> int | None:
        """Read one battle frame without disturbing the run-scoped cache.

        The reading is offered to the ledger, so a genuine death still needs
        two consistent frames and a bogus higher reading is rejected.  The
        cached value is always returned to the caller.
        """

        max_count = min(max(self.selected_count, 0), 12)
        if max_count < 1:
            return None
        screenshot, _, _ = self._capture_battle_color_frame(0)
        reading, diagnostics = _battle_visible_portrait_detection(screenshot, max_count=max_count)
        self._last_battle_diagnostics = diagnostics
        check_cancelled()
        self._record_run_count(reading, source="battle_row")
        return reading

    def _capture_battle_color_frame(self, sample_index: int) -> tuple[object, str, object]:
        """Capture one color frame without replacing the business screenshot."""

        original_screenshot = getattr(auto, "screenshot", None)
        screenshot = original_screenshot
        screenshot_source = "business_frame"

        # The battle loop normally keeps a grayscale business frame.  A fresh
        # color monitor capture is required for every sample so the sampler
        # can wait out the transition animation.  RGB test frames and callers
        # that already provide a color business frame remain usable without
        # forcing an unrelated screen capture.
        if getattr(original_screenshot, "mode", None) == "L":
            screenshot_source = "monitor_color_frame"
            take_monitor_screenshot = getattr(auto, "take_monitor_screenshot", None)
            if callable(take_monitor_screenshot):
                try:
                    try:
                        screenshot = take_monitor_screenshot(gray=False, max_age=0)
                    except TypeError:
                        screenshot = take_monitor_screenshot(gray=False)
                except userStopError:
                    # 取消信号不是截图失败，禁止降级为 None 继续采样。
                    raise
                except Exception as error:
                    log.debug(
                        "伪单通战斗界面彩色截图失败: "
                        f"sample={sample_index + 1} error={error}"
                    )
                    screenshot = None
                    screenshot_source = "monitor_capture_failed"
            else:
                screenshot = None
                screenshot_source = "no_color_capture_api"

        return screenshot, screenshot_source, original_screenshot

    @staticmethod
    def _battle_diagnostic_centers(diagnostics: dict[str, object]) -> list[float]:
        centers = diagnostics.get("visible_badge_centers", [])
        if not isinstance(centers, (list, tuple)):
            return []
        result: list[float] = []
        for center in centers:
            try:
                value = float(center)
            except (TypeError, ValueError):
                continue
            if np.isfinite(value):
                result.append(value)
        return result

    @staticmethod
    def _same_battle_sample(
        previous_count: int | None,
        previous_centers: Sequence[float],
        current_count: int,
        current_centers: Sequence[float],
        current_diagnostics: dict[str, object],
    ) -> bool:
        """Check that two accepted samples describe the same visible row."""

        if previous_count is None or previous_count != current_count:
            return False
        if len(previous_centers) != len(current_centers) or len(current_centers) != current_count:
            return False
        shape = current_diagnostics.get("shape")
        try:
            width = float(shape[1])  # type: ignore[index]
        except (IndexError, TypeError, ValueError):
            width = 1920.0
        tolerance = max(8.0, width * BATTLE_ROSTER_CENTER_TOLERANCE_RATIO)
        return all(abs(float(left) - float(right)) <= tolerance for left, right in zip(previous_centers, current_centers))

    @property
    def last_battle_live_count(self) -> int | None:
        """Return the last count obtained from a battle portrait row."""

        return self._last_battle_live_count

    @property
    def last_battle_verification_reading(self) -> int | None:
        """Return the raw battle-row reading taken while the run cache was used."""

        return self._last_battle_verification_reading

    @property
    def last_observation(self) -> PseudoSoloObservation:
        """Return the last battle observation."""

        return self._last_observation

    @property
    def last_battle_diagnostics(self) -> dict[str, object]:
        """Return detailed diagnostics for the last battle-row observation."""

        return self._last_battle_diagnostics

    @property
    def last_battle_slot_states(self) -> list[dict[str, object]]:
        """Return classifications for the visible badge positions."""

        return list(self._last_battle_slot_states)

    def read_team_page_live_count(self) -> int | None:
        """Read the live sinner count shown on the team-selection page.

        The 10/11/12 templates are tried first; they are the only counts with
        packaged assets and the current UI scores the 10-sinner template around
        0.818.  Every other count, including the smaller post-death rosters,
        is read from the right-side counter with OCR.  The old header crop
        picked up ``Backup Deployed 3/5`` instead of the roster count, so it is
        deliberately not used anymore.
        """

        frame_token = self._frame_token()
        if frame_token == self._last_team_page_frame_token:
            return self._last_team_page_live_count
        self._last_team_page_frame_token = frame_token
        self._last_team_page_live_count = None

        max_count = min(max(self.selected_count, 0), 12)
        try:
            for live_count in range(max_count, 9, -1):
                if auto.find_element(
                    f"teams/{live_count}_sinner_live_assets.png",
                    threshold=TEAM_PAGE_LIVE_COUNT_THRESHOLD,
                ):
                    self._last_team_page_live_count = live_count
                    self._record_run_count(live_count, source="team_page_asset")
                    log.debug(
                        "伪单通队伍页人数识别: "
                        f"frame_id={frame_token[0]} screenshot_id={frame_token[1]} "
                        f"source=asset selected_count={self.selected_count} live_count={live_count}"
                    )
                    return live_count

            if max_count < 1:
                return None
            count_crop = self._team_page_count_crop()
            texts = auto.get_text_from_screenshot(my_crop=count_crop)
        except Exception as error:
            log.debug(f"伪单通队伍页人数识别失败: {error}")
            return None

        text = "".join(str(value) for value in texts or [])
        # The old wrong crop returned this deployment hint.  Its first number
        # is not the roster count, so never parse it as one.
        if "backup" in text.casefold() or "deployed" in text.casefold():
            log.debug(
                "伪单通队伍页人数识别忽略部署提示: "
                f"frame_id={frame_token[0]} screenshot_id={frame_token[1]} "
                f"crop={count_crop} text={text!r}"
            )
            return None

        slash_match = re.search(r"(?<!\d)(\d{1,2})\s*/\s*(\d{1,2})(?!\d)", text)
        if slash_match:
            live_count, total_count = (int(value) for value in slash_match.groups())
            if total_count == max_count and 1 <= live_count <= total_count:
                self._last_team_page_live_count = live_count
                self._record_run_count(live_count, source="team_page_ocr")
                log.debug(
                    "伪单通队伍页人数识别: "
                    f"frame_id={frame_token[0]} screenshot_id={frame_token[1]} "
                    f"source=ocr crop={count_crop} text={text!r} "
                    f"live_count={live_count} total_count={total_count}"
                )
                return live_count
            log.debug(
                "伪单通队伍页人数识别分数不匹配: "
                f"frame_id={frame_token[0]} screenshot_id={frame_token[1]} "
                f"crop={count_crop} text={text!r} expected_total={max_count}"
            )
            return None

        bare_count = self._parse_bare_count(text, max_count)
        if bare_count is not None:
            self._last_team_page_live_count = bare_count
            self._record_run_count(bare_count, source="team_page_ocr")
            log.debug(
                "伪单通队伍页人数识别: "
                f"frame_id={frame_token[0]} screenshot_id={frame_token[1]} "
                f"source=ocr crop={count_crop} text={text!r} live_count={bare_count}"
            )
            return bare_count
        log.debug(
            "伪单通队伍页人数识别无有效结果: "
            f"frame_id={frame_token[0]} screenshot_id={frame_token[1]} "
            f"selected_count={self.selected_count} crop={count_crop} text={text!r}"
        )
        return None

    @staticmethod
    def _parse_bare_count(text: str, max_count: int) -> int | None:
        """Return the count only when the crop read as one bare number.

        OCR used to glue neighbouring panel glyphs onto the counter
        (``3in``, ``0/12in preeee``) and the first digit was still accepted,
        which is how a single-survivor run could be cached as ``3``.  Only an
        exact digit string is treated as a roster count.
        """

        matched = TEAM_PAGE_COUNT_TEXT_PATTERN.match(text or "")
        if matched is None:
            return None
        value = int(matched.group(1))
        if not 1 <= value <= max(1, int(max_count)):
            return None
        return value

    @staticmethod
    def _team_page_count_crop() -> tuple[int, int, int, int]:
        """Locate the roster counter from the packaged template bbox.

        The ratio fallback also covered the neighbouring panel, which let OCR
        report an unrelated ``3`` while a single ally was alive.  Preferring
        the template's own bounding box keeps the crop on the counter itself
        and degrades to the ratio crop only when the asset is unavailable.
        """

        width, height = BattleRosterObserver._screenshot_size()
        ratio_crop = (
            max(0, int(width * TEAM_PAGE_COUNT_CROP_RATIO[0])),
            max(0, int(height * TEAM_PAGE_COUNT_CROP_RATIO[1])),
            min(width, int(width * TEAM_PAGE_COUNT_CROP_RATIO[2])),
            min(height, int(height * TEAM_PAGE_COUNT_CROP_RATIO[3])),
        )
        try:
            bbox = ImageUtils.get_bbox_from_path("teams/10_sinner_live_assets.png")
        except Exception as error:
            log.debug(f"伪单通队伍页计数模板回退到比例区域: {error}")
            return ratio_crop
        if not bbox:
            # ``get_bbox_from_path`` depends on the active resource path state.
            # Resolve the packaged template directly so the counter crop stays
            # tight even before the resource manager is primed.
            try:
                template = ImageUtils.load_from_specific_path(
                    "teams/10_sinner_live_assets.png",
                    "default/share",
                    resize=False,
                )
            except Exception as error:
                log.debug(f"伪单通队伍页计数模板加载失败: {error}")
                template = None
            if template is not None:
                try:
                    bbox = tuple(int(value) for value in ImageUtils.get_bbox(template))
                except Exception as error:
                    log.debug(f"伪单通队伍页计数模板包围盒计算失败: {error}")
                    bbox = None
        if not bbox:
            return ratio_crop
        margin_x = max(12, round(TEAM_PAGE_COUNT_CROP_MARGIN[0] * height / 1440))
        margin_y = max(8, round(TEAM_PAGE_COUNT_CROP_MARGIN[1] * height / 1440))
        return (
            max(0, int(bbox[0]) - margin_x),
            max(0, int(bbox[1]) - margin_y),
            min(width, int(bbox[2]) + margin_x),
            min(height, int(bbox[3]) + margin_y),
        )

    @staticmethod
    def _frame_token() -> tuple[object, int]:
        frame_id = getattr(auto, "_frame_id", None)
        screenshot = getattr(auto, "screenshot", None)
        return frame_id, id(screenshot)

    def _read_battle_slot_geometry(self) -> tuple[list[float] | None, dict[str, object]]:
        """Resolve the fixed command-slot centers from the existing gear anchors."""

        width, height = self._screenshot_size()
        scale = height / 1440
        left_crop = (
            round(width * BATTLE_GEAR_LEFT_CROP_RATIO[0]),
            round(height * BATTLE_GEAR_LEFT_CROP_RATIO[1]),
            round(width * BATTLE_GEAR_LEFT_CROP_RATIO[2]),
            round(height * BATTLE_GEAR_LEFT_CROP_RATIO[3]),
        )
        right_crop = (
            round(width * BATTLE_GEAR_RIGHT_CROP_RATIO[0]),
            round(height * BATTLE_GEAR_RIGHT_CROP_RATIO[1]),
            round(width * BATTLE_GEAR_RIGHT_CROP_RATIO[2]),
            round(height * BATTLE_GEAR_RIGHT_CROP_RATIO[3]),
        )
        diagnostics: dict[str, object] = {
            "screen": (width, height),
            "scale": round(scale, 4),
            "left_crop": left_crop,
            "right_crop": right_crop,
        }

        def anchor_point(anchor: object) -> tuple[float, float] | None:
            try:
                point = (float(anchor[0]), float(anchor[1]))  # type: ignore[index]
            except (IndexError, KeyError, TypeError, ValueError):
                return None
            if not all(np.isfinite(value) for value in point):
                return None
            return point

        try:
            gear_left = auto.find_element(
                "battle/gear_left.png",
                threshold=0.75,
                my_crop=left_crop,
            )
            gear_right = auto.find_element(
                "battle/gear_right.png",
                threshold=0.75,
                my_crop=right_crop,
            )
        except Exception as error:
            diagnostics["reason"] = f"anchor_exception:{type(error).__name__}"
            return None, diagnostics

        left_point = anchor_point(gear_left)
        right_point = anchor_point(gear_right)
        diagnostics["gear_left"] = left_point
        diagnostics["gear_right"] = right_point
        if left_point is None or right_point is None:
            diagnostics["reason"] = "missing_gear_anchor"
            return None, diagnostics

        first_click_x = left_point[0] + 100 * scale
        last_click_x = right_point[0] - 100 * scale
        skill_nums = int((last_click_x - first_click_x) / (145 * scale)) if scale > 0 else 0
        diagnostics["skill_nums"] = skill_nums
        if skill_nums < 1 or skill_nums > 12:
            diagnostics["reason"] = "invalid_skill_count"
            return None, diagnostics

        # _calculate_skills_position uses the same x origin/spacing for the
        # command cards.  The level badge is 43.5 reference pixels to the
        # right of that click position.
        x_offset = 220 - 4.5 * skill_nums
        skill_size = 161 * scale
        slot_centers = [
            left_point[0] + x_offset * scale + skill_size * index + 43.5 * scale
            for index in range(skill_nums)
        ]
        diagnostics["slot_centers"] = [round(value, 2) for value in slot_centers]
        diagnostics["reason"] = "ok"
        return slot_centers, diagnostics

    @staticmethod
    def _screenshot_size() -> tuple[int, int]:
        screenshot = getattr(auto, "screenshot", None)
        if screenshot is not None:
            size = getattr(screenshot, "size", None)
            try:
                if size and len(size) >= 2:
                    return int(size[0]), int(size[1])
            except TypeError:
                # numpy arrays expose ``size`` as a scalar element count;
                # their two-dimensional shape below is the useful geometry.
                pass
            shape = getattr(screenshot, "shape", None)
            if shape and len(shape) >= 2:
                return int(shape[1]), int(shape[0])
        height = int(getattr(cfg, "set_win_size", 1440))
        return int(height * 16 / 9), height

    def _read_current_frame(self) -> PseudoSoloObservation:
        if self.selected_count < 2:
            return PseudoSoloObservation.UNKNOWN

        live_count = self.read_battle_live_count()
        if live_count is None:
            return PseudoSoloObservation.UNKNOWN
        return PseudoSoloObservation.SINGLE_SURVIVOR if live_count == 1 else PseudoSoloObservation.MULTIPLE_SURVIVORS
