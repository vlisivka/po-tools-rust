//! Command to filter and print only regular messages (no plural, no context).

use crate::parser::Parser;
use anyhow::{Result, bail};

/// Implementation of the `regular` command.
pub fn command_print_regular(parser: &Parser, cmdline: &[&str]) -> Result<()> {
    match cmdline {
        ["-h", ..] | ["--help", ..] => println!("{}", tr!("Usage: po-tools regular FILE[...]")),

        files if !files.is_empty() => {
            for file in files {
                let messages = parser.parse_messages_from_file(file)?;

                for message in messages.iter() {
                    if !message.is_header() && !message.is_plural() {
                        println!("{message}");
                    }
                }
            }
        }

        _ => bail!(tr!("At least one file is expected.")),
    }

    Ok(())
}
