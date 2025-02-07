use std::collections::HashMap;
use anyhow::{Result, bail};
use crate::parser::{Parser, PoMessage};

pub fn command_find_same_and_print(parser: &Parser, cmdline: &[&str]) -> Result<()> {
  match cmdline {
    [ "-h", .. ] | [ "--help", .. ] => println!("Usage: po-tools same ORIG_FILE FILE_TO_COMPARE[...]"),

    [ orig_file, files_to_diff @ ..  ] if files_to_diff.len() > 0 => {
      let messages1 = parser.parse_messages_from_file(orig_file)?;

      let mut map: HashMap<PoMessage, &PoMessage> = HashMap::with_capacity(messages1.len());

      for m in messages1.iter() {
        map.insert(m.to_key(), m);
      }

      for file_to_diff in files_to_diff {
        println!("# File: {file_to_diff}\n");

        let messages2 = parser.parse_messages_from_file(file_to_diff)?;

        for m2 in messages2.iter() {
          if let Some(m1) = map.get(&m2.to_key()) {
            if **m1 == *m2 {
              println!("{m2}");
            }
          }
        }
      }
    }

    _ => bail!("At least two files are required."),
  }

  Ok(())
}
