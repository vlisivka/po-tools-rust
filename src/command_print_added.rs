//! Commands to find added or removed messages between two PO files.
//!
//! `added` finds messages in the second file not present in the first.
//! `removed` finds messages in the first file not present in the second.

use crate::parser::{Parser, PoMessage};
use anyhow::{Result, bail};
use std::collections::HashMap;

/// Implementation of the `added` command.
pub fn command_print_added(parser: &Parser, cmdline: &[&str]) -> Result<()> {
    match cmdline {
        ["-h", ..] | ["--help", ..] => {
            println!(
                "{}",
                tr!("Usage: po-tools added ORIG_FILE FILE_TO_COMPARE[...]")
            )
        }

        [orig_file, files_to_diff @ ..] if !files_to_diff.is_empty() => {
            let messages1 = parser.parse_messages_from_file(orig_file)?;

            let mut map: HashMap<PoMessage, &PoMessage> = HashMap::with_capacity(messages1.len());

            for m in messages1.iter() {
                map.insert(m.to_key(), m);
            }

            for file_to_diff in files_to_diff {
                println!("{}: {file_to_diff}\n", tr!("# File"));

                let messages2 = parser.parse_messages_from_file(file_to_diff)?;

                for m in messages2 {
                    if !map.contains_key(&m.to_key()) {
                        println!("{m}")
                    }
                }
            }
        }

        _ => bail!(tr!("At least two files are required.")),
    }

    Ok(())
}

/// Implementation of the `removed` command.
pub fn command_print_removed(parser: &Parser, cmdline: &[&str]) -> Result<()> {
    if cmdline.len() < 2 {
        bail!(tr!("At least two files are required."));
    }
    let cmdline_rev = [cmdline[1], cmdline[0]];
    command_print_added(parser, &cmdline_rev)
}

pub fn command_diff_by_id_and_print(parser: &Parser, cmdline: &[&str]) -> Result<()> {
    println!("{}:\n", tr!("# Added messages"));
    command_print_added(parser, cmdline)?;

    println!("{}:\n", tr!("# Removed messages"));
    command_print_removed(parser, cmdline)?;

    Ok(())
}
