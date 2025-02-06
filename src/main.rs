use std::collections::HashMap;
use anyhow::{Result, bail};

mod parser;
use self::parser::{Parser, PoMessage};

mod command_sort;
use self::command_sort::command_sort_and_print;

fn command_parse_and_dump(multiline: bool, messages: &Vec<PoMessage>) -> Result<()> {
  if multiline {
    println!("{:#?}", messages);
  } else {
    println!("{:?}", messages);
  }

  Ok(())
}

fn command_merge_and_print(parser: &Parser, cmdline: &[&str]) -> Result<()> {

  match cmdline {
    [ "-h", .. ] | [ "--help", .. ] => println!("Usage: po-tools same ORIG_FILE FILE_TO_COMPARE[...]"),

    [ orig_file, files_to_merge @ ..  ] if files_to_merge.len() > 0 => {
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

fn command_print_added(parser: &Parser, cmdline: &[&str]) -> Result<()> {
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

        for m in messages2 {
          if !map.contains_key(&m.to_key()) {
            println!("{m}")
          }
        }
      }
    }

    _ => bail!("Two files at least are required."),
  }

  Ok(())
}

fn command_print_removed(parser: &Parser, cmdline: &[&str]) -> Result<()> {
  let cmdline_rev = [ cmdline[1], cmdline[0] ];
  command_print_added(parser, &cmdline_rev)
}

