use crate::parser::{Parser, PoMessage};
use anyhow::{bail, Result};

pub fn command_print_with_word(parser: &Parser, cmdline: &[&str]) -> Result<()> {
    use PoMessage::*;

    match cmdline {
        ["-h", ..] | ["--help", ..] => println!("Usage: po-tools with-word KEYWORD FILE[...]"),

        [keyword, files @ ..] if !files.is_empty() => {
            for file in files {
                let messages = parser.parse_messages_from_file(file)?;

                for message in messages.iter() {
                    match message {
                        Regular { msgid, .. } | RegularWithContext { msgid, .. } => {
                            let mut msgid = msgid.clone();
                            msgid.make_ascii_lowercase();
                            if msgid.contains(keyword) {
                                println!("{message}");
                            }
                        }
                        Plural {
                            msgid,
                            msgid_plural,
                            ..
                        }
                        | PluralWithContext {
                            msgid,
                            msgid_plural,
                            ..
                        } => {
                            let mut msgid = msgid.clone();
                            msgid.make_ascii_lowercase();
                            let mut msgid_plural = msgid_plural.clone();
                            msgid_plural.make_ascii_lowercase();
                            if msgid.contains(keyword) || msgid_plural.contains(keyword) {
                                println!("{message}");
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        _ => bail!("At least one file is expected."),
    }

    Ok(())
}
