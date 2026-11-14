//! TSX-facing saved-team store.
//!
//! The editor draft is screen state in TSX. This class owns the saved list, the
//! formation-slot rules and every `team.*` mutation, so TSX only mirrors the
//! arrays returned by these methods.

use nsh_core::prelude::*;
use serde_json::Value;

use super::{
    store::TeamsState,
    types::{SinnerView, TeamPresetView, TeamSaveInput, TeamSlotView, TeamView},
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

    /// Every game loadout number (1–40), with the team saved there, if any.
    /// Purpose filtering is a page concern: a slot is not owned by a purpose.
    pub fn slots(&self) -> Vec<TeamSlotView> {
        self.state.slots().iter().map(TeamSlotView::from).collect()
    }

    /// Legacy teams outside the forty game loadouts.
    pub fn extras(&self) -> Vec<TeamView> {
        self.state
            .extra_teams()
            .iter()
            .map(TeamView::from)
            .collect()
    }

    /// The built-in team templates from `team.preset.list`.
    pub fn presets(&self) -> Vec<TeamPresetView> {
        self.state
            .presets()
            .iter()
            .map(TeamPresetView::from)
            .collect()
    }

    /// Whether the automation selects a team by physical row instead of name.
    pub fn selection(&self) -> bool {
        self.state.select_team_by_order
    }

    /// Switch between OCR-by-name and physical-row selection.
    #[allow(non_snake_case)]
    pub fn setSelection(&mut self, enabled: bool) -> Result<bool, String> {
        self.state.set_selection(enabled)
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

impl Default for TeamsStore {
    fn default() -> Self {
        Self::new()
    }
}
