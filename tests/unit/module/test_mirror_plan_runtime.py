from pytest import raises

from module.mirror_plan_runtime import (
    MirrorPlanProgressError,
    MirrorPlanRuntime,
    extract_mirror_floor,
    select_mirror_floor_count,
)


def test_task_mode_selects_the_supported_run_length() -> None:
    assert select_mirror_floor_count((5, 15), hard_mirror=False) == 5
    assert select_mirror_floor_count((5, 15), hard_mirror=True) == 15
    assert select_mirror_floor_count((5, 15), hard_mirror=True, target_floors=5) == 5
    assert select_mirror_floor_count((5, 15), hard_mirror=True, target_floors=15) == 15
    assert select_mirror_floor_count((5,), hard_mirror=True, target_floors=15) == 5
    assert select_mirror_floor_count((5,), hard_mirror=True) == 5


def test_route_without_a_compatible_length_is_rejected() -> None:
    with raises(ValueError, match="没有适配当前任务模式"):
        select_mirror_floor_count((10, 20), hard_mirror=False)


def test_first_segment_markers_are_not_total_floor_count() -> None:
    runtime = MirrorPlanRuntime((5, 15), floor_count=15)

    assert [runtime.detect_floor(remaining) for remaining in (4, 3, 2, 1, 0)] == [0, 1, 2, 3, 4]
    assert runtime.floor_count == 15


def test_hard_runtime_tracks_three_five_floor_segments() -> None:
    runtime = MirrorPlanRuntime((5, 15), floor_count=15)
    observed_floors = []

    for segment in range(3):
        for remaining in (4, 3, 2, 1, 0):
            floor = runtime.detect_floor(remaining)
            observed_floors.append(floor)
            runtime.record_floor_start(floor, floor + 1)
        if segment < 2:
            assert runtime.advance_segment() == (segment + 1) * 5

    assert observed_floors == list(range(15))
    assert runtime.complete


def test_short_runtime_uses_only_the_first_segment() -> None:
    runtime = MirrorPlanRuntime((5, 15), floor_count=5)

    for remaining in (4, 3, 2, 1, 0):
        floor = runtime.detect_floor(remaining)
        runtime.record_floor_start(floor, floor + 1)

    assert runtime.complete
    assert not runtime.can_advance_segment
    assert len(runtime.floor_times) == 15


def test_marker_count_five_is_tolerated_before_the_first_marker_is_consumed() -> None:
    runtime = MirrorPlanRuntime((5,), floor_count=5)

    assert runtime.detect_floor(5) == 0
    assert runtime.detect_floor(4) == 0
    assert runtime.detect_floor(3) == 1


def test_short_resume_marker_count_resolves_the_fourth_floor() -> None:
    runtime = MirrorPlanRuntime((5, 15), floor_count=5)

    assert runtime.detect_floor(1) == 3


def test_unconfirmed_segment_reset_is_rejected() -> None:
    runtime = MirrorPlanRuntime((5, 15), floor_count=15)

    runtime.detect_floor(4)
    runtime.detect_floor(3)
    with raises(MirrorPlanProgressError, match="标记发生重置"):
        runtime.detect_floor(4)


def test_invalid_segment_marker_is_rejected() -> None:
    runtime = MirrorPlanRuntime((5,), floor_count=5)

    with raises(MirrorPlanProgressError, match="当前五层段剩余标记"):
        runtime.detect_floor(6)
    with raises(MirrorPlanProgressError, match="当前五层段剩余标记"):
        runtime.detect_floor(-1)


def test_resume_floor_seeds_the_correct_segment() -> None:
    runtime = MirrorPlanRuntime((5, 15), floor_count=15)

    assert runtime.detect_floor(4, absolute_floor=10) == 10
    assert runtime.detect_floor(3) == 11
    assert runtime.segment_start_floor == 10


def test_floor_title_ocr_supports_one_to_fifteen() -> None:
    assert extract_mirror_floor("Exploring Floor 1") == 1
    assert extract_mirror_floor("Exploring Floor 10") == 10
    assert extract_mirror_floor("第15层") == 15
    assert extract_mirror_floor("Line 1") is None


def test_floor_timing_keeps_the_previous_floor_start() -> None:
    runtime = MirrorPlanRuntime((5,), floor_count=5)

    runtime.detect_floor(4)
    assert runtime.record_floor_start(0, 100.0) is None
    assert runtime.record_floor_start(1, 150.0) == 100.0
    assert runtime.last_floor_start(1) == 100.0
