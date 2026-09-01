"""Data-only mirror route catalog.

Route definitions are intentionally independent from team presets.  A preset
stores only a stable route id; the automation layer resolves that id at the
point where it needs stage, gift, shop, or fusion priorities.  Keeping this
module free of UI and input code makes adding a future remote catalog a
validation/loading concern instead of a rewrite of the mirror task.
"""

from __future__ import annotations

from collections.abc import Callable, Iterable
from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class GiftRouteTarget:
    """A process gift and the stages in which it should be preferred."""

    gift_id: str
    names_zh: tuple[str, ...]
    names_en: tuple[str, ...]
    start_floor: int
    end_floor: int
    priority: int
    protected: bool = True
    asset: str | None = None
    required: bool = False

    def applies_to(self, floor: int) -> bool:
        logical_floor = max(1, floor)
        return self.start_floor <= logical_floor <= self.end_floor


@dataclass(frozen=True, slots=True)
class GiftFusionRecipe:
    """A bounded, optional route-specific fusion recipe.

    The recipe is data only.  The shop adapter may execute it when the game
    exposes the result and all materials through the current OCR/assets;
    otherwise it leaves the screen untouched and lets the existing generic
    fusion fallback run with the listed materials protected.
    """

    result_gift_id: str
    result_names_zh: tuple[str, ...]
    result_names_en: tuple[str, ...]
    material_gift_ids: tuple[str, ...]
    start_floor: int
    end_floor: int
    priority: int
    keyword: str | None = None
    skip_if_pseudo_solo: bool = False

    def applies_to(self, floor: int) -> bool:
        logical_floor = max(1, floor)
        return self.start_floor <= logical_floor <= self.end_floor


def route_target_priority(
    route: MirrorRouteDefinition,
    floor: int,
    matches: Callable[[GiftRouteTarget], bool],
) -> int | None:
    """Return the first matching target's stage priority.

    ``matches`` is supplied by the runtime (OCR, image matching, or a future
    remote-game adapter), keeping the catalog independent from automation
    and making a new route require data rather than branching code.
    """

    for target in route.gift_targets_for_floor(floor):
        if matches(target):
            return target.priority
    return None


@dataclass(frozen=True, slots=True)
class MirrorRouteStage:
    start_floor: int
    end_floor: int
    theme_pack_names: tuple[str, ...]
    target_gift_ids: tuple[str, ...]

    def contains(self, floor: int) -> bool:
        logical_floor = max(1, floor)
        return self.start_floor <= logical_floor <= self.end_floor


@dataclass(frozen=True, slots=True)
class MirrorRouteDefinition:
    route_id: str
    name_zh: str
    name_en: str
    summary_zh: str
    summary_en: str
    stages: tuple[MirrorRouteStage, ...] = ()
    gifts: tuple[GiftRouteTarget, ...] = ()
    recipes: tuple[GiftFusionRecipe, ...] = ()
    # A route can be entered in either a short (5-floor) or a full
    # (15-floor) mirror.  The progress marker selects one of these at
    # runtime; it is intentionally not encoded in the team preset.
    floor_counts: tuple[int, ...] = (5,)
    # Exact floor windows take precedence over the broad legacy stages.  This
    # avoids matching ``Line 1`` as a substring of ``Line 10`` while keeping
    # the old stage catalog available to callers that only need boundaries.
    floor_theme_pack_names: tuple[tuple[int, int, tuple[str, ...]], ...] = ()
    # The guide expresses skill replacement as the desired S1/S2/S3 counts.
    # The current shop UI exposes only a global replacement mode, so the
    # adapter can report this target and apply its compatible fallback without
    # pretending the two representations are identical.
    skill_replacement_target_counts: tuple[int, int, int] = (0, 0, 0)

    def stage_for_floor(self, floor: int) -> MirrorRouteStage | None:
        return next((stage for stage in self.stages if stage.contains(floor)), None)

    def gift_targets_for_floor(self, floor: int) -> tuple[GiftRouteTarget, ...]:
        return tuple(
            sorted(
                (gift for gift in self.gifts if gift.applies_to(floor)),
                key=lambda gift: gift.priority,
            )
        )

    def fusion_recipes_for_floor(self, floor: int) -> tuple[GiftFusionRecipe, ...]:
        return tuple(
            sorted(
                (recipe for recipe in self.recipes if recipe.applies_to(floor)),
                key=lambda recipe: recipe.priority,
            )
        )

    def gift_target(self, gift_id: str) -> GiftRouteTarget | None:
        return next((gift for gift in self.gifts if gift.gift_id == gift_id), None)

    def theme_pack_names_for_floor(self, floor: int) -> tuple[str, ...]:
        """Return the most precise theme-pack aliases for ``floor``."""

        logical_floor = max(1, int(floor))
        for start_floor, end_floor, names in self.floor_theme_pack_names:
            if start_floor <= logical_floor <= end_floor:
                return names
        stage = self.stage_for_floor(logical_floor)
        return stage.theme_pack_names if stage is not None else ()


