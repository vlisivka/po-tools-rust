//! Dictionary support for translation terminology.
//!
//! This module provides the `Dictionary` struct for loading terminology
//! from TSV files and searching for matches in text.

use anyhow::{Context, Result};
use regex::Regex;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

/// A single entry in the dictionary.
#[derive(Debug, Clone)]
pub struct DictionaryEntry {
    /// The source term (usually in English).
    pub key: String,
    /// The translated term in the target language.
    pub translation: String,
    /// Pre-compiled regex for searching this term.
    pub regex: Regex,
}

/// A collection of dictionary entries.
#[derive(Debug)]
pub struct Dictionary {
    /// List of entries in the dictionary.
    pub entries: Vec<DictionaryEntry>,
}

impl Dictionary {
    /// Loads a dictionary from a TSV file.
    ///
    /// Each line should contain a term and its translation separated by a tab.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let file = File::open(&path).with_context(|| {
            format!(
                "{}: {:?}",
                tr!("Failed to open dictionary file"),
                path.as_ref()
            )
        })?;
        let reader = BufReader::new(file);
        let mut entries = Vec::new();

        for (line_no, line) in reader.lines().enumerate() {
            let line = line?;
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Split by tab char
            match line.split_once('\t') {
                Some((key, translation)) => {
                    let key = key.trim();
                    if key.is_empty() {
                        continue;
                    }
                    let escaped_key = regex::escape(key);
                    let pattern = format!(r"(?i)\b{}(s)?\b", escaped_key);
                    let regex = Regex::new(&pattern).with_context(|| {
                        format!("Failed to compile regex for dictionary key: {key}")
                    })?;

                    entries.push(DictionaryEntry {
                        key: key.to_string(),
                        translation: translation.trim().to_string(),
                        regex,
                    });
                }
                None => {
                    eprintln!(
                        "{}: \"{}\"",
                        tr!("WARNING: Invalid dictionary entry at line {line}: missing tab separator. Line:").replace("{line}", &(line_no + 1).to_string()),
                        line
                    );
                }
            }
        }

        Ok(Self { entries })
    }

    /// Finds all dictionary terms that appear in the given text.
    ///
    /// Search is case-insensitive and respects word boundaries. It also
    /// handles simple plurals (id + 's').
    pub fn find_matches<'a>(&'a self, text: &str) -> Vec<&'a DictionaryEntry> {
        let mut matches = Vec::new();

        for entry in &self.entries {
            if entry.regex.is_match(text) {
                matches.push(entry);
            }
        }

        matches
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_dictionary() {
        // We'll mock file reading by using Cursor in real impl, but here standard io::Result
        // actually from_file takes path, so let's stick to unit tests of logic if possible.
        // Since we can't easily mock file system here without extra crates,
        // let's test `find_matches` with manually created dict.

        let bug_re = Regex::new(r"(?i)\bbug(s)?\b").unwrap();
        let feat_re = Regex::new(r"(?i)\bfeature(s)?\b").unwrap();

        let dict = Dictionary {
            entries: vec![
                DictionaryEntry {
                    key: "bug".to_string(),
                    translation: "латка".to_string(),
                    regex: bug_re,
                },
                DictionaryEntry {
                    key: "feature".to_string(),
                    translation: "можливість".to_string(),
                    regex: feat_re,
                },
            ],
        };

        let matches = dict.find_matches("Fixing bugs in the system.");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].key, "bug");

        let matches = dict.find_matches("This feature is great.");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].key, "feature");

        let matches = dict.find_matches("No match here.");
        assert_eq!(matches.len(), 0);
    }
}
