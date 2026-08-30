//! Settings cards grouped by the user-facing concern they edit.

use super::*;

mod appearance;
mod hotkeys;
mod notifications;
mod system;
mod updates;

pub(super) use appearance::appearance_card;
pub(super) use hotkeys::hotkey_card;
pub(super) use notifications::notification_card;
pub(super) use system::{experimental_card, system_card};
pub(super) use updates::{about_card, update_card};
