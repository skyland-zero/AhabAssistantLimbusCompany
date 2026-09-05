import importlib

from tasks.battle.battle import Battle, DefenseForSoloState

battle_module = importlib.import_module("tasks.battle.battle")


class FakeDynamicSoloState:
    def __init__(self, should_defend: bool) -> None:
        self._should_defend = should_defend
        self.remaining_turns = 3
        self.consume_calls = 0

    def should_defend(self) -> bool:
        return self._should_defend

    def consume_turn(self) -> None:
        self.consume_calls += 1
        if self.remaining_turns > 0:
            self.remaining_turns -= 1


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


def test_dynamic_pseudo_solo_defends_without_fixed_turn_budget(monkeypatch) -> None:
    pressed_keys: list[str] = []
    seen_thresholds: list[float] = []

    monkeypatch.setattr(battle_module.auto, "mouse_click_blank", lambda: None)
    monkeypatch.setattr(battle_module.auto, "key_press", pressed_keys.append)
    monkeypatch.setattr(
        battle_module.auto,
        "find_element",
        lambda target, **kwargs: target in {"battle/pause_assets.png", "battle/gear_left.png"},
    )
    monkeypatch.setattr(battle_module, "sleep", lambda _seconds: None)

    def fake_defense(*_args, **kwargs):
        seen_thresholds.append(kwargs.get("threshold", battle_module.DEFENSE_GEAR_THRESHOLD))
        return True

    monkeypatch.setattr(battle_module.Battle, "_defense_this_round", fake_defense)

    state = FakeDynamicSoloState(True)
    result = Battle()._battle_operation(
        first_turn=True,
        defense_first_round=True,
        avoid_skill_3=False,
        defense_for_solo_state=state,
    )

    assert result is True
    assert pressed_keys == []
    assert seen_thresholds == [battle_module.DEFENSE_GEAR_THRESHOLD]
    assert state.consume_calls == 1
    assert state.remaining_turns == 2


def test_dynamic_pseudo_solo_preserves_existing_fallback_when_defense_fails(monkeypatch) -> None:
    pressed_keys: list[str] = []

    monkeypatch.setattr(battle_module.auto, "mouse_click_blank", lambda: None)
    monkeypatch.setattr(battle_module.auto, "key_press", pressed_keys.append)
    monkeypatch.setattr(battle_module, "sleep", lambda _seconds: None)
    monkeypatch.setattr(
        battle_module.auto,
        "find_element",
        lambda target, **_kwargs: target == "battle/gear_left.png",
    )
    monkeypatch.setattr(battle_module.Battle, "_defense_this_round", lambda *_args, **_kwargs: False)

    result = Battle()._battle_operation(
        first_turn=False,
        defense_first_round=False,
        avoid_skill_3=False,
        defense_for_solo_state=FakeDynamicSoloState(True),
    )

    assert result is False
    assert pressed_keys == ["p", "enter", "p", "enter"]


def test_dynamic_pseudo_solo_falls_back_when_defense_operation_fails(monkeypatch) -> None:
    pressed_keys: list[str] = []
    defense_calls: list[object] = []

    monkeypatch.setattr(battle_module.auto, "mouse_click_blank", lambda: None)
    monkeypatch.setattr(battle_module.auto, "key_press", pressed_keys.append)
    monkeypatch.setattr(battle_module.auto, "find_element", lambda target, **_kwargs: False)
    monkeypatch.setattr(battle_module, "sleep", lambda _seconds: None)
    monkeypatch.setattr(
        battle_module.Battle,
        "_defense_this_round",
        lambda *_args, **_kwargs: defense_calls.append(object()) or False,
    )

    result = Battle()._battle_operation(
        first_turn=False,
        defense_first_round=False,
        avoid_skill_3=False,
        defense_for_solo_state=FakeDynamicSoloState(True),
    )

    assert result is False
    assert pressed_keys == ["p", "enter", "p", "enter"]
    assert len(defense_calls) == 1


def test_dynamic_pseudo_solo_retries_when_pause_marker_is_missing(monkeypatch) -> None:
    pressed_keys: list[str] = []

    monkeypatch.setattr(battle_module.auto, "mouse_click_blank", lambda: None)
    monkeypatch.setattr(battle_module.auto, "key_press", pressed_keys.append)
    monkeypatch.setattr(
        battle_module.auto,
        "find_element",
        lambda target, **_kwargs: target == "battle/gear_left.png",
    )
    monkeypatch.setattr(battle_module, "sleep", lambda _seconds: None)
    monkeypatch.setattr(battle_module.Battle, "_defense_this_round", lambda *_args, **_kwargs: True)

    result = Battle()._battle_operation(
        first_turn=False,
        defense_first_round=False,
        avoid_skill_3=False,
        defense_for_solo_state=FakeDynamicSoloState(True),
    )

    assert result is True
    assert pressed_keys == ["p", "enter"]


