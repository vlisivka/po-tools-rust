//! Command to compare translations across multiple PO files side-by-side.
//!
//! This module helps in identifying differences in how the same `msgid`
//! is translated in different files.

use crate::parser::{Parser, PoMessage};
use crate::util::IoContext;
use anyhow::{Result, bail};

/// Implementation of the `compare` command.
pub fn command_compare_files_and_print(
    parser: &Parser,
    cmdline: &[&str],
    ctx: &mut IoContext,
) -> Result<()> {
    let skip_same = true;

    if cmdline.len() < 2 {
        bail!(tr!("At least two files are required to compare."));
    }

    let mut messages: Vec<Vec<PoMessage>> = Vec::new();
    for file in cmdline {
        let file_messages = parser.parse_messages_from_file(file)?;
        messages.push(file_messages);
    }

    for msgs in messages.iter_mut() {
        msgs.sort();
    }

    let (head, tail) = messages.split_at(1);

    'outer: for (i, m1) in head[0].iter().enumerate() {
        if skip_same && !tail.iter().any(|msgs| msgs[i] != *m1) {
            // All messages are same, skip them entirely
            writeln!(ctx.out, "{m1}")?;
            continue 'outer;
        }

        //print!("# Message #{i} Variant 1:\n{m1}");
        write!(ctx.out, "{}:\n{m1}", tr!("# Variant 1"))?;

        let k1 = m1.to_key();

        for (j, msgs) in tail.iter().enumerate() {
            let j = j + 2;
            let k2 = msgs[i].to_key();

            if k2 != k1 {
                bail!(tr!("To compare, msgid's must be same in all files. In message #{index}, \"{key1}\" != \"{key2}\".")
                    .replace("{index}", &i.to_string())
                    .replace("{key1}", &format!("{k1}"))
                    .replace("{key2}", &format!("{k2}")));
            }

            write!(
                ctx.out,
                "{}:\n{}",
                tr!("# Variant {variant}").replace("{variant}", &j.to_string()),
                msgs[i]
            )?;
        }

        writeln!(ctx.out)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_compare_positive() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let f1 = NamedTempFile::new()?;
        fs::write(f1.path(), "msgid \"a\"\nmsgstr \"v1\"\n")?;

        let f2 = NamedTempFile::new()?;
        fs::write(f2.path(), "msgid \"a\"\nmsgstr \"v2\"\n")?;

        command_compare_files_and_print(
            &parser,
            &[f1.path().to_str().unwrap(), f2.path().to_str().unwrap()],
            &mut ctx,
        )?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("# Variant 1"));
        assert!(result.contains("# Variant 2"));
        assert!(result.contains("v1"));
        assert!(result.contains("v2"));
        Ok(())
    }

    #[test]
    fn test_compare_mismatched_msgids() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let f1 = NamedTempFile::new()?;
        fs::write(f1.path(), "msgid \"a\"\nmsgstr \"v1\"\n")?;

        let f2 = NamedTempFile::new()?;
        fs::write(f2.path(), "msgid \"b\"\nmsgstr \"v2\"\n")?;

        let result = command_compare_files_and_print(
            &parser,
            &[f1.path().to_str().unwrap(), f2.path().to_str().unwrap()],
            &mut ctx,
        );
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("msgid's must be same")
        );
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

        let result = command_compare_files_and_print(&parser, &["file1.po"], &mut ctx);
        assert!(result.is_err());
        Ok(())
    }
}
