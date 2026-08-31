from module.observe_ego_gift import (
    SPIDERWEB_ENTANGLED_IN_RED,
    SPIDERWEB_ENTANGLED_IN_RED_SELECTION,
    canonical_observe_ego_gift,
    normalize_observe_ego_gifts,
    resolve_observe_ego_gift,
)


def test_spiderweb_aliases_resolve_to_stable_target() -> None:
    for value in (
        SPIDERWEB_ENTANGLED_IN_RED,
        "赤红纠缠的蜘蛛巢",
        "赤紅糾纏的蜘蛛巢",
        "Spiderweb Entangled in Red",
        "general_gift_3_32.png",
    ):
        target = resolve_observe_ego_gift(value)
        assert target is not None
        assert target.key == SPIDERWEB_ENTANGLED_IN_RED
        assert target.coordinate == SPIDERWEB_ENTANGLED_IN_RED_SELECTION


def test_coordinate_values_are_validated_and_canonicalized() -> None:
    assert canonical_observe_ego_gift(" BLEED_3_2_5 ") == "bleed_3_2_5"
    assert canonical_observe_ego_gift("general_3_0_1") is None
    assert canonical_observe_ego_gift("general_4_1_1") is None
    assert canonical_observe_ego_gift("unknown_3_1_1") is None


def test_normalize_observe_gifts_deduplicates_skips_invalid_and_caps_at_three() -> None:
    assert normalize_observe_ego_gifts(
        [
            "赤红纠缠的蜘蛛巢",
            "spiderweb_entangled_in_red",
            "bleed_3_1_1",
            "burn_3_1_1",
            "tremor_3_1_1",
            "not-a-gift",
        ]
    ) == [SPIDERWEB_ENTANGLED_IN_RED, "bleed_3_1_1", "burn_3_1_1"]

