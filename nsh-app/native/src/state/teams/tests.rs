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
        enabled: purpose != TeamPurpose::Luxcavation,
        teamCode: String::new(),
        mirrorConfig: (purpose != TeamPurpose::Luxcavation).then(TeamMirrorConfig::default),
    }
}

#[test]
fn load_reads_teams_roster_and_presets() {
    let state = state();
    assert_eq!(state.teams.len(), 3);
    assert_eq!(state.sinners.len(), 12);
    assert_eq!(state.presets.len(), 6);
    assert_eq!(state.count_for(TeamFilter::All), 3);
    assert_eq!(state.count_for(TeamFilter::Mirror), 1);
    assert_eq!(state.count_for(TeamFilter::Luxcavation), 1);
    assert_eq!(state.count_for(TeamFilter::General), 1);
}

#[test]
fn fixed_slots_use_the_twenty_formation_numbers() {
    let state = state();
    let slots = state.slots_for(TeamFilter::All);
    assert_eq!(slots.len(), 20);
    assert_eq!(slots[0].number, 1);
    assert_eq!(slots[0].team.as_ref().unwrap().id, "team-1");
    assert!(slots[3].team.is_none());

    let mirror = state.slots_for(TeamFilter::Mirror);
    assert!(!mirror.iter().any(|slot| slot.number == 1));
    assert!(mirror.iter().any(|slot| slot.number == 4));

    // Slot 1 belongs to an existing mirror team, so the experience-dungeon
    // filter has no fixed slot and shows team-2 as an extra team instead.
    assert!(state.slots_for(TeamFilter::Luxcavation).is_empty());
    assert!(state.slots_for(TeamFilter::General).is_empty());
}

#[test]
fn extra_teams_are_everything_outside_the_fixed_numbers() {
    let state = state();
    let mirror = state.extra_teams_for(TeamFilter::Mirror);
    assert_eq!(mirror[0].id, "team-1");
    let lux = state.extra_teams_for(TeamFilter::Luxcavation);
    assert_eq!(lux[0].id, "team-2");
    assert!(state.extra_teams_for(TeamFilter::All).is_empty());
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
    let mut lux = team("", "经验本", TeamPurpose::Luxcavation);
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
fn enabled_toggle_uses_a_partial_save() {
    let mut state = state();
    let saved = state.set_enabled("team-1", false).unwrap();
    assert!(!saved.enabled);
    assert!(!state.teams[0].enabled);

    let error = state.set_enabled("team-2", true).unwrap_err();
    assert!(error.contains("经验本"));
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
