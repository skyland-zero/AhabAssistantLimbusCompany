//! Settings, theme packs, resources and the toolbox.
//!
//! The GPUI client's `state/other/` is split the same way. Each store owns its
//! RPC boundary and returns fresh snapshots TSX mirrors into signals.

mod bridge;
mod resources;
mod settings;
mod theme_packs;
mod toolbox;

pub use bridge::{ResourcesStore, SettingsStore, ThemePacksStore, ToolboxStore};

#[cfg(test)]
mod tests;
