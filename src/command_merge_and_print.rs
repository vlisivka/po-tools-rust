use crate::parser::{Parser, PoMessage};
use anyhow::{bail, Result};
use std::collections::HashMap;

pub fn command_merge_and_print(parser: &Parser, cmdline: &[&str]) -> Result<()> {
    match cmdline {
        ["-h", ..] | ["--help", ..] => {
            println!("Usage: po-tools same ORIG_FILE FILE_TO_COMPARE[...]")
        }

        [orig_file, files_to_merge @ ..] if !files_to_merge.is_empty() => {
            let messages1 = parser.parse_messages_from_file(orig_file)?;

            let mut map: HashMap<PoMessage, PoMessage> = HashMap::new();

            for m in messages1 {
                map.insert(m.to_key(), m);
            }

            for file in files_to_merge {
                let messages2 = parser.parse_messages_from_file(file)?;

                for m in messages2 {
                    map.insert(m.to_key(), m);
                }
            }

            let mut vec = map.into_values().collect::<Vec<PoMessage>>();
            vec.sort();

            vec.iter().for_each(|m| println!("{m}"));
        }

        _ => bail!("Two files at least are required."),
    }

    Ok(())
}
