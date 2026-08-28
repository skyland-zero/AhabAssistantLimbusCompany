use super::*;

pub(super) const STARLIGHT_NAMES: [&str; 10] = [
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
];
pub(super) const STARLIGHT_COSTS: [u32; 10] = [10, 10, 20, 20, 30, 30, 40, 40, 50, 60];
pub(super) const STARLIGHT_DESCRIPTIONS: [&str; 10] = [
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
];
pub(super) const STARLIGHT_NAMES_EN: [&str; 10] = [
    "Initial Star",
    "Gathered Nebula",
    "Star Wanderer",
    "Meteor",
    "Binary Star Shop",
    "Satellite Shop",
    "Nebula's Favor",
    "Starlight Guide",
    "Chance Comet",
    "All Possibilities",
];
pub(super) const STARLIGHT_DESCRIPTIONS_EN: [&str; 10] = [
    "Increase starting cost, gift displays by 1, and grant a free normal refresh",
    "Increase advanced-cost interest and gift sale cost bonus",
    "Add one gift pack, refreshes, and unrecorded pack upgrades",
    "Increase starting cost and the number of starting gifts to choose",
    "Add one displayed gift and improve battle cost and high-tier odds",
    "Grant a free keyword refresh and 1-3 level-1 gifts on floor 1",
    "Raise sinner levels on floor 1 and after clearing stages",
    "Improve maximum speed, clash power, damage and protection",
    "Grant shop gifts for fusion/sales and matching keywords",
    "Choose a level-3 gift at start and receive a remnant gift",
];
pub(super) const SYSTEM_LABELS_EN: [&str; 10] = [
    "Burn", "Bleed", "Tremor", "Rupture", "Sinking", "Poise", "Charge", "Slash", "Pierce", "Blunt",
];

// Small inline vectors keep the controls recognisable without substituting
// text glyphs for the Lucide actions used by the React page.
pub(super) const ICON_PLUS: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M12 5v14M5 12h14"/></svg>"##;
pub(super) const ICON_EDIT: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 20h9"/><path d="M16.5 3.5a2.12 2.12 0 0 1 3 3L7 19l-4 1 1-4Z"/></svg>"##;
pub(super) const ICON_TRASH: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18"/><path d="M8 6V4h8v2"/><path d="M19 6v14H5V6"/><path d="M10 11v5M14 11v5"/></svg>"##;
pub(super) const ICON_COPY: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><rect x="9" y="9" width="11" height="11" rx="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>"##;
pub(super) const ICON_PASTE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/><rect x="8" y="2" width="8" height="4" rx="1"/><path d="m9 14 2 2 4-4"/></svg>"##;
pub(super) const ICON_CLOSE: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M6 6l12 12M18 6 6 18"/></svg>"##;
pub(super) const ICON_SPARKLES: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="m12 3-1.3 5.7L5 10l5.7 1.3L12 17l1.3-5.7L19 10l-5.7-1.3Z"/><path d="m19 16-.7 3.3L15 20l3.3.7L19 24l.7-3.3L23 20l-3.3-.7Z"/></svg>"##;
#[allow(dead_code)]
pub(super) const ICON_USERS: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2"/><circle cx="9" cy="7" r="4"/><path d="M22 21v-2a4 4 0 0 0-3-3.87M16 3.13a4 4 0 0 1 0 7.75"/></svg>"##;

pub(super) fn icon(data: &'static [u8], size: f32, color: ColorToken) -> impl IntoElement {
    svg_icon_bytes(data, size, palette_rgb(color))
}

#[allow(dead_code)]
pub(super) fn cycle_u8_value(current: u8, max: u8, direction: i8) -> u8 {
    if max == 0 {
        return 0;
    }
    let current = current.min(max) as i16;
    let max = i16::from(max);
    (current + i16::from(direction)).rem_euclid(max + 1) as u8
}

pub(super) fn system_label(index: usize, language: Language) -> &'static str {
    let index = index.min(SYSTEM_NAMES.len().saturating_sub(1));
    if matches!(language, Language::EnUs) {
        SYSTEM_LABELS_EN[index]
    } else {
        crate::state::SYSTEM_LABELS[index]
    }
}

pub(super) fn starlight_name(index: usize, language: Language) -> &'static str {
    if matches!(language, Language::EnUs) {
        STARLIGHT_NAMES_EN[index]
    } else {
        STARLIGHT_NAMES[index]
    }
}

pub(super) fn starlight_description(index: usize, language: Language) -> &'static str {
    if matches!(language, Language::EnUs) {
        STARLIGHT_DESCRIPTIONS_EN[index]
    } else {
        STARLIGHT_DESCRIPTIONS[index]
    }
}