_HOS_GIFTS = (
    GiftRouteTarget(
        "hoarfrost_footprint",
        ("霜冻足迹", "霜花足迹"),
        ("Hoarfrost Footprint",),
        1,
        5,
        5,
    ),
    GiftRouteTarget(
        "frozen_cries",
        ("冻结的哭声", "冰冻的哭声", "冰封的哀嚎"),
        ("Frozen Cries",),
        1,
        2,
        10,
    ),
    GiftRouteTarget(
        "haunted_shoes",
        ("闹鬼的鞋", "鬼鞋", "鬼附鞋"),
        ("Haunted Shoes",),
        1,
        2,
        20,
    ),
    GiftRouteTarget(
        "sharp_needle_and_thread",
        ("锋利的针线", "锐利的针线", "锐利的针与线"),
        ("Sharp Needle & Thread", "Sharp Needle and Thread"),
        1,
        5,
        40,
        required=True,
    ),
    GiftRouteTarget(
        "tango_marinade",
        ("探戈腌料", "探戈鸡酱料"),
        ("Tango Marinade",),
        1,
        5,
        50,
    ),
    GiftRouteTarget(
        "hot_n_juicy_drumstick",
        ("又热又多汁的鸡腿", "热辣多汁的鸡腿", "火热多汁琵琶腿"),
        ("Hot ’n Juicy Drumstick", "Hot 'n Juicy Drumstick"),
        1,
        5,
        60,
    ),
    GiftRouteTarget(
        "contaminated_needle_and_thread",
        ("被污染的针线", "污染的针线", "脏污的针与线"),
        ("Contaminated Needle & Thread", "Contaminated Needle and Thread"),
        1,
        5,
        70,
    ),
    GiftRouteTarget(
        "spicebush_branch",
        ("香料灌木枝", "香料灌木的枝条", "山茶花枝"),
        ("Spicebush Branch",),
        3,
        5,
        80,
    ),
    GiftRouteTarget(
        "chief_butler_secret_arts",
        ("首席管家的秘籍",),
        ("Chief Butler's Secret Arts", "Chief Butler’s Secret Arts"),
        3,
        5,
        82,
    ),
    GiftRouteTarget(
        "handheld_mirror",
        ("手镜",),
        ("Handheld Mirror",),
        3,
        5,
        84,
    ),
    GiftRouteTarget(
        "ragged_umbrella",
        ("破旧的雨伞", "破旧雨伞", "破损的雨伞"),
        ("Ragged Umbrella",),
        3,
        5,
        85,
    ),
    GiftRouteTarget(
        "gear_shrapnel",
        ("齿轮碎片",),
        ("Gear Shrapnel",),
        4,
        5,
        90,
    ),
    GiftRouteTarget(
        "darkflame_smoking_pipe",
        ("暗焰烟斗", "黑焰烟斗"),
        ("Darkflame Smoking Pipe",),
        4,
        5,
        100,
    ),
    GiftRouteTarget(
        "implicit_contract_renewal",
        ("默示契约续订", "隐性契约续订", "默示契约更新"),
        ("Implicit Contract Renewal",),
        4,
        5,
        110,
    ),
    GiftRouteTarget(
        "broken_glasses",
        ("破碎的眼镜",),
        ("Broken Glasses",),
        5,
        5,
        120,
    ),
    GiftRouteTarget(
        "unmailed_letter",
        ("未寄出的信",),
        ("Unmailed Letter",),
        5,
        5,
        130,
    ),
    GiftRouteTarget(
        "spicebush_glasses_mailed_letter",
        ("香料灌木、眼镜与寄出的信", "香料灌木、眼镜和寄出的信", "山茶花、眼镜和送达的信"),
        ("Spicebush, Glasses, and Mailed Letter",),
        3,
        5,
        135,
    ),
    GiftRouteTarget(
        "bridle",
        ("马勒", "缰绳", "枷锁"),
        ("Bridle",),
        6,
        10,
        140,
        required=True,
    ),
    GiftRouteTarget(
        "false_halo",
        ("虚假的光环", "虚假的光相"),
        ("False Halo",),
        6,
        10,
        150,
    ),
    GiftRouteTarget(
        "snake_slough",
        ("蛇蜕",),
        ("Snake Slough",),
        6,
        10,
        160,
    ),
    GiftRouteTarget(
        "jolly_plushie",
        ("欢乐的毛绒玩具", "快乐玩偶", "快乐的毛绒玩偶"),
        ("Jolly Plushie",),
        6,
        10,
        170,
    ),
    GiftRouteTarget(
        "shadow_monster",
        ("影子怪物",),
        ("Shadow Monster",),
        6,
        10,
        171,
    ),
    GiftRouteTarget(
        "gift",
        ("礼物",),
        ("Gift",),
        6,
        10,
        172,
    ),
    GiftRouteTarget(
        "pom_pom_hat",
        ("毛球帽",),
        ("Pom-pom Hat", "Pom Pom Hat"),
        6,
        10,
        173,
    ),
    GiftRouteTarget(
        "huge_gift_sack",
        ("巨大的礼物袋",),
        ("Huge Gift Sack",),
        6,
        10,
        174,
    ),
    GiftRouteTarget(
        "sad_plushie",
        ("悲伤的毛绒玩偶",),
        ("Sad Plushie",),
        6,
        10,
        175,
    ),
    GiftRouteTarget(
        "snuffed_lantern",
        ("熄灭的提灯",),
        ("Snuffed Lantern",),
        6,
        10,
        176,
    ),
    GiftRouteTarget(
        "snuffed_candlestick",
        ("熄灭的烛台",),
        ("Snuffed Candlestick",),
        6,
        10,
        177,
    ),
    GiftRouteTarget(
        "packaging_box",
        ("包装盒",),
        ("Packaging Box",),
        6,
        10,
        178,
    ),
    GiftRouteTarget(
        "packaging_ribbon",
        ("包装缎带",),
        ("Packaging Ribbon",),
        6,
        10,
        179,
    ),
    GiftRouteTarget(
        "emergency_investigator_badge",
        ("紧急调查员徽章", "紧急授予型搜查官徽章"),
        ("Emergency Investigator Badge",),
        6,
        10,
        180,
    ),
    GiftRouteTarget(
        "piece_of_relationship",
        ("关系的一部分", "缘分残片"),
        ("Piece of Relationship",),
        6,
        10,
        190,
    ),
    GiftRouteTarget(
        "warning_notice",
        ("警告通知", "警告函"),
        ("Warning Notice",),
        6,
        10,
        195,
    ),
    GiftRouteTarget(
        "prepaid_time_receipt",
        ("预付时间收据", "预支时间收据"),
        ("Prepaid Time Receipt",),
        6,
        10,
        196,
    ),
    GiftRouteTarget(
        "silver_watch_case",
        ("银色表壳", "银色的表壳"),
        ("Silver Watch Case",),
        6,
        10,
        186,
    ),
    GiftRouteTarget(
        "faded_watch_case",
        ("褪色的表壳",),
        ("Faded Watch Case",),
        6,
        10,
        187,
    ),
    GiftRouteTarget(
        "etched_clock_hands",
        ("刻蚀的时针", "蚀刻的时针"),
        ("Etched Clock Hands",),
        6,
        10,
        188,
    ),
    GiftRouteTarget(
        "rusted_clock_hands",
        ("生锈的时针",),
        ("Rusted Clock Hands",),
        6,
        10,
        189,
    ),
    GiftRouteTarget(
        "entanglement_override_sequencer",
        ("纠缠覆盖序列器",),
        ("Entanglement Override Sequencer",),
        6,
        10,
        197,
    ),
    GiftRouteTarget(
        "lunar_memory",
        ("月之记忆",),
        ("Lunar Memory",),
        6,
        10,
        200,
        asset="mirror/shop/level_IV_gifts/lunar_memory.png",
    ),
    GiftRouteTarget(
        "metronome",
        ("节拍器",),
        ("Metronome",),
        6,
        10,
        205,
    ),
    GiftRouteTarget(
        "bongy_plush",
        ("Bongy 毛绒玩具", "Bongy毛绒玩具", "小凤玩偶"),
        ("Bongy Plush",),
        6,
        10,
        206,
    ),
    GiftRouteTarget(
        "red_necktie",
        ("红领带",),
        ("Red Necktie",),
        6,
        10,
        207,
    ),
    GiftRouteTarget(
        "searing_brass",
        ("灼热黄铜", "灼热的铜管"),
        ("Searing Brass",),
        6,
        10,
        208,
    ),
    GiftRouteTarget(
        "entangled_fate",
        ("纠缠的命运", "纠缠的缘分"),
        ("Entangled Fate",),
        6,
        10,
        209,
    ),
    GiftRouteTarget(
        "mid_range_k_corp_ampule",
        ("中程 K 公司安瓿", "中程K公司安瓿", "量产型K公司安瓿"),
        ("Mid-range K Corp. Ampule", "Mid-range K Corp Ampule"),
        11,
        15,
        210,
        required=True,
    ),
    GiftRouteTarget(
        "contempt_of_the_gaze_of_contempt",
        ("蔑视之眼的蔑视", "轻蔑的视线的轻蔑"),
        ("Contempt of the Gaze of Contempt",),
        11,
        15,
        211,
    ),
    GiftRouteTarget(
        "unhatched_embers",
        ("未孵化的余烬", "未孵化的火种"),
        ("Unhatched Embers",),
        11,
        15,
        220,
    ),
    GiftRouteTarget(
        "for_the_capo",
        ("献给首领", "给首领", "为了指挥官"),
        ("For the Capo",),
        11,
        15,
        230,
    ),
    GiftRouteTarget(
        "kkomis_mini_gift",
        ("Kkomi 的小礼物", "Kkomi的小礼物", "可米的小小礼物"),
        ("Kkomi’s Mini-Gift", "Kkomi's Mini-Gift"),
        11,
        15,
        240,
    ),
    GiftRouteTarget(
        "shatterbound_cannon",
        ("碎裂束缚之炮", "注定破碎的火炮"),
        ("Shatterbound Cannon",),
        11,
        15,
        250,
    ),
    GiftRouteTarget(
        "coveting_thorn",
        ("觊觎之刺", "贪欲之棘"),
        ("Coveting Thorn",),
        11,
        15,
        260,
    ),
    # Safe alternatives listed by the guide.  They remain lower priority
    # than the preferred floor-11 gifts but are still protected when found.
    GiftRouteTarget(
        "bridle_of_infinity",
        ("无限的枷锁",),
        ("Bridle of Infinity",),
        11,
        15,
        270,
    ),
    GiftRouteTarget(
        "bloodtinged_ichthyic_odor",
        ("血染的鱼腥味",),
        ("Bloodtinged Ichthyic Odor",),
        11,
        15,
        271,
    ),
    GiftRouteTarget(
        "into_certain_library_book",
        ("某个图书馆的书",),
        ("Into a Certain Library's Book", "Into a Certain Library’s Book"),
        11,
        15,
        272,
    ),
)

