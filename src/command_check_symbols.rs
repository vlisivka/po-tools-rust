//! Command to verify consistency of special symbols between source and translation.
//!
//! This module checks if symbols like `%d`, `{name}`, etc., are preserved
//! in the translated strings.

use crate::parser::{Parser, PoMessage};
use anyhow::{Result, bail};

fn strip_non_symbols(s: &str) -> String {
    s.chars()
        .filter(|c| !(c.is_alphanumeric() || c.is_whitespace() || *c == ','))
        .collect::<String>()
}

/// Checks a single message for symbol consistency.
///
/// Returns a warning message if symbols in `msgid` don't match those in `msgstr`.
pub fn check_symbols(message: &PoMessage) -> Option<String> {
    if message.is_header() {
        return None;
    }

    let msgid_syms = strip_non_symbols(&message.msgid);

    if message.is_plural() {
        for msgstr in &message.msgstr {
            let msgstr_syms = strip_non_symbols(msgstr);
            if msgid_syms != msgstr_syms {
                return Some(format!("{}", tr!("# Warning: Incorrect symbols:\n# msgid:  {msgid_syms}\n# msgstr: {msgstr_syms}\n").replace("{msgid_syms}", &msgid_syms).replace("{msgstr_syms}", &msgstr_syms)));
            }
        }
    } else {
        let msgstr_syms = strip_non_symbols(message.msgstr_first());
        if msgid_syms != msgstr_syms {
            return Some(format!("{}", tr!("# Warning: Incorrect symbols:\n# msgid:  {msgid_syms}\n# msgstr: {msgstr_syms}\n").replace("{msgid_syms}", &msgid_syms).replace("{msgstr_syms}", &msgstr_syms)));
        }
    }

    None
}

/// Implementation of the `check-symbols` command.
pub fn command_check_symbols(parser: &Parser, cmdline: &[&str]) -> Result<()> {
    match cmdline {
        ["-h", ..] | ["--help", ..] => help(),

        files if !files.is_empty() => {
            for file in files {
                let messages = parser.parse_messages_from_file(file)?;

                for message in messages.iter() {
                    if message.is_header() {
                        println!("{message}");
                    } else if let Some(errors) = check_symbols(message) {
                        println!("{errors}\n#, fuzzy\n{message}");
                    }
                }
            }
        }

        _ => bail!(tr!("At least one file is expected.")),
    }

    Ok(())
}

fn help() {
    println!(
        "{}",
        tr!(r#"Usage: po-tools check-symbols FILE[...]

Remove all alphanumeric characters, whitespace, and commas, then compare resulting strings.
"#)
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;

    #[test]
    fn header() -> Result<()> {
        let parser = Parser::new(None);

        let message = parser.parse_message_from_str(r#"
msgid "B<%man_recode%> B<-t> I<to-code> {\\|B<--suffix=>I<suffix\\/>\\||\\|B<--in-place>\\|} [\\|B<-dqhV>\\|] [\\|I<filename>\\|]"
msgstr "B<%man_recode%> B<-t> I<в-кодування> {\\|B<--suffix=>I<суфікс\\/>\\||\\|B<--in-place>\\|} [\\|B<-dqhV>\\|] [\\|I<імʼя_файлу>\\|]"
"#)?;

        let result = check_symbols(&message);

        assert_eq!(result, Some("# Warning: Incorrect symbols:\n# msgid:  <%_%><-><->{\\|<--=><\\/>\\||\\|<--->\\|}[\\|<->\\|][\\|<>\\|]\n# msgstr: <%_%><-><->{\\|<--=><\\/>\\||\\|<--->\\|}[\\|<->\\|][\\|<_>\\|]\n".into()));

        Ok(())
    }
}
