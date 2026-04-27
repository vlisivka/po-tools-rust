//! Command to translate PO messages using AI and translation memory.
//!
//! This is the most complex command, involving fuzzy matching from TM,
//! dictionary lookups, and interaction with an AI model.

use crate::dictionary::Dictionary;
use crate::parser::{Parser, PoMessage};
use crate::util::{AiBackend, IoContext, validate_message};
use anyhow::{Context, Result, bail};
use std::collections::HashSet;
use std::io::Write;
use strsim::normalized_levenshtein;

/// Implementation of the `translate` command.
pub fn command_translate_and_print(
    parser: &Parser,
    cmdline: &[&str],
    ctx: &mut IoContext,
) -> Result<()> {
    let mut language = "Ukrainian";
    let mut model = "ollama:translategemma:12b";
    let mut role = "translate-po";
    let mut rag = "";
    let mut tm_file = "";
    let mut dictionary_files: Vec<&str> = Vec::new();
    let mut debug = false;
    let mut ai_command_str: Option<&str> = None;
    let mut force_keyword: Option<String> = None;

    // Parse "translate" command options
    let mut cmdline = cmdline;
    loop {
        match cmdline[..] {
            ["-m", model_name, ..] | ["--model", model_name, ..] => {
                model = model_name;
                cmdline = &cmdline[2..];
            }

            ["-R", rag_name, ..] | ["--rag", rag_name, ..] => {
                rag = rag_name;
                cmdline = &cmdline[2..];
            }

            ["-M", tm_file_name, ..]
            | ["--tm", tm_file_name, ..]
            | ["--translation-memory", tm_file_name, ..] => {
                tm_file = tm_file_name;
                cmdline = &cmdline[2..];
            }

            ["-d", dict_file, ..] | ["--dictionary", dict_file, ..] => {
                dictionary_files.push(dict_file);
                cmdline = &cmdline[2..];
            }

            ["-k", k, ..] | ["--force-by-keyword", k, ..] => {
                force_keyword = Some(k.to_string());
                cmdline = &cmdline[2..];
            }

            ["--debug", ..] => {
                debug = true;
                cmdline = &cmdline[1..];
            }

            ["--ai-command", cmd, ..] => {
                ai_command_str = Some(cmd);
                cmdline = &cmdline[2..];
            }

            ["-r", role_name, ..] | ["--role", role_name, ..] => {
                role = role_name;
                cmdline = &cmdline[2..];
            }

            ["-l", lang_name, ..] | ["--lang", lang_name, ..] | ["--language", lang_name, ..] => {
                language = lang_name;
                cmdline = &cmdline[2..];
            }

            ["-h", ..] | ["-help", ..] | ["--help", ..] => {
                help_translate(ctx.out)?;
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
        bail!(tr!(
            "Expected at least one argument: the name of the file to translate."
        ));
    }

    let backend = if let Some(cmd) = ai_command_str {
        AiBackend::from_command_line(cmd)
    } else {
        AiBackend::with_aichat_defaults(model, role, if rag.is_empty() { None } else { Some(rag) })
    };

    let tm_messages = if !tm_file.is_empty() {
        let msgs = parser.parse_messages_from_file(tm_file).with_context(|| {
            tr!("Cannot open file \"{file}\" with translation memory.").replace("{file}", tm_file)
        })?;
        writeln!(
            ctx.err,
            "{}: {}",
            tr!("INFO"),
            tr!("Loaded {count} messages from \"{file}\" file with translation memory.")
                .replace("{count}", &msgs.len().to_string())
                .replace("{file}", tm_file)
        )?;
        msgs
    } else {
        Vec::new()
    };

    let mut dictionaries = Vec::new();
    for dict_file in dictionary_files {
        let dict = Dictionary::from_file(dict_file).with_context(|| {
            tr!("Cannot open dictionary file \"{file}\".").replace("{file}", dict_file)
        })?;
        writeln!(
            ctx.err,
            "{}: {}",
            tr!("INFO"),
            tr!("Loaded dictionary from {file} file ({count} entries).")
                .replace("{file}", dict_file)
                .replace("{count}", &dict.entries.len().to_string())
        )?;
        dictionaries.push(dict);
    }

    for file in cmdline {
        let messages = parser
            .parse_messages_from_file(file)
            .with_context(|| tr!("Cannot open file \"{}\" for translation.").replace("{}", file))?;
        writeln!(
            ctx.err,
            "{}: {}",
            tr!("INFO"),
            tr!("Processing file {file}, found {count} messages")
                .replace("{file}", file)
                .replace("{count}", &messages.len().to_string())
        )?;

        let config = TranslateConfig {
            backend: backend.clone(),
            language,
            number_of_plural_cases: parser.number_of_plural_cases,
            tm_messages: &tm_messages,
            dictionaries: &dictionaries,
            debug,
            copy_comments: true,
            force_keyword: force_keyword.clone(),
        };
        translate_and_print(ctx, &config, &messages)?;
    }

    Ok(())
}

fn find_fuzzy_matches<'a>(message: &PoMessage, tm_messages: &'a [PoMessage]) -> Vec<&'a PoMessage> {
    if message.is_header() {
        return Vec::new();
    }
    let msgid = &message.msgid;

    let mut matches: Vec<(f64, &PoMessage)> = tm_messages
        .iter()
        .filter(|tm_msg| !tm_msg.is_header())
        .map(|tm_msg| (normalized_levenshtein(msgid, &tm_msg.msgid), tm_msg))
        .collect();

    // Sort by score descending
    matches.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    // Take top 5
    // TODO: make number of matches configurable
    matches.into_iter().take(5).map(|(_, msg)| msg).collect()
}