_HOS_RECIPES = (
    GiftFusionRecipe(
        result_gift_id="hoarfrost_footprint",
        result_names_zh=("霜冻足迹", "霜花足迹"),
        result_names_en=("Hoarfrost Footprint",),
        material_gift_ids=("haunted_shoes", "frozen_cries"),
        start_floor=1,
        end_floor=5,
        priority=10,
        keyword="sinking",
    ),
    GiftFusionRecipe(
        result_gift_id="unmailed_letter",
        result_names_zh=("未寄出的信",),
        result_names_en=("Unmailed Letter",),
        material_gift_ids=("ragged_umbrella", "broken_glasses"),
        start_floor=5,
        end_floor=15,
        priority=20,
        keyword="rupture",
    ),
    GiftFusionRecipe(
        result_gift_id="spicebush_glasses_mailed_letter",
        result_names_zh=(
            "香料灌木、眼镜与寄出的信",
            "香料灌木、眼镜和寄出的信",
            "山茶花、眼镜和送达的信",
        ),
        result_names_en=(
            "Spicebush, Glasses, and Mailed Letter",
            "Spicebush, Glasses and Mailed Letter",
        ),
        material_gift_ids=("spicebush_branch", "broken_glasses", "unmailed_letter"),
        start_floor=5,
        end_floor=15,
        priority=30,
        keyword="rupture",
        skip_if_pseudo_solo=True,
    ),
)

