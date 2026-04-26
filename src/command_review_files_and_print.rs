//! Command to review multiple translations and select the best one using AI.
//!
//! This module compares different versions of translations for the same PO messages
//! and uses an AI model to pick or synthesize the best version.

use crate::parser::{Parser, PoMessage};
use crate::util::{IoContext, pipe_to_command, validate_message};
use anyhow::{Result, bail};
use std::io::Write;

/// Implementation of the `review` command.
pub fn command_review_files_and_print(
    parser: &Parser,
    cmdline: &[&str],
    ctx: &mut IoContext,
) -> Result<()> {
    let mut language = "Ukrainian";
    let mut model = "ollama:translategemma:12b";
    let mut role = "translate-po";
    let aichat_command = "aichat";

    // Parse "translate" command options
    let mut cmdline = cmdline;

    // Parse "review" command options
    loop {
        match cmdline[..] {
            ["-m", model_name, ref tail @ ..] | ["--model", model_name, ref tail @ ..] => {
                model = model_name;
                cmdline = tail;
            }

            ["-r", role_name, ref tail @ ..] | ["--role", role_name, ref tail @ ..] => {
                role = role_name;
                cmdline = tail;
            }

            ["-l", lang_name, ref tail @ ..]
            | ["--lang", lang_name, ref tail @ ..]
            | ["--language", lang_name, ref tail @ ..] => {
                language = lang_name;
                cmdline = tail;
            }

            ["-h", ..] | ["-help", ..] | ["--help", ..] => {
                help_review(ctx.out)?;
                return Ok(());
            }
            ["--", ref tail @ ..] => {
                cmdline = tail;
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
        bail!(tr!(
            "Expected at least one argument: the name of the file to review."
        ));
    }

    let mut messages = Vec::new();
    for file in cmdline {
        let file_messages = parser.parse_messages_from_file(file)?;
        messages.push(file_messages);
    }

    review_files_and_print(
        ctx,
        aichat_command,
        &["-r", role, "-m", model],
        language,
        parser.number_of_plural_cases,
        messages,
    )?;

    Ok(())
}

fn review_files_and_print(
    ctx: &mut IoContext,
    aichat_command: &str,
    aichat_options: &[&str],
    language: &str,
    number_of_plural_cases: Option<usize>,
    mut messages: Vec<Vec<PoMessage>>,
) -> Result<()> {
    let parser = Parser {
        number_of_plural_cases,
        ignore_garbage_after_msgstr: false,
    };

    for msgs in messages.iter_mut() {
        msgs.sort();
    }

    let (head, tail) = messages.split_at(1);

    'outer: for (i, message) in head[0].iter().enumerate() {
        if !tail.iter().any(|msgs| msgs[i] != *message) {
            // All messages are same, skip review
            writeln!(
                ctx.out,
                "{}:\n{message}",
                tr!("# All translations are same")
            )?;
            continue 'outer;
        }

        let mut text = format!("{}:\n{message}", tr!("# Variant 1"));

        let k1 = message.to_key();

        for (j, msgs) in tail.iter().enumerate() {
            let j = j + 2;
            let k2 = msgs[i].to_key();

            if k2 != k1 {
                bail!("{}", tr!("To review, msgid's must be same in all files. In message #{}, \"{}\" != \"{}\".")
                    .replace("{}", &i.to_string())
                    .replace("{}", &format!("{k1}"))
                    .replace("{}", &format!("{k2}")));
            }

            text = format!(
                "{text}{}:\n{}",
                tr!("# Variant {}").replace("{}", &j.to_string()),
                msgs[i]
            );
        }

        text += "\n";

        // Review messages
        let message_text = format!(
            r#"
<instruction>
Act as technical translator for Gettext .po files.
Review PO message translation variants in <message></message> tag to {language} Language. List cons for varians in <review></review> tag.
Check for technical correctness, translation correctness, correct gender, correct plural form, correct line breaks.
Choose the variant most pleasing for a native speaker in {language} language.
Write review in <review></review> tag first, then write one correct PO message without flaws in <message></message> tag.
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
"#,
            language = language
        );
        //eprintln!("{message_text}");

        // Translate
        let new_message_text = pipe_to_command(aichat_command, aichat_options, &message_text)?;
        //eprintln!("# Review:\n{new_message_text}\n");

        // Extract text between <message> and </message>, if they are present
        let new_message_text_slice = if let (Some(start), Some(end)) = (
            new_message_text.find("<message>"),
            new_message_text.find("</message>"),
        ) {
            let tag_len = "<message>".len();
            &new_message_text[(start + tag_len)..end]
        } else {
            &new_message_text[..]
        };

        match parser.parse_message_from_str(new_message_text_slice) {
            Ok(new_message) => {
                let errors = validate_message(&new_message);
                if message.to_key() == new_message.to_key() {
                    writeln!(
                        ctx.out,
                        "{}:\n#{errors}#, fuzzy\n{new_message}",
                        tr!("# Reviewed message")
                    )?;
                } else {
                    writeln!(
                        ctx.err,
                        "{}:\n{message}\n# {}:\n=====\n{new_message_text_slice}\n=====",
                        tr!("# ERROR: Wrong msgid field when trying to review"),
                        tr!("Review")
                    )?;
                    writeln!(
                        ctx.out,
                        "{}:\n{errors}#, fuzzy\n{message}",
                        tr!("# Reviewed message (warning:wrong id after review)")
                    )?;
                }
            }

            Err(e) => {
                writeln!(
                    ctx.err,
                    "{}: {:#}:\n{message}\n# {}:\n=====\n{new_message_text_slice}\n=====",
                    tr!("# ERROR: Cannot parse review of message"),
                    e,
                    tr!("Review text")
                )?;
                writeln!(
                    ctx.out,
                    "{}:\n#, fuzzy\n{message}",
                    tr!("#UNReviewed message (cannot parse review)")
                )?;
            }
        }
    }

    Ok(())
}

fn help_review(out: &mut dyn Write) -> Result<()> {
    writeln!(
        out,
        "{}",
        tr!(
            r#"Usage: po-tools [GLOBAL_OPTIONS] review [OPTIONS] [--] FILE1 [FILE2...]

WORK IN PROGRESS.

Review multiple different translations of same messages and select the best one among them using AI tools (aichat, ollama).

OPTIONS:

  -l | --language LANG  Language to use. Default value: "Ukrainian".

  -m | --model MODEL    AI model to use with aichat. Default value: "ollama:phi4:14b-q8_0".
                        Additional models: "aya-expanse:32b-q3_K_S", "codestral:22b-v0.1-q5_K_S".

  -r | --role ROLE      AI role to use with aichat.  Default value: "translate-po".
                        For better reproducibility, set temperature and top_p to 0, to remove randomness.
"#
        )
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::NamedTempFile;

    #[test]
    fn test_review_positive() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        // Mock aichat: returns a valid PO message block wrapped in <message>
        let mock_script = NamedTempFile::new()?;
        fs::write(
            mock_script.path(),
            "#!/bin/sh\necho '<message>msgid \"a\"\nmsgstr \"reviewed_a\"</message>'",
        )?;
        let mut perms = fs::metadata(mock_script.path())?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(mock_script.path(), perms)?;
        let mock_script_path = mock_script.into_temp_path();

        let f1 = NamedTempFile::new()?;
        fs::write(f1.path(), "msgid \"a\"\nmsgstr \"v1\"\n")?;

        let f2 = NamedTempFile::new()?;
        fs::write(f2.path(), "msgid \"a\"\nmsgstr \"v2\"\n")?;

        // We use review_files_and_print directly to override aichat_command easily
        let messages = vec![
            parser.parse_messages_from_file(f1.path().to_str().unwrap())?,
            parser.parse_messages_from_file(f2.path().to_str().unwrap())?,
        ];

        review_files_and_print(
            &mut ctx,
            mock_script_path.to_str().unwrap(), // override aichat_command
            &[],
            "Ukrainian",
            None,
            messages,
        )?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("msgid \"a\""));
        assert!(result.contains("msgstr \"reviewed_a\""));
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

        command_review_files_and_print(&parser, &["--help"], &mut ctx)?;

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

        let result = command_review_files_and_print(&parser, &[], &mut ctx);
        assert!(result.is_err());
        Ok(())
    }
}
