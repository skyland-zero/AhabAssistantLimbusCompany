//! The TSX-facing catalog: stable keys and state feedback translations.

use nsh_core::prelude::*;

use crate::model::Language;

use super::{Key, feedback, text};

/// Localized shell and page copy.
#[module]
pub struct I18n;

fn language(value: &str) -> Language {
    match value {
        "enUs" | "en-US" | "en" => Language::EnUs,
        _ => Language::ZhCn,
    }
}

#[module]
impl I18n {
    /// The catalog entry for `key` in `language` (`zhCn` or `enUs`).
    /// Unknown keys return an empty string.
    pub fn text(language: String, key: String) -> String {
        Key::parse(&key)
            .map(|key| text(self::language(&language), key).to_owned())
            .unwrap_or_default()
    }

    /// Translates a state-layer feedback message for display.
    pub fn feedback(message: String, language: String) -> String {
        feedback(&message, self::language(&language))
    }
}
