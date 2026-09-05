from __future__ import annotations

import threading
from dataclasses import dataclass

import numpy as np
import pytest
from PIL import Image

from core import pseudo_solo
from core.pseudo_solo import (
    BattleRosterObserver,
    PseudoSoloDefenseState,
    PseudoSoloObservation,
    battle_portrait_badge_count,
    defense_turns_for_live_count,
)


@dataclass
class FakeDefenseState:
    remaining_turns: int = 5

    def consume_turn(self) -> None:
        if self.remaining_turns > 0:
            self.remaining_turns -= 1


@dataclass
class FakeTeamPageObserver:
    live_count: int | None = None
    reset_calls: int = 0

    def __call__(self) -> PseudoSoloObservation:
        return PseudoSoloObservation.UNKNOWN

    def read_team_page_live_count(self) -> int | None:
        return self.live_count

    def reset(self) -> None:
        self.reset_calls += 1


def test_dynamic_state_observes_once_per_battle_and_resets_for_next_battle() -> None:
    observations = iter(
        [
            PseudoSoloObservation.UNKNOWN,
            PseudoSoloObservation.SINGLE_SURVIVOR,
        ]
    )
    base_state = FakeDefenseState()
    state = PseudoSoloDefenseState(base_state, lambda: next(observations))

    assert state.remaining_turns == 5
    assert base_state.remaining_turns == 5
    state.begin_battle()
    assert state.should_defend() is False
    assert state.remaining_turns == 0

    state.begin_battle()
    assert state.should_defend() is False
    assert state.remaining_turns == 0


def test_dynamic_state_does_not_stop_on_unknown_observation() -> None:
    observations = iter(
        [
            PseudoSoloObservation.UNKNOWN,
        ]
    )
    state = PseudoSoloDefenseState(FakeDefenseState(), lambda: next(observations))

    assert state.remaining_turns == 5


def test_dynamic_state_defends_only_for_multiple_and_stops_for_single() -> None:
    observations = iter(
        [
            PseudoSoloObservation.MULTIPLE_SURVIVORS,
            PseudoSoloObservation.UNKNOWN,
            PseudoSoloObservation.SINGLE_SURVIVOR,
        ]
    )
    state = PseudoSoloDefenseState(FakeDefenseState(), lambda: next(observations))

    state.begin_battle()
    assert state.should_defend() is True
    assert state.should_defend() is True

    state.begin_battle()
    assert state.should_defend() is False

    state.begin_battle()
    assert state.should_defend() is False


def test_dynamic_state_resumes_defense_after_survivor_reappears() -> None:
    observations = iter(
        [
            PseudoSoloObservation.SINGLE_SURVIVOR,
            PseudoSoloObservation.MULTIPLE_SURVIVORS,
        ]
    )
    state = PseudoSoloDefenseState(FakeDefenseState(), lambda: next(observations))

    state.begin_battle()
    assert state.should_defend() is False
    state.begin_battle()
    assert state.should_defend() is True


def test_dynamic_state_calculates_and_consumes_battle_defense_budget() -> None:
    class FakeBattleObserver:
        selected_count = 4
        last_battle_live_count = 4
        calls = 0

        def __call__(self) -> PseudoSoloObservation:
            self.calls += 1
            return PseudoSoloObservation.MULTIPLE_SURVIVORS

    base_state = FakeDefenseState(5)
    observer = FakeBattleObserver()
    state = PseudoSoloDefenseState(base_state, observer)

    state.begin_battle()
    assert state.should_defend() is True
    assert base_state.remaining_turns == 3
    assert observer.calls == 1
    assert state.should_defend() is True
    assert observer.calls == 1

    state.consume_turn()
    state.consume_turn()
    assert state.should_defend() is True
    state.consume_turn()
    assert state.should_defend() is False
    assert base_state.remaining_turns == 0


def test_dynamic_state_uses_live_team_count_instead_of_fixed_budget() -> None:
    base_state = FakeDefenseState(5)
    observer = FakeTeamPageObserver(live_count=12)
    state = PseudoSoloDefenseState(base_state, observer)

    assert state.observe_team_page(2) is True
    assert base_state.remaining_turns == 11
    assert defense_turns_for_live_count(12) == 11

    state.consume_turn()
    assert state.observe_team_page(2) is False
    assert base_state.remaining_turns == 10
    assert observer.reset_calls == 1

    observer.live_count = 2
    assert state.observe_team_page(2) is True
    assert base_state.remaining_turns == 1

    observer.live_count = 1
    assert state.observe_team_page(2) is True
    assert base_state.remaining_turns == 0


