//! Command to filter and print messages where `msgstr` contains a specific keyword.

use crate::parser::Parser;
use crate::util::IoContext;
use anyhow::{Result, bail};

/// Implementation of the `with-wordstr` command.
pub fn command_print_with_wordstr(
    parser: &Parser,
    cmdline: &[&str],
    ctx: &mut IoContext,
) -> Result<()> {
    match cmdline {
        ["-h", ..] | ["--help", ..] => writeln!(
            ctx.out,
            "{}",
            tr!("Usage: po-tools with-wordstr KEYWORD FILE[...]")
        )?,

        [keyword, files @ ..] if !files.is_empty() => {
            let keyword = keyword.to_lowercase();
            for file in files {
                let messages = parser.parse_messages_from_file(file)?;

                for message in messages.iter() {
                    if message.is_header() {
                        continue;
                    }

                    for msgstr in &message.msgstr {
                        if msgstr.to_lowercase().contains(&keyword) {
                            writeln!(ctx.out, "{message}")?;
                            break;
                        }
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
    fn test_with_wordstr_positive() -> Result<()> {
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
            "msgid \"a\"\nmsgstr \"HELLO world\"\n\nmsgid \"b\"\nmsgstr \"bye\"\n",
        )?;

        command_print_with_wordstr(&parser, &["HELLO", f.path().to_str().unwrap()], &mut ctx)?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("HELLO world"));
        assert!(!result.contains("bye"));
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

        command_print_with_wordstr(&parser, &["--help"], &mut ctx)?;

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

        let result = command_print_with_wordstr(&parser, &["keyword"], &mut ctx);
        assert!(result.is_err());
        Ok(())
    }
}
