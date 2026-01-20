use crate::parser::{Parser, PoMessage};
use anyhow::{bail, Result};

pub fn command_compare_files_and_print(parser: &Parser, cmdline: &[&str]) -> Result<()> {
    let skip_same = true;

    if cmdline.len() < 2 {
        bail!("At least two files are required to compare.");
    }

    let mut messages: Vec<Vec<PoMessage>> = Vec::new();
    for file in cmdline {
        let file_messages = parser.parse_messages_from_file(file)?;
        messages.push(file_messages);
    }

    for msgs in messages.iter_mut() {
        msgs.sort();
    }

    let (head, tail) = messages.split_at(1);

    'outer: for (i, m1) in head[0].iter().enumerate() {
        if skip_same && !tail.iter().any(|msgs| msgs[i] != *m1) {
            // All messages are same, skip them entirely
            println!("{m1}");
            continue 'outer;
        }

        //print!("# Message #{i} Variant 1:\n{m1}");
        print!("# Variant 1:\n{m1}");

        let k1 = m1.to_key();

        for (j, msgs) in tail.iter().enumerate() {
            let j = j + 2;
            let k2 = msgs[i].to_key();

            if k2 != k1 {
                bail!("To compare, msgid's must be same in all files. In message #{i}, \"{k1}\" != \"{k2}\".");
            }

            print!("# Variant {j}:\n{}", msgs[i]);
        }

        println!();
    }

    Ok(())
}
