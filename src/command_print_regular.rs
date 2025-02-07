use anyhow::{Result, bail};
use crate::parser::{Parser, PoMessage};

pub fn command_print_regular(parser: &Parser, cmdline: &[&str]) -> Result<()> {
  use PoMessage::*;

  match cmdline {
    [ "-h", .. ] | [ "--help", .. ] => println!("Usage: po-tools regular FILE[...]"),

    [ files @ ..  ] if files.len() > 0 => {
      for file in files {
         let messages = parser.parse_messages_from_file(file)?;

        for message in messages.iter() {
          match message {
            Regular{..} | RegularWithContext{..} => println!("{message}"),
            _ => {},
          }
        }
      }
    }

    _ => bail!("At least one file is expected."),
  }

  Ok(())
}
