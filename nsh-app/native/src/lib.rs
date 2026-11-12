//! AALC's Native Shell backend crate (`nsh.json` `"rust": "native"`).
//!
//! `App.tsx` imports everything this crate registers from `aalc/native`; the
//! TypeScript declarations are generated from these Rust types into
//! `.nsh/types/aalc/native.d.ts`, so the TSX is checked against them.
//!
//! P0 registers only [`Runtime`]. The RPC gateway, the execution state and the
//! domain stores are ported in P1 — see `MIGRATION.md`.

mod runtime;

use nsh_core::prelude::*;

pub use crate::runtime::Runtime;

/// The crate's UI set: what `App.tsx` imports from `aalc/native`.
#[native_module("aalc/native")]
fn native(registry: &mut Registry) {
    registry.module::<Runtime>();
}
