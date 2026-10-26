"""Built-in team preset catalog exposed to the GPUI team picker.

The catalog deliberately owns presentation metadata separately from the
persisted team settings.  ``preset_id`` is stable even when a display name is
updated, while ``route_id`` points at the independent mirror-route catalog.
The return shape is already suitable for a future remote catalog loader; this
version only ships the built-in definitions.
"""

from __future__ import annotations

from collections.abc import Callable, Mapping
from dataclasses import dataclass
from typing import Any

from module.config.config_typing import TeamSetting
from module.observe_ego_gift import SPIDERWEB_ENTANGLED_IN_RED


@dataclass(frozen=True)
class BuiltinTeamPreset:
    preset_id: str
    route_id: str
    name_zh: str
    name_en: str
    description_zh: str
    description_en: str
    floor_hint_zh: str
    floor_hint_en: str
    route_name_zh: str
    route_name_en: str
    setting: Mapping[str, Any]


_HOS_RYOSHU_SOLO_SETTING: dict[str, Any] = {
    "purpose": "mirror",
    "team_system": 4,
    "sinners_be_select": 12,
    "chosen_sinners": [1] * 12,
    # Array index follows the legacy sinner id order.  The resulting
    # formation is Ryoshu, Yi Sang, Rodion, weak Meursault, weak
    # Gregor, Heathcliff, Outis, Hong Lu, Faust, Ishmael, Don, Sinclair.
    "sinner_order": [2, 9, 11, 1, 4, 8, 6, 10, 3, 12, 7, 5],
    "shop_strategy": 1,
    "do_not_heal": True,
    "do_not_buy": False,
    "do_not_fuse": False,
    "do_not_sell": True,
    "do_not_enhance": False,
    "use_starlight": True,
    "opening_items": True,
    # The first permutation is the guide's displayed starting-gift
    # order: Cigarette Holder II, Stone Tomb II, Old Wooden Doll II.
    "opening_items_select": 0,
    "opening_items_system": 4,
    "skill_replacement": True,
    "skill_replacement_select": 0,
    # Qt-compatible replacement mode: 0 means 1 -> 3.  The route keeps
    # the guide's finer S1/S2/S3 target as metadata while using this safe
    # compatible mode at runtime.
    "skill_replacement_mode": 0,
    "defense_for_solo": True,
    "use_damage_p": False,
    # This remains the compatibility fallback when live roster OCR is
    # unavailable.  Known counts are handled dynamically at runtime.
    "defense_for_solo_turns": 5,
    "reward_cards": True,
    "reward_cards_select": 3,
    "max_keyword_refresh": 2,
    "max_normal_refresh": 3,
    "use_team_code": True,
    "team_code": "H4sIAAAAAAAACnMxcUwvD8x2DAh0dgQBc0dPEOVS4ZgOop0iIcKm5WBhVxeIsH8xWNjJORCiuhIiHJAPUe0GEXZ0tLUFAH9Z+5NgAAAA",
    "observe_ego_gift": True,
    "observe_ego_gift_selected": [SPIDERWEB_ENTANGLED_IN_RED],
    "mirror_route_profile": "hos_ryoshu_solo_route",
}


_SPIDERWEB_NORMAL_FARM_SETTING: dict[str, Any] = {
    "purpose": "mirror",
    "team_system": 4,
    "sinners_be_select": 10,
    "chosen_sinners": [1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 0],
    # Positions: Ryoshu, Outis, Hong Lu, Rodion, Heathcliff, Yi Sang,
    # Sinclair, Faust, Ishmael, Don Quixote.
    "sinner_order": [6, 8, 10, 1, 0, 3, 5, 9, 4, 7, 2, 0],
    "shop_strategy": 1,
    "do_not_heal": True,
    "do_not_buy": False,
    "do_not_fuse": False,
    "do_not_sell": True,
    "do_not_enhance": False,
    "only_system_fuse": True,
    "shopping_strategy": True,
    # Route targets are bought before the generic pass.  Option 0 keeps the
    # generic pass to mandatory purchases only, so a shop visit stays short.
    "shopping_strategy_select": 0,
    "max_keyword_refresh": 0,
    "max_normal_refresh": 1,
    "use_starlight": True,
    "opening_items": True,
    "opening_items_select": 0,
    "opening_items_system": 4,
    "opening_bonus": [1, 1, 1, 1, 0, 0, 0, 0, 0, 0],
    "observe_ego_gift": True,
    "observe_ego_gift_selected": [SPIDERWEB_ENTANGLED_IN_RED],
    "skill_replacement": True,
    "skill_replacement_select": 0,
    # Replace both S1 and S2 with S3 when the shop offers the option.
    "skill_replacement_mode": 3,
    "reward_cards": True,
    "reward_cards_select": 3,
    "defense_for_solo": True,
    "defense_for_solo_turns": 5,
    "use_team_code": False,
    "team_code": "",
    # Visit floors 1–3, skip the late-floor shops.
    "ignore_shop": [0, 0, 0, 1, 1],
    "mirror_route_profile": "spiderweb_family_normal_farm_route",
    "remark_name": "蜘蛛巢全家桶（普牢速刷）",
}


