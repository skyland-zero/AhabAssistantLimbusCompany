from __future__ import annotations

from tasks.mirror import mirror as mirror_module


def test_loaded_team_code_is_not_overwritten_by_manual_formation() -> None:
    runner = object.__new__(mirror_module.Mirror)
    runner.team_code_loaded = True

    assert runner._form_team_for_battle() is True
    assert runner.team_code_loaded is False


def test_manual_formation_is_used_when_team_code_was_not_loaded(monkeypatch) -> None:
    runner = object.__new__(mirror_module.Mirror)
    runner.team_code_loaded = False
    runner.sinner_team = [2, 1] + [0] * 10
    runner.chosen_sinners = [1, 1] + [0] * 10
    called = []

    def fake_team_formation(sinner_team, chosen_sinners):
        called.append((sinner_team, chosen_sinners))
        return True

    monkeypatch.setattr(mirror_module, "team_formation", fake_team_formation)

    assert runner._form_team_for_battle() is True
    assert called == [(runner.sinner_team, runner.chosen_sinners)]
