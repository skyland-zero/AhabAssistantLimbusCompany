// The mirror-team configuration model and the labels its editor shows.
//
// The field names are the contract's (`TeamMirrorConfig` in
// `native/crates/model/src/teams.rs` and `contract/contract.json`), so a future
// `Teams.*` RPC can replace the fixtures without renaming anything. Defaults
// and label tables are ported 1:1 from the GPUI client's `model/teams.rs` and
// `pages/teams/shared.rs`.

/** The ten gift systems, in the order the client draws them. */
export const SYSTEM_NAMES = [
  "burn",
  "bleed",
  "tremor",
  "rupture",
  "sinking",
  "poise",
  "charge",
  "slash",
  "pierce",
  "blunt",
] as const;

export type SystemName = (typeof SYSTEM_NAMES)[number];

/** The system as the page labels it. */
export const SYSTEM_LABELS: Record<SystemName, string> = {
  burn: "燃烧",
  bleed: "流血",
  tremor: "震颤",
  rupture: "破裂",
  sinking: "沉沦",
  poise: "呼吸",
  charge: "充能",
  slash: "斩击",
  pierce: "突刺",
  blunt: "打击",
};

/** Which systems a shop pass may not buy, fuse or keep. */
export type DiscardSystems = {
  burn: boolean;
  bleed: boolean;
  tremor: boolean;
  rupture: boolean;
  sinking: boolean;
  poise: boolean;
  charge: boolean;
  slash: boolean;
  pierce: boolean;
  blunt: boolean;
};

/** One team's mirror-run configuration (the contract's `TeamMirrorConfig`). */
export type TeamMirrorConfig = {
  team_system: number;
  shop_strategy: number;
  reward_cards: boolean;
  reward_cards_select: number;
  shopping_strategy: boolean;
  shopping_strategy_select: number;
  opening_items: boolean;
  opening_items_select: number;
  opening_items_system: number;
  discard_systems: DiscardSystems;
  do_not_heal: boolean;
  do_not_buy: boolean;
  do_not_fuse: boolean;
  do_not_sell: boolean;
  do_not_enhance: boolean;
  only_aggressive_fuse: boolean;
  do_not_system_fuse: boolean;
  only_system_fuse: boolean;
  aggressive_also_enhance: boolean;
  aggressive_save_systems: boolean;
  after_level_IV: boolean;
  after_level_IV_select: number;
  ignore_shop: boolean[];
  max_keyword_refresh: number;
  max_normal_refresh: number;
  second_system: boolean;
  second_system_select: number;
  second_system_setting: number;
  second_system_fuse_IV: boolean;
  second_system_buy: boolean;
  second_system_select_reward: boolean;
  second_system_power_up: boolean;
  avoid_skill_3: boolean;
  prioritize_skill_3: boolean;
  use_damage_p: boolean;
  re_formation_each_floor: boolean;
  defense_first_round: boolean;
  defense_for_solo: boolean;
  defense_for_solo_turns: number;
  skill_replacement: boolean;
  skill_replacement_select: number;
  skill_replacement_mode: number;
  use_starlight: boolean;
  opening_bonus: number[];
  fixed_team_use: boolean;
  fixed_team_use_select: number;
  use_team_code: boolean;
  team_code: string;
  use_custom_theme_pack_weight: boolean;
  observe_ego_gift: boolean;
  observe_ego_gift_selected: string[];
  mirror_route_profile: string;
};

/** The ten opening bonuses, their costs and their descriptions. */
export const STARLIGHT_NAMES = [
  "初始之星",
  "积聚的星云",
  "星际漫游",
  "流星",
  "双星商店",
  "卫星商店",
  "星云的宠爱",
  "星芒的引导",
  "偶然的彗星",
  "全面的可能性",
] as const;

export const STARLIGHT_COSTS = [10, 10, 20, 20, 30, 30, 40, 40, 50, 60] as const;

