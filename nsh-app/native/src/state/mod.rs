//! The state layer: saved-domain state, RPC mutations and persistence.
//!
//! `model/` owns payload shapes; `ipc/` owns transport; this module owns the
//! state containers and the TSX-facing bridges. The teams editor draft stays in
//! TSX (it is screen state); saved teams, their persistence and their stats are
//! owned here.

pub mod app_state;
pub mod home;
pub mod other;
pub mod settings_file;
pub mod teams;
