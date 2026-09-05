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
from enum import StrEnum
from typing import Protocol

import cv2
import numpy as np

from core.execution_control import check_cancelled, interruptible_sleep
from module.automation import auto
from module.config import cfg
from module.logger import log
from module.my_error.my_error import userStopError

# Kept as a compatibility constant for integrations that imported the old
# setting.  Battle-row sampling is configured separately below.
PSEUDO_SOLO_OBSERVATION_STABILITY = 1
TEAM_PAGE_LIVE_COUNT_THRESHOLD = 0.90
# The command row can remain hidden behind the battle-entry animation for
# roughly one second.  Keep sampling the same battle-entry window a little
# longer so a transient no-badge frame is not cached as UNKNOWN.
BATTLE_ROSTER_MAX_SAMPLES = 12
BATTLE_ROSTER_REQUIRED_STABLE_SAMPLES = 2
BATTLE_ROSTER_SAMPLE_INTERVAL = 0.15
BATTLE_ROSTER_CENTER_TOLERANCE_RATIO = 0.01
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
# At 1920x1080 a real orange level badge is about 40x41 px and has an orange
# component area of roughly 750-900 px.  Short UI effects can have a similar
# horizontal projection but are much shorter; validate the connected
# component instead of accepting projection width alone.
BATTLE_BADGE_MIN_HEIGHT_AT_1080 = 30
BATTLE_BADGE_MIN_WIDTH_AT_1080 = 28
BATTLE_BADGE_MIN_AREA_AT_1080 = 500


class PseudoSoloObservation(StrEnum):
    """Conservative observations used by the pseudo-solo policy."""

    UNKNOWN = "unknown"
    MULTIPLE_SURVIVORS = "multiple_survivors"
    SINGLE_SURVIVOR = "single_survivor"


def defense_turns_for_live_count(live_count: int) -> int:
    """Return the turns needed for every living ally except the solo unit."""

    return max(int(live_count) - 1, 0)


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


