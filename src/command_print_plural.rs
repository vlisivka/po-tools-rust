//! Command to filter and print only plural messages from a PO file.

use crate::parser::Parser;
use anyhow::{Result, bail};

/// Implementation of the `plural` command.
pub fn command_print_plural(parser: &Parser, cmdline: &[&str]) -> Result<()> {
    match cmdline {
        ["-h", ..] | ["--help", ..] => println!("{}", tr!("Usage: po-tools plural FILE[...]")),

        files if !files.is_empty() => {
            for file in files {
                let messages = parser.parse_messages_from_file(file)?;

                for message in messages.iter() {
                    if message.is_plural() {
                        println!("{message}");
                    }
                }
            }
        }

        _ => bail!(tr!("At least one file is expected.")),
    }

    Ok(())
}
