from __future__ import annotations

from types import SimpleNamespace

from PIL import Image

from tasks.mirror.in_shop import Shop, skill_replacement_mode_sequence


def test_route_gift_matching_skips_positions_outside_the_current_frame(monkeypatch) -> None:
    shop = object.__new__(Shop)
    shop.route = SimpleNamespace(gifts=(object(),))

    def fail_if_called(*_args, **_kwargs):
        raise AssertionError("an off-screen position must not trigger image/OCR matching")

    monkeypatch.setattr(shop, "_route_gift_id_for_crop", fail_if_called)
    monkeypatch.setattr("tasks.mirror.in_shop.auto.screenshot", Image.new("L", (1920, 1080)), raising=False)

    assert shop._route_gift_id_at_position((2032.25, 459.25), hover=True) is None


def test_skill_replacement_mode_sequence_covers_all_modes() -> None:
    assert skill_replacement_mode_sequence(0) == (0,)
    assert skill_replacement_mode_sequence(1) == (1,)
    assert skill_replacement_mode_sequence(2) == (2,)
    assert skill_replacement_mode_sequence(3) == (1, 0)
    assert skill_replacement_mode_sequence(4) == (0, 1)
    assert skill_replacement_mode_sequence(99) == (0,)

