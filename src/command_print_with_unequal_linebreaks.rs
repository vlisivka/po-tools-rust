use anyhow::{Result, bail};
use crate::parser::{Parser, PoMessage};

pub fn command_print_with_unequal_linebreaks(parser: &Parser, cmdline: &[&str]) -> Result<()> {
  use PoMessage::*;

  match cmdline {
    [ "-h", .. ] | [ "--help", .. ] => println!("Usage: po-tools with-unequal-linebreaks FILE[...]"),

    [ files @ ..  ] if files.len() > 0 => {
      for file in files {
         let messages = parser.parse_messages_from_file(file)?;

        for message in messages.iter() {
          match message {
            Regular{msgid, msgstr, ..} | RegularWithContext{msgid, msgstr, ..} => {
              let msgid_nl: u32 = msgid.matches('\n').map(|_| 1).sum();
              let msgstr_nl = msgstr.matches('\n').map(|_| 1).sum();
              if  msgid_nl != msgstr_nl {
                println!("{message}");
              }
            }
            Plural{msgid, msgstr, ..} | PluralWithContext{msgid, msgstr, ..}=> {
              let msgid_nl: u32 = msgid.matches('\n').map(|_| 1).sum();
              for msgstr in msgstr {
                let msgstr_nl = msgstr.matches('\n').map(|_| 1).sum();
                if  msgid_nl != msgstr_nl {
                  println!("{message}");
                }
              }
            }
            _ => {},
          }
        }
      }
    }

    _ => bail!("At least one file is expected."),
  }

  Ok(())
}
