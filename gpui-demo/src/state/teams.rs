use serde_json::{Value, json};

use crate::{
    ipc::{MockClient, contract::method},
    model::{SinnerInfo, TeamDetail, TeamMirrorConfig, TeamPurpose},
};

/// The five sections of the mirror-team editor.  Keeping this as a model enum
/// makes tab selection testable without constructing a GPUI window.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TeamEditorTab {
    #[default]
    Basic,
    Shop,
    Combat,
    Starlight,
    Advanced,
}

impl TeamEditorTab {
    pub const ALL: [Self; 5] = [
        Self::Basic,
        Self::Shop,
        Self::Combat,
        Self::Starlight,
        Self::Advanced,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Basic => "基础编成",
            Self::Shop => "商店与合成",
            Self::Combat => "二体系与战斗",
            Self::Starlight => "开局星光",
            Self::Advanced => "观测与高级",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TeamFilter {
    #[default]
    All,
    Mirror,
    Luxcavation,
    General,
}

impl TeamFilter {
    pub const ALL: [Self; 4] = [Self::All, Self::Mirror, Self::Luxcavation, Self::General];

    pub const fn label(self) -> &'static str {
        match self {
            Self::All => "全部",
            Self::Mirror => "镜牢",
            Self::Luxcavation => "经验本",
            Self::General => "通用",
        }
    }

    pub const fn purpose(self) -> Option<TeamPurpose> {
        match self {
            Self::All => None,
            Self::Mirror => Some(TeamPurpose::Mirror),
            Self::Luxcavation => Some(TeamPurpose::Luxcavation),
            Self::General => Some(TeamPurpose::General),
        }
    }
}

pub struct TeamEditorState {
    pub team: TeamDetail,
    pub tab: TeamEditorTab,
    pub json_import_open: bool,
}

impl TeamEditorState {
    pub fn new(team: TeamDetail) -> Self {
        Self {
            team,
            tab: TeamEditorTab::Basic,
            json_import_open: false,
        }
    }

    pub fn mirror_config(&self) -> TeamMirrorConfig {
        self.team.mirrorConfig.clone().unwrap_or_default()
    }
}

/// State and RPC boundary for the Teams page.  The page only mutates this
/// object and calls `cx.notify`; all contract serialization remains here.
pub struct TeamsState {
    pub client: MockClient,
    pub teams: Vec<TeamDetail>,
    pub sinners: Vec<SinnerInfo>,
    pub filter: TeamFilter,
    pub editor: Option<TeamEditorState>,
    pub delete_target: Option<TeamDetail>,
    pub feedback: Option<String>,
}

impl Default for TeamsState {
    fn default() -> Self {
        Self::new()
    }
}

impl TeamsState {
    pub fn new() -> Self {
        Self::with_client(MockClient::default())
    }

    pub fn with_client(client: MockClient) -> Self {
        let mut state = Self {
            client,
            teams: Vec::new(),
            sinners: Vec::new(),
            filter: TeamFilter::All,
            editor: None,
            delete_target: None,
            feedback: None,
        };
        state.reload();
        state
    }

    pub fn reload(&mut self) {
        self.teams = self
            .request_value(method::TEAM_LIST, None)
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default();
        self.sinners = self
            .request_value(method::SINNER_LIST, None)
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default();
    }

    pub fn filtered_teams(&self) -> impl Iterator<Item = &TeamDetail> {
        let purpose = self.filter.purpose();
        self.teams
            .iter()
            .filter(move |team| purpose.is_none() || Some(team.purpose) == purpose)
    }

    pub fn count_for(&self, filter: TeamFilter) -> usize {
        match filter.purpose() {
            None => self.teams.len(),
            Some(purpose) => self
                .teams
                .iter()
                .filter(|team| team.purpose == purpose)
                .count(),
        }
    }

    pub fn sinner_name(&self, id: &str) -> String {
        self.sinners
            .iter()
            .find(|sinner| sinner.id == id)
            .map(|sinner| sinner.name.clone())
            .unwrap_or_else(|| id.to_owned())
    }

    pub fn open_new(&mut self) {
        let purpose = self.filter.purpose().unwrap_or(TeamPurpose::General);
        self.editor = Some(TeamEditorState::new(TeamDetail {
            id: String::new(),
            name: String::new(),
            sinners: Vec::new(),
            purpose,
            accessoryScheme: "burn".into(),
            enabled: true,
            mirrorConfig: Some(TeamMirrorConfig::default()),
        }));
        self.feedback = None;
    }

