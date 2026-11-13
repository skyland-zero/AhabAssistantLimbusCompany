//! AALC's Native Shell backend crate (`nsh.json` `"rust": "native"`).
//!
//! `App.tsx` imports everything this crate registers from `aalc/native`; the
//! TypeScript declarations are generated from these Rust types into
//! `.nsh/types/aalc/native.d.ts`, so the TSX is checked against them.
//!
//! P1 is being ported bottom-up: `model` (the contract's payloads) and `ipc`
//! (the mock and the real sidecar transport) land first, then `state`, `i18n`
//! and the app lifecycle. See `MIGRATION.md` for the source mapping.
//!
//! The two pure-data crates live in `crates/` and are re-exported at their old
//! paths, so `crate::model::..` and `crate::ipc::contract::..` keep working:
//! [`aalc_model`] has no `nsh-core` dependency at all, and [`aalc_contract`]
//! only what `serde` needs.

pub mod i18n;
pub mod ipc;
pub mod state;
pub mod window;

mod runtime;

use nsh_core::prelude::*;

pub use crate::runtime::Runtime;
/// The contract's payloads, as `crate::model` (crate `aalc-model`).
pub use aalc_model as model;

/// The crate's UI set: what `App.tsx` imports from `aalc/native`.
#[native_module("aalc/native")]
fn native(registry: &mut Registry) {
    registry
        .module::<Runtime>()
        .module::<crate::ipc::bridge::Backend>()
        .module::<crate::i18n::I18n>()
        .class::<crate::state::teams::TeamsStore>()
        .class::<crate::state::other::SettingsStore>()
        .class::<crate::state::other::ThemePacksStore>()
        .class::<crate::state::other::ResourcesStore>()
        .class::<crate::state::other::ToolboxStore>()
        .class::<crate::state::home::HomeStore>()
        .class::<crate::state::app_state::AppStore>();
}
