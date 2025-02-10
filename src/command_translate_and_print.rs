use anyhow::{Result, bail};
use crate::parser::{Parser, PoMessage};

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

pub fn command_translate_and_print(parser: &Parser, cmdline: &[&str]) -> Result<()> {
  let dictionary = r#"
patch - латка
bug - помилка
"#;
  let mut language = "Ukrainian";
  let mut model = "ollama:phi4:14b-q8_0";
  let mut role = "translate-po";
  let aichat_command = "aichat";

  // Parse "translate" command options
  let mut cmdline = cmdline;
  loop {
    match cmdline[..] {
      [ "-m", model_name, ..] | [ "--model", model_name, ..] => {
        model = model_name;
        cmdline = &cmdline[2..];
      }

      [ "-r", role_name, ..] | [ "--role", role_name, ..] => {
        role = role_name;
        cmdline = &cmdline[2..];
      }

      [ "-l", lang_name, ..] | [ "--lang", lang_name, ..] | [ "--language", lang_name, ..] => {
        language = lang_name;
        cmdline = &cmdline[2..];
      }

      [ "-h", .. ] | [ "-help", .. ] | [ "--help", .. ] => {
        help_translate();
        return Ok(());
      }
      [ "--", .. ] => {
        cmdline = &cmdline[1..];
        break;
      }
      [ arg, ..] if arg.starts_with('-') => bail!("Unknown option: \"{arg}\". Use --help for list of options."),
      _ => break,
    }
  }

  if cmdline.len() ==0 { bail!("Expected one argument at least: name of the file to translate."); }

  for file in cmdline {
    let messages = parser.parse_messages_from_file(file)?;
    translate_and_print(aichat_command, &[ "-r", role, "-m", model ], language, parser.number_of_plural_cases, dictionary, &messages)?;
  }

  Ok(())
}


fn translate_and_print(aichat_command: &str, aichat_options: &[&str], language: &str, number_of_plural_cases: Option<usize>, dictionary: &str, messages: &Vec<PoMessage>) -> Result<()> {

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

        // Extract text between <message> and </message>, if they are present
        let new_message_text_slice = if let (Some(start), Some(end)) = (new_message_text.find("<message>"), new_message_text.find("</message>")) {
          let tag_len="<message>".len();
          &new_message_text[(start+tag_len) .. end]
        } else {
          &new_message_text[..]
        };

        match parser.parse_message_from_str(new_message_text_slice) {
          Ok(new_message) =>  {
            let errors = validate_message(&new_message);
            if message.to_key() == new_message.to_key() {
              println!("# Translated message:\n{errors}#, fuzzy\n{new_message}");
              prev_message = new_message;
            } else {
              eprintln!("# ERROR: Wrong msgid field when trying to translate. Replacing wrong ID with correct id.\n# Translation:\n=====\n{new_message_text_slice}\n=====");
              let fixed_message = new_message.with_key(&message.to_key());
              println!("# Translated message (WARNING: wrong id after translation):\n{errors}#, fuzzy\n{fixed_message}");
            }
          },

          Err(e) => {
            eprintln!("# ERROR: Cannot parse translation of message: {:#}:\n{message}\n# Translation:\n=====\n{new_message_text_slice}\n=====", e);
            println!("# UNTranslated message (cannot parse translation):\n#, fuzzy\n{message}");
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

        match parser.parse_message_from_str(new_message_text_slice) {
          Ok(new_message) =>  {
            let errors = validate_message(&new_message);
            if message.to_key() == new_message.to_key() {
              println!("# Translated message:\n{errors}#, fuzzy\n{new_message}");
              prev_message = new_message;
            } else {
              eprintln!("# ERROR: Wrong msgid field when trying to translate. Replacing wrong ID with correct id.\n# Translation:\n=====\n{new_message_text_slice}\n=====");
              let fixed_message = new_message.with_key(&message.to_key());
              println!("# Translated message (WARNING: wrong id after translation):\n#, fuzzy\n{fixed_message}");
            }
          },

          Err(e) => {
            eprintln!("# ERROR: Cannot parse translation of message: {:#}:\n{message}\n# Translation:\n=====\n{new_message_text_slice}\n=====", e);
            println!("# UNTranslated message (cannot parse translation):\n#, fuzzy\n{message}");
          },
        }
      }
    }
  }

  Ok(())
}

pub fn command_review_files_and_print(parser: &Parser, cmdline: &[&str]) -> Result<()> {
  let dictionary = r#"
patch - латка
bug - помилка
"#;
  let mut language = "Ukrainian";
  let mut model = "ollama:phi4:14b-q8_0";
  let mut role = "translate-po";
  let aichat_command = "aichat";

  // Parse "translate" command options
  let mut cmdline = cmdline;

  // Parse "review" command options
  loop {
    match cmdline[..] {
      [ "-m", model_name, ..] | [ "--model", model_name, ..] => {
        model = model_name;
        cmdline = &cmdline[2..];
      }

      [ "-r", role_name, ..] | [ "--role", role_name, ..] => {
        role = role_name;
        cmdline = &cmdline[2..];
      }

      [ "-l", lang_name, ..] | [ "--lang", lang_name, ..] | [ "--language", lang_name, ..] => {
        language = lang_name;
        cmdline = &cmdline[2..];
      }

      [ "-h", .. ] | [ "-help", .. ] | [ "--help", .. ] => {
        help_review();
        return Ok(());
      }
      [ "--", .. ] => {
        cmdline = &cmdline[1..];
        break;
      }
      [ arg, ..] if arg.starts_with('-') => bail!("Unknown option: \"{arg}\". Use --help for list of options."),
      _ => break,
    }
  }

  if cmdline.len() ==0 { bail!("Expected one argument at least: name of the file to review."); }

  let mut messages = Vec::new();
  for file in cmdline {
    let file_messages = parser.parse_messages_from_file(file)?;
    messages.push(file_messages);
  }

  review_files_and_print(aichat_command, &[ "-r", role, "-m", model ], language, parser.number_of_plural_cases, dictionary, messages)?;

  Ok(())

}

fn review_files_and_print(aichat_command: &str, aichat_options: &[&str], language: &str, number_of_plural_cases: Option<usize>, dictionary: &str, mut messages: Vec<Vec<PoMessage>>) -> Result<()> {

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

    match parser.parse_message_from_str(new_message_text_slice) {
      Ok(new_message) =>  {
        let errors = validate_message(&new_message);
        if message.to_key() == new_message.to_key() {
          println!("# Reviewed message:\n{errors}#, fuzzy\n{new_message}");
        } else {
          eprintln!("# ERROR: Wrong msgid field when trying to review:\n{message}\n# Review:\n=====\n{new_message_text_slice}\n=====");
          println!("# Reviewed message (warning:wrong id after review):\n{errors}#, fuzzy\n{message}");
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


fn validate_message(message: &PoMessage) -> String {
  use crate::command_check_symbols::check_symbols;

  match check_symbols(message) {
    None => "".into(),
    Some(errors) => errors,
  }
}