    pub fn open_edit(&mut self, team: &TeamDetail) {
        self.editor = Some(TeamEditorState::new(team.clone()));
        self.feedback = None;
    }

    pub fn close_editor(&mut self) {
        self.editor = None;
    }

    pub fn set_editor_tab(&mut self, tab: TeamEditorTab) {
        if let Some(editor) = self.editor.as_mut() {
            editor.tab = tab;
        }
    }

    pub fn set_filter(&mut self, filter: TeamFilter) {
        self.filter = filter;
    }

    pub fn set_editor_purpose(&mut self, purpose: TeamPurpose) {
        if let Some(editor) = self.editor.as_mut() {
            editor.team.purpose = purpose;
        }
    }

    pub fn set_editor_scheme(&mut self, scheme: impl Into<String>, system: u8) {
        if let Some(editor) = self.editor.as_mut() {
            editor.team.accessoryScheme = scheme.into();
            editor
                .team
                .mirrorConfig
                .get_or_insert_with(Default::default)
                .team_system = system;
        }
    }

    pub fn set_editor_enabled(&mut self, enabled: bool) {
        if let Some(editor) = self.editor.as_mut() {
            editor.team.enabled = enabled;
        }
    }

    pub fn toggle_sinner(&mut self, id: &str) {
        let Some(editor) = self.editor.as_mut() else {
            return;
        };
        if let Some(index) = editor
            .team
            .sinners
            .iter()
            .position(|selected| selected == id)
        {
            editor.team.sinners.remove(index);
        } else if editor.team.sinners.len() < 12 {
            editor.team.sinners.push(id.to_owned());
        }
    }

    pub fn clear_sinners(&mut self) {
        if let Some(editor) = self.editor.as_mut() {
            editor.team.sinners.clear();
        }
    }

    pub fn update_mirror(&mut self, update: impl FnOnce(&mut TeamMirrorConfig)) {
        if let Some(editor) = self.editor.as_mut() {
            update(
                editor
                    .team
                    .mirrorConfig
                    .get_or_insert_with(Default::default),
            );
        }
    }

    pub fn toggle_discard_system(&mut self, index: usize) {
        self.update_mirror(|config| {
            let systems = &mut config.discard_systems;
            match index {
                0 => systems.burn = !systems.burn,
                1 => systems.bleed = !systems.bleed,
                2 => systems.tremor = !systems.tremor,
                3 => systems.rupture = !systems.rupture,
                4 => systems.sinking = !systems.sinking,
                5 => systems.poise = !systems.poise,
                6 => systems.charge = !systems.charge,
                7 => systems.slash = !systems.slash,
                8 => systems.pierce = !systems.pierce,
                9 => systems.blunt = !systems.blunt,
                _ => {}
            }
        });
    }

    pub fn set_mirror_bool(&mut self, field: MirrorBool, value: bool) {
        self.update_mirror(|config| match field {
            MirrorBool::DoNotHeal => config.do_not_heal = value,
            MirrorBool::DoNotBuy => config.do_not_buy = value,
            MirrorBool::DoNotFuse => config.do_not_fuse = value,
            MirrorBool::DoNotSell => config.do_not_sell = value,
            MirrorBool::DoNotEnhance => config.do_not_enhance = value,
            MirrorBool::OnlyAggressiveFuse => config.only_aggressive_fuse = value,
            MirrorBool::DoNotSystemFuse => config.do_not_system_fuse = value,
            MirrorBool::OnlySystemFuse => config.only_system_fuse = value,
            MirrorBool::AggressiveAlsoEnhance => config.aggressive_also_enhance = value,
            MirrorBool::AggressiveSaveSystems => config.aggressive_save_systems = value,
            MirrorBool::AfterLevelIv => config.after_level_IV = value,
            MirrorBool::SecondSystem => config.second_system = value,
            MirrorBool::SecondSystemFuseIv => config.second_system_fuse_IV = value,
            MirrorBool::SecondSystemBuy => config.second_system_buy = value,
            MirrorBool::SecondSystemSelectReward => config.second_system_select_reward = value,
            MirrorBool::SecondSystemPowerUp => config.second_system_power_up = value,
            MirrorBool::AvoidSkill3 => {
                config.avoid_skill_3 = value;
                if value {
                    config.prioritize_skill_3 = false;
                }
            }
            MirrorBool::PrioritizeSkill3 => {
                config.prioritize_skill_3 = value;
                if value {
                    config.avoid_skill_3 = false;
                }
            }
            MirrorBool::ReformationEachFloor => config.re_formation_each_floor = value,
            MirrorBool::DefenseFirstRound => {
                config.defense_first_round = value;
                if value {
                    config.defense_for_solo = false;
                }
            }
            MirrorBool::DefenseForSolo => {
                config.defense_for_solo = value;
                if value {
                    config.defense_first_round = false;
                }
            }
            MirrorBool::SkillReplacement => config.skill_replacement = value,
            MirrorBool::UseStarlight => config.use_starlight = value,
            MirrorBool::FixedTeamUse => config.fixed_team_use = value,
            MirrorBool::UseTeamCode => config.use_team_code = value,
            MirrorBool::UseCustomThemeWeight => config.use_custom_theme_pack_weight = value,
            MirrorBool::ObserveEgoGift => config.observe_ego_gift = value,
        });
    }

