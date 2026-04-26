//! Command to verify consistency of special symbols between source and translation.
//!
//! This module checks if symbols like `%d`, `{name}`, etc., are preserved
//! in the translated strings.

use std::io::Write;

use crate::parser::{Parser, PoMessage};
use crate::util::IoContext;
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
                return Some(tr!("# Warning: Incorrect symbols:\n# msgid:  {msgid_syms}\n# msgstr: {msgstr_syms}\n").replace("{msgid_syms}", &msgid_syms).replace("{msgstr_syms}", &msgstr_syms));
            }
        }
    } else {
        let msgstr_syms = strip_non_symbols(message.msgstr_first());
        if msgid_syms != msgstr_syms {
            return Some(tr!("# Warning: Incorrect symbols:\n# msgid:  {msgid_syms}\n# msgstr: {msgstr_syms}\n").replace("{msgid_syms}", &msgid_syms).replace("{msgstr_syms}", &msgstr_syms));
        }
    }

    None
}

/// Implementation of the `check-symbols` command.
pub fn command_check_symbols(parser: &Parser, cmdline: &[&str], ctx: &mut IoContext) -> Result<()> {
    match cmdline {
        ["-h", ..] | ["--help", ..] => help(ctx.out),

        files if !files.is_empty() => {
            for file in files {
                let messages = parser.parse_messages_from_file(file)?;

                for message in messages.iter() {
                    if message.is_header() {
                        writeln!(ctx.out, "{message}")?;
                    } else if let Some(errors) = check_symbols(message) {
                        writeln!(ctx.out, "{errors}\n#, fuzzy\n{message}")?;
                    }
                }
            }
        }

        _ => bail!(tr!("At least one file is expected.")),
    }

    Ok(())
}

fn help(out: &mut dyn Write) {
    let _ = writeln!(
        out,
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
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_check_symbols_command_positive() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let f = NamedTempFile::new()?;
        fs::write(f.path(), "msgid \"hello %d\"\nmsgstr \"привіт %d\"\n")?;

        command_check_symbols(&parser, &[f.path().to_str().unwrap()], &mut ctx)?;

        let result = String::from_utf8(out)?;
        // If no symbols are missing, it should NOT print anything for the message (or only header if present)
        assert!(!result.contains("Warning"));
        Ok(())
    }

    #[test]
    fn test_check_symbols_command_negative() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let f = NamedTempFile::new()?;
        fs::write(f.path(), "msgid \"hello %d\"\nmsgstr \"привіт\"\n")?;

        command_check_symbols(&parser, &[f.path().to_str().unwrap()], &mut ctx)?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("Warning"));
        assert!(result.contains("msgid:  %"));
        assert!(result.contains("msgstr: "));
        Ok(())
    }

    #[test]
    fn test_help() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        command_check_symbols(&parser, &["--help"], &mut ctx)?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("Usage:"));
        Ok(())
    }

    #[test]
    fn test_no_files() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let result = command_check_symbols(&parser, &[], &mut ctx);
        assert!(result.is_err());
        Ok(())
    }
}
