//! Commands to find added or removed messages between two PO files.
//!
//! `added` finds messages in the second file not present in the first.
//! `removed` finds messages in the first file not present in the second.

use crate::parser::{Parser, PoMessage};
use crate::util::IoContext;
use anyhow::{Result, bail};
use std::collections::HashMap;

/// Implementation of the `added` command.
pub fn command_print_added(parser: &Parser, cmdline: &[&str], ctx: &mut IoContext) -> Result<()> {
    match cmdline {
        ["-h", ..] | ["--help", ..] => writeln!(
            ctx.out,
            "{}",
            tr!("Usage: po-tools added ORIG_FILE FILE_TO_COMPARE[...]")
        )?,

        [orig_file, files_to_diff @ ..] if !files_to_diff.is_empty() => {
            let messages1 = parser.parse_messages_from_file(orig_file)?;

            let mut map: HashMap<PoMessage, &PoMessage> = HashMap::with_capacity(messages1.len());

            for m in messages1.iter() {
                map.insert(m.to_key(), m);
            }

            for file_to_diff in files_to_diff {
                writeln!(ctx.out, "{}: {file_to_diff}\n", tr!("# File"))?;

                let messages2 = parser.parse_messages_from_file(file_to_diff)?;

                for m in messages2 {
                    if !map.contains_key(&m.to_key()) {
                        writeln!(ctx.out, "{m}")?
                    }
                }
            }
        }

        _ => bail!(tr!("At least two files are required.")),
    }

    Ok(())
}

/// Implementation of the `removed` command.
pub fn command_print_removed(parser: &Parser, cmdline: &[&str], ctx: &mut IoContext) -> Result<()> {
    if cmdline.len() < 2 {
        bail!(tr!("At least two files are required."));
    }
    let cmdline_rev = [cmdline[1], cmdline[0]];
    command_print_added(parser, &cmdline_rev, ctx)
}

pub fn command_diff_by_id_and_print(
    parser: &Parser,
    cmdline: &[&str],
    ctx: &mut IoContext,
) -> Result<()> {
    writeln!(ctx.out, "{}:\n", tr!("# Added messages"))?;
    command_print_added(parser, cmdline, ctx)?;

    writeln!(ctx.out, "{}:\n", tr!("# Removed messages"))?;
    command_print_removed(parser, cmdline, ctx)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_added_positive() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let f1 = NamedTempFile::new()?;
        fs::write(f1.path(), "msgid \"a\"\nmsgstr \"\"\n")?;

        let f2 = NamedTempFile::new()?;
        fs::write(
            f2.path(),
            "msgid \"a\"\nmsgstr \"\"\n\nmsgid \"b\"\nmsgstr \"\"\n",
        )?;

        command_print_added(
            &parser,
            &[f1.path().to_str().unwrap(), f2.path().to_str().unwrap()],
            &mut ctx,
        )?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("msgid \"b\""));
        assert!(!result.contains("msgid \"a\""));
        Ok(())
    }

    #[test]
    fn test_removed_positive() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let f1 = NamedTempFile::new()?;
        fs::write(
            f1.path(),
            "msgid \"a\"\nmsgstr \"\"\n\nmsgid \"b\"\nmsgstr \"\"\n",
        )?;

        let f2 = NamedTempFile::new()?;
        fs::write(f2.path(), "msgid \"a\"\nmsgstr \"\"\n")?;

        command_print_removed(
            &parser,
            &[f1.path().to_str().unwrap(), f2.path().to_str().unwrap()],
            &mut ctx,
        )?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("msgid \"b\""));
        assert!(!result.contains("msgid \"a\""));
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

        command_print_added(&parser, &["--help"], &mut ctx)?;

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

        let result = command_print_added(&parser, &["file1.po"], &mut ctx);
        assert!(result.is_err());
        Ok(())
    }
}