def test_dynamic_state_restarts_same_roster_count_on_a_new_floor() -> None:
    base_state = FakeDefenseState(5)
    observer = FakeTeamPageObserver(live_count=2)
    state = PseudoSoloDefenseState(base_state, observer)

    assert state.observe_team_page(2) is True
    state.consume_turn()
    assert base_state.remaining_turns == 0

    assert state.observe_team_page(3) is True
    assert base_state.remaining_turns == 1


def test_dynamic_state_stops_for_single_survivor_and_keeps_fallback_on_unknown() -> None:
    base_state = FakeDefenseState(2)
    observer = FakeTeamPageObserver()
    state = PseudoSoloDefenseState(base_state, observer)

    assert state.observe_team_page(3) is False
    assert base_state.remaining_turns == 2
    observer.live_count = 1
    assert state.observe_team_page(3) is True
    assert base_state.remaining_turns == 0
    observer.live_count = 12
    assert state.observe_team_page(4) is True
    assert base_state.remaining_turns == 11


def test_observer_accepts_single_survivor_on_first_complete_frame(monkeypatch) -> None:
    observer = BattleRosterObserver([1] * 12)
    monkeypatch.setattr(pseudo_solo.auto, "_frame_id", 1, raising=False)
    monkeypatch.setattr(pseudo_solo.auto, "screenshot", object(), raising=False)
    monkeypatch.setattr(observer, "_read_current_frame", lambda: PseudoSoloObservation.SINGLE_SURVIVOR)

    assert observer() is PseudoSoloObservation.SINGLE_SURVIVOR


def _portrait_frame(count: int) -> np.ndarray:
    frame = np.zeros((1080, 1920, 3), dtype=np.uint8)
    for index in range(count):
        center_x = round(571 + 121.5 * index)
        for y in range(970, 1015):
            for x in range(center_x - 20, center_x + 21):
                frame[y, x] = (
                    (x * 7 + y * 3) % 180 + 20,
                    (x * 5 + y * 11) % 150 + 20,
                    (x * 13 + y * 2) % 170 + 40,
                )
        frame[1017:1060, center_x - 20 : center_x + 20] = (255, 128, 20)
    return frame


def test_battle_portrait_badge_count_matches_visible_portraits() -> None:
    assert battle_portrait_badge_count(_portrait_frame(7)) == 7
    assert battle_portrait_badge_count(_portrait_frame(1)) == 1


def test_battle_portrait_badge_count_rejects_more_portraits_than_selected() -> None:
    assert battle_portrait_badge_count(_portrait_frame(4), max_count=3) is None


def test_battle_portrait_badge_count_ignores_short_orange_effects() -> None:
    frame = _portrait_frame(1)
    # This has enough horizontal width and orange area to pass the old
    # projection-only detector, but it is only 18 px high and is not a level
    # badge.
    frame[1017:1035, 400:460] = (255, 128, 20)

    assert battle_portrait_badge_count(frame) == 1


def test_observer_recognizes_only_once_per_battle(monkeypatch) -> None:
    observer = BattleRosterObserver([1] * 12)
    monkeypatch.setattr(pseudo_solo.auto, "_frame_id", 1, raising=False)
    monkeypatch.setattr(pseudo_solo.auto, "screenshot", _portrait_frame(1), raising=False)

    assert observer.read_battle_live_count() == 1

    monkeypatch.setattr(pseudo_solo.auto, "_frame_id", 2, raising=False)
    monkeypatch.setattr(pseudo_solo.auto, "screenshot", _portrait_frame(7), raising=False)
    assert observer.read_battle_live_count() == 1

    observer.begin_battle()
    assert observer.read_battle_live_count() == 7


def _patch_battle_gear_anchors(monkeypatch, left: tuple[int, int], right: tuple[int, int]) -> None:
    def fake_find_element(target, **kwargs):
        if target == "battle/gear_left.png":
            return left
        if target == "battle/gear_right.png":
            return right
        return None

    monkeypatch.setattr(pseudo_solo.auto, "find_element", fake_find_element)


