from module.mirror_routes import (
    DEFAULT_ROUTE,
    HOS_RYOSHU_SOLO_ROUTE,
    get_mirror_route,
    route_target_priority,
)


def test_hos_route_switches_stages_at_five_and_ten() -> None:
    assert HOS_RYOSHU_SOLO_ROUTE.stage_for_floor(1).start_floor == 1
    assert HOS_RYOSHU_SOLO_ROUTE.stage_for_floor(5).end_floor == 5
    assert HOS_RYOSHU_SOLO_ROUTE.stage_for_floor(6).start_floor == 6
    assert HOS_RYOSHU_SOLO_ROUTE.stage_for_floor(10).end_floor == 10
    assert HOS_RYOSHU_SOLO_ROUTE.stage_for_floor(11).start_floor == 11
    assert HOS_RYOSHU_SOLO_ROUTE.stage_for_floor(15).end_floor == 15


def test_hos_route_exposes_short_and_full_floor_windows() -> None:
    assert HOS_RYOSHU_SOLO_ROUTE.floor_counts == (5, 15)
    assert "unloving" in HOS_RYOSHU_SOLO_ROUTE.theme_pack_names_for_floor(1)
    assert "chick" in HOS_RYOSHU_SOLO_ROUTE.theme_pack_names_for_floor(2)
    assert HOS_RYOSHU_SOLO_ROUTE.theme_pack_names_for_floor(3)[0] == "Falling Flowers"
    assert "Line 1" in HOS_RYOSHU_SOLO_ROUTE.theme_pack_names_for_floor(6)
    assert "Line 3" in HOS_RYOSHU_SOLO_ROUTE.theme_pack_names_for_floor(11)


def test_route_priority_is_ordered_and_unknown_gifts_fall_back() -> None:
    targets = HOS_RYOSHU_SOLO_ROUTE.gift_targets_for_floor(5)
    assert (
        route_target_priority(
            HOS_RYOSHU_SOLO_ROUTE,
            5,
            lambda target: target.gift_id == "sharp_needle_and_thread",
        )
        == 40
    )
    assert targets[0].priority < targets[-1].priority
    assert route_target_priority(HOS_RYOSHU_SOLO_ROUTE, 5, lambda _: False) is None


def test_route_registers_bounded_process_fusions_and_their_materials() -> None:
    recipes = HOS_RYOSHU_SOLO_ROUTE.fusion_recipes_for_floor(5)
    assert [recipe.result_gift_id for recipe in recipes] == [
        "hoarfrost_footprint",
        "unmailed_letter",
        "spicebush_glasses_mailed_letter",
    ]
    assert recipes[0].material_gift_ids == ("haunted_shoes", "frozen_cries")
    assert recipes[0].keyword == "sinking"
    assert recipes[-1].skip_if_pseudo_solo
    assert HOS_RYOSHU_SOLO_ROUTE.gift_target("ragged_umbrella") is not None


def test_hos_route_has_unique_process_gifts_and_required_targets() -> None:
    gifts = HOS_RYOSHU_SOLO_ROUTE.gifts
    assert len({gift.gift_id for gift in gifts}) == len(gifts)
    assert HOS_RYOSHU_SOLO_ROUTE.gift_target("chief_butler_secret_arts") is not None
    assert HOS_RYOSHU_SOLO_ROUTE.gift_target("shadow_monster") is not None
    assert HOS_RYOSHU_SOLO_ROUTE.gift_target("packaging_ribbon") is not None
    assert HOS_RYOSHU_SOLO_ROUTE.gift_target("sharp_needle_and_thread").required
    assert HOS_RYOSHU_SOLO_ROUTE.gift_target("bridle").required
    assert HOS_RYOSHU_SOLO_ROUTE.gift_target("mid_range_k_corp_ampule").required
    assert "lunar_memory" not in {gift.gift_id for gift in HOS_RYOSHU_SOLO_ROUTE.gift_targets_for_floor(11)}
    assert "mid_range_k_corp_ampule" in {gift.gift_id for gift in HOS_RYOSHU_SOLO_ROUTE.gift_targets_for_floor(11)}


def test_unknown_route_id_keeps_default_behavior() -> None:
    assert get_mirror_route("not-shipped-yet") is DEFAULT_ROUTE
    assert get_mirror_route("") is DEFAULT_ROUTE


def test_legacy_route_aliases_resolve_to_independent_route_ids() -> None:
    assert HOS_RYOSHU_SOLO_ROUTE.route_id == "hos_ryoshu_solo_route"
    assert get_mirror_route("hos_ryoshu_solo") is HOS_RYOSHU_SOLO_ROUTE
