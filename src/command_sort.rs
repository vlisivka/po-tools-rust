//! Command to sort messages in a PO file by `msgid`.
//!
//! This ensures that the PO file has a deterministic order, which is useful
//! for version control and comparing different versions of the file.

use crate::parser::Parser;
use crate::util::IoContext;
use anyhow::{Result, bail};

/// Implementation of the `sort` command.
pub fn command_sort_and_print(
    parser: &Parser,
    cmdline: &[&str],
    ctx: &mut IoContext,
) -> Result<()> {
    match cmdline {
        ["-h", ..] | ["--help", ..] => {
            writeln!(ctx.out, "{}", tr!("Usage: po-tools sort FILE[...]"))?
        }

        files if !files.is_empty() => {
            for file in files {
                let mut messages = parser.parse_messages_from_file(file)?;
                messages.sort();

                for m in messages {
                    writeln!(ctx.out, "{m}")?;
                }
            }
        }

        _ => bail!(tr!("At least one file is required.")),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    #[test]
    fn test_capture_output() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };

        let parser = Parser::new(None);

        // Create a temporary PO file
        let f = NamedTempFile::new().unwrap();
        fs::write(
            f.path(),
            "msgid \"b\"\nmsgstr \"B\"\n\nmsgid \"a\"\nmsgstr \"A\"\n",
        )
        .unwrap();

        let path_str = f.path().to_str().unwrap();
        command_sort_and_print(&parser, &[path_str], &mut ctx)?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("msgid \"a\""));
        assert!(result.contains("msgid \"b\""));

        // Check order
        let a_pos = result.find("msgid \"a\"").unwrap();
        let b_pos = result.find("msgid \"b\"").unwrap();
        assert!(a_pos < b_pos);

        Ok(())
    }
}