_SPIDERWEB_EXP_LUX_SETTING: dict[str, Any] = {
    "purpose": "luxcavation",
    "team_system": 4,
    "sinners_be_select": 8,
    "chosen_sinners": [1, 1, 1, 1, 0, 1, 1, 1, 0, 1, 0, 0],
    # Ryoshu, Thumb Apprentice Heathcliff, Faust, Ishmael, Yi Sang,
    # Sinclair, Don Quixote, Hong Lu.  Heathcliff is moved to order 2 so
    # Hong Lu stays the latest House ally and dies first for sever-fate.
    "sinner_order": [5, 3, 7, 1, 0, 8, 2, 4, 0, 6, 0, 0],
    "use_team_code": True,
    "team_code": (
        "H4sIAAAAAAAACgXBwQ2DMAxA0ZmoaG85fNtpclFULE6ZwIk6AIzf92wxMpOjqYkkz8t7/WA3I+NHQzeZ8"
        "MLjdKoyIzrf0GoSgnjbN+xBDk8OzqojWYnub6CUP39lOM9gAAAA"
    ),
    "remark_name": "蜘蛛巢经验本（8人）",
}

_SPIDERWEB_THREAD_LUX_SETTING: dict[str, Any] = {
    "purpose": "luxcavation",
    "team_system": 4,
    "sinners_be_select": 7,
    "chosen_sinners": [1, 1, 1, 1, 0, 1, 0, 1, 0, 1, 0, 0],
    # Ryoshu, Faust, Ishmael, Yi Sang, Sinclair, Don Quixote, Hong Lu.
    "sinner_order": [4, 2, 6, 1, 0, 7, 0, 3, 0, 5, 0, 0],
    "use_team_code": True,
    "team_code": (
        "H4sIAAAAAAAACgXBwQ2DMBBFwZYSCY4c3nodtCALVpxcwTdKAaH8zPjLkcQZxc3ErOh1wx+G8kujrHbD/Iu8"
        "ggp3ZudopbqlYdH2FX+jEeLkqmWIiezxAZblD9bzziNgAAAA"
    ),
    "remark_name": "蜘蛛巢纽本（7人）",
}