def _fixed_slot_centers(slot_count: int) -> list[int]:
    if slot_count == 6:
        return [632, 754, 876, 997, 1119, 1240]
    if slot_count == 4:
        return [571, 693, 815, 937]
    raise AssertionError(slot_count)


def _fixed_battle_frame(live_indexes: set[int], slot_count: int = 6) -> np.ndarray:
    frame = np.zeros((1080, 1920, 3), dtype=np.uint8)
    for index, center_x in enumerate(_fixed_slot_centers(slot_count)):
        if index in live_indexes:
            for y in range(970, 1015):
                for x in range(center_x - 20, center_x + 21):
                    frame[y, x] = (
                        (x * 7 + y * 3) % 180 + 20,
                        (x * 5 + y * 11) % 150 + 20,
                        (x * 13 + y * 2) % 170 + 40,
                    )
            frame[1018:1060, center_x - 20 : center_x + 21] = (255, 128, 20)
        else:
            # A simplified version of the blue/cyan sword/reload placeholder
            # that marks an empty command slot.
            frame[970:1010, center_x - 20 : center_x + 21] = (20, 100, 180)
    return frame


def test_observer_reads_each_fixed_battle_slot(monkeypatch) -> None:
    observer = BattleRosterObserver([1] * 12)
    monkeypatch.setattr(pseudo_solo.auto, "_frame_id", 1, raising=False)
    monkeypatch.setattr(pseudo_solo.auto, "screenshot", _fixed_battle_frame({0, 2, 3, 4}), raising=False)
    _patch_battle_gear_anchors(monkeypatch, (455, 811), (1315, 822))

    assert observer.read_battle_live_count() == 4
    assert observer() is PseudoSoloObservation.MULTIPLE_SURVIVORS
    assert [slot["state"] for slot in observer.last_battle_slot_states] == ["LIVE"] * 4


def test_observer_uses_color_monitor_frame_without_replacing_business_frame(monkeypatch) -> None:
    observer = BattleRosterObserver([1] * 12)
    business_frame = Image.fromarray(np.zeros((1080, 1920), dtype=np.uint8), mode="L")
    color_frame = Image.fromarray(_fixed_battle_frame(set(range(6))), mode="RGB")
    monitor_calls: list[dict[str, object]] = []

    monkeypatch.setattr(pseudo_solo.auto, "_frame_id", 1, raising=False)
    monkeypatch.setattr(pseudo_solo.auto, "screenshot", business_frame, raising=False)
    _patch_battle_gear_anchors(monkeypatch, (455, 811), (1315, 822))

    def take_monitor_screenshot(**kwargs):
        monitor_calls.append(kwargs)
        return color_frame

    monkeypatch.setattr(pseudo_solo.auto, "take_monitor_screenshot", take_monitor_screenshot)

    assert observer.read_battle_live_count() == 6
    assert pseudo_solo.auto.screenshot is business_frame
    assert monitor_calls == [
        {"gray": False, "max_age": 0},
        {"gray": False, "max_age": 0},
    ]


def test_observer_waits_for_two_stable_color_frames(monkeypatch) -> None:
    observer = BattleRosterObserver([1] * 12)
    business_frame = Image.fromarray(np.zeros((1080, 1920), dtype=np.uint8), mode="L")
    animation_frame = _fixed_battle_frame(set(range(7)))
    animation_frame[160:810, 384:1536] = (30, 80, 180)
    frames = [animation_frame, _portrait_frame(7), _portrait_frame(7)]

    monkeypatch.setattr(pseudo_solo.auto, "_frame_id", 1, raising=False)
    monkeypatch.setattr(pseudo_solo.auto, "screenshot", business_frame, raising=False)
    monkeypatch.setattr(pseudo_solo, "interruptible_sleep", lambda _seconds: None)
    monkeypatch.setattr(
        pseudo_solo.auto,
        "take_monitor_screenshot",
        lambda **_kwargs: frames.pop(0),
    )

    assert observer.read_battle_live_count() == 7
    assert observer.last_battle_diagnostics["sample_count"] == 3
    assert observer.last_battle_diagnostics["valid_sample_count"] == 2
    assert observer.last_battle_diagnostics["stable_sample_count"] == 2
    assert observer.last_battle_diagnostics["lock_reason"] == "stable_samples"


