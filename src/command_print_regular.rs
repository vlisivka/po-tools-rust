use crate::parser::{Parser, PoMessage};
use anyhow::{Result, bail};

pub fn command_print_regular(parser: &Parser, cmdline: &[&str]) -> Result<()> {
    use PoMessage::*;

    match cmdline {
        ["-h", ..] | ["--help", ..] => println!("{}", tr!("Usage: po-tools regular FILE[...]")),

        files if !files.is_empty() => {
            for file in files {
                let messages = parser.parse_messages_from_file(file)?;

                for message in messages.iter() {
                    match message {
                        Regular { .. } | RegularWithContext { .. } => println!("{message}"),
                        _ => {}
                    }
                }
            }
        }

        _ => bail!(tr!("At least one file is expected.")),
    }

    Ok(())
}