BUILTIN_TEAM_PRESETS: tuple[BuiltinTeamPreset, ...] = (
    BuiltinTeamPreset(
        preset_id="hos_ryoshu_solo_normal",
        route_id="hos_ryoshu_solo_route",
        name_zh="小指良伪单通（普牢）",
        name_en="Ryoshu Pseudo-Solo (Normal)",
        description_zh=(
            "锁定 7 人编队（良秀首位，中指/环指父辈前置促成斩杀目标裂变），后备席留空实现 1 回合启动。"
            "普通镜牢全流程零商店纯 P 速刷（不买、不合、不强化），极大压缩现实过图时间。"
        ),
        description_en=(
            "7-sinner lineup (Ryoshu first, Middle/Ring patriarchs placed early to split kill targets) "
            "with empty bench for 1-turn startup. Pure-P zero-shop speedrun for normal Mirror Dungeons "
            "(no buy, fuse, or enhance) to minimize real-world run time."
        ),
        floor_hint_zh="适用于普通镜牢，执行 1–5 层",
        floor_hint_en="For normal Mirror Dungeons; runs floors 1–5",
        route_name_zh="House of Spiders 良秀伪单通路线",
        route_name_en="House of Spiders Ryoshu route",
        setting={
            **_HOS_RYOSHU_SOLO_SETTING,
            "sinners_be_select": 7,
            "chosen_sinners": [1, 0, 0, 1, 0, 1, 1, 0, 1, 1, 1, 0],
            "sinner_order": [6, 0, 0, 1, 0, 3, 5, 0, 4, 7, 2, 0],
            "use_team_code": False,
            "team_code": "",
            "shop_strategy": 0,
            "do_not_buy": True,
            "do_not_fuse": True,
            "do_not_enhance": True,
            "do_not_sell": True,
            "do_not_heal": True,
            "skill_replacement": False,
            "ignore_shop": [1, 1, 1, 1, 1],
            "opening_bonus": [1, 1, 1, 1, 0, 0, 0, 0, 0, 0],
            "remark_name": "小指良伪单通（普牢）",
        },
    ),
    BuiltinTeamPreset(
        preset_id="hos_ryoshu_solo_hard",
        route_id="hos_ryoshu_solo_route",
        name_zh="小指良伪单通（困牢）",
        name_en="Ryoshu Pseudo-Solo (Hard)",
        description_zh=(
            "良秀首位，李箱与罗佳随后；4–6 号位安排优先牺牲人格。"
            "困难镜牢开局使用 House of Spiders 攻略的十个 ++ 星光。"
        ),
        description_en=(
            "Ryoshu starts first, followed by Yi Sang and Rodion; slots 4–6 "
            "are prioritized sacrifices. Uses ten level-++ starting bonuses "
            "from the House of Spiders guide for hard Mirror Dungeons."
        ),
        floor_hint_zh="适用于困难镜牢，执行 1–15 层并开启平行叠加",
        floor_hint_en="For hard Mirror Dungeons; runs floors 1–15 with Parallel Superposition",
        route_name_zh="House of Spiders 良秀伪单通路线",
        route_name_en="House of Spiders Ryoshu route",
        setting={
            **_HOS_RYOSHU_SOLO_SETTING,
            "opening_bonus": [3] * 10,
            "remark_name": "小指良伪单通（困牢）",
        },
    ),
    BuiltinTeamPreset(
        preset_id="spiderweb_family",
        route_id="spiderweb_family_route",
        name_zh="蜘蛛巢全家桶",
        name_en="House of Spiders Full Roster",
        description_zh="保留现有蜘蛛巢专属 Gift Search 的全家桶编队，使用默认饰品流程。",
        description_en="The full House of Spiders roster with the existing exclusive Gift Search preset.",
        floor_hint_zh="沿用当前镜牢流程",
        floor_hint_en="Uses the current mirror-dungeon flow",
        route_name_zh="蜘蛛巢默认路线",
        route_name_en="Spiderweb default route",
        setting={
            "purpose": "mirror",
            "team_system": 4,
            "sinners_be_select": 10,
            "chosen_sinners": [1, 1, 1, 1, 0, 1, 1, 1, 1, 1, 1, 0],
            "sinner_order": [2, 8, 4, 1, 0, 5, 7, 9, 3, 10, 6, 0],
            "use_team_code": True,
            "team_code": "H4sIAAAAAAAACg3MQRJAMAxA0UthZ/HTFGE6ZKx6gtS4ALfnHeDpq0QERyQVCXqs5hV9aOE3hTTLBb2bn0ZeuNwr+52yiguPlW1GB6LZn9QzpxZ0eLUJGMcPt8GoUGAAAAA=",
            "observe_ego_gift": True,
            "observe_ego_gift_selected": [SPIDERWEB_ENTANGLED_IN_RED],
            "mirror_route_profile": "spiderweb_family_route",
            "remark_name": "蜘蛛巢全家桶",
        },
    ),
    BuiltinTeamPreset(
        preset_id="spiderweb_family_normal_farm",
        route_id="spiderweb_family_normal_farm_route",
        name_zh="蜘蛛巢全家桶（普牢速刷）",
        name_en="House of Spiders Normal Speed Farm",
        description_zh=(
            "10 人蜘蛛巢全家桶，良秀首位。普通镜牢默认只进 1–3 层商店："
            "优先换良秀三技能、购买明镜止水/斩击四级等路线饰品、只合成呼吸四级并强化关键小件；"
            "4–5 层商店跳过，减少菜单时间。"
        ),
        description_en=(
            "Ten-sinner House of Spiders roster with Ryoshu first. Normal Mirror Dungeons "
            "visit only floors 1–3 shops: replace Ryoshu's skills with S3, buy route gifts "
            "such as Clear Mirror, Calm Water and slash tier-IV gifts, fuse poise tier-IV only, "
            "and enhance key pieces. Floors 4–5 shops are skipped to save menu time."
        ),
        floor_hint_zh="适用于普通镜牢，执行 1–5 层；默认只进 1–3 层商店",
        floor_hint_en="For normal Mirror Dungeons; runs floors 1–5 and visits only floors 1–3 shops",
        route_name_zh="蜘蛛巢全家桶 普牢速刷路线",
        route_name_en="House of Spiders Normal Speed Farm Route",
        setting=_SPIDERWEB_NORMAL_FARM_SETTING,
    ),
    BuiltinTeamPreset(
        preset_id="spiderweb_exp_luxcavation",
        route_id="",
        name_zh="蜘蛛巢经验本（8人）",
        name_en="House of Spiders EXP (8-sinner)",
        description_zh=(
            "严格 8 人全蜘蛛巢：良秀首位，拇指子辈希斯克利夫放编队顺序 2；"
            "鸿璐先死开断缘，随后堂吉诃德→李箱→以实玛利→浮士德→拇指夫→辛克莱。"
            "适用于经验本 10 连战，斩击日最佳。编队码已内置，可在队伍页复制。"
        ),
        description_en=(
            "Strict eight-sinner House of Spiders lineup with Ryoshu first and "
            "Thumb Apprentice Heathcliff at order 2 so Hong Lu remains the "
            "latest House ally and dies first for the sever-fate buff, followed "
            "by Don Quixote, Yi Sang, Ishmael, Faust, Heathcliff, then Sinclair. "
            "Built for the ten-wave EXP Luxcavation, best on Slash days. The team "
            "code is built in and can be copied from the team editor."
        ),
        floor_hint_zh="8 人：良秀核心，鸿璐先死开断缘",
        floor_hint_en="8 sinners: Ryoshu core; Hong Lu dies first for sever-fate",
        route_name_zh="经验本 · 10 连战",
        route_name_en="EXP Luxcavation · 10-wave",
        setting=_SPIDERWEB_EXP_LUX_SETTING,
    ),
    BuiltinTeamPreset(
        preset_id="spiderweb_thread_luxcavation",
        route_id="",
        name_zh="蜘蛛巢纽本（7人）",
        name_en="House of Spiders Thread (7-sinner)",
        description_zh=(
            "严格 7 人蜘蛛巢：良秀首位，6 名队友按顺序献祭；"
            "适用于纽本 Lv60 三连战。周一火蛾或周五黑檀女王如拖时间，"
            "可改用单核良秀（见 docs/纽本与经验本-蜘蛛巢编队方案.md）。"
            "编队码已内置，可在队伍页复制。"
        ),
        description_en=(
            "Strict seven-sinner House of Spiders lineup with Ryoshu first and "
            "six allies sacrificed in order for the Lv60 Thread Luxcavation "
            "three-run chain. If Faelantern or Ebony Queen stalls, switch to the "
            "solo Ryoshu variant in docs/纽本与经验本-蜘蛛巢编队方案.md. The "
            "team code is built in and can be copied from the team editor."
        ),
        floor_hint_zh="7 人：良秀核心，按顺序献祭",
        floor_hint_en="7 sinners: Ryoshu core with ordered sacrifices",
        route_name_zh="纽本 · 3 连战",
        route_name_en="Thread Luxcavation · 3-run",
        setting=_SPIDERWEB_THREAD_LUX_SETTING,
    ),
)


def builtin_team_presets(
    team_detail: Callable[[int, Any, bool], dict[str, Any]],
) -> list[dict[str, Any]]:
    """Build complete UI templates from the shared TeamSetting model.

    The callback is supplied by ``BackendApplication`` so the catalog does
    not duplicate the backend's normalization rules for sinner order,
    defaults, and mirror fields.
    """

    result: list[dict[str, Any]] = []
    for definition in BUILTIN_TEAM_PRESETS:
        setting = TeamSetting.model_validate(dict(definition.setting))
        template = team_detail(0, setting, False)
        template["id"] = ""
        template["enabled"] = False
        result.append(
            {
                "presetId": definition.preset_id,
                "routeId": definition.route_id,
                "name": {"zhCn": definition.name_zh, "enUs": definition.name_en},
                "description": {"zhCn": definition.description_zh, "enUs": definition.description_en},
                "floorHint": {"zhCn": definition.floor_hint_zh, "enUs": definition.floor_hint_en},
                "routeName": {"zhCn": definition.route_name_zh, "enUs": definition.route_name_en},
                "team": template,
            }
        )
    return result
