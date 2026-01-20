use crate::parser::{Parser, PoMessage};
use anyhow::{bail, Result};

pub fn command_print_with_context(parser: &Parser, cmdline: &[&str]) -> Result<()> {
    use PoMessage::*;

    match cmdline {
        ["-h", ..] | ["--help", ..] => println!("Usage: po-tools with-context FILE[...]"),

        files if !files.is_empty() => {
            for file in files {
                let messages = parser.parse_messages_from_file(file)?;

                for message in messages.iter() {
                    match message {
                        RegularWithContext { .. } | PluralWithContext { .. } => {
                            println!("{message}")
                        }
                        _ => {}
                    }
                }
            }
        }

        _ => bail!("At least one file is expected."),
    }

    Ok(())
}
