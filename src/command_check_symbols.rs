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

fn get_whitespace(s: &str) -> (String, String) {
    let leading = s.chars().take_while(|c| c.is_whitespace()).collect();
    let trailing = s
        .chars()
        .rev()
        .take_while(|c| c.is_whitespace())
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    (leading, trailing)
}

fn check_strings(src: &str, dst: &str) -> Option<String> {
    let mut warnings = String::new();

    // Symbols check
    let src_syms = strip_non_symbols(src);
    let dst_syms = strip_non_symbols(dst);
    if src_syms != dst_syms {
        warnings.push_str(
            &tr!(
                "# Warning: Incorrect symbols:\n# msgid:  {msgid_syms}\n# msgstr: {msgstr_syms}\n"
            )
            .replace("{msgid_syms}", &src_syms)
            .replace("{msgstr_syms}", &dst_syms),
        );
    }

    // Whitespace check
    let src_ws = get_whitespace(src);
    let dst_ws = get_whitespace(dst);
    if src_ws != dst_ws {
        warnings.push_str(
            &tr!("# Warning: Whitespace mismatch:\n# msgid:  \"{msgid_ws}\"\n# msgstr: \"{msgstr_ws}\"\n")
                .replace("{msgid_ws}", &format!("{}{}", src_ws.0, src_ws.1))
                .replace("{msgstr_ws}", &format!("{}{}", dst_ws.0, dst_ws.1)),
        );
    }

    if warnings.is_empty() {
        None
    } else {
        Some(warnings)
    }
}

/// Checks a single message for symbol consistency.
///
/// Returns a warning message if symbols in `msgid` don't match those in `msgstr`.
pub fn check_symbols(message: &PoMessage) -> Option<String> {
    if message.is_header() {
        return None;
    }

    if message.is_plural() {
        let mut all_warnings = String::new();
        for msgstr in &message.msgstr {
            if let Some(w) = check_strings(&message.msgid, msgstr) {
                all_warnings.push_str(&w);
            }
        }
        if all_warnings.is_empty() {
            None
        } else {
            Some(all_warnings)
        }
    } else {
        check_strings(&message.msgid, message.msgstr_first())
    }
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
