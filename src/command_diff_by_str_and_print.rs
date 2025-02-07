use std::collections::HashMap;
use anyhow::{Result, bail};
use crate::parser::{Parser, PoMessage};

fn diff_by_str_and_print(m1: &PoMessage, m2: &PoMessage) -> Result<()> {
  use PoMessage::*;

  match m1 {
    Header { msgstr: msgstr1 } => {
      match m2 {
        Header { msgstr: msgstr2 } => {
          if msgstr1 != msgstr2 {
            println!("# Original header:\n{m1}# New header:\n{m2}");
          }
        },
        _ => bail!("Unexpected kind of PO message for comparison. Expected: header message. Got:\n{m2}"),
      }
    },

    Regular { msgstr: msgstr1, .. }
    | RegularWithContext { msgstr: msgstr1, .. } => {
      match m2 {
        Regular { msgstr: msgstr2, .. }
        | RegularWithContext { msgstr: msgstr2, .. } => {
          if msgstr1 != msgstr2 {
            println!("# Original message:\n{m1}# New translation:\n{m2}");
          }
        }
        _ => {
          println!("# Original message:\n{m1}# New message:\n{m2}");
        }
      }
    },

    Plural { msgstr: msgstr1, .. }
    | PluralWithContext { msgstr: msgstr1, .. } => {
      match m2 {
        Plural { msgstr: msgstr2, .. }
        | PluralWithContext { msgstr: msgstr2, .. } => {
          if msgstr1.len() < msgstr2.len() {
            println!("# Original message:\n{m1}# New plural cases:\n{m2}");
          } else if msgstr1.len() > msgstr2.len() {
            println!("# Original message:\n{m1}# Removed plural cases:\n{m2}");
          } else if std::iter::zip(msgstr1, msgstr2).any(|(msgstr1, msgstr2)| msgstr1 != msgstr2) {
            println!("# Original message:\n{m1}# New translation:\n{m2}");
          }
        }
        _ => {
          println!("# Original message:\n{m1}# New message:\n{m2}");
        }
      }
    },
  }

  Ok(())
}

pub fn command_diff_by_str_and_print(parser: &Parser, cmdline: &[&str]) -> Result<()> {
  match cmdline {
    [ "-h", .. ] | [ "--help", .. ] => { println!("Usage pot-tools diffstr FILE FILE[S...]"); },
    [ orig_file, files_to_diff @ .. ] if files_to_diff.len() > 0 => {
      let orig_messages = parser.parse_messages_from_file(orig_file)?;

      let mut map: HashMap<PoMessage, &PoMessage> = HashMap::with_capacity(orig_messages.len());

      for m in orig_messages.iter() {
        map.insert(m.to_key(), m);
      }

      for file_to_diff in files_to_diff {
        println!("# File: {file_to_diff}\n");

        let messages_to_diff = parser.parse_messages_from_file(file_to_diff)?;

        for m in messages_to_diff {
          if let Some(orig_message) = map.get(&m.to_key()) {
            diff_by_str_and_print(orig_message, &m)?;
          }
        }
      }
    }

    _ => { println!("ERROR: at least two files are expected."); },
  }

  Ok(())
}