def test_observer_does_not_lock_on_nonconsecutive_conflicting_samples(monkeypatch) -> None:
    observer = BattleRosterObserver([1] * 12)
    business_frame = Image.fromarray(np.zeros((1080, 1920), dtype=np.uint8), mode="L")
    frames = [
        _portrait_frame(7),
        object(),
        _portrait_frame(6),
        object(),
        _portrait_frame(7),
        object(),
    ]

    monkeypatch.setattr(pseudo_solo.auto, "_frame_id", 1, raising=False)
    monkeypatch.setattr(pseudo_solo.auto, "screenshot", business_frame, raising=False)
    monkeypatch.setattr(pseudo_solo, "BATTLE_ROSTER_MAX_SAMPLES", 6)
    monkeypatch.setattr(pseudo_solo, "interruptible_sleep", lambda _seconds: None)
    monkeypatch.setattr(
        pseudo_solo.auto,
        "take_monitor_screenshot",
        lambda **_kwargs: frames.pop(0),
    )

    assert observer.read_battle_live_count() is None
    assert observer.last_battle_diagnostics["sample_count"] == 6
    assert observer.last_battle_diagnostics["valid_sample_count"] == 3
    assert observer.last_battle_diagnostics["stable_sample_count"] == 0
    assert observer.last_battle_diagnostics["lock_reason"] == "no_stable_result"


def test_observer_returns_unknown_when_battle_portraits_are_missing(monkeypatch) -> None:
    observer = BattleRosterObserver([1] * 12)
    monkeypatch.setattr(pseudo_solo.auto, "find_element", lambda *args, **kwargs: None)
    monkeypatch.setattr(pseudo_solo.auto, "_frame_id", 1, raising=False)
    monkeypatch.setattr(pseudo_solo.auto, "screenshot", object(), raising=False)

    assert observer() is PseudoSoloObservation.UNKNOWN


def test_observer_counts_visible_badges_without_fixed_slot_geometry(monkeypatch) -> None:
    observer = BattleRosterObserver([1] * 12)
    monkeypatch.setattr(pseudo_solo.auto, "_frame_id", 1, raising=False)
    monkeypatch.setattr(pseudo_solo.auto, "screenshot", _portrait_frame(7), raising=False)

    assert observer.read_battle_live_count() == 7
    assert observer.last_battle_diagnostics["detection_source"] == "visible_badges"
    assert len(observer.last_battle_diagnostics["visible_badge_centers"]) == 7


def test_observer_reads_existing_team_page_count_asset(monkeypatch) -> None:
    observer = BattleRosterObserver([1] * 12)

    def fake_find_element(target, **kwargs):
        if target == "teams/12_sinner_live_assets.png":
            return (200, 200)
        return None

    monkeypatch.setattr(pseudo_solo.auto, "find_element", fake_find_element)
    monkeypatch.setattr(pseudo_solo.auto, "_frame_id", 1, raising=False)
    monkeypatch.setattr(pseudo_solo.auto, "screenshot", object(), raising=False)

    assert observer.read_team_page_live_count() == 12


def test_observer_reads_small_team_page_count_with_strict_ocr(monkeypatch) -> None:
    observer = BattleRosterObserver([1, 1, 1])
    monkeypatch.setattr(pseudo_solo.auto, "get_text_from_screenshot", lambda **kwargs: ["2/3"])
    monkeypatch.setattr(pseudo_solo.auto, "_frame_id", 1, raising=False)
    monkeypatch.setattr(pseudo_solo.auto, "screenshot", object(), raising=False)

    assert observer.read_team_page_live_count() == 2


def test_observer_rejects_more_battle_portraits_than_selected_sinners(monkeypatch) -> None:
    observer = BattleRosterObserver([1] * 3)
    monkeypatch.setattr(pseudo_solo.auto, "_frame_id", 1, raising=False)
    monkeypatch.setattr(pseudo_solo.auto, "screenshot", _fixed_battle_frame(set(range(4)), slot_count=4), raising=False)
    monkeypatch.setattr(observer, "_read_battle_slot_geometry", lambda: (_fixed_slot_centers(4), {"reason": "test"}))

    assert observer() is PseudoSoloObservation.UNKNOWN


