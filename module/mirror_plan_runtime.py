"""Runtime state for mirror plans with more than one possible floor count.

The route catalog describes what to prefer.  This module owns the small amount
of mutable state needed to apply that catalog to either a five-floor or a
fifteen-floor mirror run.  Keeping detection and timing here prevents the
legacy five-floor assumptions from leaking into route-specific code.
"""

from __future__ import annotations

from dataclasses import dataclass, field


class MirrorPlanProgressError(RuntimeError):
    """The visible progress marker cannot identify one supported run shape."""


@dataclass(slots=True)
class MirrorPlanRuntime:
    """Track the selected floor count and per-floor timing for one run."""

    supported_floor_counts: tuple[int, ...]
    floor_count: int | None = None
    current_floor: int = 0
    floor_times: list[float] = field(default_factory=list)
    progress_observed: bool = False
    deviations: list[str] = field(default_factory=list)

    def __post_init__(self) -> None:
        counts = tuple(sorted({int(count) for count in self.supported_floor_counts if int(count) > 0}))
        if not counts:
            raise ValueError("镜牢路线至少需要一个有效的层数")
        self.supported_floor_counts = counts
        if not self.floor_times:
            self.floor_times = [-9999.0 for _ in range(max(counts))]
        elif len(self.floor_times) < max(counts):
            self.floor_times.extend([-9999.0 for _ in range(max(counts) - len(self.floor_times))])

    @property
    def complete(self) -> bool:
        """Whether every floor in the detected run has a recorded start."""

        return self.floor_count is not None and all(
            self.floor_times[index] > 0 for index in range(self.floor_count)
        )

    def detect_floor(self, not_passed_floor_count: int, *, initial: bool = False) -> int:
        """Resolve the current floor from the visible not-passed marker count.

        On a route supporting both five and fifteen floors, the first
        ambiguous marker count is deliberately rejected.  Once a run shape is
        identified it is latched for the rest of the run, so a later count can
        never silently switch a fifteen-floor run to the five-floor formula.
        """

        try:
            remaining = int(not_passed_floor_count)
        except (TypeError, ValueError) as error:
            raise MirrorPlanProgressError("无法读取镜牢剩余层数，已暂停本轮路线") from error
        if remaining < 0:
            raise MirrorPlanProgressError("镜牢剩余层数无效，已暂停本轮路线")

        if self.floor_count is None:
            exact_counts = tuple(count for count in self.supported_floor_counts if remaining == count)
            candidate_counts = tuple(count for count in self.supported_floor_counts if remaining <= count)
            if initial and len(exact_counts) == 1:
                selected_count = exact_counts[0]
            elif len(candidate_counts) == 1:
                selected_count = candidate_counts[0]
            else:
                supported = "/".join(str(count) for count in self.supported_floor_counts)
                raise MirrorPlanProgressError(
                    f"无法区分{supported}层镜牢路线（剩余标记{remaining}个），已暂停等待确认"
                )
            self.floor_count = selected_count

        if remaining > self.floor_count:
            raise MirrorPlanProgressError(
                f"镜牢剩余层数{remaining}超过已识别的{self.floor_count}层路线，已暂停本轮路线"
            )

        self.current_floor = self.floor_count - remaining
        self.progress_observed = True
        return self.current_floor

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
