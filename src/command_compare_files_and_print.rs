use crate::parser::{Parser, PoMessage};
use anyhow::{Result, bail};

pub fn command_compare_files_and_print(parser: &Parser, cmdline: &[&str]) -> Result<()> {
    let skip_same = true;

    if cmdline.len() < 2 {
        bail!(tr!("At least two files are required to compare."));
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
        print!("{}:\n{m1}", tr!("# Variant 1"));

        let k1 = m1.to_key();

        for (j, msgs) in tail.iter().enumerate() {
            let j = j + 2;
            let k2 = msgs[i].to_key();

            if k2 != k1 {
                bail!("{}", tr!("To compare, msgid's must be same in all files. In message #{}, \"{}\" != \"{}\".")
                    .replace("{}", &i.to_string())
                    .replace("{}", &format!("{k1}"))
                    .replace("{}", &format!("{k2}")));
            }

            print!(
                "{}:\n{}",
                tr!("# Variant {}").replace("{}", &j.to_string()),
                msgs[i]
            );
        }

        println!();
    }

    Ok(())
}