# The guide names a concrete theme-pack choice for floors 1–5 and a set of
# interchangeable choices for floors 6–10 and 11–15.  Keep these windows
# separate from the legacy three-stage bounds above: the former are used for
# exact runtime boosting, while the latter remain a useful public summary of
# the route's progression.
_HOS_FLOOR_THEME_PACKS = (
    (1, 1, ("The Unloving", "无法去爱", "无慈悲", "unloving")),
    (2, 2, ("Hell's Chicken", "地狱鸡", "chick")),
    (
        3,
        3,
        ("Falling Flowers", "落花", "flowers", "A Certain World", "某个世界", "certain"),
    ),
    (4, 4, ("Full-Stopped by a Bullet", "由子弹画下的句点", "句点", "bullet")),
    (5, 5, ("The Unchanging", "无改变", "unchanging")),
    (
        6,
        10,
        (
            "Line 2",
            "2号线",
            "Line 1",
            "1号线",
            "line",
            "Miracle in District 20 BokGak",
            "区的奇",
            "miracle",
            "Timekilling Time BokGak",
            "时间杀人",
            "time",
            "Textbook",
            "教材",
            "Text",
        ),
    ),
    (
        11,
        15,
        (
            "Code Purple",
            "紫色编码",
            "codepurple",
            "violet",
            "Line 3",
            "3号线",
            "line",
            "Chachihu",
            "chachihu",
            "A Midspring Night's Dream 2",
            "仲春之夜",
            "Textbook",
            "教材",
            "Text",
            "Line 5",
            "5号线",
            "Bridle of Infinity",
            "Bloodtinged Ichthyic Odor",
            "Into a Certain Library's Book",
        ),
    ),
)


