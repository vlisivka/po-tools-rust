//! Command to remove all translations from a PO file.
//!
//! This is useful for creating a "template" or "empty" translation file
//! where only the `msgid` keys remain.

use crate::parser::{Parser, PoMessage};
use crate::util::IoContext;
use anyhow::{Result, bail};

/// Implementation of the `erase` command.
pub fn command_erase_and_print(
    parser: &Parser,
    cmdline: &[&str],
    ctx: &mut IoContext,
) -> Result<()> {
    if cmdline.is_empty() {
        bail!(tr!(
            "Expected at least one argument: the name of the file with translations to erase."
        ));
    }

    for file in cmdline {
        let messages = parser.parse_messages_from_file(file)?;
        erase_and_print(ctx, &messages)?;
    }

    Ok(())
}

fn erase_and_print(ctx: &mut IoContext, messages: &[PoMessage]) -> Result<()> {
    for message in messages {
        writeln!(ctx.out, "{}", message.to_key())?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_erase_positive() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let f = NamedTempFile::new()?;
        fs::write(f.path(), "msgid \"a\"\nmsgstr \"delete me\"\n")?;

        command_erase_and_print(&parser, &[f.path().to_str().unwrap()], &mut ctx)?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("msgid \"a\""));
        assert!(result.contains("msgstr \"\""));
        assert!(!result.contains("delete me"));
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

        let result = command_erase_and_print(&parser, &[], &mut ctx);
        assert!(result.is_err());
        Ok(())
    }
}
