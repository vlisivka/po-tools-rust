//! Command to filter and print messages where `msgid` contains a specific keyword.

use crate::parser::Parser;
use crate::util::IoContext;
use anyhow::{Result, bail};

/// Implementation of the `with-word` command.
pub fn command_print_with_word(
    parser: &Parser,
    cmdline: &[&str],
    ctx: &mut IoContext,
) -> Result<()> {
    match cmdline {
        ["-h", ..] | ["--help", ..] => writeln!(
            ctx.out,
            "{}",
            tr!("Usage: po-tools with-word KEYWORD FILE[...]")
        )?,

        [keyword, files @ ..] if !files.is_empty() => {
            let keyword = keyword.to_lowercase();
            for file in files {
                let messages = parser.parse_messages_from_file(file)?;

                for message in messages.iter() {
                    if message.is_header() {
                        continue;
                    }

                    if message.msgid.to_lowercase().contains(&keyword) {
                        writeln!(ctx.out, "{message}")?;
                    } else if let Some(ref msgid_plural) = message.msgid_plural
                        && msgid_plural.to_lowercase().contains(&keyword)
                    {
                        writeln!(ctx.out, "{message}")?;
                    }
                }
            }
        }

        _ => bail!(tr!("At least one file is expected.")),
    }

    Ok(())
}
