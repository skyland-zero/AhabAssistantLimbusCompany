import importlib

from tasks.battle.battle import Battle, DefenseForSoloState

battle_module = importlib.import_module("tasks.battle.battle")


def test_solo_defense_accepts_scaled_left_anchor(monkeypatch) -> None:
    seen_thresholds: list[float] = []

    def fake_find_element(target, **kwargs):
        if target == "battle/gear_left.png":
            seen_thresholds.append(kwargs.get("threshold"))
            return (100, 100)
        if target == "battle/gear_right.png":
            return (900, 100)
        if target == "battle/pause_assets.png":
            return True
        raise AssertionError(target)

    monkeypatch.setattr(battle_module.auto, "find_element", fake_find_element)
    monkeypatch.setattr(battle_module.auto, "mouse_click_blank", lambda: None)
    monkeypatch.setattr(battle_module.auto, "mouse_to_blank", lambda **_: None)
    monkeypatch.setattr(battle_module.auto, "mouse_click", lambda *_args, **_kwargs: None)
    monkeypatch.setattr(battle_module.auto, "mouse_drag_link", lambda *_args, **_kwargs: None)
    monkeypatch.setattr(battle_module, "sleep", lambda _seconds: None)

    state = DefenseForSoloState(1)
    assert Battle()._battle_operation(
        first_turn=True,
        defense_first_round=False,
        avoid_skill_3=False,
        defense_for_solo_state=state,
    ) is True
    assert state.remaining_turns == 0
    assert battle_module.DEFENSE_GEAR_THRESHOLD in seen_thresholds
