//! Command to filter and print messages where `msgid` contains a specific keyword.

use crate::parser::Parser;
use crate::util::IoContext;
use anyhow::{Result, bail};

/// Implementation of the `with-word` command.
pub fn command_print_with_word(
    parser: &Parser,
    cmdline: &[&str],
    ctx: &mut IoContext,
) -> Result<()> {
    match cmdline {
        ["-h", ..] | ["--help", ..] => writeln!(
            ctx.out,
            "{}",
            tr!("Usage: po-tools with-word KEYWORD FILE[...]")
        )?,

        [keyword, files @ ..] if !files.is_empty() => {
            let keyword = keyword.to_lowercase();
            for file in files {
                let messages = parser.parse_messages_from_file(file)?;

                for message in messages.iter() {
                    if message.is_header() {
                        continue;
                    }

                    if message.msgid.to_lowercase().contains(&keyword) {
                        writeln!(ctx.out, "{message}")?;
                    } else if let Some(ref msgid_plural) = message.msgid_plural
                        && msgid_plural.to_lowercase().contains(&keyword)
                    {
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
    fn test_with_word_positive() -> Result<()> {
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
            "msgid \"hello world\"\nmsgstr \"\"\n\nmsgid \"bye\"\nmsgstr \"\"\n",
        )?;

        command_print_with_word(&parser, &["HELLO", f.path().to_str().unwrap()], &mut ctx)?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("hello world"));
        assert!(!result.contains("bye"));
        Ok(())
    }

    #[test]
    fn test_with_word_plural() -> Result<()> {
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
            "msgid \"apple\"\nmsgid_plural \"apples\"\nmsgstr[0] \"\"\n",
        )?;

        command_print_with_word(&parser, &["APPLES", f.path().to_str().unwrap()], &mut ctx)?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("msgid_plural \"apples\""));
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

        command_print_with_word(&parser, &["--help"], &mut ctx)?;

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

        let result = command_print_with_word(&parser, &["keyword"], &mut ctx);
        assert!(result.is_err());
        Ok(())
    }
}
