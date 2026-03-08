//! Command to sort messages in a PO file alphabetically by `msgctxt` and `msgid`.

use crate::parser::Parser;
use anyhow::{Result, bail};

/// Implementation of the `sort` command.
pub fn command_sort_and_print(parser: &Parser, cmdline: &[&str]) -> Result<()> {
    match cmdline {
        ["-h", ..] | ["--help", ..] => help(),

        [file] => {
            let messages = parser.parse_messages_from_file(file)?;

            let (headers, mut others): (Vec<_>, Vec<_>) =
                messages.into_iter().partition(|m| m.is_header());

            others.sort();

            for m in headers {
                println!("{m}");
            }
            for m in others {
                println!("{m}");
            }
        }

        _ => bail!(tr!(
            "Single argument is required: PO file to sort. See --help."
        )),
    }

    Ok(())
}

fn help() {
    println!("{}", tr!("Usage: po-tools sort FILE"));
}
