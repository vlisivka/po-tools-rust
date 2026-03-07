use crate::parser::Parser;
use anyhow::{Result, bail};

pub fn command_parse_and_dump(parser: &Parser, cmdline: &[&str]) -> Result<()> {
    let mut multiline = false;
    let mut cmdline = cmdline;

    loop {
        match cmdline[..] {
            ["-m", ..] | ["--multiline", ..] => {
                multiline = true;
                cmdline = &cmdline[1..];
            }

            ["-h", ..] | ["-help", ..] | ["--help", ..] => {
                help_parse();
                return Ok(());
            }
            ["--", ..] => {
                cmdline = &cmdline[1..];
                break;
            }
            [arg, ..] if arg.starts_with('-') => {
                bail!(
                    "{}",
                    tr!("Unknown option: \"{}\". Use --help for list of options.")
                        .replace("{}", arg)
                )
            }
            _ => break,
        }
    }

    if cmdline.is_empty() {
        bail!("{}", tr!("Expected one argument only: name of the file to parse and dump. Actual arguments list: {}").replace("{}", &format!("{:?}", cmdline)));
    }

    for file in cmdline {
        let messages = parser.parse_messages_from_file(file)?;
        if multiline {
            println!("{:#?}", messages);
        } else {
            println!("{:?}", messages);
        }
    }

    Ok(())
}

fn help_parse() {
    println!(
        "{}",
        tr!(r#"Usage: po-tools [OPTIONS] [--] parse [OPTIONS] FILE

Parse a PO file and dump to standard output for debugging.
"#)
    );
}
