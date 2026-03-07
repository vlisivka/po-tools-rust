use crate::parser::{Parser, PoMessage};
use std::collections::HashMap;
use std::sync::OnceLock;

static TRANSLATIONS: OnceLock<HashMap<String, String>> = OnceLock::new();

pub fn load_translations(parser: &Parser) {
    let lang = std::env::var("LANG").unwrap_or_else(|_| "C".to_string());

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
            match msg {
                PoMessage::Regular { msgid, msgstr } if !msgstr.is_empty() => {
                    map.insert(msgid, msgstr);
                }
                PoMessage::RegularWithContext { msgid, msgstr, .. } if !msgstr.is_empty() => {
                    map.insert(msgid, msgstr);
                }
                PoMessage::Plural { msgid, msgstr, .. } if !msgstr.is_empty() => {
                    map.insert(msgid, msgstr[0].clone());
                }
                PoMessage::PluralWithContext { msgid, msgstr, .. } if !msgstr.is_empty() => {
                    map.insert(msgid, msgstr[0].clone());
                }
                _ => {}
            }
        }
    }

    TRANSLATIONS.set(map).ok();
}

pub fn translate(msgid: &str) -> &str {
    TRANSLATIONS
        .get()
        .and_then(|map| map.get(msgid))
        .map(|s| s.as_str())
        .unwrap_or(msgid)
}

#[macro_export]
macro_rules! tr {
    ($msgid:expr) => {
        $crate::localization::translate($msgid)
    };
}