HOS_RYOSHU_SOLO_ROUTE = MirrorRouteDefinition(
    route_id="hos_ryoshu_solo_route",
    name_zh="House of Spiders 良秀伪单通路线",
    name_en="House of Spiders Ryoshu route",
    summary_zh="按 1–5、6–10、11–15 层阶段优先过程饰品，并保护路线目标。",
    summary_en="Prioritize and protect process gifts by the 1–5, 6–10, and 11–15 floor stages.",
    stages=(
        MirrorRouteStage(
            1,
            5,
            (
                "无慈悲",
                "无法去爱",
                "无作为",
                "unloving",
                "信仰与侵蚀",
                "信仰",
                "faith",
                "地狱鸡",
                "chick",
                "落花",
                "Falling Flowers",
                "flowers",
                "某个世界",
                "certain",
                "憎恶与绝望",
                "Hatred",
                "由子弹画下的句点",
                "句点",
                "Full-Stopped by a Bullet",
                "bullet",
                "无改变",
                "unchanging",
                "LCB体检复刻",
                "体检",
                "check",
                "区的奇",
                "Miracle in District 20 BokGak",
                "miracle",
            ),
            tuple(gift.gift_id for gift in _HOS_GIFTS if gift.start_floor <= 5 and gift.end_floor >= 1),
        ),
        MirrorRouteStage(
            6,
            10,
            (
                "肉斩骨断复刻",
                "骨断",
                "bones",
                "1号线",
                "Line 1",
                "line1",
                "2号线",
                "Line 2",
                "line2",
                "经验记忆",
                "经验",
                "Experience Memory",
                "mnestic",
                "深夜清扫复刻",
                "清扫",
                "Nocturnal Sweeping BokGak",
                "sweep",
                "时间杀人",
                "Timekilling Time BokGak",
                "time",
                "教材",
                "Textbook",
                "Text",
            ),
            tuple(gift.gift_id for gift in _HOS_GIFTS if gift.start_floor <= 10 and gift.end_floor >= 6),
        ),
        MirrorRouteStage(
            11,
            15,
            (
                "Code Purple",
                "codepurple",
                "紫色编码",
                "紫罗兰",
                "violet",
                "隐藏Boss",
                "Theb",
                "插翅虎",
                "家族",
                "four",
                "3号线",
                "Line 3",
                "line3",
                "5号线",
                "Line 5",
                "line5",
                "Chachihu",
                "A Midspring Night's Dream 2",
                "仲春之夜",
                "教材",
                "Text",
                "Bridle of Infinity",
                "Bloodtinged Ichthyic Odor",
                "Into a Certain Library's Book",
            ),
            tuple(gift.gift_id for gift in _HOS_GIFTS if gift.start_floor <= 15 and gift.end_floor >= 11),
        ),
    ),
    gifts=_HOS_GIFTS,
    recipes=_HOS_RECIPES,
    floor_counts=(5, 15),
    floor_theme_pack_names=_HOS_FLOOR_THEME_PACKS,
    skill_replacement_target_counts=(1, 1, 4),
)

