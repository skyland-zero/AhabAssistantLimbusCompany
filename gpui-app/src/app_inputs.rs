use gpui::{Entity, Subscription};

use crate::components::TextInput;

/// Text entities and observers owned by the team editor.
///
/// Keeping these handles together makes their lifecycle explicit: opening an
/// editor creates the group, while closing it drops the group and its
/// subscriptions as one unit.
#[derive(Default)]
pub struct TeamInputs {
    pub name: Option<Entity<TextInput>>,
    pub code: Option<Entity<TextInput>>,
    pub observe: Option<Entity<TextInput>>,
    pub json: Option<Entity<TextInput>>,
    pub keyword_refresh: Option<Entity<TextInput>>,
    pub normal_refresh: Option<Entity<TextInput>>,
    pub subscriptions: Vec<Subscription>,
}

/// Text entities and observers owned by the settings page.
#[derive(Default)]
pub struct SettingsInputs {
    pub cdk: Option<Entity<TextInput>>,
    pub port: Option<Entity<TextInput>>,
    pub timeout: Option<Entity<TextInput>>,
    pub subscriptions: Vec<Subscription>,
}
