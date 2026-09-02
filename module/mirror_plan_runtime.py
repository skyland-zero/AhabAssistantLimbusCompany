"""Runtime state for mirror plans with more than one possible floor count.

The mirror settings page shows the number of *unpassed floors in the current
five-floor segment*.  It does not show the total length of the run.  The task
mode therefore selects the total floor count before the run starts, while this
module only tracks progress inside each segment.
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field

MIRROR_SEGMENT_LENGTH = 5
_FLOOR_NUMBER = r"(?:1[0-5]|[1-9])"


def _normalize_floor_counts(floor_counts) -> tuple[int, ...]:
    counts = tuple(sorted({int(count) for count in floor_counts if int(count) > 0}))
    if not counts:
        raise ValueError("镜牢路线至少需要一个有效的层数")
    return counts


def select_mirror_floor_count(
    supported_floor_counts,
    hard_mirror: bool,
    target_floors: int | None = None,
) -> int:
    """Select the run length from the task mode, target configuration, and route capabilities.

    A route may advertise several supported run lengths. A normal run uses five floors.
    A hard run may target five floors (fast weekly speedrun / 普转困) or fifteen floors
    (Parallel Superposition / 平行叠加).
    Routes which only expose one length keep that length for compatibility;
    this lets legacy five-floor routes continue working in hard mode without
    pretending that they have a fifteen-floor strategy.
    """

    counts = _normalize_floor_counts(supported_floor_counts)
    if bool(hard_mirror):
        if target_floors is not None:
            try:
                target = int(target_floors)
                if target in counts:
                    return target
            except (TypeError, ValueError):
                pass
        preferred_count = 15
    else:
        preferred_count = 5

    if preferred_count in counts:
        return preferred_count
    if 5 in counts:
        return 5
    if len(counts) == 1:
        return counts[0]
    supported = "/".join(str(count) for count in counts)
    raise ValueError(f"镜牢路线支持{supported}层，但没有适配当前任务模式的目标层数")


def extract_mirror_floor(text: str | None) -> int | None:
    """Extract an absolute 1-based floor number from the map title OCR."""

    normalized = "".join(str(text or "").casefold().split())
    if not normalized:
        return None

    patterns = (
        rf"第({_FLOOR_NUMBER})层",
        rf"floor({_FLOOR_NUMBER})(?!\d)",
        rf"oor({_FLOOR_NUMBER})(?!\d)",
        rf"({_FLOOR_NUMBER})f(?!\d)",
    )
    for pattern in patterns:
        match = re.search(pattern, normalized)
        if match:
            return int(match.group(1))

    # OCR occasionally drops the last Chinese character or inserts unrelated
    # punctuation.  Keep a narrow fallback anchored to the floor marker.
    chinese_match = re.search(rf"第({_FLOOR_NUMBER})", normalized)
    if chinese_match:
        return int(chinese_match.group(1))
    english_match = re.search(rf"(?:floor|oor)\D*({_FLOOR_NUMBER})(?!\d)", normalized)
    return int(english_match.group(1)) if english_match else None


class MirrorPlanProgressError(RuntimeError):
    """The visible progress marker cannot describe a valid run position."""


@dataclass(slots=True)
class MirrorPlanRuntime:
    """Track the selected floor count and per-floor timing for one run."""

    supported_floor_counts: tuple[int, ...]
    floor_count: int | None = None
    current_floor: int = 0
    floor_times: list[float] = field(default_factory=list)
    progress_observed: bool = False
    deviations: list[str] = field(default_factory=list)
    segment_start_floor: int = 0
    last_not_passed_floor_count: int | None = None

    def __post_init__(self) -> None:
        self.supported_floor_counts = _normalize_floor_counts(self.supported_floor_counts)
        if self.floor_count is None and len(self.supported_floor_counts) == 1:
            self.floor_count = self.supported_floor_counts[0]
        elif self.floor_count is not None:
            self.floor_count = int(self.floor_count)
            if self.floor_count not in self.supported_floor_counts:
                supported = "/".join(str(count) for count in self.supported_floor_counts)
                raise ValueError(f"目标层数{self.floor_count}不在路线支持范围{supported}内")
        if not self.floor_times:
            self.floor_times = [-9999.0 for _ in range(max(self.supported_floor_counts))]
        elif len(self.floor_times) < max(self.supported_floor_counts):
            self.floor_times.extend(
                [-9999.0 for _ in range(max(self.supported_floor_counts) - len(self.floor_times))]
            )
        if self.segment_start_floor < 0 or self.segment_start_floor % MIRROR_SEGMENT_LENGTH:
            raise ValueError("镜牢五层段起始楼层必须是非负的五的倍数")
        if self.floor_count is not None and self.segment_start_floor >= self.floor_count:
            raise ValueError("镜牢五层段起始楼层超出目标路线范围")

    @property
    def complete(self) -> bool:
        """Whether every floor in the selected run has a recorded start."""

        return self.floor_count is not None and all(self.floor_times[index] > 0 for index in range(self.floor_count))

    @property
    def can_advance_segment(self) -> bool:
        """Whether the selected route has another five-floor segment."""

        return (
            self.floor_count is not None
            and self.segment_start_floor + MIRROR_SEGMENT_LENGTH < self.floor_count
        )

    def seed_floor(self, absolute_floor: int) -> int:
        """Seed progress from a 0-based absolute floor read during resume."""

        if self.floor_count is None:
            raise MirrorPlanProgressError("镜牢路线尚未根据任务模式选择目标层数")
        try:
            floor = int(absolute_floor)
        except (TypeError, ValueError) as error:
            raise MirrorPlanProgressError("恢复镜牢时读取到的楼层无效") from error
        if floor < 0 or floor >= self.floor_count:
            raise MirrorPlanProgressError(f"恢复镜牢楼层{floor + 1}超出{self.floor_count}层路线范围")
        if self.progress_observed and floor < self.current_floor:
            raise MirrorPlanProgressError(f"恢复镜牢楼层{floor + 1}早于已记录楼层{self.current_floor + 1}")
        self.segment_start_floor = floor // MIRROR_SEGMENT_LENGTH * MIRROR_SEGMENT_LENGTH
        self.current_floor = floor
        self.last_not_passed_floor_count = None
        self.progress_observed = True
        return floor

    def detect_floor(
        self,
        not_passed_floor_count: int,
        *,
        initial: bool = False,
        absolute_floor: int | None = None,
    ) -> int:
        """Resolve an absolute 0-based floor from the current segment markers.

        ``initial`` remains accepted for compatibility with older callers but
        no longer changes the result.  A marker count of four is the normal
        first-floor observation; five is also accepted for the short moment
        before the first floor marker is consumed.
        """

        del initial
        try:
            remaining = int(not_passed_floor_count)
        except (TypeError, ValueError) as error:
            raise MirrorPlanProgressError("无法读取镜牢剩余层数，已暂停本轮路线") from error
        if self.floor_count is None:
            raise MirrorPlanProgressError("镜牢路线尚未根据任务模式选择目标层数")
        if remaining < 0 or remaining > MIRROR_SEGMENT_LENGTH:
            raise MirrorPlanProgressError(f"镜牢当前五层段剩余标记{remaining}个无效")

        if absolute_floor is not None:
            self.seed_floor(absolute_floor)
            current_floor = self.current_floor
        else:
            if (
                self.last_not_passed_floor_count is not None
                and remaining > self.last_not_passed_floor_count
            ):
                raise MirrorPlanProgressError("镜牢五层段标记发生重置，但尚未确认进入下一段")
            floor_offset = max(0, MIRROR_SEGMENT_LENGTH - 1 - remaining)
            current_floor = self.segment_start_floor + floor_offset
            if self.progress_observed and current_floor < self.current_floor:
                raise MirrorPlanProgressError(
                    f"镜牢进度从第{self.current_floor + 1}层回退到第{current_floor + 1}层"
                )
            if current_floor >= self.floor_count:
                raise MirrorPlanProgressError(
                    f"镜牢当前楼层{current_floor + 1}超出{self.floor_count}层路线范围"
                )
            self.current_floor = current_floor

        self.current_floor = current_floor
        self.last_not_passed_floor_count = remaining
        self.progress_observed = True
        return self.current_floor

    def advance_segment(self) -> int:
        """Commit a confirmed 5-floor-to-next-segment transition."""

        if not self.can_advance_segment:
            target = self.floor_count or max(self.supported_floor_counts)
            raise MirrorPlanProgressError(f"{target}层镜牢没有可继续的后续五层段")
        self.segment_start_floor += MIRROR_SEGMENT_LENGTH
        self.current_floor = self.segment_start_floor
        self.last_not_passed_floor_count = None
        self.progress_observed = True
        return self.segment_start_floor

    def record_floor_start(self, floor: int, timestamp: float) -> float | None:
        """Record a theme-pack selection and return the preceding start time."""

        floor = int(floor)
        max_floor = self.floor_count if self.floor_count is not None else max(self.supported_floor_counts)
        if floor < 0 or floor >= max_floor:
            raise MirrorPlanProgressError(f"镜牢层数{floor}超出路线范围，无法记录时间")
        previous_start = self.floor_times[floor - 1] if floor > 0 else None
        self.floor_times[floor] = float(timestamp)
        self.current_floor = floor
        return previous_start

    def last_floor_start(self, floor: int | None = None) -> float | None:
        """Return the start timestamp of the floor immediately before ``floor``."""

        floor = self.current_floor if floor is None else int(floor)
        if floor <= 0 or floor > len(self.floor_times):
            return None
        timestamp = self.floor_times[floor - 1]
        return timestamp if timestamp > 0 else None

    def record_deviation(self, reason: str) -> None:
        """Keep a deduplicated, inspectable record of a tolerant fallback."""

        reason = str(reason).strip()
        if reason and reason not in self.deviations:
            self.deviations.append(reason)
