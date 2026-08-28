//! Teams page state split by concern: selection/editor navigation, mirror
//! configuration mutations, and persistence/RPC serialization.

mod mirror;
mod persistence;
mod selection;
mod types;

pub use types::*;

use crate::{
    ipc::RpcGateway,
    model::{SinnerInfo, TeamDetail, TeamMirrorConfig, TeamPurpose},
};

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
    fn editor_selects_are_exclusive_and_reset_on_tab_change() {
        let mut state = TeamsState::default();
        state.open_new();
        state.toggle_select(TeamSelect::ShopStrategy);
        assert!(state.is_select_open(TeamSelect::ShopStrategy));
        state.toggle_select(TeamSelect::SecondSystem);
        assert!(!state.is_select_open(TeamSelect::ShopStrategy));
        assert!(state.is_select_open(TeamSelect::SecondSystem));
        state.set_editor_tab(TeamEditorTab::Combat);
        assert!(state.open_select.is_none());
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
