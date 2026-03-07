use crate::parser::{Parser, PoMessage};
use anyhow::{Result, bail};
use std::collections::HashMap;

fn diff_by_str_and_print(m1: &PoMessage, m2: &PoMessage) -> Result<()> {
    use PoMessage::*;

    match m1 {
        Header { msgstr: msgstr1 } => match m2 {
            Header { msgstr: msgstr2 } => {
                if msgstr1 != msgstr2 {
                    println!(
                        "{}{m1}{}{m2}",
                        tr!("# Original header:\n"),
                        tr!("# New header:\n")
                    );
                }
            }
            _ => bail!(
                "{}.\n{m2}",
                tr!("Unexpected kind of PO message for comparison. Expected: header message. Got:")
            ),
        },

        Regular {
            msgstr: msgstr1, ..
        }
        | RegularWithContext {
            msgstr: msgstr1, ..
        } => match m2 {
            Regular {
                msgstr: msgstr2, ..
            }
            | RegularWithContext {
                msgstr: msgstr2, ..
            } => {
                if msgstr1 != msgstr2 {
                    println!(
                        "{}{m1}{}{m2}",
                        tr!("# Original message:\n"),
                        tr!("# New translation:\n")
                    );
                }
            }
            _ => {
                println!(
                    "{}{m1}{}{m2}",
                    tr!("# Original message:\n"),
                    tr!("# New message:\n")
                );
            }
        },

        Plural {
            msgstr: msgstr1, ..
        }
        | PluralWithContext {
            msgstr: msgstr1, ..
        } => match m2 {
            Plural {
                msgstr: msgstr2, ..
            }
            | PluralWithContext {
                msgstr: msgstr2, ..
            } => {
                if msgstr1.len() < msgstr2.len() {
                    println!(
                        "{}{m1}{}{m2}",
                        tr!("# Original message:\n"),
                        tr!("# New plural cases:\n")
                    );
                } else if msgstr1.len() > msgstr2.len() {
                    println!(
                        "{}{m1}{}{m2}",
                        tr!("# Original message:\n"),
                        tr!("# Removed plural cases:\n")
                    );
                } else if std::iter::zip(msgstr1, msgstr2)
                    .any(|(msgstr1, msgstr2)| msgstr1 != msgstr2)
                {
                    println!(
                        "{}{m1}{}{m2}",
                        tr!("# Original message:\n"),
                        tr!("# New translation:\n")
                    );
                }
            }
            _ => {
                println!(
                    "{}{m1}{}{m2}",
                    tr!("# Original message:\n"),
                    tr!("# New message:\n")
                );
            }
        },
    }

    Ok(())
}

pub fn command_diff_by_str_and_print(parser: &Parser, cmdline: &[&str]) -> Result<()> {
    match cmdline {
        ["-h", ..] | ["--help", ..] => {
            println!("{}", tr!("Usage: po-tools diffstr FILE FILE[S...]"));
        }
        [orig_file, files_to_diff @ ..] if !files_to_diff.is_empty() => {
            let orig_messages = parser.parse_messages_from_file(orig_file)?;

            let mut map: HashMap<PoMessage, &PoMessage> =
                HashMap::with_capacity(orig_messages.len());

            for m in orig_messages.iter() {
                map.insert(m.to_key(), m);
            }

            for file_to_diff in files_to_diff {
                println!("{}: {file_to_diff}\n", tr!("# File"));

                let messages_to_diff = parser.parse_messages_from_file(file_to_diff)?;

                for m in messages_to_diff {
                    if let Some(orig_message) = map.get(&m.to_key()) {
                        diff_by_str_and_print(orig_message, &m)?;
                    }
                }
            }
        }

        _ => {
            println!("ERROR: {}", tr!("at least two files are expected."));
        }
    }

    Ok(())
}