    pub fn set_mirror_u8(&mut self, field: MirrorU8, value: u8) {
        self.update_mirror(|config| match field {
            MirrorU8::ShopStrategy => config.shop_strategy = value.min(2),
            MirrorU8::AfterLevelIvSelect => config.after_level_IV_select = value.min(2),
            MirrorU8::MaxKeywordRefresh => config.max_keyword_refresh = value.min(10),
            MirrorU8::MaxNormalRefresh => config.max_normal_refresh = value.min(10),
            MirrorU8::SecondSystemSelect => config.second_system_select = value.min(9),
            MirrorU8::SecondSystemStartFloor => config.second_system_setting = value.clamp(2, 5),
            MirrorU8::DefenseTurns => config.defense_for_solo_turns = value.clamp(1, 5),
            MirrorU8::SkillReplacementSelect => config.skill_replacement_select = value,
            MirrorU8::SkillReplacementMode => config.skill_replacement_mode = value.min(1),
            MirrorU8::FixedTeamUseSelect => config.fixed_team_use_select = value.min(2),
        });
    }

    pub fn toggle_ignore_shop(&mut self, floor: usize) {
        self.update_mirror(|config| {
            if let Some(value) = config.ignore_shop.get_mut(floor) {
                *value = !*value;
            }
        });
    }

    pub fn set_starlight_level(&mut self, index: usize, level: u8) {
        self.update_mirror(|config| {
            if index < 10 {
                if config.opening_bonus.len() < 10 {
                    config.opening_bonus.resize(10, 0);
                }
                config.opening_bonus[index] = level.min(3);
            }
        });
    }

    pub fn set_all_starlight(&mut self, level: u8) {
        self.update_mirror(|config| config.opening_bonus = vec![level.min(3); 10]);
    }

    pub fn starlight_cost(&self) -> u32 {
        let costs = [10_u32, 10, 20, 20, 30, 30, 40, 40, 50, 60];
        self.editor
            .as_ref()
            .map(TeamEditorState::mirror_config)
            .map(|config| {
                config
                    .opening_bonus
                    .iter()
                    .take(10)
                    .enumerate()
                    .map(|(index, level)| costs[index] * u32::from(*level))
                    .sum()
            })
            .unwrap_or(0)
    }

    pub fn add_observe_gift(&mut self, gift: &str) -> bool {
        let gift = gift.trim();
        if gift.is_empty() {
            return false;
        }
        let mut added = false;
        self.update_mirror(|config| {
            if !config
                .observe_ego_gift_selected
                .iter()
                .any(|item| item == gift)
            {
                config.observe_ego_gift_selected.push(gift.to_owned());
                added = true;
            }
        });
        added
    }

    pub fn remove_observe_gift(&mut self, gift: &str) {
        self.update_mirror(|config| {
            config.observe_ego_gift_selected.retain(|item| item != gift);
        });
    }

    pub fn export_editor_json(&self) -> Option<String> {
        self.editor
            .as_ref()
            .and_then(|editor| serde_json::to_string_pretty(&editor.team).ok())
    }

