//! Teams page state split by concern: selection/editor navigation, mirror
//! configuration mutations, and persistence/RPC serialization.

mod mirror;
mod persistence;
mod selection;
mod types;

pub use types::*;

use crate::{
    ipc::RpcGateway,
    model::{
        SinnerInfo, TEAM_SLOT_COUNT, TeamDetail, TeamMirrorConfig, TeamPreset, TeamPurpose,
        team_number_from_id,
    },
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
    fn spiderweb_gift_preset_adds_canonical_id_and_enables_search() {
        let mut state = TeamsState::default();
        state.open_new();

        assert!(state.add_spiderweb_entangled_in_red());
        let config = state.editor.as_ref().unwrap().mirror_config();
        assert!(config.observe_ego_gift);
        assert_eq!(
            config.observe_ego_gift_selected,
            vec![crate::model::SPIDERWEB_ENTANGLED_IN_RED_GIFT_ID]
        );
        assert!(!state.add_spiderweb_entangled_in_red());
    }

    #[test]
    fn observe_gift_input_normalizes_spiderweb_alias_and_caps_selection() {
        let mut state = TeamsState::default();
        state.open_new();

        assert!(state.add_observe_gift("赤红纠缠的蜘蛛巢"));
        assert!(state.add_observe_gift("bleed_3_1_1"));
        assert!(state.add_observe_gift("burn_3_1_1"));
        assert!(!state.add_observe_gift("general_3_1_1"));
        let config = state.editor.as_ref().unwrap().mirror_config();
        assert_eq!(config.observe_ego_gift_selected.len(), 3);
        assert_eq!(
            config.observe_ego_gift_selected[0],
            crate::model::SPIDERWEB_ENTANGLED_IN_RED_GIFT_ID
        );
    }

    #[test]
    fn spiderweb_preset_recognizes_imported_legacy_alias() {
        let mut state = TeamsState::default();
        state.open_new();
        state.update_mirror(|config| {
            config.observe_ego_gift_selected = vec!["general_gift_3_32.png".to_owned()];
        });

        assert!(!state.add_spiderweb_entangled_in_red());
        let config = state.editor.as_ref().unwrap().mirror_config();
        assert!(config.observe_ego_gift);
        assert_eq!(
            config.observe_ego_gift_selected,
            vec!["general_gift_3_32.png".to_owned()]
        );
    }

    #[test]
    fn built_in_presets_have_stable_ids_and_complete_routes() {
        let state = TeamsState::default();
        assert_eq!(state.presets.len(), 3);
        let normal = state
            .presets
            .iter()
            .find(|preset| preset.presetId == "hos_ryoshu_solo_normal")
            .unwrap();
        let hard = state
            .presets
            .iter()
            .find(|preset| preset.presetId == "hos_ryoshu_solo_hard")
            .unwrap();
        assert_eq!(normal.routeId, "hos_ryoshu_solo_route");
        assert_eq!(hard.routeId, "hos_ryoshu_solo_route");
        assert_eq!(normal.team.sinners.len(), 12);
        assert_eq!(hard.team.sinners.len(), 12);
        assert_eq!(normal.team.name, "小指良伪单通（普牢）");
        assert_eq!(hard.team.name, "小指良伪单通（困牢）");
        assert_eq!(
            normal
                .team
                .mirrorConfig
                .as_ref()
                .unwrap()
                .mirror_route_profile,
            "hos_ryoshu_solo_route"
        );
        assert!(normal.team.mirrorConfig.as_ref().unwrap().do_not_fuse);
        assert!(!hard.team.mirrorConfig.as_ref().unwrap().do_not_fuse);
        assert_eq!(
            normal.team.mirrorConfig.as_ref().unwrap().opening_bonus,
            vec![1, 1, 1, 1, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            hard.team.mirrorConfig.as_ref().unwrap().opening_bonus,
            vec![3; 10]
        );
        assert!(!normal.description.zhCn.is_empty());
        assert!(!hard.description.zhCn.is_empty());
    }

    #[test]
    fn selecting_empty_slot_saves_the_complete_preset_to_requested_slot() {
        let mut state = TeamsState::default();
        state.open_preset_picker_for_slot(4);
        assert!(state.select_preset("hos_ryoshu_solo_normal").unwrap());
        let editor = state.editor.as_ref().unwrap();
        assert_eq!(editor.requested_team_number, Some(4));
        assert!(editor.team.id.is_empty());
        assert_eq!(editor.team.name, "小指良伪单通（普牢）");
        assert_eq!(
            editor
                .team
                .mirrorConfig
                .as_ref()
                .unwrap()
                .mirror_route_profile,
            "hos_ryoshu_solo_route"
        );
    }

    #[test]
    fn overwriting_requires_confirmation_and_preserves_enabled_state() {
        let mut state = TeamsState::default();
        let target = state.teams[0].clone();
        state.open_preset_picker_for_team(&target);
        assert!(!state.select_preset("hos_ryoshu_solo_hard").unwrap());
        assert!(state.editor.is_none());
        assert!(state.preset_overwrite.is_some());

        assert!(state.confirm_preset_overwrite().unwrap());
        let editor = state.editor.as_ref().unwrap();
        assert_eq!(editor.team.id, target.id);
        assert_eq!(editor.team.enabled, target.enabled);
        assert_eq!(editor.team.name, "小指良伪单通（困牢）");
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

    #[test]
    fn luxcavation_teams_have_no_mirror_editor_or_queue_toggle() {
        let mut state = TeamsState::default();
        state.set_filter(TeamFilter::Luxcavation);
        state.open_new();
        state.editor.as_mut().unwrap().team.name = "经验本".into();
        assert_eq!(state.editor.as_ref().unwrap().team.mirrorConfig, None);
        assert!(!state.editor.as_ref().unwrap().team.enabled);

        state.set_editor_tab(TeamEditorTab::Shop);
        assert_eq!(state.editor.as_ref().unwrap().tab, TeamEditorTab::Basic);
        state.set_mirror_bool(MirrorBool::SecondSystem, true);
        assert_eq!(state.editor.as_ref().unwrap().team.mirrorConfig, None);

        let (_, payload) = state.prepare_save().unwrap();
        assert!(payload["mirrorConfig"].is_null());
        assert_eq!(payload["enabled"], false);
        state.fail_save("test".to_owned());
    }

    #[test]
    fn switching_from_luxcavation_initializes_mirror_editor_defaults() {
        let mut state = TeamsState::default();
        state.set_filter(TeamFilter::Luxcavation);
        state.open_new();
        state.set_editor_purpose(TeamPurpose::Mirror);

        assert_eq!(
            state.editor.as_ref().unwrap().team.mirrorConfig,
            Some(TeamMirrorConfig::default())
        );
        state.set_mirror_bool(MirrorBool::SecondSystem, true);
        assert!(state.editor.as_ref().unwrap().mirror_config().second_system);
    }

    #[test]
    fn canonical_save_response_replaces_editor_with_backend_team() {
        let mut state = TeamsState::default();
        let initial = state.teams.len();
        state.open_new();
        state.editor.as_mut().unwrap().team.name = "后端队伍".into();
        let (submitted, _) = state.prepare_save().unwrap();
        let saved = TeamDetail {
            schemaVersion: 1,
            id: "team-42".into(),
            name: "后端队伍".into(),
            sinners: vec!["faust".into()],
            purpose: TeamPurpose::General,
            accessoryScheme: "burn".into(),
            enabled: true,
            mirrorConfig: Some(TeamMirrorConfig::default()),
        };
        state.apply_saved_team(&submitted, saved);

        assert_eq!(state.teams.len(), initial + 1);
        assert_eq!(state.teams.last().unwrap().id, "team-42");
        assert!(state.editor.is_none());
        assert!(!state.saving);
    }

    #[test]
    fn fixed_slots_keep_numbers_stable_without_materializing_empty_teams() {
        let mut state = TeamsState::default();
        let slots = state.fixed_slots_for_filter(TeamFilter::All);

        assert_eq!(slots.len(), 20);
        assert_eq!(slots[0].number, 1);
        assert_eq!(slots[0].team.as_ref().unwrap().id, "team-1");
        assert_eq!(slots[3].number, 4);
        assert!(slots[3].team.is_none());
        assert_eq!(state.count_for(TeamFilter::All), 3);

        let mirror_slots = state.fixed_slots_for_filter(TeamFilter::Mirror);
        assert!(!mirror_slots.iter().any(|slot| slot.number == 1));
        assert!(mirror_slots.iter().any(|slot| slot.number == 4));
        assert_eq!(
            state
                .extra_teams_for_filter(TeamFilter::Mirror)
                .first()
                .unwrap()
                .id,
            "team-1"
        );

        let luxcavation_slots = state.fixed_slots_for_filter(TeamFilter::Luxcavation);
        assert!(luxcavation_slots.is_empty());
        assert_eq!(
            state
                .extra_teams_for_filter(TeamFilter::Luxcavation)
                .first()
                .unwrap()
                .id,
            "team-2"
        );

        let mut overflow = state.teams[0].clone();
        overflow.id = "team-21".into();
        overflow.name = "额外队伍".into();
        state.teams.push(overflow);
        assert_eq!(
            state
                .extra_teams_for_filter(TeamFilter::All)
                .iter()
                .map(|team| team.id.as_str())
                .collect::<Vec<_>>(),
            vec!["team-21"]
        );
    }

    #[test]
    fn empty_slots_use_soft_purpose_defaults_and_bind_save_number() {
        let mut state = TeamsState::default();
        state.open_new_for_slot(1);
        state.editor.as_mut().unwrap().team.name = "经验本".into();
        let (_, payload) = state.prepare_save().unwrap();
        assert_eq!(payload["teamNumber"], 1);
        assert_eq!(payload["purpose"], "luxcavation");
        assert_eq!(payload["enabled"], false);
        state.fail_save("test".into());

        state.open_new_for_slot(5);
        state.editor.as_mut().unwrap().team.name = "镜牢".into();
        let (_, payload) = state.prepare_save().unwrap();
        assert_eq!(payload["teamNumber"], 5);
        assert_eq!(payload["purpose"], "mirror");
    }

    #[test]
    fn list_enabled_update_uses_partial_save_and_syncs_open_editor() {
        let mut state = TeamsState::default();
        let team = state.teams[0].clone();
        state.open_edit(&team);

        state.set_team_enabled(&team, false).unwrap();

        assert!(!state.teams[0].enabled);
        assert!(!state.editor.as_ref().unwrap().team.enabled);
        assert!(state.team_toggle_in_flight.is_none());
        assert_eq!(state.feedback.as_deref(), Some("队伍已停用"));
    }
}