SPIDERWEB_FAMILY_ROUTE = MirrorRouteDefinition(
    route_id="spiderweb_family_route",
    name_zh="蜘蛛巢默认路线",
    name_en="Spiderweb default route",
    summary_zh="保持现有镜牢饰品逻辑，仅使用蜘蛛巢专属 Gift Search。",
    summary_en="Keep the existing mirror-gift behavior and Spiderweb-exclusive Gift Search.",
)

DEFAULT_ROUTE = MirrorRouteDefinition(
    route_id="",
    name_zh="默认饰品路线",
    name_en="Default gift route",
    summary_zh="使用现有饰品选择、商店和合成逻辑。",
    summary_en="Use the existing gift, shop, and fusion behavior.",
)

ROUTES: dict[str, MirrorRouteDefinition] = {
    route.route_id: route for route in (HOS_RYOSHU_SOLO_ROUTE, SPIDERWEB_FAMILY_ROUTE, DEFAULT_ROUTE)
}

# These aliases cover the first development shape of the built-in catalog and
# make hand-edited configs harmless when the preset id and route id become
# intentionally independent.
ROUTE_ALIASES: dict[str, MirrorRouteDefinition] = {
    "hos_ryoshu_solo": HOS_RYOSHU_SOLO_ROUTE,
    "spiderweb_family": SPIDERWEB_FAMILY_ROUTE,
}


def get_mirror_route(route_id: str | None) -> MirrorRouteDefinition:
    """Resolve an unknown/empty route to the backwards-compatible default."""

    normalized_id = (route_id or "").strip()
    return ROUTES.get(normalized_id, ROUTE_ALIASES.get(normalized_id, DEFAULT_ROUTE))


def route_ids() -> Iterable[str]:
    return ROUTES.keys()