def _battle_portrait_slot_detection(
    array: np.ndarray,
    *,
    slot_centers: Sequence[float],
    max_count: int,
) -> tuple[int | None, dict[str, object]]:
    """Classify every fixed command slot as live, empty, or unknown."""

    height, width = array.shape[:2]
    diagnostics: dict[str, object] = {
        "shape": tuple(int(value) for value in array.shape),
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
    orange_mask = cv2.inRange(
        hsv,
        np.array((8, 120, 120), dtype=np.uint8),
        np.array((22, 255, 255), dtype=np.uint8),
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
        badge = orange_mask[badge_y1:badge_y2, badge_x1:badge_x2]
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


def _battle_portrait_badge_detection(
    image: object,
    *,
    max_count: int = 12,
    slot_centers: Sequence[float] | None = None,
) -> tuple[int | None, dict[str, object]]:
    """Return the portrait count and compact diagnostics for one frame."""

    diagnostics: dict[str, object] = {
        "shape": None,
        "roi": None,
        "orange_pixels": 0,
        "candidate_runs": [],
        "accepted_runs": [],
        "reason": "unknown",
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
            return _battle_portrait_slot_detection(array, slot_centers=slot_centers, max_count=max_count)

        animation_detected, animation_diagnostics = _battle_animation_overlay_detection(array)
        diagnostics["animation"] = animation_diagnostics
        if animation_detected:
            diagnostics["reason"] = "animation_overlay"
            return None, diagnostics

        hsv = cv2.cvtColor(array, cv2.COLOR_RGB2HSV)
        # The orange level circles are stable across the two captured battle
        # scenes.  Restricting the scan to the bottom badge band excludes the
        # orange skill/effect icons above the portrait row.
        mask = cv2.inRange(
            hsv,
            np.array((8, 120, 120), dtype=np.uint8),
            np.array((22, 255, 255), dtype=np.uint8),
        )
        y1 = max(0, min(height, round(height * BATTLE_PORTRAIT_Y_START_RATIO)))
        y2 = max(y1, min(height, round(height * BATTLE_PORTRAIT_Y_END_RATIO)))
        x1 = max(0, min(width, round(width * BATTLE_PORTRAIT_X_START_RATIO)))
        x2 = max(x1, min(width, round(width * BATTLE_PORTRAIT_X_END_RATIO)))
        diagnostics["roi"] = (x1, y1, x2, y2)
        roi_mask = mask[y1:y2, x1:x2]
        diagnostics["orange_pixels"] = int((roi_mask > 0).sum())
        projection = (roi_mask > 0).sum(axis=0)
        active_columns = projection >= 10
        badge_scale = height / 1080
        min_badge_height = max(16, round(BATTLE_BADGE_MIN_HEIGHT_AT_1080 * badge_scale))
        min_badge_width = max(18, round(BATTLE_BADGE_MIN_WIDTH_AT_1080 * badge_scale))
        min_badge_area = max(180, round(BATTLE_BADGE_MIN_AREA_AT_1080 * badge_scale**2))
        diagnostics["badge_validation_thresholds"] = {
            "min_component_width": min_badge_width,
            "min_component_height": min_badge_height,
            "min_component_area": min_badge_area,
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

            # A visible level badge is approximately 40 px wide and 41 px
            # high at the reference screenshot scale.  Short orange effects
            # can have a similar horizontal projection, so width/area alone
            # is not enough; the largest connected orange component must also
            # have the expected two-dimensional badge shape.
            shape_valid = (
                min(largest_component_size) >= min_badge_width
                and largest_component_size[1] >= min_badge_height
                and largest_component >= min_badge_area
            )
            if len(badge_component_details) < 64:
                badge_component_details.append(
                    {
                        "run": run,
                        "largest_component": largest_component,
                        "component_size": largest_component_size,
                        "shape_valid": shape_valid,
                    }
                )
            if 25 <= run_width <= 65 and shape_valid:
                accepted_runs.append(run)
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

    A defeated sinner no longer has a bottom portrait, including its orange
    level badge.  The badge is a repeated, localized-independent visual
    element, so counting it avoids OCR and avoids using the decorative gear
    anchors that can also match skill icons or battle effects.

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
    is covered by an effect.  First locate the orange level badges globally,
    then validate the avatar area around each badge.  This makes the badge
    runs themselves the dynamic slot list and keeps an obscured avatar as
    UNKNOWN instead of silently counting it as alive.
    """

    badge_count, badge_diagnostics = _battle_portrait_badge_detection(image, max_count=max_count)
    diagnostics = dict(badge_diagnostics)
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
    diagnostics["visible_badge_centers"] = [round(value, 2) for value in centers]
    diagnostics["detection_source"] = "visible_badges"

    # No badge, an animation veil, or more visible portraits than the
    # selected roster are all deliberately UNKNOWN.  In particular, do not
    # fall back to a presumed number of empty command slots here.
    if badge_count is None:
        return None, diagnostics

    array = np.asarray(image)
    if array.ndim != 3 or array.shape[2] not in (3, 4):
        diagnostics["reason"] = "not_rgb"
        return None, diagnostics
    if array.shape[2] == 4:
        array = array[:, :, :3]

    live_count, slot_diagnostics = _battle_portrait_slot_detection(
        array,
        slot_centers=centers,
        max_count=max_count,
    )
    # Preserve the global scan details in the same top-level diagnostics used
    # by the GUI log while adding the per-visible-avatar validation details.
    for key in ("slot_centers", "slot_states", "live_indexes", "unknown_indexes", "animation", "reason"):
        if key in slot_diagnostics:
            diagnostics[key] = slot_diagnostics[key]
    diagnostics["badge_validation"] = slot_diagnostics
    return live_count, diagnostics


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
        self._stop_confirmed = False
        self._base_state.remaining_turns = 0
        begin_observation = getattr(self._observer, "begin_battle", None)
        if callable(begin_observation):
            begin_observation()
        log.debug("伪单通开始本场战斗识别周期：人数只识别一次")

    def consume_turn(self) -> None:
        if self.remaining_turns > 0:
            self._base_state.consume_turn()

    def should_defend(self) -> bool:
        """Return whether pseudo-solo should submit an all-defense round.

        The first call in a battle captures the roster and converts it to the
        exact defense budget (living count minus one).  Later calls only
        consult that cached result and the remaining budget.  A frame that is
        covered by an animation or cannot classify every portrait is
        deliberately sent through the normal automatic-P path.
        """

        new_battle_observation = not self._battle_observed
        if new_battle_observation:
            observation = self._observer()
            observed_count = getattr(self._observer, "last_battle_live_count", None)
            if observed_count is None:
                observed_count = getattr(self._observer, "battle_live_count", None)
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
                log.info("伪单通本场战斗人数识别为UNKNOWN，本场按普通P处理")
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
            f"should_defend={should_defend}"
        )
        if observation == PseudoSoloObservation.UNKNOWN:
            if new_battle_observation:
                log.info("伪单通战斗人数为UNKNOWN，本场后续按普通P处理")
            else:
                log.debug("伪单通复用本场UNKNOWN人数结果，本回合按普通P处理")
        return should_defend

    @property
    def defense_cycle_complete(self) -> bool:
        """Return whether the current defense cycle has reached its stop point."""

        return self._stop_confirmed or self._base_state.remaining_turns <= 0

    @property
    def live_count(self) -> int | None:
        """Return the last reliable team-page count, if one was observed."""

        return self._last_live_count

    @property
    def last_battle_live_count(self) -> int | None:
        """Return the last battle-row count used for the current decision."""

        if self._battle_observed:
            return self._battle_live_count
        return getattr(self._observer, "last_battle_live_count", None)

    @property
    def last_observation(self) -> PseudoSoloObservation:
        """Return the observer's last battle observation."""

        if self._battle_observed:
            return self._battle_observation
        return getattr(self._observer, "last_observation", PseudoSoloObservation.UNKNOWN)

    @property
    def battle_observation_pending(self) -> bool:
        """Return whether this battle still needs its first row observation."""

        return not self._battle_observed

    def observe_team_page(self, floor: int) -> bool:
        """Synchronize the defense budget with the visible live roster.

        The team-selection page is the authoritative place to count living
        allies.  For every newly observed roster state, the pseudo-solo budget
        is exactly the number of living teammates besides Ryoshu.  Repeated
        frames with the same count do not reset a partially consumed budget.
        If the count cannot be read, the configured compatibility fallback is
        left untouched.
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
        if self._last_team_page_observation == observation:
            return False
        previous_observation = self._last_team_page_observation
        floor_changed = previous_observation is not None and previous_observation[0] != floor
        self._last_team_page_observation = observation
        self._last_live_count = live_count

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

    def reset_for_run(self) -> None:
        """Restore the initial cycle when the mirror run is restarted."""

        self._base_state.remaining_turns = self._fallback_turn_limit
        self._stop_confirmed = False
        self._battle_observed = False
        self._battle_observation = PseudoSoloObservation.UNKNOWN
        self._battle_live_count = None
        self._last_team_page_observation = None
        self._last_live_count = None
        reset_observer = getattr(self._observer, "reset", None)
        if callable(reset_observer):
            reset_observer()


class BattleRosterObserver:
    """Read the number of currently visible ally battle portraits.

    The lower battle row removes a dead sinner's portrait and its orange level
    badge.  Counting those repeated badges gives the number of visible living
    combatants without OCR or a localized death label.  The visible badge
    positions are used as the slot list, so the detector does not assume
    there are seven (or any other fixed number of) deployed combatants.  A
    missing or partial portrait row returns ``None`` so the pseudo-solo policy
    can send the frame through normal automatic P instead of guessing.
    """

    def __init__(self, selected_sinners: Sequence[int], *, selected_count: int | None = None) -> None:
        selected = list(selected_sinners[:12])
        self.selected_count = selected_count if selected_count is not None else sum(bool(value) for value in selected)
        log.debug(f"伪单通人数观察器初始化: selected_count={self.selected_count}")
        self._last_frame_token: tuple[object, int] | None = None
        self._last_observation = PseudoSoloObservation.UNKNOWN
        self._last_battle_frame_token: tuple[object, int] | None = None
        self._battle_detection_attempted = False
        self._last_battle_live_count: int | None = None
        self._last_battle_diagnostics: dict[str, object] = {}
        self._last_battle_slot_states: list[dict[str, object]] = []
        self._last_team_page_frame_token: tuple[object, int] | None = None
        self._last_team_page_live_count: int | None = None

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

    def begin_battle(self) -> None:
        """Clear the battle cache before entering one new battle."""

        self._last_frame_token = None
        self._last_observation = PseudoSoloObservation.UNKNOWN
        self._last_battle_frame_token = None
        self._battle_detection_attempted = False
        self._last_battle_live_count = None
        self._last_battle_diagnostics = {}
        self._last_battle_slot_states = []

    def read_battle_live_count(self) -> int | None:
        """Read the number of visible living combatants in the battle row.

        Dead personalities no longer have a portrait or orange level badge.
        Only the badges actually found in the frame become candidate slots;
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
        if self._battle_detection_attempted:
            log.debug(
                "伪单通战斗人数识别复用本场缓存（本场仅识别一次）: "
                f"frame_id={frame_id} screenshot_id={screenshot_id} "
                f"live_count={self._last_battle_live_count} "
                f"reason={self._last_battle_diagnostics.get('reason')}"
            )
            return self._last_battle_live_count

        if frame_token == self._last_battle_frame_token:
            return self._last_battle_live_count
        self._last_battle_frame_token = frame_token
        self._last_battle_live_count = None
        self._last_battle_diagnostics = {}
        self._last_battle_slot_states = []
        self._battle_detection_attempted = True
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
            f"selected_count={self.selected_count} max_samples={BATTLE_ROSTER_MAX_SAMPLES} "
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

        for sample_index in range(max(1, int(BATTLE_ROSTER_MAX_SAMPLES))):
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
                f"sample={sample_index + 1}/{max(1, int(BATTLE_ROSTER_MAX_SAMPLES))} "
                f"frame_id={getattr(auto, '_frame_id', None)} screenshot_id={id(screenshot)} "
                f"selected_count={self.selected_count} source={screenshot_source} "
                f"original={image_description(original_screenshot)} image={image_description(screenshot)} "
                f"result={live_count} reason={diagnostics.get('reason')} "
                f"stable_run={stable_samples} animation={diagnostics.get('animation')} "
                f"slot_states={diagnostics.get('slot_states')} "
                f"visible_badge_centers={diagnostics.get('visible_badge_centers')} "
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
        observation_name = "single_survivor" if locked_count == 1 else "multiple_survivors"
        log.info(f"伪单通战斗界面识别到{locked_count}名存活人格，状态={observation_name}")
        return locked_count

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

        The existing image assets cover the normal 10/11/12-sinner cases.
        Smaller pseudo-solo teams use a strict ``live/selected`` OCR pattern;
        an unknown frame is deliberately returned as ``None``.
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
                    log.debug(
                        "伪单通队伍页人数识别: "
                        f"frame_id={frame_token[0]} screenshot_id={frame_token[1]} "
                        f"source=asset selected_count={self.selected_count} live_count={live_count}"
                    )
                    return live_count

            if max_count < 1:
                return None
            width, height = self._screenshot_size()
            team_header_crop = (
                max(0, int(width * 0.35)),
                max(0, int(height * 0.10)),
                min(width, int(width * 0.65)),
                min(height, int(height * 0.28)),
            )
            texts = auto.get_text_from_screenshot(my_crop=team_header_crop)
        except Exception as error:
            log.debug(f"伪单通队伍页人数识别失败: {error}")
            return None

        text = "".join(str(value) for value in texts or [])
        for match in re.finditer(r"(\d{1,2})\s*/\s*(\d{1,2})", text):
            live_count, total_count = (int(value) for value in match.groups())
            if total_count != max_count or live_count > total_count:
                continue
            self._last_team_page_live_count = live_count
            log.debug(
                "伪单通队伍页人数识别: "
                f"frame_id={frame_token[0]} screenshot_id={frame_token[1]} "
                f"source=ocr crop={team_header_crop} text={text!r} live_count={live_count} total_count={total_count}"
            )
            return live_count
        log.debug(
            "伪单通队伍页人数识别无有效结果: "
            f"frame_id={frame_token[0]} screenshot_id={frame_token[1]} "
            f"selected_count={self.selected_count} crop={locals().get('team_header_crop')} text={text!r}"
        )
        return None

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
        # command cards.  The orange level badge is 43.5 reference pixels to
        # the right of that click position.
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
