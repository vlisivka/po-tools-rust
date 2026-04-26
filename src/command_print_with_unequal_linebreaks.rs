//! Command to find messages where the number of linebreaks in `msgid` and `msgstr` differ.
//!
//! This is often a sign of a formatting error in the translation.

use crate::parser::Parser;
use crate::util::IoContext;
use anyhow::{Result, bail};

/// Implementation of the `with-unequal-linebreaks` command.
pub fn command_print_with_unequal_linebreaks(
    parser: &Parser,
    cmdline: &[&str],
    ctx: &mut IoContext,
) -> Result<()> {
    match cmdline {
        ["-h", ..] | ["--help", ..] => writeln!(
            ctx.out,
            "{}",
            tr!("Usage: po-tools with-unequal-linebreaks FILE[...]")
        )?,

        files if !files.is_empty() => {
            for file in files {
                let messages = parser.parse_messages_from_file(file)?;

                for message in messages.iter() {
                    if message.is_header() {
                        writeln!(ctx.out, "{message}")?;
                    } else if !message.is_plural() {
                        let msgid_nl: u32 = message.msgid.matches('\n').map(|_| 1).sum();
                        let msgstr_nl: u32 = message.msgstr_first().matches('\n').map(|_| 1).sum();
                        if msgid_nl != msgstr_nl {
                            writeln!(ctx.out, "{message}")?;
                        }
                    } else {
                        let msgid_nl: u32 = message.msgid.matches('\n').map(|_| 1).sum();
                        for msgstr in &message.msgstr {
                            let msgstr_nl: u32 = msgstr.matches('\n').map(|_| 1).sum();
                            if msgid_nl != msgstr_nl {
                                writeln!(ctx.out, "{message}")?;
                                break; // no need to print multiple times
                            }
                        }
                    }
                }
            }
        }

        _ => bail!(tr!("At least one file is expected.")),
    }

    Ok(())
}
