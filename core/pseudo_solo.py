"""Pseudo-solo battle state and conservative survivor observation.

The upstream battle runner only needs a state with ``remaining_turns`` and
``consume_turn``.  Keeping the dynamic policy behind that small interface
allows the fork to observe the battle screen without changing the upstream
battle loop.
"""

from __future__ import annotations

import re
from collections.abc import Callable, Sequence
from enum import StrEnum
from typing import Protocol

from module.automation import auto
from module.config import cfg
from module.logger import log

PSEUDO_SOLO_OBSERVATION_STABILITY = 2
DEAD_MARKER_THRESHOLD = 0.9
DEAD_MARKER_MIN_DISTANCE = 80


class PseudoSoloObservation(StrEnum):
    """Conservative observations used by the pseudo-solo policy."""

    UNKNOWN = "unknown"
    MULTIPLE_SURVIVORS = "multiple_survivors"
    SINGLE_SURVIVOR = "single_survivor"


def defense_turns_for_live_count(live_count: int) -> int:
    """Return the turns needed for every living ally except the solo unit."""

    return max(int(live_count) - 1, 0)


class _DefenseState(Protocol):
    remaining_turns: int

    def consume_turn(self) -> None: ...


class PseudoSoloDefenseState:
    """Adapt the existing fixed budget with a dynamic stop observation.

    The wrapped upstream state remains the source of truth for the fallback
    turn budget.  The observer can only shorten the budget after reporting a
    single survivor on consecutive frames; unknown observations never cause
    an early transition.
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
        self._last_team_page_observation: tuple[int, int] | None = None
        self._last_live_count: int | None = None

    @property
    def remaining_turns(self) -> int:
        if self._base_state.remaining_turns <= 0:
            return 0

        if not self._stop_confirmed:
            observation = self._observer()
            if observation == PseudoSoloObservation.SINGLE_SURVIVOR:
                self._stop_confirmed = True
                log.info("伪单通检测到只剩一名己方人格，停止连续防御")

        return 0 if self._stop_confirmed else self._base_state.remaining_turns

    def consume_turn(self) -> None:
        if self.remaining_turns > 0:
            self._base_state.consume_turn()

    @property
    def defense_cycle_complete(self) -> bool:
        """Return whether the current defense cycle has reached its stop point."""

        return self._stop_confirmed or self._base_state.remaining_turns <= 0

    @property
    def live_count(self) -> int | None:
        """Return the last reliable team-page count, if one was observed."""

        return self._last_live_count

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
            return False
        live_count = read_live_count()
        if live_count is None:
            return False

        live_count = max(0, int(live_count))
        observation = (floor, live_count)
        if self._last_team_page_observation == observation:
            return False
        self._last_team_page_observation = observation
        self._last_live_count = live_count

        if live_count <= 1:
            changed = self._base_state.remaining_turns > 0 or not self._stop_confirmed
            self._base_state.remaining_turns = 0
            self._stop_confirmed = True
            if changed:
                log.info(f"第{floor}层队伍页确认仅剩{live_count}名己方人格，停止连续防御")
            return changed

        required_turns = defense_turns_for_live_count(live_count)
        changed = (
            self._base_state.remaining_turns != required_turns
            or self._stop_confirmed
        )
        self._base_state.remaining_turns = required_turns
        self._stop_confirmed = False
        reset_observer = getattr(self._observer, "reset", None)
        if callable(reset_observer):
            reset_observer()
        if changed:
            log.info(
                f"第{floor}层队伍页检测到{live_count}名存活己方人格，"
                f"伪单通连续防御调整为{required_turns}回合"
            )
        return changed

    def reset_for_run(self) -> None:
        """Restore the initial cycle when the mirror run is restarted."""

        self._base_state.remaining_turns = self._fallback_turn_limit
        self._stop_confirmed = False
        self._last_team_page_observation = None
        self._last_live_count = None
        reset_observer = getattr(self._observer, "reset", None)
        if callable(reset_observer):
            reset_observer()


class BattleRosterObserver:
    """Read a conservative survivor count from the current battle frame.

    ``battle/dead.png`` is the existing localized marker for a dead sinner.
    The observer only returns ``SINGLE_SURVIVOR`` when exactly
    ``selected_count - 1`` distinct ally death markers are visible, or when
    the ally roster OCR reports one survivor.  Missing anchors, malformed
    frames, or partial visibility return ``UNKNOWN`` so the fixed defense
    budget remains the safety fallback.
    """

    def __init__(self, selected_sinners: Sequence[int], *, selected_count: int | None = None) -> None:
        selected = list(selected_sinners[:12])
        self.selected_count = selected_count if selected_count is not None else sum(bool(value) for value in selected)
        self._last_frame_token: tuple[object, int] | None = None
        self._last_observation = PseudoSoloObservation.UNKNOWN
        self._single_candidate_frames = 0
        self._single_survivor_confirmed = False
        self._last_team_page_frame_token: tuple[object, int] | None = None
        self._last_team_page_live_count: int | None = None

    def __call__(self) -> PseudoSoloObservation:
        frame_token = self._frame_token()
        if frame_token == self._last_frame_token:
            return self._last_observation
        self._last_frame_token = frame_token
        current_observation = self._read_current_frame()
        if current_observation == PseudoSoloObservation.SINGLE_SURVIVOR:
            self._single_candidate_frames += 1
            if self._single_candidate_frames >= PSEUDO_SOLO_OBSERVATION_STABILITY:
                self._single_survivor_confirmed = True
            elif not self._single_survivor_confirmed:
                current_observation = PseudoSoloObservation.UNKNOWN
        else:
            self._single_candidate_frames = 0
        self._last_observation = (
            PseudoSoloObservation.SINGLE_SURVIVOR if self._single_survivor_confirmed else current_observation
        )
        return self._last_observation

    def reset(self) -> None:
        """Forget battle and team-page observations for a new defense cycle."""

        self._last_frame_token = None
        self._last_observation = PseudoSoloObservation.UNKNOWN
        self._single_candidate_frames = 0
        self._single_survivor_confirmed = False
        self._last_team_page_frame_token = None
        self._last_team_page_live_count = None

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
                    threshold=DEAD_MARKER_THRESHOLD,
                ):
                    self._last_team_page_live_count = live_count
                    return live_count

            if max_count < 1:
                return None
            texts = auto.get_text_from_screenshot()
        except Exception as error:
            log.debug(f"伪单通队伍页人数识别失败: {error}")
            return None

        text = "".join(str(value) for value in texts or [])
        for match in re.finditer(r"(\d{1,2})\s*/\s*(\d{1,2})", text):
            live_count, total_count = (int(value) for value in match.groups())
            if total_count != max_count or live_count > total_count:
                continue
            self._last_team_page_live_count = live_count
            return live_count
        return None

    @staticmethod
    def _frame_token() -> tuple[object, int]:
        frame_id = getattr(auto, "_frame_id", None)
        screenshot = getattr(auto, "screenshot", None)
        return frame_id, id(screenshot)

    @staticmethod
    def _screenshot_size() -> tuple[int, int]:
        screenshot = getattr(auto, "screenshot", None)
        if screenshot is not None:
            size = getattr(screenshot, "size", None)
            if size and len(size) >= 2:
                return int(size[0]), int(size[1])
            shape = getattr(screenshot, "shape", None)
            if shape and len(shape) >= 2:
                return int(shape[1]), int(shape[0])
        height = int(getattr(cfg, "set_win_size", 1440))
        return int(height * 16 / 9), height

    @classmethod
    def _ally_battle_crop(cls, gear_left: Sequence[float], gear_right: Sequence[float]) -> tuple[float, ...]:
        """Build a lower-screen crop from the existing battle gear anchors."""

        scale = float(getattr(cfg, "set_win_size", 1440)) / 1440
        width, height = cls._screenshot_size()
        left = max(0.0, min(float(gear_left[0]), float(gear_right[0])) - 600 * scale)
        top = max(0.0, float(gear_left[1]) - 500 * scale)
        right = min(float(width), max(float(gear_left[0]), float(gear_right[0])) + 600 * scale)
        bottom = min(float(height), max(float(gear_left[1]), float(gear_right[1])) + 250 * scale)
        return left, top, right, bottom

    @staticmethod
    def _deduplicate_markers(markers: Sequence[Sequence[float]]) -> list[tuple[float, float]]:
        unique: list[tuple[float, float]] = []
        min_distance_squared = DEAD_MARKER_MIN_DISTANCE**2
        for marker in markers:
            if len(marker) < 2:
                continue
            point = (float(marker[0]), float(marker[1]))
            if all(
                (point[0] - existing[0]) ** 2 + (point[1] - existing[1]) ** 2 > min_distance_squared
                for existing in unique
            ):
                unique.append(point)
        return unique

    def _read_current_frame(self) -> PseudoSoloObservation:
        if self.selected_count < 2:
            return PseudoSoloObservation.UNKNOWN

        gear_left = auto.find_element("battle/gear_left.png", threshold=DEAD_MARKER_THRESHOLD)
        gear_right = auto.find_element("battle/gear_right.png", threshold=DEAD_MARKER_THRESHOLD)
        if not gear_left or not gear_right:
            return PseudoSoloObservation.UNKNOWN

        crop = self._ally_battle_crop(gear_left, gear_right)
        markers = auto.find_element(
            "battle/dead.png",
            find_type="image_with_multiple_targets",
            threshold=DEAD_MARKER_THRESHOLD,
            min_dist=DEAD_MARKER_MIN_DISTANCE,
            my_crop=crop,
        )
        marker_observation = PseudoSoloObservation.UNKNOWN
        if markers:
            dead_count = len(self._deduplicate_markers(markers))
            if dead_count == self.selected_count - 1:
                return PseudoSoloObservation.SINGLE_SURVIVOR
            if dead_count < self.selected_count:
                marker_observation = PseudoSoloObservation.MULTIPLE_SURVIVORS

        ocr_observation = self._read_live_count_from_ocr(crop)
        if ocr_observation != PseudoSoloObservation.UNKNOWN:
            return ocr_observation
        return marker_observation

    def _read_live_count_from_ocr(self, crop: tuple[float, ...]) -> PseudoSoloObservation:
        try:
            texts = auto.get_text_from_screenshot(my_crop=crop)
        except Exception as error:
            log.debug(f"伪单通队伍人数 OCR 失败，使用固定防御回合兜底: {error}")
            return PseudoSoloObservation.UNKNOWN

        text = "".join(str(value) for value in texts or [])
        for match in re.finditer(r"(\d{1,2})\s*/\s*(\d{1,2})", text):
            live_count, total_count = (int(value) for value in match.groups())
            if total_count != self.selected_count or live_count > total_count:
                continue
            return (
                PseudoSoloObservation.SINGLE_SURVIVOR if live_count <= 1 else PseudoSoloObservation.MULTIPLE_SURVIVORS
            )
        return PseudoSoloObservation.UNKNOWN