    /// Merge pasted JSON with the current form and all mirror defaults.  This
    /// intentionally mirrors the React editor's compatibility behavior: the
    /// current id survives imports, omitted fields keep their current values,
    /// and new mirror fields receive defaults.
    pub fn import_editor_json(&mut self, input: &str) -> Result<(), String> {
        let parsed: Value =
            serde_json::from_str(input.trim()).map_err(|error| error.to_string())?;
        let object = parsed
            .as_object()
            .ok_or_else(|| "队伍 JSON 必须是对象".to_owned())?;
        let Some(name) = object.get("name").and_then(Value::as_str) else {
            return Err("队伍 JSON 缺少 name".to_owned());
        };
        if name.trim().is_empty() {
            return Err("队伍名称不能为空".to_owned());
        }

        let Some(editor) = self.editor.as_mut() else {
            return Err("当前没有打开队伍编辑器".to_owned());
        };
        let current = &editor.team;
        let id = current.id.clone();
        let purpose = object
            .get("purpose")
            .cloned()
            .map(|value| serde_json::from_value(value).map_err(|_| "purpose 无效".to_owned()))
            .transpose()?
            .unwrap_or(current.purpose);
        let sinners = object
            .get("sinners")
            .cloned()
            .map(|value| serde_json::from_value(value).map_err(|_| "sinners 无效".to_owned()))
            .transpose()?
            .unwrap_or_else(|| current.sinners.clone());
        if sinners.len() > 12 {
            return Err("队伍最多选择 12 名人格".to_owned());
        }
        let accessory_scheme = object
            .get("accessoryScheme")
            .and_then(Value::as_str)
            .unwrap_or(&current.accessoryScheme)
            .to_owned();
        let enabled = object
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(current.enabled);

        let mut mirror = current.mirrorConfig.clone().unwrap_or_default();
        if let Some(mirror_patch) = object.get("mirrorConfig") {
            let Some(patch) = mirror_patch.as_object() else {
                return Err("mirrorConfig 必须是对象".to_owned());
            };
            let mut value = serde_json::to_value(&mirror).map_err(|error| error.to_string())?;
            let Some(base) = value.as_object_mut() else {
                return Err("mirrorConfig 默认值无效".to_owned());
            };
            let mut patch = patch.clone();
            if let Some(discard_patch) = patch.get("discard_systems").and_then(Value::as_object) {
                let mut discard = serde_json::to_value(&mirror.discard_systems)
                    .map_err(|error| error.to_string())?;
                if let Some(discard_base) = discard.as_object_mut() {
                    discard_base.extend(discard_patch.clone());
                    patch.insert("discard_systems".to_owned(), discard);
                }
            }
            base.extend(patch);
            mirror = serde_json::from_value(value).map_err(|error| error.to_string())?;
        }
        mirror.opening_bonus.resize(10, 0);
        mirror.opening_bonus.truncate(10);
        mirror.ignore_shop.resize(5, false);
        mirror.ignore_shop.truncate(5);

        editor.team = TeamDetail {
            id,
            name: name.to_owned(),
            sinners,
            purpose,
            accessoryScheme: accessory_scheme,
            enabled,
            mirrorConfig: Some(mirror),
        };
        editor.json_import_open = false;
        self.feedback = Some("已导入队伍 JSON（尚未保存）".to_owned());
        Ok(())
    }

    pub fn save_editor(&mut self) -> Result<(), String> {
        let Some(editor) = self.editor.as_mut() else {
            return Err("当前没有打开队伍编辑器".to_owned());
        };
        if editor.team.name.trim().is_empty() {
            return Err("队伍名称不能为空".to_owned());
        }
        let value = serde_json::to_value(&editor.team).map_err(|error| error.to_string())?;
        let response = self.client.call(method::TEAM_SAVE, Some(value));
        if let Some(error) = response.error {
            return Err(error.message);
        }
        self.reload_teams_only();
        self.editor = None;
        self.feedback = Some("队伍已保存".to_owned());
        Ok(())
    }

    pub fn request_delete(&mut self, team: TeamDetail) {
        self.delete_target = Some(team);
    }

    pub fn cancel_delete(&mut self) {
        self.delete_target = None;
    }

    pub fn confirm_delete(&mut self) -> Result<(), String> {
        let Some(team) = self.delete_target.take() else {
            return Ok(());
        };
        let response = self
            .client
            .call(method::TEAM_DELETE, Some(json!({"id": team.id})));
        if let Some(error) = response.error {
            self.delete_target = Some(team);
            return Err(error.message);
        }
        self.reload_teams_only();
        self.feedback = Some("队伍已删除".to_owned());
        Ok(())
    }

    pub fn take_feedback(&mut self) -> Option<String> {
        self.feedback.take()
    }

    fn reload_teams_only(&mut self) {
        self.teams = self
            .request_value(method::TEAM_LIST, None)
            .and_then(|value| serde_json::from_value(value).ok())
            .unwrap_or_default();
    }

