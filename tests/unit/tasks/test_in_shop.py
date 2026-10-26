from __future__ import annotations

from types import SimpleNamespace

from PIL import Image

from module.automation import auto
from tasks.mirror.in_shop import Shop, skill_replacement_mode_sequence


def test_route_gift_matching_skips_positions_outside_the_current_frame(monkeypatch) -> None:
    shop = object.__new__(Shop)
    shop.route = SimpleNamespace(gifts=(object(),))

    def fail_if_called(*_args, **_kwargs):
        raise AssertionError("an off-screen position must not trigger image/OCR matching")

    monkeypatch.setattr(shop, "_route_gift_id_for_crop", fail_if_called)
    monkeypatch.setattr("tasks.mirror.in_shop.auto.screenshot", Image.new("L", (1920, 1080)), raising=False)

    assert shop._route_gift_id_at_position((2032.25, 459.25), hover=True) is None


def test_route_gift_matching_skips_pre_hover_ocr_for_grid_scans(monkeypatch) -> None:
    shop = object.__new__(Shop)
    shop.route = SimpleNamespace(gifts=(object(),))
    calls = []
    blank_calls = []

    monkeypatch.setattr(shop, "_route_gift_id_for_crop", lambda *_args, **_kwargs: calls.append(1) or None)
    monkeypatch.setattr("tasks.mirror.in_shop.auto.screenshot", Image.new("L", (1920, 1080)), raising=False)
    monkeypatch.setattr("tasks.mirror.in_shop.auto.mouse_move", lambda *_args, **_kwargs: None)
    monkeypatch.setattr(
        "tasks.mirror.in_shop.auto.take_screenshot",
        lambda *_args, **_kwargs: Image.new("L", (1920, 1080)),
    )
    monkeypatch.setattr("tasks.mirror.in_shop.auto.mouse_to_blank", lambda *_args, **_kwargs: blank_calls.append(1))
    monkeypatch.setattr("tasks.mirror.in_shop.sleep", lambda *_args, **_kwargs: None)

    shop._route_gift_id_at_position(
        (500, 500),
        hover=True,
        visible_names=False,
        reset_mouse=False,
    )

    assert len(calls) == 1
    assert blank_calls == []


def test_route_gift_matching_keeps_pre_hover_ocr_for_visible_name_contexts(monkeypatch) -> None:
    shop = object.__new__(Shop)
    shop.route = SimpleNamespace(gifts=(object(),))
    calls = []
    blank_calls = []

    monkeypatch.setattr(shop, "_route_gift_id_for_crop", lambda *_args, **_kwargs: calls.append(1) or None)
    monkeypatch.setattr("tasks.mirror.in_shop.auto.screenshot", Image.new("L", (1920, 1080)), raising=False)
    monkeypatch.setattr("tasks.mirror.in_shop.auto.mouse_move", lambda *_args, **_kwargs: None)
    monkeypatch.setattr(
        "tasks.mirror.in_shop.auto.take_screenshot",
        lambda *_args, **_kwargs: Image.new("L", (1920, 1080)),
    )
    monkeypatch.setattr("tasks.mirror.in_shop.auto.mouse_to_blank", lambda *_args, **_kwargs: blank_calls.append(1))
    monkeypatch.setattr("tasks.mirror.in_shop.sleep", lambda *_args, **_kwargs: None)

    shop._route_gift_id_at_position((500, 500), hover=True)

    assert len(calls) == 2
    assert blank_calls == [1]


def test_remove_route_targets_resets_mouse_once_after_grid_scan(monkeypatch) -> None:
    shop = object.__new__(Shop)
    shop.route = SimpleNamespace(gifts=(object(),))
    priority_calls = []
    blank_calls = []

    def fake_priority(position, **kwargs):
        priority_calls.append((position, kwargs))
        return None

    monkeypatch.setattr(shop, "_route_priority_at_position", fake_priority)
    monkeypatch.setattr(
        "tasks.mirror.in_shop.auto.mouse_to_blank",
        lambda *_args, **_kwargs: blank_calls.append(1),
    )

    positions = [(100, 200), (300, 400)]
    assert shop._remove_route_targets(positions, hover=True) == positions
    assert len(priority_calls) == 2
    assert all(kwargs["visible_names"] is False for _, kwargs in priority_calls)
    assert all(kwargs["reset_mouse"] is False for _, kwargs in priority_calls)
    assert blank_calls == [1]


def test_route_enhance_candidates_resets_mouse_once_after_grid_scan(monkeypatch) -> None:
    shop = object.__new__(Shop)
    shop.route = SimpleNamespace(gifts=(object(),))
    find_calls = []
    priority_calls = []
    blank_calls = []

    def fake_find(target, find_type="image", **_kwargs):
        find_calls.append(target)
        if find_type == "image_with_multiple_targets" and len(find_calls) == 1:
            return [(500, 500)]
        return []

    def fake_priority(position, **kwargs):
        priority_calls.append((position, kwargs))
        return None

    monkeypatch.setattr("tasks.mirror.in_shop.auto.find_element", fake_find)
    monkeypatch.setattr(shop, "_route_priority_at_position", fake_priority)
    monkeypatch.setattr(
        "tasks.mirror.in_shop.auto.mouse_to_blank",
        lambda *_args, **_kwargs: blank_calls.append(1),
    )

    assert shop._route_enhance_candidates() == []
    assert priority_calls == [((500, 500), {"hover": True, "visible_names": False, "reset_mouse": False})]
    assert blank_calls == [1]