pub(super) fn filter_label(filter: TeamFilter, language: Language) -> &'static str {
    match filter {
        TeamFilter::All => text("全部", "All").get(language),
        TeamFilter::Mirror => text("镜牢", "Mirror").get(language),
        TeamFilter::Luxcavation => text("经验本", "EXP Dungeon").get(language),
        TeamFilter::General => text("通用", "General").get(language),
    }
}

pub(super) fn purpose_label(purpose: TeamPurpose, language: Language) -> &'static str {
    match purpose {
        TeamPurpose::Mirror => text("镜牢", "Mirror").get(language),
        TeamPurpose::Luxcavation => text("经验本", "EXP Dungeon").get(language),
        TeamPurpose::General => text("通用", "General").get(language),
    }
}

pub(super) fn purpose_key(purpose: TeamPurpose) -> &'static str {
    match purpose {
        TeamPurpose::Mirror => "mirror",
        TeamPurpose::Luxcavation => "luxcavation",
        TeamPurpose::General => "general",
    }
}

pub(super) fn editor_tab_label(tab: TeamEditorTab, language: Language) -> &'static str {
    match tab {
        TeamEditorTab::Basic => text("基础编成", "Basic & Formation").get(language),
        TeamEditorTab::Shop => text("商店与合成", "Shop & Fusion").get(language),
        TeamEditorTab::Combat => text("二体系与战斗", "Second System & Combat").get(language),
        TeamEditorTab::Starlight => text("开局星光", "Starlight Bonus").get(language),
        TeamEditorTab::Advanced => text("观测与高级", "Observe & Advanced").get(language),
    }
}

pub(super) fn scheme_label(scheme: &str, language: Language) -> &'static str {
    let labels = [
        ("burn", text("燃烧", "Burn")),
        ("bleed", text("流血", "Bleed")),
        ("tremor", text("震颤", "Tremor")),
        ("rupture", text("破裂", "Rupture")),
        ("sinking", text("沉沦", "Sinking")),
        ("poise", text("呼吸", "Poise")),
        ("charge", text("充能", "Charge")),
        ("slash", text("斩击", "Slash")),
        ("pierce", text("突刺", "Pierce")),
        ("blunt", text("打击", "Blunt")),
    ];
    labels
        .iter()
        .find(|(id, _)| *id == scheme)
        .map(|(_, label)| label.get(language))
        .unwrap_or_else(|| text("燃烧", "Burn").get(language))
}

pub(super) fn shop_strategy_label(value: u8, language: Language) -> &'static str {
    match value {
        0 => text("默认", "Default").get(language),
        1 => text("保守", "Conservative").get(language),
        2 => text("激进", "Aggressive").get(language),
        _ => text("默认", "Default").get(language),
    }
}

pub(super) fn after_level_label(value: u8, language: Language) -> &'static str {
    match value {
        0 => text("停止", "Stop").get(language),
        1 => text("继续", "Continue").get(language),
        2 => text("升级", "Enhance").get(language),
        _ => text("停止", "Stop").get(language),
    }
}

pub(super) fn fixed_team_use_label(value: u8, language: Language) -> &'static str {
    match value {
        0 => text("困难专用", "Hard only").get(language),
        1 => text("普通专用", "Normal only").get(language),
        2 => text("全部通用", "All modes").get(language),
        _ => text("困难专用", "Hard only").get(language),
    }
}

pub(super) fn floor_label(floor: u8, language: Language) -> String {
    match language {
        Language::ZhCn => format!("第 {floor} 层"),
        Language::EnUs => format!("Floor {floor}"),
    }
}

pub(super) fn turns_label(turns: u8, language: Language) -> String {
    match language {
        Language::ZhCn => format!("{turns} 回合"),
        Language::EnUs => format!("{turns} turns"),
    }
}

pub(super) fn starlight_level_label(level: u8, language: Language) -> &'static str {
    match level {
        0 => text("全部关闭", "All off").get(language),
        1 => text("全部基础", "All base").get(language),
        2 => text("全部 2+", "All 2+").get(language),
        3 => text("全部 3++", "All 3++").get(language),
        _ => text("全部关闭", "All off").get(language),
    }
}

pub(super) fn starlight_short_level(level: u8) -> &'static str {
    match level {
        0 => "0",
        1 => "1",
        2 => "2+",
        3 => "3++",
        _ => "0",
    }
}

pub(super) fn starlight_cost_label(cost: u32, language: Language) -> String {
    match language {
        Language::ZhCn => format!("总消耗 {cost} 点"),
        Language::EnUs => format!("Total Starlight Cost: {cost}"),
    }
}

