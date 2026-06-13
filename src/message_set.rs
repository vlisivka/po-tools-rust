//! MessageSet — set operations on PoMessage using `to_key()` for identity.
//!
//! Provides `keyed_map`, `difference`, and `intersection` so that command modules
//! no longer duplicate HashMap boilerplate.

use crate::parser::PoMessage;
use std::collections::HashMap;

/// Builds a lookup map from a slice of messages using their identity key.
///
/// When multiple messages share the same key, the first one is kept.
pub fn keyed_map(messages: &[PoMessage]) -> HashMap<PoMessage, &PoMessage> {
    let mut map = HashMap::new();
    for message in messages {
        let key = message.to_key();
        map.entry(key).or_insert(message);
    }
    map
}

/// Returns messages present in `a` but not in `b`, by identity key.
#[allow(dead_code)]
pub fn difference<'a>(a: &'a [PoMessage], b: &[PoMessage]) -> Vec<&'a PoMessage> {
    let b_map = keyed_map(b);
    a.iter()
        .filter(|m| !b_map.contains_key(&m.to_key()))
        .collect()
}

/// Returns messages present in both `a` and `b`, by identity key.
#[allow(dead_code)]
pub fn intersection<'a>(a: &'a [PoMessage], b: &[PoMessage]) -> Vec<&'a PoMessage> {
    let b_map = keyed_map(b);
    a.iter()
        .filter(|m| b_map.contains_key(&m.to_key()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyed_map_returns_first_for_duplicate_keys() {
        let msgs = vec![
            PoMessage {
                msgid: "hello".to_string(),
                ..Default::default()
            },
            PoMessage {
                msgid: "hello".to_string(),
                msgstr: vec!["привіт".to_string()],
                ..Default::default()
            },
        ];

        let map = keyed_map(&msgs);
        // Should contain the first message (empty msgstr)
        let entry = map.get(&msgs[0].to_key()).unwrap();
        assert_eq!(entry.msgid, "hello");
        assert!(entry.msgstr.is_empty());
    }

    #[test]
    fn difference_returns_messages_only_in_first() {
        let a = vec![
            PoMessage {
                msgid: "a".to_string(),
                ..Default::default()
            },
            PoMessage {
                msgid: "b".to_string(),
                ..Default::default()
            },
        ];
        let b = vec![PoMessage {
            msgid: "b".to_string(),
            ..Default::default()
        }];

        let result = difference(&a, &b);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].msgid, "a");
    }

    #[test]
    fn intersection_returns_shared_messages() {
        let a = vec![
            PoMessage {
                msgid: "shared".to_string(),
                ..Default::default()
            },
            PoMessage {
                msgid: "only_a".to_string(),
                ..Default::default()
            },
        ];
        let b = vec![
            PoMessage {
                msgid: "shared".to_string(),
                msgstr: vec!["переклад".to_string()],
                ..Default::default()
            },
            PoMessage {
                msgid: "only_b".to_string(),
                ..Default::default()
            },
        ];

        let result = intersection(&a, &b);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].msgid, "shared");
    }
}
