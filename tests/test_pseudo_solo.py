from __future__ import annotations

from dataclasses import dataclass

from core import pseudo_solo
from core.pseudo_solo import (
    BattleRosterObserver,
    PseudoSoloDefenseState,
    PseudoSoloObservation,
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


def test_dynamic_state_keeps_budget_until_observer_confirms_single_survivor() -> None:
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
    assert state.remaining_turns == 0


def test_dynamic_state_does_not_stop_on_unknown_observation() -> None:
    observations = iter(
        [
            PseudoSoloObservation.UNKNOWN,
        ]
    )
    state = PseudoSoloDefenseState(FakeDefenseState(), lambda: next(observations))

    assert state.remaining_turns == 5


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


def test_observer_requires_two_distinct_frames(monkeypatch) -> None:
    observer = BattleRosterObserver([1] * 12)
    monkeypatch.setattr(pseudo_solo.auto, "_frame_id", 1, raising=False)
    monkeypatch.setattr(pseudo_solo.auto, "screenshot", object(), raising=False)
    monkeypatch.setattr(observer, "_read_current_frame", lambda: PseudoSoloObservation.SINGLE_SURVIVOR)

    assert observer() is PseudoSoloObservation.UNKNOWN

    monkeypatch.setattr(pseudo_solo.auto, "_frame_id", 2, raising=False)
    monkeypatch.setattr(pseudo_solo.auto, "screenshot", object(), raising=False)
    assert observer() is PseudoSoloObservation.SINGLE_SURVIVOR


def test_observer_counts_distinct_dead_markers(monkeypatch) -> None:
    observer = BattleRosterObserver([1] * 12)
    dead_markers = [(100 + index * 100, 900) for index in range(11)]

    def fake_find_element(target, **kwargs):
        if target == "battle/gear_left.png":
            return (100, 1000)
        if target == "battle/gear_right.png":
            return (2200, 1000)
        if target == "battle/dead.png":
            return dead_markers
        raise AssertionError(target)

    monkeypatch.setattr(pseudo_solo.auto, "find_element", fake_find_element)
    monkeypatch.setattr(pseudo_solo.auto, "_frame_id", 1, raising=False)
    monkeypatch.setattr(pseudo_solo.auto, "screenshot", object(), raising=False)

    assert observer() is PseudoSoloObservation.UNKNOWN

    monkeypatch.setattr(pseudo_solo.auto, "_frame_id", 2, raising=False)
    monkeypatch.setattr(pseudo_solo.auto, "screenshot", object(), raising=False)
    assert observer() is PseudoSoloObservation.SINGLE_SURVIVOR


def test_observer_returns_unknown_when_battle_anchors_are_missing(monkeypatch) -> None:
    observer = BattleRosterObserver([1] * 12)
    monkeypatch.setattr(pseudo_solo.auto, "find_element", lambda *args, **kwargs: None)
    monkeypatch.setattr(pseudo_solo.auto, "_frame_id", 1, raising=False)
    monkeypatch.setattr(pseudo_solo.auto, "screenshot", object(), raising=False)

    assert observer() is PseudoSoloObservation.UNKNOWN


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


def test_observer_rejects_more_dead_markers_than_selected_sinners(monkeypatch) -> None:
    observer = BattleRosterObserver([1] * 3)

    def fake_find_element(target, **kwargs):
        if target == "battle/gear_left.png":
            return (100, 1000)
        if target == "battle/gear_right.png":
            return (2200, 1000)
        if target == "battle/dead.png":
            return [(100, 900), (200, 900), (300, 900)]
        raise AssertionError(target)

    monkeypatch.setattr(pseudo_solo.auto, "find_element", fake_find_element)
    monkeypatch.setattr(pseudo_solo.auto, "_frame_id", 1, raising=False)
    monkeypatch.setattr(pseudo_solo.auto, "screenshot", object(), raising=False)

    assert observer() is PseudoSoloObservation.UNKNOWN


def test_observer_uses_roster_ocr_when_dead_markers_are_incomplete(monkeypatch) -> None:
    observer = BattleRosterObserver([1] * 12)

    def fake_find_element(target, **kwargs):
        if target == "battle/gear_left.png":
            return (100, 1000)
        if target == "battle/gear_right.png":
            return (2200, 1000)
        if target == "battle/dead.png":
            return [(100, 900)]
        raise AssertionError(target)

    monkeypatch.setattr(pseudo_solo.auto, "find_element", fake_find_element)
    monkeypatch.setattr(pseudo_solo.auto, "get_text_from_screenshot", lambda **kwargs: ["1/12"])
    monkeypatch.setattr(pseudo_solo.auto, "_frame_id", 1, raising=False)
    monkeypatch.setattr(pseudo_solo.auto, "screenshot", object(), raising=False)

    assert observer() is PseudoSoloObservation.UNKNOWN

    monkeypatch.setattr(pseudo_solo.auto, "_frame_id", 2, raising=False)
    monkeypatch.setattr(pseudo_solo.auto, "screenshot", object(), raising=False)
    assert observer() is PseudoSoloObservation.SINGLE_SURVIVOR
