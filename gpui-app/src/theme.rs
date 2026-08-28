#![allow(dead_code)]

//! Runtime theme derivation for the GPUI client.
//!
//! `AppSettings` remains the only persisted and writable source for theme
//! mode, accent, and language.  The types and helpers here are deliberately
//! snapshots: a root entity can derive a new snapshot after settings or the
//! system appearance changes, call `cx.notify()`, and render it without
//! maintaining a second mutable settings store.

pub use crate::components::style::{AccentId, ColorScheme, Palette};
use crate::model::AppSettings;
pub use crate::model::{Language, ThemeMode};

/// The stable fallback used by the web theme when a settings file contains an
/// unknown accent id.
pub const DEFAULT_ACCENT: &str = "crimson";

/// Return the platform appearance used when `ThemeMode::System` is selected.
///
/// The environment override keeps screenshot and CI runs deterministic. A
/// future Windows appearance listener can replace this seam without changing
/// the persisted settings contract.
pub fn system_is_dark() -> bool {
    matches!(
        std::env::var("AHAB_SYSTEM_THEME").as_deref(),
        Ok("dark") | Ok("Dark") | Ok("1") | Ok("true") | Ok("TRUE")
    )
}

/// Resolve a persisted mode against the current operating-system appearance.
/// `system_is_dark` is supplied by the platform integration so this module is
/// testable and does not invent another system-theme state.
pub const fn resolve_scheme(mode: ThemeMode, system_is_dark: bool) -> ColorScheme {
    match mode {
        ThemeMode::Light => ColorScheme::Light,
        ThemeMode::Dark => ColorScheme::Dark,
        ThemeMode::System => {
            if system_is_dark {
                ColorScheme::Dark
            } else {
                ColorScheme::Light
            }
        }
    }
}

/// Derive the complete visual palette from canonical persisted settings.
pub fn palette_for_settings(settings: &AppSettings, system_is_dark: bool) -> Palette {
    Palette::for_scheme(
        resolve_scheme(settings.themeMode, system_is_dark),
        AccentId::parse(&settings.accentId),
    )
}

/// A render-time view of settings. This is intentionally not serializable and
/// has no setters; update `AppSettings`, then derive a fresh snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ThemeSnapshot {
    pub mode: ThemeMode,
    pub accent: AccentId,
    pub language: Language,
    pub scheme: ColorScheme,
    pub palette: Palette,
}

impl ThemeSnapshot {
    pub fn from_settings(settings: &AppSettings, system_is_dark: bool) -> Self {
        let scheme = resolve_scheme(settings.themeMode, system_is_dark);
        let accent = AccentId::parse(&settings.accentId);
        Self {
            mode: settings.themeMode,
            accent,
            language: settings.language,
            scheme,
            palette: Palette::for_scheme(scheme, accent),
        }
    }

    pub const fn is_dark(self) -> bool {
        matches!(self.scheme, ColorScheme::Dark)
    }
}

/// Normalize an accent id at the settings boundary while retaining the
/// original string in `AppSettings` for forward-compatible persistence.
pub fn normalize_accent_id(accent_id: &str) -> &'static str {
    AccentId::parse(accent_id).as_str()
}

pub const fn language_tag(language: Language) -> &'static str {
    match language {
        Language::ZhCn => "zh-CN",
        Language::EnUs => "en-US",
    }
}

pub fn parse_language(tag: &str) -> Language {
    match tag {
        "en-US" | "en-us" | "en" => Language::EnUs,
        _ => Language::ZhCn,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_mode_resolves_without_mutating_settings() {
        assert_eq!(resolve_scheme(ThemeMode::Light, true), ColorScheme::Light);
        assert_eq!(resolve_scheme(ThemeMode::Dark, false), ColorScheme::Dark);
        assert_eq!(resolve_scheme(ThemeMode::System, true), ColorScheme::Dark);
        assert_eq!(resolve_scheme(ThemeMode::System, false), ColorScheme::Light);
    }

    #[test]
    fn snapshot_derives_all_runtime_values_from_app_settings() {
        let settings = AppSettings {
            themeMode: ThemeMode::Dark,
            accentId: "violet".into(),
            language: Language::EnUs,
            ..AppSettings::default()
        };
        let snapshot = ThemeSnapshot::from_settings(&settings, false);
        assert_eq!(snapshot.scheme, ColorScheme::Dark);
        assert_eq!(snapshot.accent, AccentId::Violet);
        assert_eq!(snapshot.language, Language::EnUs);
        assert_eq!(snapshot.palette, Palette::dark(AccentId::Violet));
    }

    #[test]
    fn language_tags_keep_the_model_contract() {
        assert_eq!(language_tag(Language::ZhCn), "zh-CN");
        assert_eq!(language_tag(Language::EnUs), "en-US");
        assert_eq!(parse_language("en-US"), Language::EnUs);
        assert_eq!(parse_language("unknown"), Language::ZhCn);
    }
}