export const STARLIGHT_DESCRIPTIONS = [
  "初始经费增加，卡包/饰品展出+1，免费普通刷新",
  "进阶经费利息+10%~30%，售卖饰品经费加成",
  "卡包出现+1，卡包刷新+2~4，未记录卡包等级提升",
  "初始经费+400~700，初始饰品可选择数+1",
  "展出饰品+1，战斗经费+20%~40%，高阶饰品概率提升",
  "免费关键词刷新，进入第1层送1~3件1级饰品",
  "进入第1层人格等级+3，通关阶段人格等级提升",
  "最大速度+2~3，拼点威力、伤害强化和守护提升",
  "进商店赠送合成/售卖专用饰品，赠送对应关键词饰品",
  "开局自选3级饰品，获得残影饰品",
] as const;

/** The one gift id the advanced tab names instead of echoing. */
export const SPIDERWEB_GIFT_ID = "spiderweb_entangled_in_red";

/** The mirror routes the team editor can assign. Keep the ids in sync with
 *  `module/mirror_routes.py`; the labels are UI copy. */
export const ROUTE_OPTIONS: { value: string; label: string }[] = [
  { value: "", label: "默认饰品路线" },
  { value: "hos_ryoshu_solo_route", label: "House of Spiders 良秀伪单通" },
  { value: "spiderweb_family_route", label: "蜘蛛巢默认路线" },
  { value: "spiderweb_family_normal_farm_route", label: "蜘蛛巢普牢速刷路线" },
];

/** The label for a route id, or the raw id when a config names a newer route. */
export function routeLabel(route: string | null | undefined): string {
  if (!route) return "默认饰品路线";
  return ROUTE_OPTIONS.find((option) => option.value === route)?.label ?? route;
}

/** A fresh config: the Rust `Default`, with the team's system set by the caller. */
export function defaultMirrorConfig(
  change: Partial<TeamMirrorConfig> = {},
): TeamMirrorConfig {
  return {
    team_system: 0,
    shop_strategy: 0,
    reward_cards: false,
    reward_cards_select: 0,
    shopping_strategy: false,
    shopping_strategy_select: 0,
    opening_items: false,
    opening_items_select: 0,
    opening_items_system: 0,
    discard_systems: {
      burn: false,
      bleed: false,
      tremor: false,
      rupture: false,
      sinking: false,
      poise: false,
      charge: false,
      slash: false,
      pierce: false,
      blunt: false,
    },
    do_not_heal: false,
    do_not_buy: false,
    do_not_fuse: false,
    do_not_sell: false,
    do_not_enhance: false,
    only_aggressive_fuse: false,
    do_not_system_fuse: false,
    only_system_fuse: false,
    aggressive_also_enhance: false,
    aggressive_save_systems: false,
    after_level_IV: false,
    after_level_IV_select: 0,
    ignore_shop: [false, false, false, false, false],
    max_keyword_refresh: 1,
    max_normal_refresh: 1,
    second_system: false,
    second_system_select: 0,
    second_system_setting: 0,
    second_system_fuse_IV: false,
    second_system_buy: false,
    second_system_select_reward: false,
    second_system_power_up: false,
    avoid_skill_3: false,
    prioritize_skill_3: false,
    use_damage_p: false,
    re_formation_each_floor: false,
    defense_first_round: false,
    defense_for_solo: false,
    defense_for_solo_turns: 5,
    skill_replacement: false,
    skill_replacement_select: 0,
    skill_replacement_mode: 0,
    use_starlight: false,
    opening_bonus: [1, 1, 1, 1, 0, 0, 0, 0, 0, 0],
    fixed_team_use: false,
    fixed_team_use_select: 0,
    use_team_code: false,
    team_code: "",
    use_custom_theme_pack_weight: false,
    observe_ego_gift: false,
    observe_ego_gift_selected: [],
    mirror_route_profile: "",
    ...change,
  };
}

/** The system's index in `SYSTEM_NAMES`, or 0 for an unknown id. */
export function systemIndex(scheme: string): number {
  const index = (SYSTEM_NAMES as readonly string[]).indexOf(scheme);
  return index < 0 ? 0 : index;
}

/** The opening starlight the config spends: each level multiplies its cost. */
export function starlightCost(config: TeamMirrorConfig | null): number {
  if (config === null) return 0;
  return config.opening_bonus
    .slice(0, STARLIGHT_COSTS.length)
    .reduce((sum, level, index) => sum + STARLIGHT_COSTS[index] * level, 0);
}

