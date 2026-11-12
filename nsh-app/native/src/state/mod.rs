//! The state layer's file boundary.
//!
//! `model/` owns the settings JSON and its migration; this module owns where
//! the file lives and how it is replaced. P1's next step wires the settings
//! store to these functions.

pub mod settings_file;
