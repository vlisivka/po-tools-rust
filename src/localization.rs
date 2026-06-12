//! Localization support for the application itself.
//!
//! This module handles loading translations from `.po` files and provides
//! the `tr!` macro for translating user-facing strings.

use crate::parser::{Parser, PoMessage};
use std::collections::HashMap;
use std::io::Write;
use std::sync::OnceLock;

static TRANSLATIONS: OnceLock<HashMap<String, String>> = OnceLock::new();

/// Loads translations for the application from disk or embedded resources.
///
/// It checks for `.po` files based on the `LANG` environment variable.
pub fn load_translations(parser: &Parser) {
    let lang = std::env::var("LANGUAGE")
        .or_else(|_| std::env::var("LC_ALL"))
        .or_else(|_| std::env::var("LC_MESSAGES"))
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_else(|_| "C".to_string());

    // Try: po-tools.uk_UA.po, then po-tools.uk.po, then po-tools.po
    let lang_full = lang.split('.').next().unwrap_or(&lang);
    let lang_base = lang_full.split('_').next().unwrap_or(lang_full);

    let mut messages: Option<Vec<PoMessage>> = None;

    // 1. Try to load from disk
    let filenames = [
        format!("locales/{}.po", lang_full),
        format!("locales/{}.po", lang_base),
        format!("po-tools.{}.po", lang_full),
        format!("po-tools.{}.po", lang_base),
        "po-tools.po".to_string(),
    ];

    for file in filenames {
        if std::path::Path::new(&file).exists()
            && let Ok(msgs) = parser.parse_messages_from_file(&file)
        {
            messages = Some(msgs);
            break;
        }
    }

    // 2. Fallback to embedded Ukrainian if LANG is uk and disk load failed
    #[cfg(feature = "bundled-translations")]
    if messages.is_none() && lang_base == "uk" {
        let embedded = include_str!("../locales/uk.po");
        if let Ok(msgs) = parser.parse_messages_from_str(embedded) {
            messages = Some(msgs);
        }
    }

    let mut map = HashMap::new();
    if let Some(msgs) = messages {
        for msg in msgs {
            if !msg.is_header() && msg.is_translated() {
                // TODO: Support plural messages
                map.insert(msg.msgid.clone(), msg.msgstr_single().to_string());
            }
        }
    }

    TRANSLATIONS.set(map).ok();
}

/// Translates a string using the loaded translations.
///
/// If no translation is found, returns the original string.
/// If `PO_TOOLS_TRANSLATION_WARNINGS=1` environment variable is set,
/// emits a warning to stderr for each missing translation.
pub fn translate(msgid: &str) -> &str {
    let found = TRANSLATIONS
        .get()
        .and_then(|map| map.get(msgid))
        .map(|s| s.as_str());

    if found.is_none() && std::env::var("PO_TOOLS_TRANSLATION_WARNINGS").as_deref() == Ok("1") {
        let _ = std::io::stderr()
            .write_all(format!("warning: translation missing for \"{msgid}\"\n").as_bytes());
    }

    found.unwrap_or(msgid)
}

/// Macro for translating strings at runtime.
///
/// Usage: `tr!("Hello, world!")`
#[macro_export]
macro_rules! tr {
    ($msgid:expr) => {
        $crate::localization::translate($msgid)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_fallback() {
        // Translation not loaded or msgid missing, should return the same string
        assert_eq!(translate("non-existent-msgid"), "non-existent-msgid");
    }

    #[test]
    fn test_tr_macro() {
        assert_eq!(tr!("test message"), "test message");
    }

    #[test]
    #[ignore = "requires capturing stderr output"]
    fn test_translate_warning_with_env_var() {
        // Set the env var and verify that translate still returns the original msgid
        unsafe {
            std::env::set_var("PO_TOOLS_TRANSLATION_WARNINGS", "1");
        }
        assert_eq!(translate("non-existent-msgid-2"), "non-existent-msgid-2");
        unsafe {
            std::env::remove_var("PO_TOOLS_TRANSLATION_WARNINGS");
        }
    }
}