    fn request_value(&mut self, method_name: &str, params: Option<Value>) -> Option<Value> {
        let response = self.client.call(method_name, params);
        if let Some(error) = response.error {
            self.feedback = Some(error.message);
            None
        } else {
            response.result
        }
    }
}

/// Named bool fields keep the page readable and prevent ad-hoc JSON patches.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MirrorBool {
    DoNotHeal,
    DoNotBuy,
    DoNotFuse,
    DoNotSell,
    DoNotEnhance,
    OnlyAggressiveFuse,
    DoNotSystemFuse,
    OnlySystemFuse,
    AggressiveAlsoEnhance,
    AggressiveSaveSystems,
    AfterLevelIv,
    SecondSystem,
    SecondSystemFuseIv,
    SecondSystemBuy,
    SecondSystemSelectReward,
    SecondSystemPowerUp,
    AvoidSkill3,
    PrioritizeSkill3,
    ReformationEachFloor,
    DefenseFirstRound,
    DefenseForSolo,
    SkillReplacement,
    UseStarlight,
    FixedTeamUse,
    UseTeamCode,
    UseCustomThemeWeight,
    ObserveEgoGift,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MirrorU8 {
    ShopStrategy,
    AfterLevelIvSelect,
    MaxKeywordRefresh,
    MaxNormalRefresh,
    SecondSystemSelect,
    SecondSystemStartFloor,
    DefenseTurns,
    SkillReplacementSelect,
    SkillReplacementMode,
    FixedTeamUseSelect,
}

pub const SYSTEM_NAMES: [&str; 10] = [
    "burn", "bleed", "tremor", "rupture", "sinking", "poise", "charge", "slash", "pierce", "blunt",
];

pub const SYSTEM_LABELS: [&str; 10] = [
    "燃烧", "流血", "震颤", "破裂", "沉潜", "呼吸", "充能", "斩击", "突刺", "打击",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_counts_and_new_team_follow_selected_purpose() {
        let mut state = TeamsState::default();
        assert_eq!(state.count_for(TeamFilter::All), 3);
        assert_eq!(state.count_for(TeamFilter::Mirror), 1);
        state.set_filter(TeamFilter::Luxcavation);
        state.open_new();
        assert_eq!(
            state.editor.as_ref().unwrap().team.purpose,
            TeamPurpose::Luxcavation
        );
    }

    #[test]
    fn sinner_order_is_capped_at_twelve_and_can_be_cleared() {
        let mut state = TeamsState::default();
        state.open_new();
        for index in 0..13 {
            state.toggle_sinner(&format!("sinner-{index}"));
        }
        assert_eq!(state.editor.as_ref().unwrap().team.sinners.len(), 12);
        state.clear_sinners();
        assert!(state.editor.as_ref().unwrap().team.sinners.is_empty());
    }

    #[test]
    fn mutually_exclusive_combat_options_are_enforced() {
        let mut state = TeamsState::default();
        state.open_new();
        state.set_mirror_bool(MirrorBool::AvoidSkill3, true);
        state.set_mirror_bool(MirrorBool::PrioritizeSkill3, true);
        let config = state.editor.as_ref().unwrap().mirror_config();
        assert!(!config.avoid_skill_3 && config.prioritize_skill_3);
        state.set_mirror_bool(MirrorBool::DefenseFirstRound, true);
        state.set_mirror_bool(MirrorBool::DefenseForSolo, true);
        let config = state.editor.as_ref().unwrap().mirror_config();
        assert!(!config.defense_first_round && config.defense_for_solo);
    }

    #[test]
    fn starlight_cost_and_json_import_use_contract_defaults() {
        let mut state = TeamsState::default();
        state.open_new();
        state.set_starlight_level(0, 2);
        state.set_starlight_level(9, 3);
        assert_eq!(state.starlight_cost(), 250);
        state
            .import_editor_json(
                r#"{"name":"导入队伍","purpose":"mirror","mirrorConfig":{"second_system":true}}"#,
            )
            .unwrap();
        let editor = state.editor.as_ref().unwrap();
        assert_eq!(editor.team.name, "导入队伍");
        assert!(editor.mirror_config().second_system);
        assert_eq!(editor.mirror_config().opening_bonus.len(), 10);
        state
            .import_editor_json(
                r#"{"name":"部分舍弃","mirrorConfig":{"discard_systems":{"burn":true}}}"#,
            )
            .unwrap();
        assert!(
            state
                .editor
                .as_ref()
                .unwrap()
                .mirror_config()
                .discard_systems
                .burn
        );
    }

    #[test]
    fn save_and_delete_use_the_same_mock_rpc_contract() {
        let mut state = TeamsState::default();
        let initial = state.teams.len();
        state.open_new();
        state.editor.as_mut().unwrap().team.name = "新队伍".into();
        state.save_editor().unwrap();
        assert_eq!(state.teams.len(), initial + 1);
        let created = state.teams.last().unwrap().clone();
        state.request_delete(created);
        state.confirm_delete().unwrap();
        assert_eq!(state.teams.len(), initial);
    }
}
