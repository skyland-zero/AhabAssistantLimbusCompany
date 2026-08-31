from tasks.battle.battle import DefenseForSoloState


def test_defense_for_solo_state_consumes_only_available_budget() -> None:
    state = DefenseForSoloState(2)

    assert state.remaining_turns == 2
    state.consume_turn()
    state.consume_turn()
    state.consume_turn()

    assert state.remaining_turns == 0
