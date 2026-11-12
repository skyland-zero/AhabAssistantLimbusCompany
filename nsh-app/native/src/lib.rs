//! AALC's Native Shell backend crate (`nsh.json` `"rust": "native"`).
//!
//! `App.tsx` imports everything this crate registers from `aalc/native`; the
//! TypeScript declarations are generated from these Rust types into
//! `.nsh/types/aalc/native.d.ts`, so the TSX is checked against them.
//!
//! P1 is being ported bottom-up: `model` (the contract's payloads) and `ipc`
//! (the mock and the real sidecar transport) land first, then `state`, `i18n`
//! and the app lifecycle. See `MIGRATION.md` for the source mapping.

pub mod ipc;
pub mod model;
pub mod state;

mod runtime;

use nsh_core::prelude::*;

pub use crate::runtime::Runtime;

/// The crate's UI set: what `App.tsx` imports from `aalc/native`.
#[native_module("aalc/native")]
fn native(registry: &mut Registry) {
    registry
        .module::<Runtime>()
        .module::<crate::ipc::bridge::Backend>();
}
