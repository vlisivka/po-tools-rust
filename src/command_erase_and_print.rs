//! Command to remove all translations from a PO file.
//!
//! This is useful for creating a "template" or "empty" translation file
//! where only the `msgid` keys remain.

use crate::parser::{Parser, PoMessage};
use anyhow::{Result, bail};

/// Implementation of the `erase` command.
pub fn command_erase_and_print(parser: &Parser, cmdline: &[&str]) -> Result<()> {
    if cmdline.is_empty() {
        bail!(tr!(
            "Expected at least one argument: the name of the file with translations to erase."
        ));
    }

    for file in cmdline {
        let messages = parser.parse_messages_from_file(file)?;
        erase_and_print(&messages)?;
    }

    Ok(())
}

fn erase_and_print(messages: &[PoMessage]) -> Result<()> {
    for message in messages {
        println!("{}", message.to_key());
    }

    Ok(())
}
