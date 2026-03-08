//! Command to filter and print only messages with context (msgctxt) from a PO file.

use crate::parser::Parser;
use anyhow::{Result, bail};

/// Implementation of the `with-context` command.
pub fn command_print_with_context(parser: &Parser, cmdline: &[&str]) -> Result<()> {
    match cmdline {
        ["-h", ..] | ["--help", ..] => {
            println!("{}", tr!("Usage: po-tools with-context FILE[...]"))
        }

        files if !files.is_empty() => {
            for file in files {
                let messages = parser.parse_messages_from_file(file)?;

                for message in messages.iter() {
                    if message.has_context() {
                        println!("{message}")
                    }
                }
            }
        }

        _ => bail!(tr!("At least one file is expected.")),
    }

    Ok(())
}
