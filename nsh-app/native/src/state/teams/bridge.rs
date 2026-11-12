//! TSX-facing saved-team store.
//!
//! The editor draft is screen state in TSX. This class owns the saved list, the
//! formation-slot rules and every `team.*` mutation, so TSX only mirrors the
//! arrays returned by these methods.

use nsh_core::prelude::*;
use serde_json::Value;

use super::{
    store::TeamsState,
    types::{SinnerView, TeamFilter, TeamSaveInput, TeamSlotView, TeamView},
};

/// The saved teams and their RPC boundary.
#[class]
pub struct TeamsStore {
    state: TeamsState,
}

#[class]
impl TeamsStore {
    /// Loads the saved teams, the roster and the built-in presets. A transport
    /// failure is kept in [`TeamsStore::error`] instead of throwing.
    pub fn new() -> Self {
        Self {
            state: TeamsState::from_shared(),
        }
    }

    /// Loads the teams off the engine thread; rejects when the backend fails.
    pub async fn connect() -> Result<TeamsStore, String> {
        blocking(TeamsStore::try_new)
            .await
            .map_err(|error| error.to_string())?
    }

    fn try_new() -> Result<Self, String> {
        let state = TeamsState::from_shared();
        match state.error.clone() {
            Some(error) => Err(error),
            None => Ok(Self { state }),
        }
    }

    /// The first load error, when the backend was unreachable.
    pub fn error(&self) -> Option<String> {
        self.state.error.clone()
    }

    /// Every saved team.
    pub fn teams(&self) -> Vec<TeamView> {
        self.state.teams.iter().map(TeamView::from).collect()
    }

    /// The fixed formation slots for `filter` (`all`, `mirror`, `luxcavation`
    /// or `general`).
    pub fn slots(&self, filter: String) -> Vec<TeamSlotView> {
        self.state
            .slots_for(TeamFilter::parse(&filter))
            .iter()
            .map(TeamSlotView::from)
            .collect()
    }

    /// The teams outside the fixed formation numbers for `filter`.
    pub fn extras(&self, filter: String) -> Vec<TeamView> {
        self.state
            .extra_teams_for(TeamFilter::parse(&filter))
            .iter()
            .map(TeamView::from)
            .collect()
    }

    /// How many teams `filter` holds.
    pub fn count(&self, filter: String) -> u32 {
        self.state.count_for(TeamFilter::parse(&filter)) as u32
    }

    /// The sinner roster.
    pub fn sinners(&self) -> Vec<SinnerView> {
        self.state
            .sinners
            .iter()
            .map(|sinner| SinnerView {
                id: sinner.id.clone(),
                name: sinner.name.clone(),
            })
            .collect()
    }

    /// Re-reads the saved teams and returns the new list.
    pub fn refresh(&mut self) -> Result<Vec<TeamView>, String> {
        self.state.refresh()?;
        Ok(self.teams())
    }

    /// Creates or updates a team and returns the saved list.
    pub fn save(&mut self, team: Value) -> Result<Vec<TeamView>, String> {
        let input = TeamSaveInput::parse(team)?;
        self.state.save(input)?;
        Ok(self.teams())
    }

    /// Deletes one team and returns the saved list.
    pub fn delete(&mut self, id: String) -> Result<Vec<TeamView>, String> {
        self.state.delete(&id)?;
        Ok(self.teams())
    }

    /// Toggles one team's enabled flag and returns the saved list.
    pub fn set_enabled(&mut self, id: String, enabled: bool) -> Result<Vec<TeamView>, String> {
        self.state.set_enabled(&id, enabled)?;
        Ok(self.teams())
    }

    /// Reads one team's mirror-run statistics.
    pub fn stats(&mut self, id: String) -> Result<Value, String> {
        let stats = self.state.team_stats(&id)?;
        serde_json::to_value(stats).map_err(|error| error.to_string())
    }

    /// Clears one team's mirror-run statistics and returns the empty payload.
    pub fn clear_stats(&mut self, id: String) -> Result<Value, String> {
        let stats = self.state.clear_team_stats(&id)?;
        serde_json::to_value(stats).map_err(|error| error.to_string())
    }
}
