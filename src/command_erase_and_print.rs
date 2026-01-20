use crate::parser::{Parser, PoMessage};
use anyhow::{bail, Result};

pub fn command_erase_and_print(parser: &Parser, cmdline: &[&str]) -> Result<()> {
    if cmdline.is_empty() {
        bail!("Expected one argument at least: name of the file to translate.");
    }

    for file in cmdline {
        let messages = parser.parse_messages_from_file(file)?;
        erase_and_print(&messages)?;
    }

    Ok(())
}

fn erase_and_print(messages: &Vec<PoMessage>) -> Result<()> {
    for message in messages {
        println!("{}", message.to_key());
    }

    Ok(())
}
