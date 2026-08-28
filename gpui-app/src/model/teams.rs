use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[allow(non_snake_case)]
pub struct TeamMirrorConfig {
    pub team_system: u8,
    pub shop_strategy: u8,
    pub discard_systems: DiscardSystems,
    pub do_not_heal: bool,
    pub do_not_buy: bool,
    pub do_not_fuse: bool,
    pub do_not_sell: bool,
    pub do_not_enhance: bool,
    pub only_aggressive_fuse: bool,
    pub do_not_system_fuse: bool,
    pub only_system_fuse: bool,
    pub aggressive_also_enhance: bool,
    pub aggressive_save_systems: bool,
    pub after_level_IV: bool,
    pub after_level_IV_select: u8,
    pub ignore_shop: Vec<bool>,
    pub max_keyword_refresh: u8,
    pub max_normal_refresh: u8,
    pub second_system: bool,
    pub second_system_select: u8,
    pub second_system_setting: u8,
    pub second_system_fuse_IV: bool,
    pub second_system_buy: bool,
    pub second_system_select_reward: bool,
    pub second_system_power_up: bool,
    pub avoid_skill_3: bool,
    pub prioritize_skill_3: bool,
    pub re_formation_each_floor: bool,
    pub defense_first_round: bool,
    pub defense_for_solo: bool,
    pub defense_for_solo_turns: u8,
    pub skill_replacement: bool,
    pub skill_replacement_select: u8,
    pub skill_replacement_mode: u8,
    pub use_starlight: bool,
    pub opening_bonus: Vec<u8>,
    pub fixed_team_use: bool,
    pub fixed_team_use_select: u8,
    pub use_team_code: bool,
    pub team_code: String,
    pub use_custom_theme_pack_weight: bool,
    pub observe_ego_gift: bool,
    pub observe_ego_gift_selected: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct DiscardSystems {
    pub burn: bool,
    pub bleed: bool,
    pub tremor: bool,
    pub rupture: bool,
    pub sinking: bool,
    pub poise: bool,
    pub charge: bool,
    pub slash: bool,
    pub pierce: bool,
    pub blunt: bool,
}

impl Default for TeamMirrorConfig {
    fn default() -> Self {
        Self {
            team_system: 0,
            shop_strategy: 0,
            discard_systems: Default::default(),
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
            ignore_shop: vec![false; 5],
            max_keyword_refresh: 1,
            max_normal_refresh: 1,
            second_system: false,
            second_system_select: 0,
            second_system_setting: 2,
            second_system_fuse_IV: true,
            second_system_buy: true,
            second_system_select_reward: true,
            second_system_power_up: true,
            avoid_skill_3: false,
            prioritize_skill_3: false,
            re_formation_each_floor: false,
            defense_first_round: false,
            defense_for_solo: false,
            defense_for_solo_turns: 5,
            skill_replacement: false,
            skill_replacement_select: 0,
            skill_replacement_mode: 0,
            use_starlight: false,
            opening_bonus: vec![1, 1, 1, 1, 0, 0, 0, 0, 0, 0],
            fixed_team_use: false,
            fixed_team_use_select: 0,
            use_team_code: false,
            team_code: String::new(),
            use_custom_theme_pack_weight: false,
            observe_ego_gift: false,
            observe_ego_gift_selected: Vec::new(),
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TeamSummary {
    pub id: String,
    pub name: String,
    pub sinners: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TeamPurpose {
    Mirror,
    Luxcavation,
    General,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[allow(non_snake_case)]
pub struct TeamDetail {
    #[serde(default = "schema_version")]
    pub schemaVersion: u32,
    pub id: String,
    pub name: String,
    pub sinners: Vec<String>,
    pub purpose: TeamPurpose,
    pub accessoryScheme: String,
    pub enabled: bool,
    pub mirrorConfig: Option<TeamMirrorConfig>,
}

fn schema_version() -> u32 {
    1
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SinnerInfo {
    pub id: String,
    pub name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mirror_defaults_have_expected_collection_sizes() {
        let config = TeamMirrorConfig::default();
        assert_eq!(config.ignore_shop, vec![false; 5]);
        assert_eq!(config.opening_bonus.len(), 10);
        assert_eq!(config.opening_bonus[..4], [1, 1, 1, 1]);
    }
    #[test]
    fn team_config_round_trips_json() {
        let team = TeamDetail {
            schemaVersion: 1,
            id: "t1".into(),
            name: "队伍".into(),
            sinners: vec!["faust".into()],
            purpose: TeamPurpose::General,
            accessoryScheme: "poise".into(),
            enabled: true,
            mirrorConfig: Some(Default::default()),
        };
        let json = serde_json::to_string(&team).unwrap();
        assert_eq!(serde_json::from_str::<TeamDetail>(&json).unwrap(), team);
    }
}