pub(super) fn starlight_points_label(cost: u32, language: Language) -> String {
    match language {
        Language::ZhCn => format!("{cost} 点"),
        Language::EnUs => format!("{cost} pts"),
    }
}

pub(super) fn localized_feedback(feedback: &str, language: Language) -> String {
    if matches!(language, Language::ZhCn) {
        return feedback.to_owned();
    }
    if let Some(detail) = feedback.strip_prefix("导入失败：") {
        return format!("Import failed: {detail}");
    }
    match feedback {
        "已导入队伍 JSON（尚未保存）" => "Team JSON imported (not saved yet)".to_owned(),
        "队伍 JSON 已复制" => "Team JSON copied".to_owned(),
        "队伍已保存" => "Team saved".to_owned(),
        "队伍已删除" => "Team deleted".to_owned(),
        "队伍名称不能为空" => "Team name is required".to_owned(),
        "队伍最多选择 12 名人格" => "A team can contain at most 12 sinners".to_owned(),
        "当前没有打开队伍编辑器" => "No team editor is open".to_owned(),
        "队伍 JSON 必须是对象" => "Team JSON must be an object".to_owned(),
        "队伍 JSON 缺少 name" => "Team JSON is missing name".to_owned(),
        "purpose 无效" => "Invalid team purpose".to_owned(),
        "sinners 无效" => "Invalid sinner list".to_owned(),
        "mirrorConfig 必须是对象" => "mirrorConfig must be an object".to_owned(),
        "mirrorConfig 默认值无效" => "Invalid mirrorConfig defaults".to_owned(),
        _ => feedback.to_owned(),
    }
}

pub(super) fn discard_value(systems: &crate::model::DiscardSystems, index: usize) -> bool {
    match index {
        0 => systems.burn,
        1 => systems.bleed,
        2 => systems.tremor,
        3 => systems.rupture,
        4 => systems.sinking,
        5 => systems.poise,
        6 => systems.charge,
        7 => systems.slash,
        8 => systems.pierce,
        9 => systems.blunt,
        _ => false,
    }
}

pub(super) fn discard_count(config: &TeamMirrorConfig) -> usize {
    (0..10)
        .filter(|index| discard_value(&config.discard_systems, *index))
        .count()
}

pub(super) fn normalized_scheme(scheme: &str) -> &str {
    SYSTEM_NAMES
        .iter()
        .copied()
        .find(|name| *name == scheme)
        .unwrap_or(SYSTEM_NAMES[0])
}

pub(super) fn sinner_path(id: &str) -> ImageSource {
    let asset = SinnerAsset::from_id(id).unwrap_or(SinnerAsset::DonQuixote);
    assets::image_source(Asset::Sinner(asset))
}

pub(super) fn status_effect_path(scheme: &str) -> ImageSource {
    let asset = StatusEffectAsset::from_id(scheme).unwrap_or(StatusEffectAsset::General);
    assets::image_source(Asset::StatusEffect(asset))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_labels_cover_contract_values() {
        assert_eq!(purpose_label(TeamPurpose::Mirror, Language::ZhCn), "镜牢");
        assert_eq!(scheme_label("poise", Language::ZhCn), "呼吸");
        assert_eq!(shop_strategy_label(2, Language::ZhCn), "激进");
        assert_eq!(after_level_label(1, Language::ZhCn), "继续");
        assert_eq!(fixed_team_use_label(2, Language::ZhCn), "全部通用");
    }

    #[test]
    fn unknown_scheme_falls_back_to_burn() {
        assert_eq!(scheme_label("unknown", Language::ZhCn), "燃烧");
        assert_eq!(discard_count(&TeamMirrorConfig::default()), 0);
    }

    #[test]
    fn dynamic_labels_follow_the_selected_language() {
        assert_eq!(starlight_cost_label(30, Language::ZhCn), "总消耗 30 点");
        assert_eq!(
            starlight_cost_label(30, Language::EnUs),
            "Total Starlight Cost: 30"
        );
        assert_eq!(starlight_points_label(10, Language::EnUs), "10 pts");
        assert_eq!(
            localized_feedback("队伍 JSON 已复制", Language::EnUs),
            "Team JSON copied"
        );
    }

    #[test]
    fn cycle_control_wraps_and_clamps_keyboard_values() {
        assert_eq!(cycle_u8_value(0, 2, -1), 2);
        assert_eq!(cycle_u8_value(2, 2, 1), 0);
        assert_eq!(cycle_u8_value(9, 2, -1), 1);
        assert_eq!(cycle_u8_value(1, 0, 1), 0);
    }
}
