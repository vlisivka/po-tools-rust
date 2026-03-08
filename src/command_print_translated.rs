//! Command to filter and print only translated messages from a PO file.

use crate::parser::Parser;
use anyhow::{Result, bail};

/// Implementation of the `translated` command.
pub fn command_print_translated(parser: &Parser, cmdline: &[&str]) -> Result<()> {
    match cmdline {
        ["-h", ..] | ["--help", ..] => {
            println!("{}", tr!("Usage: po-tools translated FILE[...]"))
        }

        files if !files.is_empty() => {
            for file in files {
                let messages = parser.parse_messages_from_file(file)?;

                for message in messages.iter() {
                    if message.is_header() || message.is_translated() {
                        println!("{message}");
                    }
                }
            }
        }

        _ => bail!(tr!("At least one file is expected.")),
    }

    Ok(())
}