struct TranslateConfig<'a> {
    backend: AiBackend,
    language: &'a str,
    number_of_plural_cases: Option<usize>,
    tm_messages: &'a [PoMessage],
    dictionaries: &'a [Dictionary],
    debug: bool,
    copy_comments: bool,
    force_keyword: Option<String>,
}

fn translate_and_print(
    ctx: &mut IoContext,
    config: &TranslateConfig,
    messages: &[PoMessage],
) -> Result<()> {
    for message in messages {
        let should_force = config
            .force_keyword
            .as_ref()
            .map(|k| message.msgid.contains(k))
            .unwrap_or(false);

        if message.is_header() || (message.is_translated() && !message.is_fuzzy() && !should_force)
        {
            // Just copy headers and translated messages
            writeln!(ctx.out, "{message}")?;
        } else {
            translate_single_message(ctx, config, message)?;
        }
    }

    Ok(())
}

fn translate_single_message(
    ctx: &mut IoContext,
    config: &TranslateConfig,
    message: &PoMessage,
) -> Result<()> {
    let fuzzy_matches = find_fuzzy_matches(message, config.tm_messages);
    let fuzzy_match_text = if !fuzzy_matches.is_empty() {
        let mut text = format!(
            "<context>\n{}:\n",
            tr!("# Fuzzy matches from translation memory")
        );
        for m in fuzzy_matches {
            text.push_str(&format!("{}\n", m));
        }
        text.push_str("</context>");
        text
    } else {
        "".to_string()
    };

    // Find dictionary matches
    let mut dict_context = String::new();
    let mut seen_keys = HashSet::new();

    for dict in config.dictionaries {
        for entry in dict.find_matches(&message.msgid) {
            if seen_keys.insert(&entry.key) {
                dict_context.push_str(&format!("- {} - {}\n", entry.key, entry.translation));
            }
        }
    }

    if !dict_context.is_empty() {
        dict_context = format!("<dictionary>\n{dict_context}</dictionary>\n");
    }

    let is_plural = message.is_plural();
    let example = if is_plural {
        r#"
<example>
msgid "%s new patch,"
msgid_plural "%s new patches,"
msgstr[0] "%s нова латка,"
msgstr[1] "%s нові латки,"
msgstr[2] "%s нових латок,"
</example>
"#
    } else {
        ""
    };

    // Translation template
    let message_text = format!(
        r#"
{dict_context}

{fuzzy_match_text}

<instruction>
IMPORTANT: Translate text in <message></message> tag only and _nothing else_.
IMPORTANT: Answers must be VALID Gettext PO messages. Msgid field must be verbatim copy of original msgid, while msgstr must be {language} translation.
IMPORTANT: Don't translate <context> and <dictionary>. They are just for reference.
IMPORTANT: Prefer translations proposed by dictionary.
You are a professional English (en_US) to {language} translator. Your goal is to accurately convey the meaning and nuances of the original English text while adhering to {language} grammar, vocabulary, and cultural sensitivities.
Produce only the {language} translation, without any additional explanations or commentary. Please translate the following English text in <message></message> into {language}:
</instruction>

<message>
{message}
</message>

{example}
"#,
        language = config.language
    );

    if config.debug {
        writeln!(
            ctx.err,
            "----{}-----------------------------------------------------------",
            tr!("Message to aichat")
        )?;
        writeln!(ctx.err, "{message_text}")?;
        writeln!(
            ctx.err,
            "----{}--------------------------------------------------------------",
            tr!("End of message")
        )?;
    }

    // Translate
    let new_message_text = config.backend.execute(&message_text)?;

    if config.debug {
        writeln!(
            ctx.err,
            "----{}-----------------------------------------------------------",
            tr!("Reply from aichat")
        )?;
        writeln!(ctx.err, "{new_message_text}")?;
        writeln!(
            ctx.err,
            "----{}----------------------------------------------------------------",
            tr!("End of reply")
        )?;
    }

    let new_message_text_cleaned = if let Some(start) = new_message_text.rfind("</think>") {
        // Skip thinking output from reasoning models
        &new_message_text[start..]
    } else {
        &new_message_text[..]
    };

    let new_message_text_slice = if let (Some(start), Some(end)) = (
        new_message_text_cleaned.rfind("<message>"),
        new_message_text_cleaned.rfind("</message>"),
    ) {
        // Extract text between <message> and </message>, if they are present
        let tag_len = "<message>".len();
        &new_message_text_cleaned[(start + tag_len)..end]
    } else if let Some(start) = new_message_text_cleaned.rfind("msgid ") {
        // Unwrapped message found
        &new_message_text_cleaned[start..]
    } else {
        // Message not found
        new_message_text_cleaned
    };

    let parser = Parser {
        number_of_plural_cases: if is_plural {
            Some(config.number_of_plural_cases.unwrap_or(2))
        } else {
            config.number_of_plural_cases
        },
        ignore_garbage_after_msgstr: true,
        strip_comments: true,
    };

    match parser.parse_message_from_str(new_message_text_slice) {
        Ok(mut new_message) => {
            if config.copy_comments {
                new_message.comments = message.comments.clone();
            }

            let actual_key = message.to_key();
            let result_key = new_message.to_key();

            if actual_key == result_key {
                let errors = validate_message(&new_message);
                writeln!(
                    ctx.out,
                    "{}:\n#{errors}\n#, fuzzy\n{new_message}",
                    tr!("# Translated message")
                )?;
            } else {
                writeln!(
                    ctx.err,
                    "{}. {} = \"{}\"\n# {}:\n=====\n{new_message_text_slice}\n=====",
                    tr!(
                        "# WARNING: Wrong msgid field when trying to translate. Replacing wrong ID with correct id"
                    ),
                    tr!("Actual key"),
                    actual_key,
                    tr!("Raw translation text")
                )?;
                let fixed_message = new_message.with_key(&actual_key);
                let errors = validate_message(&fixed_message);
                writeln!(
                    ctx.out,
                    "{}:\n#{errors}\n#, fuzzy\n{fixed_message}",
                    tr!("# Translated message (WARNING: wrong id after translation)")
                )?;
            }
        }

        Err(e) => {
            writeln!(
                ctx.err,
                "{}: {:#}:\n{message}\n# {}:\n=====\n{new_message_text_slice}\n=====",
                tr!("# ERROR: Cannot parse translation of message"),
                e,
                tr!("# Raw translation text")
            )?;
            writeln!(
                ctx.out,
                "{}:\n#, fuzzy\n{message}",
                tr!("# UNTranslated message (cannot parse translation)")
            )?;
        }
    }

    Ok(())
}

