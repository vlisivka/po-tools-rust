use anyhow::{Result, bail};
use crate::parser::{Parser, PoMessage};

fn strip_non_symbols(s: &str) -> String {
  s.chars().filter(|c| !(c.is_alphanumeric() || c.is_whitespace() || *c == ',')).collect::<String>()
}

pub fn check_symbols(message: &PoMessage) -> Option<String> {
  use PoMessage::*;

  match message {
    Header{..} => return None,

    Regular{msgid, msgstr, ..} | RegularWithContext{msgid, msgstr, ..} => {
      let msgid_syms = strip_non_symbols(msgid);
      let msgstr_syms = strip_non_symbols(msgstr);
      if msgid_syms != msgstr_syms {
        return Some(format!("# Warning: Incorrect symbols:\n# msgid:  {msgid_syms}\n# msgstr: {msgstr_syms}\n"));
      }
    }

    // msgid_plural is ignored, because we don't know how to match plurals here.
    Plural{msgid, msgstr, ..} | PluralWithContext{msgid, msgstr, ..}=> {
      let msgid_syms = strip_non_symbols(msgid);
      for msgstr in msgstr {
        let msgstr_syms = strip_non_symbols(msgstr);
        if msgid_syms != msgstr_syms {
          return Some(format!("# Warning: Incorrect symbols:\n# msgid:  {msgid_syms}\n# msgstr: {msgstr_syms}\n"));
        }
      }
    }
  }

  None
}

pub fn command_check_symbols(parser: &Parser, cmdline: &[&str]) -> Result<()> {
  use PoMessage::*;

  match cmdline {
    [ "-h", .. ] | [ "--help", .. ] => help(),

    files if !files.is_empty() => {
      for file in files {
         let messages = parser.parse_messages_from_file(file)?;

        for message in messages.iter() {
          match message {
            Header{..} => println!("{message}"),

            _ => {
              if let Some(errors) = check_symbols(message) {
                println!("{errors}\n#, fuzzy\n{message}");
              }
            }
          }
        }
      }
    }

    _ => bail!("At least one file is expected."),
  }

  Ok(())
}

fn help() {
  println!(r#"
Usage: po-tools check-symbols FILE[...]

Remove all alphanumeric symbols, whitespace, and commas, then compare resulting strings.
"#);
}

#[cfg(test)]
mod tests {
  use anyhow::Result;
  use super::*;

  #[test]
  fn header() -> Result<()> {
    let parser = Parser::new(None);

    let message = parser.parse_message_from_str(r#"
msgid "B<%man_recode%> B<-t> I<to-code> {\\|B<--suffix=>I<suffix\\/>\\||\\|B<--in-place>\\|} [\\|B<-dqhV>\\|] [\\|I<filename>\\|]"
msgstr "B<%man_recode%> B<-t> I<в-кодування> {\\|B<--suffix=>I<суфікс\\/>\\||\\|B<--in-place>\\|} [\\|B<-dqhV>\\|] [\\|I<імʼя_файлу>\\|]"
"#)?;

    let result = check_symbols(&message);

    assert_eq!(result, Some("# Warning: Incorrect symbols:\n# msgid:  <%_%><-><->{\\|<--=><\\/>\\||\\|<--->\\|}[\\|<->\\|][\\|<>\\|]\n# msgstr: <%_%><-><->{\\|<--=><\\/>\\||\\|<--->\\|}[\\|<->\\|][\\|<_>\\|]\n".into()));

    Ok(())
  }
}