fn command_find_same_and_print(parser: &Parser, cmdline: &[&str]) -> Result<()> {
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

fn command_diff_by_id_and_print(parser: &Parser, cmdline: &[&str]) -> Result<()> {
  println!("# Added messages\n");
  command_print_added(parser, cmdline)?;

  println!("# Removed messages\n");
  command_print_removed(parser, cmdline)?;

  Ok(())
}

fn diff_by_str_and_print(m1: &PoMessage, m2: &PoMessage) -> Result<()> {
  use PoMessage::*;

  match m1 {
    Header { msgstr: msgstr1 } => {
      match m2 {
        Header { msgstr: msgstr2 } => {
          if msgstr1 != msgstr2 {
            println!("# Original header:\n{m1}# New header:\n{m2}");
          }
        },
        _ => bail!("Unexpected kind of PO message for comparison. Expected: header message. Got:\n{m2}"),
      }
    },

    Regular { msgstr: msgstr1, .. }
    | RegularWithContext { msgstr: msgstr1, .. } => {
      match m2 {
        Regular { msgstr: msgstr2, .. }
        | RegularWithContext { msgstr: msgstr2, .. } => {
          if msgstr1 != msgstr2 {
            println!("# Original message:\n{m1}# New translation:\n{m2}");
          }
        }
        _ => {
          println!("# Original message:\n{m1}# New message:\n{m2}");
        }
      }
    },

    Plural { msgstr: msgstr1, .. }
    | PluralWithContext { msgstr: msgstr1, .. } => {
      match m2 {
        Plural { msgstr: msgstr2, .. }
        | PluralWithContext { msgstr: msgstr2, .. } => {
          if msgstr1.len() < msgstr2.len() {
            println!("# Original message:\n{m1}# New plural cases:\n{m2}");
          } else if msgstr1.len() > msgstr2.len() {
            println!("# Original message:\n{m1}# Removed plural cases:\n{m2}");
          } else if std::iter::zip(msgstr1, msgstr2).any(|(msgstr1, msgstr2)| msgstr1 != msgstr2) {
            println!("# Original message:\n{m1}# New translation:\n{m2}");
          }
        }
        _ => {
          println!("# Original message:\n{m1}# New message:\n{m2}");
        }
      }
    },
  }

  Ok(())
}

fn command_diff_by_str_and_print(parser: &Parser, cmdline: &[&str]) -> Result<()> {
  match cmdline {
    [ "-h", .. ] | [ "--help", .. ] => { println!("Usage pot-tools diffstr FILE FILE[S...]"); },
    [ orig_file, files_to_diff @ .. ] if files_to_diff.len() > 0 => {
      let orig_messages = parser.parse_messages_from_file(orig_file)?;

      let mut map: HashMap<PoMessage, &PoMessage> = HashMap::with_capacity(orig_messages.len());

      for m in orig_messages.iter() {
        map.insert(m.to_key(), m);
      }

      for file_to_diff in files_to_diff {
        println!("# File: {file_to_diff}\n");

        let messages_to_diff = parser.parse_messages_from_file(file_to_diff)?;

        for m in messages_to_diff {
          if let Some(orig_message) = map.get(&m.to_key()) {
            diff_by_str_and_print(orig_message, &m)?;
          }
        }
      }
    }

    _ => { println!("ERROR: at least two files are expected."); },
  }

  Ok(())
}

fn pipe_to_command(command: &str, args: &[&str], text: &str) -> Result<String> {
  use std::process::{Command, Stdio};
  use std::io::Write;

  let mut child = Command::new(command)
    .args(args)
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .spawn()?;

  let mut stdin = child.stdin.take().unwrap();
  let text = text.to_string();
  std::thread::spawn(move || {
    stdin.write_all(text.as_bytes()).expect("Cannot write to stdin");
  });

  let output = child.wait_with_output()?;
  if output.status.success() {
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
  } else {
    bail!("Command \"{command}\" failed with non-zero exit code. Command args: {:?}", args)
  }
}

fn command_translate_and_print(aichat_command: &str, aichat_options: &[&str], language: &str, number_of_plural_cases: Option<usize>, dictionary: &str, messages: &Vec<PoMessage>) -> Result<()> {

  let mut prev_message = PoMessage::Regular { msgid: "--help\tPrint this help message.".to_string(), msgstr: "--help\tНадрукувати цю довідку.".to_string() };
  let parser = Parser{ number_of_plural_cases };

  for message in messages {
    match message {
      // Pass header untranslated
      PoMessage::Header{..} => { println!("{message}"); },

      PoMessage::Regular{..} | PoMessage::RegularWithContext{..} => {
        // Translation template
        let message_text = format!(r#"
<instruction>
Act as technical translator for Gettext .po files.
Translate PO message in <message></message> tag to {language} Language. IMPORTANT: Copy msgid field verbatim, put translation into msgstr field.
Resulting message must be correct Gettext PO Message, wrapped in <message></message> tag.
In translated message, msgid field must be copied intact first, then msgstr field must be translation of msgid to {language} language.
IMPORTANT: Start with "<message> msgid ".
</instruction>
<message>
{message}
</message>
<example>
{prev_message}
</example>
<dictionary>
{dictionary}
</dictionary>
"#);

        // Translate
        let new_message_text = pipe_to_command(aichat_command, aichat_options, &message_text)?;
        //eprintln!("@@@@\n{new_message_text}\n@@@@\n");

        // Extract text between <message> and </message>, if they are present
        let new_message_text_slice = if let (Some(start), Some(end)) = (new_message_text.find("<message>"), new_message_text.find("</message>")) {
          let tag_len="<message>".len();
          &new_message_text[(start+tag_len) .. end]
        } else {
          &new_message_text[..]
        };

        let new_message_chars = new_message_text_slice.chars().chain("\n".chars()).collect::<Vec<char>>();

        match parser.parse_message(&new_message_chars[..]) {
          Ok(new_message) =>  {
            if message.to_key() == new_message.to_key() {
              println!("# Translated message:\n#, fuzzy\n{new_message}");
              //prev_message = new_message;
            } else {
              eprintln!("# ERROR: Wrong msgid field when trying to translate. Replacing wrong ID with correct id.\n# Translation:\n=====\n{new_message_text_slice}\n=====");
              let fixed_message = new_message.with_key(&message.to_key());
              println!("# Translated message (WARNING: wrong id after translation):\n#, fuzzy\n{fixed_message}");
            }
          },

          Err(e) => {
            eprintln!("# ERROR: Cannot parse translation of message: {:#}:\n{message}\n# Translation:\n=====\n{new_message_text_slice}\n=====", e);
            println!("#UNTranslated message (cannot parse translation):\n#, fuzzy\n{message}");
          },
        }
      },

      PoMessage::Plural{..} | PoMessage::PluralWithContext{..} => {
        let number_of_plural_cases = if let Some(number_of_plural_cases) = number_of_plural_cases { number_of_plural_cases } else { 2 };
        // Translation template
        let message_text = format!(r#"
<instruction>
Act as technical translator for Gettext .po files.
Translate PO message in <message></message> tag to {language} Language. IMPORTANT: Copy msgid and msgid_plural fields verbatim,
put translation into msgstr[] fields. Resulting message must be correct Gettext PO Message, wrapped in <message></message> tag.
In translated message, msgid and msgid_plural fields must be copied intact first, then all {number_of_plural_cases} msgstr[] fields must be translation
of msgid and msgid_plural to {language} language. IMPORTANT: Start with "<message> msgid ".
</instruction>
<message>
{message}
</message>
<example>
msgid "%s new patch,"
msgid_plural "%s new patches,"
msgstr[0] "%s нова латка,"
msgstr[1] "%s нові латки,"
msgstr[2] "%s нових латок,"
</example>
<dictionary>
{dictionary}
</dictionary>
"#);

        // Translate
        let new_message_text = pipe_to_command(aichat_command, aichat_options, &message_text)?;
        //eprintln!("@@@@\n{new_message_text}\n@@@@\n");

        let parser = Parser{ number_of_plural_cases: Some(number_of_plural_cases) };

        // Extract text between <message> and </message>, if they are present
        let new_message_text_slice = if let (Some(start), Some(end)) = (new_message_text.find("<message>"), new_message_text.find("</message>")) {
          let tag_len="<message>".len();
          &new_message_text[(start+tag_len) .. end]
        } else {
          &new_message_text[..]
        };

        let new_message_chars = new_message_text_slice.chars().chain("\n".chars()).collect::<Vec<char>>();

        match parser.parse_message(&new_message_chars[..]) {
          Ok(new_message) =>  {
            if message.to_key() == new_message.to_key() {
              println!("# Translated message:\n#, fuzzy\n{new_message}");
              prev_message = new_message;
            } else {
              eprintln!("# ERROR: Wrong msgid field when trying to translate. Replacing wrong ID with correct id.\n# Translation:\n=====\n{new_message_text_slice}\n=====");
              let fixed_message = new_message.with_key(&message.to_key());
              println!("# Translated message (WARNING: wrong id after translation):\n#, fuzzy\n{fixed_message}");
            }
          },

          Err(e) => {
            eprintln!("# ERROR: Cannot parse translation of message: {:#}:\n{message}\n# Translation:\n=====\n{new_message_text_slice}\n=====", e);
            println!("#UNTranslated message (cannot parse translation):\n#, fuzzy\n{message}");
          },
        }
      }
    }
  }

  Ok(())
}

fn command_print_translated(parser: &Parser, cmdline: &[&str]) -> Result<()> {
  use PoMessage::*;

  match cmdline {
    [ "-h", .. ] | [ "--help", .. ] => println!("Usage: po-tools same ORIG_FILE FILE_TO_COMPARE[...]"),

    [ files @ ..  ] if files.len() > 0 => {
      for file in files {
         let messages = parser.parse_messages_from_file(file)?;

        'outer: for message in messages.iter() {
          match message {
            Regular{msgstr, ..}
            | RegularWithContext{msgstr, ..}
            if msgstr.is_empty() => {},

            Plural{msgstr, ..}
            | PluralWithContext{msgstr, ..} => {
              for msgstr in msgstr {
                if msgstr.is_empty() {
                  continue 'outer;
                }
              }

              println!("{message}");
            }

            _ => println!("{message}"),
          }
        }
      }
    }

    _ => bail!("At least one file is expected."),
  }

  Ok(())
}

fn command_print_untranslated(parser: &Parser, cmdline: &[&str]) -> Result<()> {
  use PoMessage::*;

  match cmdline {
    [ "-h", .. ] | [ "--help", .. ] => println!("Usage: po-tools same ORIG_FILE FILE_TO_COMPARE[...]"),

    [ files @ ..  ] if files.len() > 0 => {
      for file in files {
         let messages = parser.parse_messages_from_file(file)?;

        for message in messages.iter() {
          match message {
            Regular{msgstr, ..}
            | RegularWithContext{msgstr, ..}
            if !msgstr.is_empty() => {},

            Plural{msgstr, ..}
            | PluralWithContext{msgstr, ..} => {
              for msgstr in msgstr {
                if msgstr.is_empty() {
                  println!("{message}");
                  break;
                }
              }
            }

            _ => println!("{message}"),
          }
        }
      }
    }

    _ => bail!("At least one file is expected."),
  }

  Ok(())
}

fn command_print_regular(parser: &Parser, cmdline: &[&str]) -> Result<()> {
  use PoMessage::*;

  match cmdline {
    [ "-h", .. ] | [ "--help", .. ] => println!("Usage: po-tools same ORIG_FILE FILE_TO_COMPARE[...]"),

    [ files @ ..  ] if files.len() > 0 => {
      for file in files {
         let messages = parser.parse_messages_from_file(file)?;

        for message in messages.iter() {
          match message {
            Regular{..} | RegularWithContext{..} => println!("{message}"),
            _ => {},
          }
        }
      }
    }

    _ => bail!("At least one file is expected."),
  }

  Ok(())
}

fn command_print_plural(parser: &Parser, cmdline: &[&str]) -> Result<()> {
  use PoMessage::*;

  match cmdline {
    [ "-h", .. ] | [ "--help", .. ] => println!("Usage: po-tools same ORIG_FILE FILE_TO_COMPARE[...]"),

    [ files @ ..  ] if files.len() > 0 => {
      for file in files {
         let messages = parser.parse_messages_from_file(file)?;

        for message in messages.iter() {
          match message {
            Plural{..} | PluralWithContext{..} => println!("{message}"),
            _ => {},
          }
        }
      }
    }

    _ => bail!("At least one file is expected."),
  }

  Ok(())
}

fn command_print_with_context(parser: &Parser, cmdline: &[&str]) -> Result<()> {
  use PoMessage::*;

  match cmdline {
    [ "-h", .. ] | [ "--help", .. ] => println!("Usage: po-tools same ORIG_FILE FILE_TO_COMPARE[...]"),

    [ files @ ..  ] if files.len() > 0 => {
      for file in files {
         let messages = parser.parse_messages_from_file(file)?;

        for message in messages.iter() {
          match message {
            RegularWithContext{..} | PluralWithContext{..} => println!("{message}"),
            _ => {},
          }
        }
      }
    }

    _ => bail!("At least one file is expected."),
  }

  Ok(())
}

fn command_print_with_word(parser: &Parser, cmdline: &[&str]) -> Result<()> {
  use PoMessage::*;

  match cmdline {
    [ "-h", .. ] | [ "--help", .. ] => println!("Usage: po-tools same ORIG_FILE FILE_TO_COMPARE[...]"),

    [ keyword, files @ ..  ] if files.len() > 0 => {
      for file in files {
         let messages = parser.parse_messages_from_file(file)?;

        for message in messages.iter() {
          match message {
            Regular{msgid, ..} | RegularWithContext{msgid, ..} => {
              let mut msgid = msgid.clone();
              msgid.make_ascii_lowercase();
              if msgid.contains(keyword) {
                println!("{message}");
              }
            }
            Plural{msgid, msgid_plural, ..} | PluralWithContext{msgid, msgid_plural, ..}=> {
              let mut msgid = msgid.clone();
              msgid.make_ascii_lowercase();
              let mut msgid_plural = msgid_plural.clone();
              msgid_plural.make_ascii_lowercase();
              if msgid.contains(keyword) || msgid_plural.contains(keyword) {
                println!("{message}");
              }
            }
            _ => {},
          }
        }
      }
    }

    _ => bail!("At least one file is expected."),
  }

  Ok(())
}

fn command_print_with_wordstr(parser: &Parser, cmdline: &[&str]) -> Result<()> {
  use PoMessage::*;

  match cmdline {
    [ "-h", .. ] | [ "--help", .. ] => println!("Usage: po-tools same ORIG_FILE FILE_TO_COMPARE[...]"),

    [ keyword, files @ ..  ] if files.len() > 0 => {
      for file in files {
         let messages = parser.parse_messages_from_file(file)?;

        for message in messages.iter() {
          match message {
            Regular{msgstr, ..} | RegularWithContext{msgstr, ..} => {
              let mut msgstr = msgstr.clone();
              msgstr.make_ascii_lowercase();
              if msgstr.contains(keyword) {
                println!("{message}");
              }
            }
            Plural{msgstr, ..} | PluralWithContext{msgstr, ..}=> {
              for msgstr in msgstr {
                let mut msgstr = msgstr.clone();
                msgstr.make_ascii_lowercase();
                if msgstr.contains(keyword) {
                  println!("{message}");
                }
              }
            }
            _ => {},
          }
        }
      }
    }

    _ => bail!("At least one file is expected."),
  }

  Ok(())
}

fn command_print_with_unequal_linebreaks(parser: &Parser, cmdline: &[&str]) -> Result<()> {
  use PoMessage::*;

  match cmdline {
    [ "-h", .. ] | [ "--help", .. ] => println!("Usage: po-tools same ORIG_FILE FILE_TO_COMPARE[...]"),

    [ files @ ..  ] if files.len() > 0 => {
      for file in files {
         let messages = parser.parse_messages_from_file(file)?;

        for message in messages.iter() {
          match message {
            Regular{msgid, msgstr, ..} | RegularWithContext{msgid, msgstr, ..} => {
              let msgid_nl: u32 = msgid.matches('\n').map(|_| 1).sum();
              let msgstr_nl = msgstr.matches('\n').map(|_| 1).sum();
              if  msgid_nl != msgstr_nl {
                println!("{message}");
              }
            }
            Plural{msgid, msgstr, ..} | PluralWithContext{msgid, msgstr, ..}=> {
              let msgid_nl: u32 = msgid.matches('\n').map(|_| 1).sum();
              for msgstr in msgstr {
                let msgstr_nl = msgstr.matches('\n').map(|_| 1).sum();
                if  msgid_nl != msgstr_nl {
                  println!("{message}");
                }
              }
            }
            _ => {},
          }
        }
      }
    }

    _ => bail!("At least one file is expected."),
  }

  Ok(())
}

fn command_compare_files_and_print(skip_same: bool, parser: &Parser, cmdline: &[&str]) -> Result<()> {

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
    if skip_same {
      if !tail.iter().any(|msgs| msgs[i] != *m1) {
        // All messages are same, skip them entirely
        println!("{m1}");
        continue 'outer;
      }
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

fn command_review_files_and_print(aichat_command: &str, aichat_options: &[&str], language: &str, number_of_plural_cases: Option<usize>, dictionary: &str, mut messages: Vec<Vec<PoMessage>>) -> Result<()> {

  let parser = Parser{ number_of_plural_cases };

  for msgs in messages.iter_mut() {
    msgs.sort();
  }

  let (head, tail) = messages.split_at(1);

  'outer: for (i, message) in head[0].iter().enumerate() {
    if !tail.iter().any(|msgs| msgs[i] != *message) {
      // All messages are same, skip review
      println!("# All translations are same:\n{message}");
      continue 'outer;
    }

    let mut text = format!("# Variant 1:\n{message}");

    let k1 = message.to_key();

    for (j, msgs) in tail.iter().enumerate() {
      let j = j + 2;
      let k2 = msgs[i].to_key();

      if k2 != k1 {
        bail!("To review, msgid's must be same in all files. In message #{i}, \"{k1}\" != \"{k2}\".");
      }

      text = format!("{text}# Variant {j}:\n{}", msgs[i]);
    }

    text += "\n";

    // Review messages
    let message_text = format!(r#"
<instruction>
Act as technical translator for Gettext .po files.
Review PO message translation variants in <message></message> tag to {language} Language. List cons for varians in <review></review> tag.
Check for technical correctness, translation correctness, correct gender, correct plural form, correct line breaks.
Chose variant pleased for a native speaker in {language} language.
Wrire review in <review></review> tag first, then write one correct PO message without flaws in <message></message> tag.
Example:
<review> the review </review>
<message>
msgid "text"
msgstr "текст"
</message>
Resulting message must be correct Gettext PO Message, wrapped in <message></message> tag.
IMPORTANT: Copy msgid field verbatim, put translation into msgstr field.
In translated message, msgid field must be copied intact first, then msgstr field must be translation of msgid to {language} language.
IMPORTANT: Start with "<message> msgid ".
</instruction>
<message>
{text}
</message>
<dictionary>
{dictionary}
</dictionary>
"#);
    //eprintln!("{message_text}");

    // Translate
    let new_message_text = pipe_to_command(aichat_command, aichat_options, &message_text)?;
    //eprintln!("# Review:\n{new_message_text}\n");

    // Extract text between <message> and </message>, if they are present
    let new_message_text_slice = if let (Some(start), Some(end)) = (new_message_text.find("<message>"), new_message_text.find("</message>")) {
      let tag_len="<message>".len();
      &new_message_text[(start+tag_len) .. end]
    } else {
      &new_message_text[..]
    };

    let new_message_chars = new_message_text_slice.chars().chain("\n".chars()).collect::<Vec<char>>();

    match parser.parse_message(&new_message_chars[..]) {
      Ok(new_message) =>  {
        if message.to_key() == new_message.to_key() {
          println!("# Reviewed message:\n#, fuzzy\n{new_message}");
        } else {
          eprintln!("# ERROR: Wrong msgid field when trying to review:\n{message}\n# Review:\n=====\n{new_message_text_slice}\n=====");
          println!("# UNReviewed message (wrong id after review):\n#, fuzzy\n{message}");
        }
      },

      Err(e) => {
        eprintln!("# ERROR: Cannot parse review of message: {:#}:\n{message}\n# Review:\n=====\n{new_message_text_slice}\n=====", e);
        println!("#UNReviewed message (cannot parse review):\n#, fuzzy\n{message}");
      },
    }
  }

  Ok(())
}

fn main() -> Result<()> {

  let dictionary = r#"
patch - латка
bug - помилка
"#;
  let mut language = "Ukrainian";
  let mut model = "ollama:phi4:14b-q8_0";
  let mut role = "translate-po";
  let aichat_command = "aichat";

  // Options
  let mut number_of_plural_cases: Option<usize> = None;

  // Parse aruments
  let args = std::env::args().collect::<Vec<String>>();
  let tail = &args[1..].iter().map(|s| &s as &str).collect::<Vec<&str>>();
  let mut tail = &tail[..];

  // Parse options
  loop {
    match tail[..] {
      [ "-c", n, ..] | [ "--cases", n, ..] => {
        match n.parse::<usize>() {
          Ok(n) if n >= 1 && n < 10 => {
            number_of_plural_cases = Some(n);
            tail = &tail[2..];
          }
          _ => bail!("Invalid argument for -c | --cases option. Expected: number of plural cases between 1 and 9. Actual value: \"{n}\"."),
        }
      }

      [ "-h", .. ] | [ "-help", .. ] | [ "--help", .. ] => {
        help();
        return Ok(());
      }
      [ "--", .. ] => {
        tail = &tail[1..];
        break;
      }
      [ arg, ..] if arg.starts_with('-') => bail!("Unknown option: \"{arg}\". Use --help for list of options."),
      _ => break,
    }
  }

  let parser = Parser{ number_of_plural_cases };

  // Parse arguments
  match tail[..] {
    [ "parse", ..] => {
      // Parse "parse" command options
      let mut multiline = false;
      let mut tail = &tail[1..];
      loop {
        match tail[..] {
          [ "-m", ..] | [ "--multiline", ..] => {
            multiline = true;
            tail = &tail[1..];
          }

          [ "-h", .. ] | [ "-help", .. ] | [ "--help", .. ] => {
            help_parse();
            return Ok(());
          }
          [ "--", .. ] => {
            tail = &tail[1..];
            break;
          }
          [ arg, ..] if arg.starts_with('-') => bail!("Unknown option: \"{arg}\". Use --help for list of options."),
          _ => break,
        }
      }

      match tail[..] {
        [ file ] => {
          let messages = parser.parse_messages_from_file(file)?;
          command_parse_and_dump(multiline, &messages)?;
        }
        _ => bail!("Expected one argument only: name of the file to parse and dump. Actual list of arguments: {:?}", tail),
      }
    }

    [ "translate", .. ] => {
      // Parse "translate" command options
      let mut tail = &tail[1..];
      loop {
        match tail[..] {
          [ "-m", model_name, ..] | [ "--model", model_name, ..] => {
            model = model_name;
            tail = &tail[2..];
          }

          [ "-r", role_name, ..] | [ "--role", role_name, ..] => {
            role = role_name;
            tail = &tail[2..];
          }

          [ "-l", lang_name, ..] | [ "--lang", lang_name, ..] | [ "--language", lang_name, ..] => {
            language = lang_name;
            tail = &tail[2..];
          }

          [ "-h", .. ] | [ "-help", .. ] | [ "--help", .. ] => {
            help_translate();
            return Ok(());
          }
          [ "--", .. ] => {
            tail = &tail[1..];
            break;
          }
          [ arg, ..] if arg.starts_with('-') => bail!("Unknown option: \"{arg}\". Use --help for list of options."),
          _ => break,
        }
      }

      match tail[..] {
        [ file ] => {
          let messages = parser.parse_messages_from_file(file)?;
          command_translate_and_print(aichat_command, &[ "-r", role, "-m", model ], language, number_of_plural_cases, dictionary, &messages)?;
        }
        _ => bail!("Expected one argument only: name of the file to parse and dump. Actual list of arguments: {:?}", tail),
      }
    }

    [ "review", .. ] => {
      // Parse "review" command options
      let mut model = "ollama:phi4:14b-q8_0";
      let mut language = "Ukrainian";
      let mut tail = &tail[1..];
      loop {
        match tail[..] {
          [ "-m", model_name, ..] | [ "--model", model_name, ..] => {
            model = model_name;
            tail = &tail[2..];
          }

          [ "-r", role_name, ..] | [ "--role", role_name, ..] => {
            role = role_name;
            tail = &tail[2..];
          }

          [ "-l", lang_name, ..] | [ "--lang", lang_name, ..] | [ "--language", lang_name, ..] => {
            language = lang_name;
            tail = &tail[2..];
          }

          [ "-h", .. ] | [ "-help", .. ] | [ "--help", .. ] => {
            help_review();
            return Ok(());
          }
          [ "--", .. ] => {
            tail = &tail[1..];
            break;
          }
          [ arg, ..] if arg.starts_with('-') => bail!("Unknown option: \"{arg}\". Use --help for list of options."),
          _ => break,
        }
      }

      let mut messages = Vec::new();
      for file in tail {
        let file_messages = parser.parse_messages_from_file(file)?;
        messages.push(file_messages);
      }
      command_review_files_and_print(aichat_command, &[ "-r", role, "-m", model ], language, number_of_plural_cases, dictionary, messages)?;
    }

    [ "compare", ref cmdline @ ..  ] => command_compare_files_and_print(true, &parser, cmdline)?,
    [ "sort", ref cmdline @ .. ] => command_sort_and_print(&parser, cmdline)?,
    [ "merge", ref cmdline @ .. ] => command_merge_and_print(&parser, cmdline)?,
    [ "diff", ref cmdline @ .. ] => command_diff_by_id_and_print(&parser, cmdline)?,
    [ "diffstr", ref cmdline @ .. ] => command_diff_by_str_and_print(&parser, cmdline)?,
    [ "same", ref cmdline @ .. ] => command_find_same_and_print(&parser, cmdline)?,
    [ "added", ref cmdline @ .. ] => command_print_added(&parser, cmdline)?,
    [ "removed", ref cmdline @ .. ] => command_print_removed(&parser, cmdline)?,
    [ "translated", ref cmdline @ .. ] => command_print_translated(&parser, cmdline)?,
    [ "untranslated", ref cmdline @ .. ] => command_print_untranslated(&parser, cmdline)?,
    [ "regular", ref cmdline @ .. ] => command_print_regular(&parser, cmdline)?,
    [ "plural", ref cmdline @ .. ] => command_print_plural(&parser, cmdline)?,
    [ "with-context", ref cmdline @ .. ] => command_print_with_context(&parser, cmdline)?,
    [ "with-word", ref cmdline @ .. ] => command_print_with_word(&parser, cmdline)?,
    [ "with-wordstr", ref cmdline @ .. ] => command_print_with_wordstr(&parser, cmdline)?,
    [ "with-unequal-linebreaks", ref cmdline @ .. ] => command_print_with_unequal_linebreaks(&parser, cmdline)?,

    // TODO: split commands and their arguments into separate files
    // TODO: check: count of special tokens in msgid vs msgstr
    // TODO: check: strip spaces, lettes and numbers, then compare strings, to check correctness of special symbols
    // TODO: check: spaces at beginning/ending of msgstr as in msgid
    // TODO: check: capital letter at beginning of msgs as in msgid
    // TODO: filter: without words
    // TODO: try to fix messages after an problem with message is found after translation or review
    // TODO: multiline/singleline
    // TODO: check: spelling

    [ "help", .. ] | [] => help(),
    [ arg, ..] => bail!("Unknown command: \"{arg}\". Use --help for list of commands."),

  }

  Ok(())
}

fn help() {
  println!(r#"
Usage: po-tools [OPTIONS] [--] COMMAND [COMMAND_OPTIONS] [--] [COMMAND_ARGUMENTS]

COMMANDS:

  * translate [OPTIONS] FILE - WIP! translate PO file using AI.
  * review [OPTIONS] FILE [FILE...] - WIP! review multiple translations of _same_ file using AI.

  * merge FILE1 FILE2 - merge two files by overwritting messages from FILE1 by messages from FILE2.

  * diff FILE1 FILE2 - diff two files by msgid.
  * diffstr FILE1 FILE2 - diff two files by msgstr.
  * added FILE1 FILE2 - print new messages in FILE2 only.
  * deleted FILE1 FILE2 - print missing messages from FILE1 only.

  * translated FILE - print messages with non-empty msgstr.
  * untranslated FILE - print messages with empty msgstr (even if just one msgstr is empty for plural messages).
  * regular FILE - print regular PO messages, not ones with context or plural messages.
  * plural FILE - print plural messages only.
  * with-context FILE - print messages with msgctxt field.
  * with-word WORD FILE - print messages with given word in msgid.
  * with-wordstr WORD FILE - print messages with given word in msgstr.
  * with-unequal-linebreaks - print messages where msgstr doesn't contain same number of linebreaks as msgid.

  * sort FILE - sort messages in lexical order.
  * parse - parse file and dump (for debugging)

"#);
}

fn help_parse() {
  println!(r#"
Usage: po-tools [OPTIONS] [--] parse [OPTIONS] FILE

Parse a PO file and dump to standard output for debugging.

"#);
}

fn help_translate() {
  println!(r#"
Usage: po-tools [GLOBAL_OPTIONS] translate [-m MODEL|-l LANG] [--] FILE

WORK IN PROGRESS.

Translate messages in PO file using AI tools (aichat, ollama).

OPTIONS:

  -l | --language LANG  Language to use. Default value: "Ukrainian".
  -m | --model MODEL    AI model to use with aichat. Default value: "ollama:phi4:14b-q8_0".
                        Additional models: "aya-expanse:32b-q3_K_S", "codestral:22b-v0.1-q5_K_S".
  -r | --role ROLE      AI role to use with aichat.  Default value: "translate-po".
                        For better reproducibility, set temperature and top_p to 0, to remove randomness.

"#);
}

fn help_review() {
  println!(r#"
Usage: po-tools [GLOBAL_OPTIONS] review [-m MODEL|-l LANG] [--] FILE1 [FILE2...]

WORK IN PROGRESS.

Review multiple different translations of same messages and select the bese one among them using AI tools (aichat, ollama).

OPTIONS:

  -l | --language LANG  Language to use. Default value: "Ukrainian".
  -m | --model MODEL    AI model to use with aichat. Default value: "ollama:phi4:14b-q8_0".
                        Additional models: "aya-expanse:32b-q3_K_S", "codestral:22b-v0.1-q5_K_S".
  -r | --role ROLE      AI role to use with aichat.  Default value: "translate-po".
                        For better reproducibility, set temperature and top_p to 0, to remove randomness.

"#);
}

