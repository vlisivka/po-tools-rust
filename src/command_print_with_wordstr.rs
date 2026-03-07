use crate::parser::{Parser, PoMessage};
use anyhow::{Result, bail};

pub fn command_print_with_wordstr(parser: &Parser, cmdline: &[&str]) -> Result<()> {
    use PoMessage::*;

    match cmdline {
        ["-h", ..] | ["--help", ..] => {
            println!("{}", tr!("Usage: po-tools with-wordstr KEYWORD FILE[...]"))
        }

        [keyword, files @ ..] if !files.is_empty() => {
            for file in files {
                let messages = parser.parse_messages_from_file(file)?;

                for message in messages.iter() {
                    match message {
                        Regular { msgstr, .. } | RegularWithContext { msgstr, .. } => {
                            let mut msgstr = msgstr.clone();
                            msgstr.make_ascii_lowercase();
                            if msgstr.contains(keyword) {
                                println!("{message}");
                            }
                        }
                        Plural { msgstr, .. } | PluralWithContext { msgstr, .. } => {
                            for msgstr in msgstr {
                                let mut msgstr = msgstr.clone();
                                msgstr.make_ascii_lowercase();
                                if msgstr.contains(keyword) {
                                    println!("{message}");
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        _ => bail!(tr!("At least one file is expected.")),
    }

    Ok(())
}