def test_dynamic_pseudo_solo_uses_normal_flow_after_single_survivor(monkeypatch) -> None:
    pressed_keys: list[str] = []
    defense_calls: list[object] = []

    monkeypatch.setattr(battle_module.auto, "mouse_click_blank", lambda: None)
    monkeypatch.setattr(battle_module.auto, "key_press", pressed_keys.append)
    monkeypatch.setattr(
        battle_module.auto,
        "find_element",
        lambda target, **kwargs: target == "battle/pause_assets.png",
    )
    monkeypatch.setattr(battle_module, "sleep", lambda _seconds: None)
    monkeypatch.setattr(
        battle_module.Battle,
        "_defense_this_round",
        lambda *_args, **_kwargs: defense_calls.append(object()) or True,
    )

    result = Battle()._battle_operation(
        first_turn=True,
        defense_first_round=True,
        avoid_skill_3=False,
        defense_for_solo_state=FakeDynamicSoloState(False),
    )

    assert result is False
    assert pressed_keys == ["p", "enter"]
    assert defense_calls == []


def test_dynamic_pseudo_solo_uses_normal_flow_for_unknown_observation(monkeypatch) -> None:
    pressed_keys: list[str] = []
    defense_calls: list[object] = []

    monkeypatch.setattr(battle_module.auto, "mouse_click_blank", lambda: None)
    monkeypatch.setattr(battle_module.auto, "key_press", pressed_keys.append)
    monkeypatch.setattr(
        battle_module.auto,
        "find_element",
        lambda target, **kwargs: target == "battle/pause_assets.png",
    )
    monkeypatch.setattr(battle_module, "sleep", lambda _seconds: None)
    monkeypatch.setattr(
        battle_module.Battle,
        "_defense_this_round",
        lambda *_args, **_kwargs: defense_calls.append(object()) or True,
    )

    result = Battle()._battle_operation(
        first_turn=True,
        defense_first_round=True,
        avoid_skill_3=False,
        defense_for_solo_state=FakeDynamicSoloState(False),
    )

    assert result is False
    assert pressed_keys == ["p", "enter"]
    assert defense_calls == []


def test_dynamic_pseudo_solo_keeps_existing_fallback_when_called_again(monkeypatch) -> None:
    pressed_keys: list[str] = []

    monkeypatch.setattr(battle_module.auto, "mouse_click_blank", lambda: None)
    monkeypatch.setattr(battle_module.auto, "key_press", pressed_keys.append)
    monkeypatch.setattr(battle_module, "sleep", lambda _seconds: None)

    result = Battle()._battle_operation(
        first_turn=False,
        defense_first_round=False,
        avoid_skill_3=False,
        defense_for_solo_state=FakeDynamicSoloState(True),
        defense_for_solo_used_this_turn=True,
    )

    assert result is False
    assert pressed_keys == ["p", "enter"]


def test_damage_p_uses_second_p_before_enter(monkeypatch) -> None:
    pressed_keys: list[str] = []

    monkeypatch.setattr(battle_module.auto, "mouse_click_blank", lambda: None)
    monkeypatch.setattr(battle_module.auto, "key_press", pressed_keys.append)
    monkeypatch.setattr(
        battle_module.auto,
        "find_element",
        lambda target, **_kwargs: target == "battle/pause_assets.png",
    )
    monkeypatch.setattr(battle_module, "sleep", lambda _seconds: None)

    result = Battle()._battle_operation(
        first_turn=False,
        defense_first_round=False,
        avoid_skill_3=False,
        use_damage_p=True,
    )

    assert result is False
    assert pressed_keys == ["p", "p", "enter"]


def test_damage_p_mouse_fallback_clicks_lower_selector(monkeypatch) -> None:
    pressed_keys: list[str] = []
    clicks: list[tuple[float, float]] = []

    monkeypatch.setattr(battle_module.auto, "mouse_click_blank", lambda: None)
    monkeypatch.setattr(battle_module.auto, "key_press", pressed_keys.append)
    monkeypatch.setattr(battle_module.auto, "mouse_click", lambda x, y: clicks.append((x, y)))
    monkeypatch.setattr(battle_module.auto, "click_element", lambda *_args, **_kwargs: None)
    monkeypatch.setattr(
        battle_module.auto,
        "find_element",
        lambda target, **_kwargs: (100, 200) if target == "battle/win_rate_card.png" else None,
    )
    monkeypatch.setattr(battle_module, "sleep", lambda _seconds: None)

    battle = Battle()
    battle.mouse_click_rate = True
    result = battle._battle_operation(
        first_turn=False,
        defense_first_round=False,
        avoid_skill_3=False,
        use_damage_p=True,
    )

    scale = battle_module.cfg.set_win_size / 1440
    assert result is False
    assert pressed_keys == ["p", "p", "enter"]
    assert clicks == [(100 + 50 * scale, 200 + 50 * scale)]
