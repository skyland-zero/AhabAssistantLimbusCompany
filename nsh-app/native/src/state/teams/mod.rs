//! Saved teams, formation slots and team statistics.
//!
//! The GPUI client's `state/teams/` is split the same way: list/selection,
//! mirror-run persistence and stats. This port keeps the saved-team owner in
//! Rust and leaves the editor draft in TSX, as `nsh-app/AGENTS.md` requires.

mod bridge;
mod store;
mod types;

pub use bridge::TeamsStore;
pub use store::TeamsState;
pub use types::{TeamFilter, TeamSaveInput, TeamSlot, TeamView};

#[cfg(test)]
mod tests;
