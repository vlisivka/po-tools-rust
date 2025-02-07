use anyhow::{Result, bail};
use crate::parser::Parser;

pub fn command_sort_and_print(parser: &Parser, cmdline: &[&str]) -> Result<()> {

  match cmdline {
    [ "-h", ..] | [ "--help", .. ] => help(),

    [ file ] => {
      let mut messages = parser.parse_messages_from_file(file)?;

      messages.sort();

      messages.iter().for_each(|m| println!("{m}"));
    }

    _ => bail!("Single argument is required: PO file to sort. See --help.")
  }


  Ok(())
}

fn help() {
  println!("Usage: po-tools sort FILE");
}