def test_fuse_inventory_crop_stays_inside_the_frame(monkeypatch) -> None:
    monkeypatch.setattr("tasks.mirror.in_shop.auto.screenshot", Image.new("L", (1920, 1080)), raising=False)

    left, top, right, bottom = Shop._fuse_inventory_crop()

    assert 0 <= left < right <= 1920
    assert 0 <= top < bottom <= 1080
    assert left >= int(1920 * 0.25)
    assert top >= int(1080 * 0.05)


def test_enhance_gifts_reuses_manifest_scan_across_stale_rounds(monkeypatch) -> None:
    shop = object.__new__(Shop)
    shop.system = "poise"
    shop.second_system = False
    shop.second_system_action = ()
    shop.second_system_select = "slash"
    shop.second_system_setting = 0
    shop.fuse_IV = False
    shop.first_gift_enhance = False
    shop.enhance_gifts_list = []
    route_scan_calls = []
    level_iv_calls = []
    first_gift_calls = []
    system_gift_calls = []

    def fake_route_scan():
        route_scan_calls.append(1)
        return []

    def fake_find(target, find_type="image", **_kwargs):
        if target.endswith("sort_button_assets.png"):
            return (100, 100)
        if target.endswith("level_IV.png"):
            level_iv_calls.append(1)
            return None
        if target.endswith("big_poise.png"):
            first_gift_calls.append(1)
            return None
        if find_type == "image_with_multiple_targets":
            system_gift_calls.append(1)
            return []
        return None

    monkeypatch.setattr(shop, "_route_enhance_candidates", fake_route_scan)
    monkeypatch.setattr("tasks.mirror.in_shop.auto.find_element", fake_find)
    monkeypatch.setattr(
        "tasks.mirror.in_shop.auto.take_screenshot",
        lambda *_args, **_kwargs: Image.new("L", (1920, 1080)),
    )
    monkeypatch.setattr("tasks.mirror.in_shop.auto.screenshot", Image.new("L", (1920, 1080)), raising=False)
    monkeypatch.setattr("tasks.mirror.in_shop.auto.mouse_click", lambda *_args, **_kwargs: None)
    monkeypatch.setattr("tasks.mirror.in_shop.auto.mouse_click_blank", lambda *_args, **_kwargs: None)
    monkeypatch.setattr("tasks.mirror.in_shop.auto.mouse_to_blank", lambda *_args, **_kwargs: None)
    monkeypatch.setattr("tasks.mirror.in_shop.retry", lambda *_args, **_kwargs: True)
    monkeypatch.setattr("tasks.mirror.in_shop.sleep", lambda *_args, **_kwargs: None)
    shop.ego_gift_to_power_up = lambda: True

    assert shop.enhance_gifts() is not False

    assert route_scan_calls == [1]
    assert level_iv_calls == [1]
    assert first_gift_calls == [1]
    assert system_gift_calls == [1]


def test_fuse_system_gifts_reuses_one_frame_per_inventory_scan(monkeypatch) -> None:
    shop = object.__new__(Shop)
    shop.system = "poise"
    screenshot_calls = []
    fuse_label_calls = []

    def fake_screenshot(*_args, **_kwargs):
        screenshot_calls.append(1)
        return Image.new("L", (1920, 1080))

    def fake_find(target, find_type="image", **_kwargs):
        if target.endswith("gifts_list_block.png"):
            return None
        if target.endswith("fuse_label.png"):
            fuse_label_calls.append(1)
            if len(fuse_label_calls) == 1:
                return [(100, 100), (200, 200)]
            return []
        if target.endswith("enhance_gifts/poise.png") and find_type == "image_with_multiple_targets":
            return [(300, 100)]
        return None

    monkeypatch.setattr("tasks.mirror.in_shop.auto.take_screenshot", fake_screenshot)
    monkeypatch.setattr("tasks.mirror.in_shop.auto.screenshot", Image.new("L", (1920, 1080)), raising=False)
    monkeypatch.setattr("tasks.mirror.in_shop.auto.find_element", fake_find)
    monkeypatch.setattr("tasks.mirror.in_shop.auto.mouse_click", lambda *_args, **_kwargs: None)
    monkeypatch.setattr("tasks.mirror.in_shop.auto.mouse_drag", lambda *_args, **_kwargs: None)
    monkeypatch.setattr("tasks.mirror.in_shop.auto.mouse_click_blank", lambda *_args, **_kwargs: None)
    monkeypatch.setattr("tasks.mirror.in_shop.auto.mouse_to_blank", lambda *_args, **_kwargs: None)
    monkeypatch.setattr(
        "tasks.mirror.in_shop.auto.click_element",
        lambda target, **_kwargs: target == "mirror/road_in_mir/ego_gift_get_confirm_assets.png",
    )
    monkeypatch.setattr("tasks.mirror.in_shop.retry", lambda *_args, **_kwargs: True)
    monkeypatch.setattr(shop, "enter_fuse", lambda: True)
    monkeypatch.setattr(shop, "_remove_route_targets", lambda positions, **_kwargs: positions)

    shop.fuse_system_gifts()

    # One capture per inventory pass, plus the confirm loop's capture.  The
    # pre-change path captured separately for list_block, fuse_label and the
    # system-gift scan on every pass.
    assert len(screenshot_calls) == 3


def test_skill_replacement_mode_sequence_covers_all_modes() -> None:
    assert skill_replacement_mode_sequence(0) == (0,)
    assert skill_replacement_mode_sequence(1) == (1,)
    assert skill_replacement_mode_sequence(2) == (2,)
    assert skill_replacement_mode_sequence(3) == (1, 0)
    assert skill_replacement_mode_sequence(4) == (0, 1)
    assert skill_replacement_mode_sequence(99) == (0,)

