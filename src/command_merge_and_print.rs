//! Command to merge two PO files.
//!
//! Messages from the second file overwrite translations for the same keys
//! in the first file.

use crate::parser::{Parser, PoMessage};
use crate::util::IoContext;
use anyhow::{Result, bail};
use std::collections::HashMap;

/// Implementation of the `merge` command.
pub fn command_merge_and_print(
    parser: &Parser,
    cmdline: &[&str],
    ctx: &mut IoContext,
) -> Result<()> {
    match cmdline {
        ["-h", ..] | ["--help", ..] => {
            writeln!(ctx.out, "{}", tr!("Usage: po-tools merge FILE1 FILE2[...]"))?
        }

        [orig_file, files_to_merge @ ..] if !files_to_merge.is_empty() => {
            let messages1 = parser.parse_messages_from_file(orig_file)?;

            // Keyed by identity (msgctxt+msgid+msgid_plural). The entry value is the full message.
            let mut map: HashMap<PoMessage, PoMessage> = HashMap::new();

            for m in messages1 {
                map.insert(m.to_key(), m);
            }

            for file in files_to_merge {
                let messages2 = parser.parse_messages_from_file(file)?;

                for m in messages2 {
                    // Merge preserves comments from the replacement message.
                    // If you want to keep original comments, merge them manually here.
                    map.insert(m.to_key(), m);
                }
            }

            let mut vec = map.into_values().collect::<Vec<PoMessage>>();
            vec.sort();

            for m in vec {
                ctx.writer.write_message(&m, ctx.out)?;
            }
        }

        _ => bail!(tr!("At least two files are required.")),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_merge_positive() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
            writer: crate::po_file::PoFileWriter::default(),
        };
        let parser = Parser::new(None);

        let f1 = NamedTempFile::new()?;
        fs::write(f1.path(), "msgid \"a\"\nmsgstr \"old\"\n")?;

        let f2 = NamedTempFile::new()?;
        fs::write(f2.path(), "msgid \"a\"\nmsgstr \"new\"\n")?;

        command_merge_and_print(
            &parser,
            &[f1.path().to_str().unwrap(), f2.path().to_str().unwrap()],
            &mut ctx,
        )?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("msgstr \"new\""));
        assert!(!result.contains("msgstr \"old\""));
        Ok(())
    }

    #[test]
    fn test_merge_same_msgid_different_comments_and_translations() -> Result<()> {
        // Messages with the same msgid but different comments and translations
        // should merge into a single entry (the second file wins).
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
            writer: crate::po_file::PoFileWriter::default(),
        };
        let parser = Parser::new(None);

        let f1 = NamedTempFile::new()?;
        fs::write(
            f1.path(),
            "# Comment from file 1\nmsgid \"a\"\nmsgstr \"old_translation\"\n",
        )?;

        let f2 = NamedTempFile::new()?;
        fs::write(
            f2.path(),
            "# Different comment from file 2\nmsgid \"a\"\nmsgstr \"new_translation\"\n",
        )?;

        command_merge_and_print(
            &parser,
            &[f1.path().to_str().unwrap(), f2.path().to_str().unwrap()],
            &mut ctx,
        )?;

        let result = String::from_utf8(out)?;
        // Should contain exactly one message with the new file's data
        assert!(result.contains(r#"msgid "a""#));
        assert!(result.contains("new_translation"));
        assert!(!result.contains("old_translation"));
        // The merged output should have only 1 msgid line (not 2)
        let count = result.matches(r#"msgid "a""#).count();
        assert_eq!(count, 1);
        Ok(())
    }

    #[test]
    fn test_help() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
            writer: crate::po_file::PoFileWriter::default(),
        };
        let parser = Parser::new(None);

        command_merge_and_print(&parser, &["--help"], &mut ctx)?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("Usage:"));
        Ok(())
    }

    #[test]
    fn test_missing_files() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
            writer: crate::po_file::PoFileWriter::default(),
        };
        let parser = Parser::new(None);

        let result = command_merge_and_print(&parser, &["file1.po"], &mut ctx);
        assert!(result.is_err());
        Ok(())
    }
}
