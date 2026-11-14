use super::*;

use crate::{
    ipc::BackendClient,
    model::{TeamDetail, TeamMirrorConfig, TeamPurpose},
};

fn state() -> TeamsState {
    let state = TeamsState::new(BackendClient::mock());
    assert!(state.error.is_none(), "mock must load: {:?}", state.error);
    state
}

fn team(id: &str, name: &str, purpose: TeamPurpose) -> TeamDetail {
    TeamDetail {
        schemaVersion: 3,
        id: id.into(),
        name: name.into(),
        sinners: vec!["faust".into()],
        purpose,
        accessoryScheme: "burn".into(),
        enabled: purpose == TeamPurpose::Mirror,
        teamCode: String::new(),
        queuePosition: 0,
        mirrorConfig: Some(TeamMirrorConfig::default()),
    }
}

#[test]
fn load_reads_teams_roster_and_presets() {
    let state = state();
    assert_eq!(state.teams.len(), 3);
    assert_eq!(state.sinners.len(), 12);
    assert_eq!(state.presets.len(), 6);
    let count = |purpose| {
        state
            .teams
            .iter()
            .filter(|team| team.purpose == purpose)
            .count()
    };
    assert_eq!(count(TeamPurpose::Mirror), 1);
    assert_eq!(count(TeamPurpose::Luxcavation), 1);
    assert_eq!(count(TeamPurpose::General), 1);
}

#[test]
fn fixed_slots_cover_the_forty_game_loadouts() {
    let state = state();
    let slots = state.slots();
    assert_eq!(slots.len(), 40);
    assert_eq!(slots[0].number, 1);
    assert_eq!(slots[0].team.as_ref().unwrap().id, "team-1");
    assert_eq!(slots[1].team.as_ref().unwrap().id, "team-2");
    assert_eq!(slots[2].team.as_ref().unwrap().id, "team-3");
    assert!(slots[3].team.is_none());
    assert_eq!(slots[39].number, 40);
}

#[test]
fn extra_teams_are_only_legacy_numbers_above_forty() {
    let mut state = state();
    state
        .teams
        .push(team("team-41", "旧队伍", TeamPurpose::Mirror));
    let extras = state.extra_teams();
    assert_eq!(extras.len(), 1);
    assert_eq!(extras[0].id, "team-41");
    assert!(state.extra_teams().iter().all(|team| team.id == "team-41"));
}

#[test]
fn creating_a_team_binds_the_requested_number() {
    let mut state = state();
    let input = TeamSaveInput {
        team: team("", "新队伍", TeamPurpose::Mirror),
        team_number: Some(4),
    };
    let saved = state.save(input).unwrap();
    assert_eq!(saved.id, "team-4");
    assert_eq!(state.teams.len(), 4);
}

#[test]
fn updating_a_team_replaces_the_saved_record() {
    let mut state = state();
    let mut edited = state.teams[0].clone();
    edited.name = "改名".into();
    let saved = state
        .save(TeamSaveInput {
            team: edited,
            team_number: None,
        })
        .unwrap();
    assert_eq!(saved.id, "team-1");
    assert_eq!(state.teams[0].name, "改名");
}

#[test]
fn luxcavation_saves_strip_the_mirror_config() {
    let mut state = state();
    let mut lux = team("", "资源本", TeamPurpose::Luxcavation);
    lux.mirrorConfig = Some(TeamMirrorConfig::default());
    lux.enabled = true;
    let saved = state
        .save(TeamSaveInput {
            team: lux,
            team_number: Some(5),
        })
        .unwrap();
    assert_eq!(saved.id, "team-5");
    assert!(!saved.enabled);
    assert!(saved.mirrorConfig.is_none());
}

#[test]
fn general_saves_are_manual_only_and_keep_the_mirror_config() {
    let mut state = state();
    let mut general = team("", "手动队伍", TeamPurpose::General);
    general.enabled = true;
    general.mirrorConfig = Some(TeamMirrorConfig::default());
    let saved = state
        .save(TeamSaveInput {
            team: general,
            team_number: Some(6),
        })
        .unwrap();
    assert_eq!(saved.id, "team-6");
    assert!(!saved.enabled);
    assert!(saved.mirrorConfig.is_some());
}

#[test]
fn enabled_toggle_uses_a_partial_save() {
    let mut state = state();
    let saved = state.set_enabled("team-1", false).unwrap();
    assert!(!saved.enabled);
    assert!(!state.teams[0].enabled);

    let error = state.set_enabled("team-2", true).unwrap_err();
    assert!(error.contains("资源本"));

    let error = state.set_enabled("team-3", true).unwrap_err();
    assert!(error.contains("通用"));
}

#[test]
fn disabling_a_queued_team_shifts_the_next_position() {
    let mut state = state();
    let created = state
        .save(TeamSaveInput {
            team: team("", "第二支镜牢队", TeamPurpose::Mirror),
            team_number: Some(4),
        })
        .unwrap();
    assert_eq!(created.queuePosition, 2);

    state.set_enabled("team-1", false).unwrap();

    let second = state.teams.iter().find(|team| team.id == "team-4").unwrap();
    assert_eq!(second.queuePosition, 1);
    assert!(second.enabled);
}

#[test]
fn stats_round_trip_and_clear() {
    let mut state = state();
    let stats = state.team_stats("team-1").unwrap();
    assert_eq!(stats.totalCount, 6);
    assert_eq!(stats.hard.count, 3);

    let cleared = state.clear_team_stats("team-1").unwrap();
    assert_eq!(cleared.totalCount, 0);
    assert_eq!(state.team_stats("team-1").unwrap().totalCount, 0);
}

#[test]
fn delete_removes_the_team() {
    let mut state = state();
    state.delete("team-3").unwrap();
    assert_eq!(state.teams.len(), 2);
    assert!(!state.teams.iter().any(|team| team.id == "team-3"));
}
