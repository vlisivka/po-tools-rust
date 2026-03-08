//! Command to filter and print messages where `msgstr` contains a specific keyword.

use crate::parser::Parser;
use anyhow::{Result, bail};

/// Implementation of the `with-wordstr` command.
pub fn command_print_with_wordstr(parser: &Parser, cmdline: &[&str]) -> Result<()> {
    match cmdline {
        ["-h", ..] | ["--help", ..] => {
            println!("{}", tr!("Usage: po-tools with-wordstr KEYWORD FILE[...]"))
        }

        [keyword, files @ ..] if !files.is_empty() => {
            let keyword = keyword.to_lowercase();
            for file in files {
                let messages = parser.parse_messages_from_file(file)?;

                for message in messages.iter() {
                    if message.is_header() {
                        continue;
                    }

                    for msgstr in &message.msgstr {
                        if msgstr.to_lowercase().contains(&keyword) {
                            println!("{message}");
                            break;
                        }
                    }
                }
            }
        }

        _ => bail!(tr!("At least one file is expected.")),
    }

    Ok(())
}
