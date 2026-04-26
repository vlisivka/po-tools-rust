//! Debugging command to parse a PO file and dump its internal representation.
//!
//! This is useful for verifying that the parser is correctly reading a file.

use crate::parser::Parser;
use crate::util::IoContext;
use anyhow::{Result, bail};
use std::io::Write;

/// Implementation of the `parse` command.
pub fn command_parse_and_dump(
    parser: &Parser,
    cmdline: &[&str],
    ctx: &mut IoContext,
) -> Result<()> {
    let mut multiline = false;
    let mut cmdline = cmdline;

    loop {
        match cmdline[..] {
            ["-m", ..] | ["--multiline", ..] => {
                multiline = true;
                cmdline = &cmdline[1..];
            }

            ["-h", ..] | ["-help", ..] | ["--help", ..] => {
                help_parse(ctx.out)?;
                return Ok(());
            }
            ["--", ..] => {
                cmdline = &cmdline[1..];
                break;
            }
            [arg, ..] if arg.starts_with('-') => {
                bail!(
                    tr!("Unknown option: \"{option}\". Use --help for list of options.")
                        .replace("{option}", arg)
                )
            }
            _ => break,
        }
    }

    if cmdline.is_empty() {
        bail!(tr!("Expected one argument only: name of the file to parse and dump. Actual arguments list: {arguments}").replace("{arguments}", &format!("{:?}", cmdline)));
    }

    for file in cmdline {
        let messages = parser.parse_messages_from_file(file)?;
        if multiline {
            writeln!(ctx.out, "{:#?}", messages)?;
        } else {
            writeln!(ctx.out, "{:?}", messages)?;
        }
    }

    Ok(())
}

fn help_parse(out: &mut dyn Write) -> Result<()> {
    writeln!(
        out,
        "{}",
        tr!(r#"Usage: po-tools [OPTIONS] [--] parse [OPTIONS] FILE

Parse a PO file and dump to standard output for debugging.
"#)
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_positive() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let f = NamedTempFile::new()?;
        fs::write(f.path(), "msgid \"a\"\nmsgstr \"b\"\n")?;

        command_parse_and_dump(&parser, &[f.path().to_str().unwrap()], &mut ctx)?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("msgid: \"a\""));
        assert!(result.contains("msgstr: [\"b\"]"));
        Ok(())
    }

    #[test]
    fn test_parse_multiline() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let f = NamedTempFile::new()?;
        fs::write(f.path(), "msgid \"a\"\nmsgstr \"b\"\n")?;

        command_parse_and_dump(&parser, &["-m", f.path().to_str().unwrap()], &mut ctx)?;

        let result = String::from_utf8(out)?;
        // Debug {:#?} format includes newlines
        assert!(result.contains("\n"));
        assert!(result.contains("msgid: \"a\""));
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

        command_parse_and_dump(&parser, &["--help"], &mut ctx)?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("Usage:"));
        Ok(())
    }

    #[test]
    fn test_unknown_option() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let result = command_parse_and_dump(&parser, &["--invalid"], &mut ctx);
        assert!(result.is_err());
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

        let result = command_parse_and_dump(&parser, &[], &mut ctx);
        assert!(result.is_err());
        Ok(())
    }
}