fn help_translate(out: &mut dyn Write) -> Result<()> {
    writeln!(
        out,
        "{}",
        tr!(
            r#"Usage: po-tools [GLOBAL_OPTIONS] translate [OPTIONS] [--] FILE

WORK IN PROGRESS.

Translate messages in PO file using AI tools (aichat, ollama).

OPTIONS:

  -l | --language LANG  Language to use. Default value: "Ukrainian".

  -m | --model MODEL    AI model to use with aichat. Default value: "ollama:phi4:14b-q8_0".
                        Additional models: "aya-expanse:32b-q3_K_S", "codestral:22b-v0.1-q5_K_S".

  -r | --role ROLE      AI role to use with aichat.  Default value: "translate-po".
                        For better reproducibility, set temperature and top_p to 0, to remove randomness.

  -R | --rag RAG        aichat RAG to use.

  --tm FILE             Local Translation Memory file (PO format) to use for fuzzy matching.

  -d | --dictionary FILE  TSV dictionary file to use for context. Can be used multiple times.

  -k | --force-by-keyword KEYWORD  Force translation of messages whose msgid contains KEYWORD.

  --ai-command COMMAND  Custom command to use for translation instead of aichat.
                        Example: --ai-command "ollama run gemma3"

  --debug               Print inputs and outputs of AI models to stderr.
"#
        )
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_translate_positive() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let config = TranslateConfig {
            backend: AiBackend::mock("msgid \"a\"\nmsgstr \"translated_a\""),
            language: "Ukrainian",
            number_of_plural_cases: None,
            tm_messages: &[],
            dictionaries: &[],
            debug: false,
            copy_comments: true,
            force_keyword: None,
        };

        let message = parser.parse_message_from_str("msgid \"a\"\nmsgstr \"\"\n")?;
        translate_and_print(&mut ctx, &config, &[message])?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("msgid \"a\""));
        assert!(result.contains("msgstr \"translated_a\""));
        Ok(())
    }

    #[test]
    fn test_translate_copy_comments() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let config = TranslateConfig {
            backend: AiBackend::mock("msgid \"a\"\nmsgstr \"translated_a\""),
            language: "Ukrainian",
            number_of_plural_cases: None,
            tm_messages: &[],
            dictionaries: &[],
            debug: false,
            copy_comments: true,
            force_keyword: None,
        };

        let message = parser.parse_message_from_str("# comment\nmsgid \"a\"\nmsgstr \"\"\n")?;
        translate_and_print(&mut ctx, &config, &[message])?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("# comment"));
        assert!(result.contains("msgid \"a\""));
        assert!(result.contains("msgstr \"translated_a\""));
        Ok(())
    }

    #[test]
    fn test_translate_skip_translated() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let config = TranslateConfig {
            // Backend should not be called
            backend: AiBackend::mock("SHOULD NOT BE CALLED"),
            language: "Ukrainian",
            number_of_plural_cases: None,
            tm_messages: &[],
            dictionaries: &[],
            debug: false,
            copy_comments: true,
            force_keyword: None,
        };

        // already translated message
        let message = parser.parse_message_from_str("msgid \"a\"\nmsgstr \"existing_a\"\n")?;
        translate_and_print(&mut ctx, &config, &[message])?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("msgid \"a\""));
        assert!(result.contains("msgstr \"existing_a\""));
        assert!(!result.contains("Translated message"));
        assert!(!result.contains("SHOULD NOT BE CALLED"));
        Ok(())
    }

    #[test]
    fn test_translate_fuzzy_messages() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let config = TranslateConfig {
            backend: AiBackend::mock("msgid \"a\"\nmsgstr \"translated_fuzzy_a\""),
            language: "Ukrainian",
            number_of_plural_cases: None,
            tm_messages: &[],
            dictionaries: &[],
            debug: false,
            copy_comments: true,
            force_keyword: None,
        };

        // fuzzy message
        let message =
            parser.parse_message_from_str("#, fuzzy\nmsgid \"a\"\nmsgstr \"old_fuzzy_a\"\n")?;
        translate_and_print(&mut ctx, &config, &[message])?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("msgid \"a\""));
        assert!(result.contains("msgstr \"translated_fuzzy_a\""));
        assert!(result.contains("Translated message"));
        Ok(())
    }

    #[test]
    fn test_translate_check_symbols() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let config = TranslateConfig {
            // AI "forgot" the %d symbol
            backend: AiBackend::mock("msgid \"a %d\"\nmsgstr \"translated_a\""),
            language: "Ukrainian",
            number_of_plural_cases: None,
            tm_messages: &[],
            dictionaries: &[],
            debug: false,
            copy_comments: true,
            force_keyword: None,
        };

        let message = parser.parse_message_from_str("msgid \"a %d\"\nmsgstr \"\"\n")?;
        translate_and_print(&mut ctx, &config, &[message])?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("Warning: Incorrect symbols"));
        assert!(result.contains("#, fuzzy"));
        Ok(())
    }

    #[test]
    fn test_translate_check_whitespace() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let config = TranslateConfig {
            // AI "forgot" the trailing space
            backend: AiBackend::mock("msgid \"a \"\nmsgstr \"translated_a\""),
            language: "Ukrainian",
            number_of_plural_cases: None,
            tm_messages: &[],
            dictionaries: &[],
            debug: false,
            copy_comments: true,
            force_keyword: None,
        };

        let message = parser.parse_message_from_str("msgid \"a \"\nmsgstr \"\"\n")?;
        translate_and_print(&mut ctx, &config, &[message])?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("Warning: Whitespace mismatch"));
        assert!(result.contains("#, fuzzy"));
        Ok(())
    }

    #[test]
    fn test_translate_force_keyword() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let config = TranslateConfig {
            backend: AiBackend::mock("msgid \"keyword message\"\nmsgstr \"forced_translation\""),
            language: "Ukrainian",
            number_of_plural_cases: None,
            tm_messages: &[],
            dictionaries: &[],
            debug: false,
            copy_comments: true,
            force_keyword: Some("keyword".to_string()),
        };

        // already translated message with keyword in msgid
        let message = parser
            .parse_message_from_str("msgid \"keyword message\"\nmsgstr \"old_translation\"\n")?;
        translate_and_print(&mut ctx, &config, &[message])?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("msgid \"keyword message\""));
        assert!(result.contains("msgstr \"forced_translation\""));
        assert!(result.contains("Translated message"));
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

        command_translate_and_print(&parser, &["--help"], &mut ctx)?;

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

        let result = command_translate_and_print(&parser, &[], &mut ctx);
        assert!(result.is_err());
        Ok(())
    }
}
