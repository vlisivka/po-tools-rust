//! Command to find differences in translations (msgstr) between two PO files.
//!
//! Unlike standard `diff` which compares keys (`msgid`), this command
//! focuses on how the translations have changed for the same keys.

use crate::parser::{Parser, PoMessage};
use crate::util::IoContext;
use anyhow::{Result, bail};
use std::collections::HashMap;

fn diff_by_str_and_print(ctx: &mut IoContext, m1: &PoMessage, m2: &PoMessage) -> Result<()> {
    if m1.is_header() {
        if m2.is_header() {
            if m1.msgstr_first() != m2.msgstr_first() {
                writeln!(
                    ctx.out,
                    "{}{m1}{}{m2}",
                    tr!("# Original header:\n"),
                    tr!("# New header:\n")
                )?;
            }
        } else {
            bail!(
                "{}.\n{m2}",
                tr!("Unexpected kind of PO message for comparison. Expected: header message. Got:")
            );
        }
        return Ok(());
    }

    if !m1.is_plural() {
        if !m2.is_plural() {
            if m1.msgstr_first() != m2.msgstr_first() {
                writeln!(
                    ctx.out,
                    "{}{m1}{}{m2}",
                    tr!("# Original message:\n"),
                    tr!("# New translation:\n")
                )?;
            }
        } else {
            writeln!(
                ctx.out,
                "{}{m1}{}{m2}",
                tr!("# Original message:\n"),
                tr!("# New message:\n")
            )?;
        }
    } else if m2.is_plural() {
        if m1.msgstr.len() < m2.msgstr.len() {
            writeln!(
                ctx.out,
                "{}{m1}{}{m2}",
                tr!("# Original message:\n"),
                tr!("# New plural cases:\n")
            )?;
        } else if m1.msgstr.len() > m2.msgstr.len() {
            writeln!(
                ctx.out,
                "{}{m1}{}{m2}",
                tr!("# Original message:\n"),
                tr!("# Removed plural cases:\n")
            )?;
        } else if std::iter::zip(&m1.msgstr, &m2.msgstr).any(|(s1, s2)| s1 != s2) {
            writeln!(
                ctx.out,
                "{}{m1}{}{m2}",
                tr!("# Original message:\n"),
                tr!("# New translation:\n")
            )?;
        }
    } else {
        writeln!(
            ctx.out,
            "{}{m1}{}{m2}",
            tr!("# Original message:\n"),
            tr!("# New message:\n")
        )?;
    }

    Ok(())
}

/// Implementation of the `diffstr` command.
pub fn command_diff_by_str_and_print(
    parser: &Parser,
    cmdline: &[&str],
    ctx: &mut IoContext,
) -> Result<()> {
    match cmdline {
        ["-h", ..] | ["--help", ..] => {
            writeln!(
                ctx.out,
                "{}",
                tr!("Usage: po-tools diffstr FILE FILE[S...]")
            )?;
        }
        [orig_file, files_to_diff @ ..] if !files_to_diff.is_empty() => {
            let orig_messages = parser.parse_messages_from_file(orig_file)?;

            let mut map: HashMap<PoMessage, &PoMessage> =
                HashMap::with_capacity(orig_messages.len());

            for m in orig_messages.iter() {
                map.insert(m.to_key(), m);
            }

            for file_to_diff in files_to_diff {
                writeln!(ctx.out, "{}: {file_to_diff}\n", tr!("# File"))?;

                let messages_to_diff = parser.parse_messages_from_file(file_to_diff)?;

                for m in messages_to_diff {
                    if let Some(orig_message) = map.get(&m.to_key()) {
                        diff_by_str_and_print(ctx, orig_message, &m)?;
                    }
                }
            }
        }

        _ => {
            writeln!(
                ctx.err,
                "ERROR: {}",
                tr!("at least two files are expected.")
            )?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_diffstr_positive() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let f1 = NamedTempFile::new()?;
        fs::write(f1.path(), "msgid \"a\"\nmsgstr \"old\"\n")?;

        let f2 = NamedTempFile::new()?;
        fs::write(f2.path(), "msgid \"a\"\nmsgstr \"new\"\n")?;

        command_diff_by_str_and_print(
            &parser,
            &[f1.path().to_str().unwrap(), f2.path().to_str().unwrap()],
            &mut ctx,
        )?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("Original message"));
        assert!(result.contains("old"));
        assert!(result.contains("New translation"));
        assert!(result.contains("new"));
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

        command_diff_by_str_and_print(&parser, &["--help"], &mut ctx)?;

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

        command_diff_by_str_and_print(&parser, &["file1.po"], &mut ctx)?;

        let result_err = String::from_utf8(err)?;
        assert!(result_err.contains("ERROR"));
        Ok(())
    }
}
