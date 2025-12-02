use anyhow::{Result, bail};
use crate::parser::{Parser, PoMessage};
use crate::util::pipe_to_command;

pub fn command_filter_with_ai_and_print(parser: &Parser, cmdline: &[&str]) -> Result<()> {
  let mut model = "ollama:gemma3n:latest";
  let mut role = "po-review";
  let mut yes_only = false;
  let mut no_only = false;
  let aichat_command = "aichat";

  // Parse "filter" command options
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

      [ "-y", ..] | [ "--yes-only", ..] => {
        yes_only = true;
        cmdline = &cmdline[1..];
      }

      [ "-n", ..] | [ "--no-only", ..] => {
        no_only = true;
        cmdline = &cmdline[1..];
      }

      [ "-h", .. ] | [ "-help", .. ] | [ "--help", .. ] => {
        help();
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

  if cmdline.is_empty() { bail!("Expected one argument at least: name of the file to translate."); }

  for file in cmdline {
    let messages = parser.parse_messages_from_file(file)?;
    filter_and_print(aichat_command, &[ "-r", role, "-m", model ], yes_only, no_only, &messages)?;
  }

  Ok(())
}


fn filter_and_print(aichat_command: &str, aichat_options: &[&str], yes_only: bool, no_only: bool, messages: &Vec<PoMessage>) -> Result<()> {

  for message in messages {
    match message {
      // Pass header untranslated
      PoMessage::Header{..} => { println!("{message}"); },

      PoMessage::Regular{..} | PoMessage::RegularWithContext{..} | PoMessage::Plural{..} | PoMessage::PluralWithContext{..} => {
        // Filter template
        let message_text = format!(r#"
<message>
{message}
</message>
"#);

        // Review
        let reply_text = pipe_to_command(aichat_command, aichat_options, &message_text)?;

        // Extract text between <reply> and </reply>, if they are present
//        let reply_text_slice = if let (Some(start), Some(end)) = (reply_text.find("<reply>"), reply_text.find("</reply")) {
//          let tag_len="<reply>".len();
//          &reply_text[(start+tag_len) .. end]
//        } else {
//          &reply_text[..]
//        };
        let reply_text_slice = &reply_text[..];

        match (yes_only, no_only, reply_text_slice) {
          (true, _, "yes") | (_, true, "no") => {
            println!("# Review: {reply_text_slice}\n#, fuzzy\n{message}");
          },
          (true, _, _) | (_, true, _) => { },
          _ => {
            println!("# Review: {reply_text_slice}\n#, fuzzy\n{message}");
          }
        }

      },

    }
  }

  Ok(())
}


fn help() {
  println!(r#"
Usage: po-tools [GLOBAL_OPTIONS] review [OPTIONS] [--] FILE

WORK IN PROGRESS.

Translate messages in PO file using AI tools (aichat, ollama).

OPTIONS:

  -l | --language LANG  Language to use. Default value: "Ukrainian".
  -m | --model MODEL    AI model to use with aichat. Default value: "ollama:phi4:14b-q8_0".
                        Additional models: "aya-expanse:32b-q3_K_S", "codestral:22b-v0.1-q5_K_S".
  -r | --role ROLE      AI role to use with aichat.  Default value: "translate-po".
                        For better reproducibility, set temperature and top_p to 0, to remove randomness.
  -y | --yes-only       Print messages with <reply>yes</reply> only.
  -n | --no-only        Print messages with <reply>no</reply> only.

"#);
}
