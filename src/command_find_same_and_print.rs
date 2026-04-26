use crate::parser::{Parser, PoMessage};
use crate::util::IoContext;
use anyhow::{Result, bail};
use std::collections::HashMap;

/// Implementation of the `same` command.
pub fn command_find_same_and_print(
    parser: &Parser,
    cmdline: &[&str],
    ctx: &mut IoContext,
) -> Result<()> {
    match cmdline {
        ["-h", ..] | ["--help", ..] => writeln!(
            ctx.out,
            "{}",
            tr!("Usage: po-tools same ORIG_FILE FILE_TO_COMPARE[...]")
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

                for m2 in messages2.iter() {
                    if let Some(m1) = map.get(&m2.to_key())
                        && **m1 == *m2
                    {
                        writeln!(ctx.out, "{m2}")?;
                    }
                }
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
    fn test_same_positive() -> Result<()> {
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
            "msgid \"a\"\nmsgstr \"b\"\n\nmsgid \"x\"\nmsgstr \"y\"\n",
        )?;

        let f2 = NamedTempFile::new()?;
        fs::write(
            f2.path(),
            "msgid \"a\"\nmsgstr \"b\"\n\nmsgid \"x\"\nmsgstr \"z\"\n",
        )?;

        command_find_same_and_print(
            &parser,
            &[f1.path().to_str().unwrap(), f2.path().to_str().unwrap()],
            &mut ctx,
        )?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("msgid \"a\""));
        assert!(!result.contains("msgid \"x\""));
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

        command_find_same_and_print(&parser, &["--help"], &mut ctx)?;

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

        let result = command_find_same_and_print(&parser, &["file1.po"], &mut ctx);
        assert!(result.is_err());
        Ok(())
    }
}
