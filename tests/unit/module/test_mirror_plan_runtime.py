from pytest import raises

from module.mirror_plan_runtime import MirrorPlanProgressError, MirrorPlanRuntime


def test_hos_runtime_latches_the_fifteen_floor_environment() -> None:
    runtime = MirrorPlanRuntime((5, 15))

    assert runtime.detect_floor(15, initial=True) == 0
    assert runtime.floor_count == 15
    assert runtime.detect_floor(14) == 1
    assert runtime.detect_floor(5) == 10


def test_ambiguous_initial_progress_is_rejected() -> None:
    runtime = MirrorPlanRuntime((5, 15))

    with raises(MirrorPlanProgressError, match="无法区分"):
        runtime.detect_floor(4, initial=True)

    later_runtime = MirrorPlanRuntime((5, 15))
    with raises(MirrorPlanProgressError, match="无法区分"):
        later_runtime.detect_floor(5)


def test_short_runtime_uses_the_five_floor_completion_window() -> None:
    runtime = MirrorPlanRuntime((5, 15))

    assert runtime.detect_floor(5, initial=True) == 0
    for floor in range(5):
        expected = None if floor == 0 else float(floor)
        assert runtime.record_floor_start(floor, floor + 1) == expected
    assert runtime.complete
    assert len(runtime.floor_times) == 15


def test_floor_timing_keeps_the_previous_floor_start() -> None:
    runtime = MirrorPlanRuntime((5,))

    runtime.detect_floor(5, initial=True)
    assert runtime.record_floor_start(0, 100.0) is None
    assert runtime.record_floor_start(1, 150.0) == 100.0
    assert runtime.last_floor_start(1) == 100.0
