//! Command to filter and print only messages with context (msgctxt) from a PO file.

use crate::parser::Parser;
use crate::util::IoContext;
use anyhow::{Result, bail};

/// Implementation of the `with-context` command.
pub fn command_print_with_context(
    parser: &Parser,
    cmdline: &[&str],
    ctx: &mut IoContext,
) -> Result<()> {
    match cmdline {
        ["-h", ..] | ["--help", ..] => {
            writeln!(ctx.out, "{}", tr!("Usage: po-tools with-context FILE[...]"))?
        }

        files if !files.is_empty() => {
            for file in files {
                let messages = parser.parse_messages_from_file(file)?;

                for message in messages.iter() {
                    if message.has_context() {
                        writeln!(ctx.out, "{message}")?;
                    }
                }
            }
        }

        _ => bail!(tr!("At least one file is expected.")),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_with_context_positive() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let f = NamedTempFile::new()?;
        fs::write(
            f.path(),
            "msgid \"a\"\nmsgstr \"b\"\n\nmsgctxt \"ctx\"\nmsgid \"c\"\nmsgstr \"d\"\n",
        )?;

        command_print_with_context(&parser, &[f.path().to_str().unwrap()], &mut ctx)?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("msgctxt \"ctx\""));
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

        command_print_with_context(&parser, &["--help"], &mut ctx)?;

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

        let result = command_print_with_context(&parser, &[], &mut ctx);
        assert!(result.is_err());
        Ok(())
    }
}