/** Which systems the config discards. */
export function discardCount(config: TeamMirrorConfig | null): number {
  if (config === null) return 0;
  return SYSTEM_NAMES.filter((name) => config.discard_systems[name]).length;
}

/** Whether the config carries opening bonuses into the run. */
export function hasStarlight(config: TeamMirrorConfig | null): boolean {
  return config !== null && config.opening_bonus.some((level) => level > 0);
}

/** Clamps a value the way the client's editor does before it saves. */
const LIMITS: Partial<Record<keyof TeamMirrorConfig, [number, number]>> = {
  shop_strategy: [0, 2],
  reward_cards_select: [0, 3],
  shopping_strategy_select: [0, 5],
  opening_items_select: [0, 5],
  opening_items_system: [0, 9],
  after_level_IV_select: [0, 3],
  max_keyword_refresh: [0, 10],
  max_normal_refresh: [0, 10],
  second_system_select: [0, 9],
  second_system_setting: [0, 1],
  defense_for_solo_turns: [1, 5],
  skill_replacement_select: [0, 3],
  skill_replacement_mode: [0, 4],
  fixed_team_use_select: [0, 2],
};

/** Puts `value` inside the contract's range for `key`. */
export function clampMirrorNumber(key: keyof TeamMirrorConfig, value: number): number {
  const [min, max] = LIMITS[key] ?? [0, 255];
  if (!Number.isFinite(value)) return min;
  return Math.min(max, Math.max(min, Math.round(value)));
}

// ---------------------------------------------------------------------------
// Labels the editor shows for the numeric choices.

export function shopStrategyLabel(value: number): string {
  return ["默认", "保守", "激进"][value] ?? "默认";
}

export function afterLevelLabel(value: number): string {
  return ["停止", "继续", "升级", "跳过商店"][value] ?? "停止";
}

export function fixedTeamUseLabel(value: number): string {
  return ["困难专用", "普通专用", "全部通用"][value] ?? "困难专用";
}

export function secondSystemSettingLabel(value: number): string {
  return value === 1 ? "主体系四级后" : "主体系四级前";
}

export function secondSystemFloorLabel(value: number): string {
  return value === 1 ? "第二层开始" : "第一层开始";
}

export function rewardCardsLabel(value: number): string {
  return (
    [
      "星光→饰品→钱→饰品/钱→罪孽",
      "星光→钱→饰品→饰品/钱→罪孽",
      "钱→饰品→饰品/钱→罪孽→星光",
      "饰品→钱→饰品/钱→罪孽→星光",
    ][value] ?? "星光→饰品→钱→饰品/钱→罪孽"
  );
}

export function shoppingStrategyLabel(value: number): string {
  return (
    [
      "仅购买回血饰品",
      "四级优先时：仅购买所有 3-4 级饰品",
      "仅购买体系饰品",
      "不购买体系饰品",
      "不购买回血饰品",
      "四级优先时：不购买所有 3-4 级饰品",
    ][value] ?? "仅购买回血饰品"
  );
}

export function openingItemsOrderLabel(value: number): string {
  return ["1 → 2 → 3", "1 → 3 → 2", "2 → 1 → 3", "2 → 3 → 1", "3 → 1 → 2", "3 → 2 → 1"][
    value
  ] ?? "1 → 2 → 3";
}

export function skillReplacementSelectLabel(value: number): string {
  return ["配队首位", "配队前 3", "配队前 7", "所有选中的人"][value] ?? "配队首位";
}

export function skillReplacementModeLabel(value: number): string {
  return ["1 → 3", "2 → 3", "1 → 2", "全换 3（先 2→3）", "全换 3（先 1→3）"][value] ?? "1 → 3";
}

export function starlightLevelLabel(value: number): string {
  return ["全部关闭", "全部基础", "全部 2+", "全部 3++"][value] ?? "全部关闭";
}

export function starlightShortLevel(value: number): string {
  return ["0", "1", "2+", "3++"][value] ?? "0";
}

export function turnsLabel(value: number): string {
  return `${value} 回合`;
}

/** A gift id as the advanced tab names it. */
export function observeGiftLabel(gift: string): string {
  return gift === SPIDERWEB_GIFT_ID ? "赤红纠缠的蜘蛛巢" : gift;
}
