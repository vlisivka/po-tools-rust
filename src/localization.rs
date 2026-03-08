//! Localization support for the application itself.
//!
//! This module handles loading translations from `.po` files and provides
//! the `tr!` macro for translating user-facing strings.

use crate::parser::{Parser, PoMessage};
use std::collections::HashMap;
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
        if std::path::Path::new(&file).exists() {
            if let Ok(msgs) = parser.parse_messages_from_file(&file) {
                messages = Some(msgs);
                break;
            }
        }
    }

    // 2. Fallback to embedded Ukrainian if LANG is uk and disk load failed
    #[cfg(feature = "embed-uk")]
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
                map.insert(msg.msgid.clone(), msg.msgstr_first().to_string());
            }
        }
    }

    TRANSLATIONS.set(map).ok();
}

/// Translates a string using the loaded translations.
///
/// If no translation is found, returns the original string.
pub fn translate(msgid: &str) -> &str {
    TRANSLATIONS
        .get()
        .and_then(|map| map.get(msgid))
        .map(|s| s.as_str())
        .unwrap_or(msgid)
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
