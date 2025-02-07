use anyhow::{Result, bail};
use crate::parser::{Parser, PoMessage};

pub fn command_print_untranslated(parser: &Parser, cmdline: &[&str]) -> Result<()> {
  use PoMessage::*;

  match cmdline {
    [ "-h", .. ] | [ "--help", .. ] => println!("Usage: po-tools same ORIG_FILE FILE_TO_COMPARE[...]"),

    [ files @ ..  ] if files.len() > 0 => {
      for file in files {
         let messages = parser.parse_messages_from_file(file)?;

        for message in messages.iter() {
          match message {
            Regular{msgstr, ..}
            | RegularWithContext{msgstr, ..}
            if !msgstr.is_empty() => {},

            Plural{msgstr, ..}
            | PluralWithContext{msgstr, ..} => {
              for msgstr in msgstr {
                if msgstr.is_empty() {
                  println!("{message}");
                  break;
                }
              }
            }

            _ => println!("{message}"),
          }
        }
      }
    }

    _ => bail!("At least one file is expected."),
  }

  Ok(())
}