def test_observer_confirms_single_visible_battle_portrait(monkeypatch) -> None:
    observer = BattleRosterObserver([1] * 12)
    monkeypatch.setattr(pseudo_solo.auto, "_frame_id", 1, raising=False)
    frame = _portrait_frame(1)
    for y in range(970, 1015):
        for x in range(551, 592):
            frame[y, x] = (
                (x * 7 + y * 3) % 180 + 20,
                (x * 5 + y * 11) % 150 + 20,
                (x * 13 + y * 2) % 170 + 40,
            )
    monkeypatch.setattr(pseudo_solo.auto, "screenshot", frame, raising=False)
    monkeypatch.setattr(observer, "_read_battle_slot_geometry", lambda: ([571], {"reason": "test"}))

    assert observer() is PseudoSoloObservation.SINGLE_SURVIVOR


def test_observer_returns_unknown_for_animation_overlay(monkeypatch) -> None:
    observer = BattleRosterObserver([1] * 6)
    frame = _fixed_battle_frame({0, 2, 3, 4})
    frame[160:810, 384:1536] = (30, 80, 180)
    monkeypatch.setattr(pseudo_solo.auto, "_frame_id", 1, raising=False)
    monkeypatch.setattr(pseudo_solo.auto, "screenshot", frame, raising=False)
    monkeypatch.setattr(observer, "_read_battle_slot_geometry", lambda: (_fixed_slot_centers(6), {"reason": "test"}))

    assert observer() is PseudoSoloObservation.UNKNOWN


def test_observer_ignores_dark_blue_battle_background(monkeypatch) -> None:
    observer = BattleRosterObserver([1] * 6)
    frame = _fixed_battle_frame({0, 2, 3, 4})
    frame[160:810, 384:1536] = (15, 45, 100)
    monkeypatch.setattr(pseudo_solo.auto, "_frame_id", 1, raising=False)
    monkeypatch.setattr(pseudo_solo.auto, "screenshot", frame, raising=False)
    monkeypatch.setattr(observer, "_read_battle_slot_geometry", lambda: (_fixed_slot_centers(6), {"reason": "test"}))

    assert observer() is PseudoSoloObservation.MULTIPLE_SURVIVORS


def test_observer_returns_unknown_for_obscured_avatar(monkeypatch) -> None:
    observer = BattleRosterObserver([1] * 6)
    frame = _fixed_battle_frame({0, 2, 3, 4})
    frame[965:1020, 876 - 31 : 876 + 32] = (10, 10, 10)
    monkeypatch.setattr(pseudo_solo.auto, "_frame_id", 1, raising=False)
    monkeypatch.setattr(pseudo_solo.auto, "screenshot", frame, raising=False)
    monkeypatch.setattr(observer, "_read_battle_slot_geometry", lambda: (_fixed_slot_centers(6), {"reason": "test"}))

    assert observer() is PseudoSoloObservation.UNKNOWN
    assert observer.last_battle_slot_states[1]["state"] == "UNKNOWN"


def test_observer_sampling_aborts_promptly_when_stop_requested(monkeypatch) -> None:
    from core.execution_control import bind_cancel_event
    from module.my_error.my_error import userStopError

    observer = BattleRosterObserver([1] * 12)
    business_frame = Image.fromarray(np.zeros((1080, 1920), dtype=np.uint8), mode="L")
    cancel_event = threading.Event()
    cancel_event.set()  # 停止按钮已在采样开始前按下

    monkeypatch.setattr(pseudo_solo.auto, "screenshot", business_frame, raising=False)
    monkeypatch.setattr(pseudo_solo, "interruptible_sleep", lambda _seconds: None)
    bind_cancel_event(cancel_event)
    try:
        with pytest.raises(userStopError):
            observer.read_battle_live_count()
    finally:
        bind_cancel_event(None)


def test_observer_propagates_cancellation_from_monitor_capture(monkeypatch) -> None:
    from module.my_error.my_error import userStopError

    observer = BattleRosterObserver([1] * 12)
    business_frame = Image.fromarray(np.zeros((1080, 1920), dtype=np.uint8), mode="L")

    monkeypatch.setattr(pseudo_solo.auto, "screenshot", business_frame, raising=False)
    monkeypatch.setattr(pseudo_solo, "interruptible_sleep", lambda _seconds: None)

    def raise_stop(**_kwargs):
        raise userStopError("用户已请求停止任务")

    # 取消信号禁止被降级为 screenshot=None 继续采样，必须直接上抛。
    monkeypatch.setattr(pseudo_solo.auto, "take_monitor_screenshot", raise_stop)
    with pytest.raises(userStopError):
        observer.read_battle_live_count()
