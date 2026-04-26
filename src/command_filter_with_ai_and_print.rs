//! Command to filter PO messages using AI-based rules.
//!
//! This command sends each message to an AI tool and filters the output
//! based on the AI's response (e.g., "yes" or "no").

use std::io::Write;

use crate::parser::{Parser, PoMessage};
use crate::util::{AiBackend, IoContext};
use anyhow::{Result, bail};

/// Implementation of the `filter` command.
pub fn command_filter_with_ai_and_print(
    parser: &Parser,
    cmdline: &[&str],
    ctx: &mut IoContext,
) -> Result<()> {
    let mut model = "ollama:gemma3n:latest";
    let mut role = "po-review";
    let mut yes_only = false;
    let mut no_only = false;
    let mut ai_command_str: Option<&str> = None;

    // Parse "filter" command options
    let mut cmdline = cmdline;
    loop {
        match cmdline[..] {
            ["-m", model_name, ..] | ["--model", model_name, ..] => {
                model = model_name;
                cmdline = &cmdline[2..];
            }

            ["-r", role_name, ..] | ["--role", role_name, ..] => {
                role = role_name;
                cmdline = &cmdline[2..];
            }

            ["-y", ..] | ["--yes-only", ..] => {
                yes_only = true;
                cmdline = &cmdline[1..];
            }

            ["-n", ..] | ["--no-only", ..] => {
                no_only = true;
                cmdline = &cmdline[1..];
            }

            ["--ai-command", cmd, ..] => {
                ai_command_str = Some(cmd);
                cmdline = &cmdline[2..];
            }

            ["-h", ..] | ["-help", ..] | ["--help", ..] => {
                help(ctx.out)?;
                return Ok(());
            }
            ["--", ..] => {
                cmdline = &cmdline[1..];
                break;
            }
            [arg, ..] if arg.starts_with('-') => {
                bail!(
                    tr!("Unknown option: \"{option}\". Use --help for list of options.")
                        .replace("{option}", arg)
                )
            }
            _ => break,
        }
    }

    if cmdline.is_empty() {
        bail!(tr!(
            "Expected one argument at least: name of the file to filter."
        ));
    }

    let backend = if let Some(cmd) = ai_command_str {
        AiBackend::from_command_line(cmd)
    } else {
        AiBackend::with_aichat_defaults(model, role, None)
    };

    for file in cmdline {
        let messages = parser.parse_messages_from_file(file)?;
        filter_and_print(ctx, backend.clone(), yes_only, no_only, &messages)?;
    }

    Ok(())
}

fn filter_and_print(
    ctx: &mut IoContext,
    backend: AiBackend,
    yes_only: bool,
    no_only: bool,
    messages: &[PoMessage],
) -> Result<()> {
    for message in messages {
        if message.is_header() {
            writeln!(ctx.out, "{message}")?;
        } else {
            // Filter template
            let message_text = format!(
                r#"
<message>
{message}
</message>
"#
            );

            let reply_text = backend.execute(&message_text)?;

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
                    writeln!(ctx.out, "# Review: {reply_text_slice}\n#, fuzzy\n{message}")?;
                }
                (true, _, _) | (_, true, _) => {}
                _ => {
                    writeln!(ctx.out, "# Review: {reply_text_slice}\n#, fuzzy\n{message}")?;
                }
            }
        }
    }

    Ok(())
}

fn help(out: &mut dyn Write) -> Result<()> {
    writeln!(
        out,
        "{}",
        tr!(
            r#"Usage: po-tools [GLOBAL_OPTIONS] filter [OPTIONS] [--] FILE

WORK IN PROGRESS.

Filter messages in PO file using AI tools (aichat, ollama).

OPTIONS:

  -l | --language LANG  Language to use. Default value: "Ukrainian".
  -m | --model MODEL    AI model to use with aichat. Default value: "ollama:phi4:14b-q8_0".
                        Additional models: "aya-expanse:32b-q3_K_S", "codestral:22b-v0.1-q5_K_S".
  -r | --role ROLE      AI role to use with aichat.  Default value: "translate-po".
                        For better reproducibility, set temperature and top_p to 0, to remove randomness.
  -y | --yes-only       Print messages with <reply>yes</reply> only.
  -n | --no-only        Print messages with <reply>no</reply> only.
  --ai-command COMMAND  Custom command to use for filtering instead of aichat.
                        Example: --ai-command "ollama run gemma3"
"#
        )
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_positive_yes() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let m = parser.parse_message_from_str("msgid \"a\"\nmsgstr \"b\"\n")?;

        filter_and_print(
            &mut ctx,
            AiBackend::mock("yes"),
            true, // yes_only
            false,
            &[m],
        )?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("Review: yes"));
        assert!(result.contains("msgid \"a\""));
        Ok(())
    }

    #[test]
    fn test_filter_positive_no_filtered_out() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let m = parser.parse_message_from_str("msgid \"a\"\nmsgstr \"b\"\n")?;

        filter_and_print(
            &mut ctx,
            AiBackend::mock("no"),
            true, // yes_only
            false,
            &[m],
        )?;

        let result = String::from_utf8(out)?;
        // Header should be there, but not the message
        assert!(!result.contains("msgid \"a\""));
        Ok(())
    }

    #[test]
    fn test_help() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        command_filter_with_ai_and_print(&parser, &["--help"], &mut ctx)?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("Usage:"));
        Ok(())
    }

    #[test]
    fn test_no_files() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let result = command_filter_with_ai_and_print(&parser, &[], &mut ctx);
        assert!(result.is_err());
        Ok(())
    }
}
