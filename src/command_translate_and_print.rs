//! Command to translate PO messages using AI and translation memory.
//!
//! This is the most complex command, involving fuzzy matching from TM,
//! dictionary lookups, and interaction with an AI model.

use crate::dictionary::Dictionary;
use crate::parser::{Parser, PoMessage};
use crate::util::{pipe_to_command, validate_message};
use anyhow::{Context, Result, bail};
use std::collections::HashSet;
use strsim::normalized_levenshtein;

/// Implementation of the `translate` command.
pub fn command_translate_and_print(parser: &Parser, cmdline: &[&str]) -> Result<()> {
    let mut language = "Ukrainian";
    let mut model = "ollama:translategemma:12b";
    let mut role = "translate-po";
    let mut rag = "";
    let mut tm_file = "";
    let mut dictionary_files: Vec<&str> = Vec::new();
    let mut debug = false;
    let aichat_command = "aichat";

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

            ["--debug", ..] => {
                debug = true;
                cmdline = &cmdline[1..];
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
                help_translate();
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

    let mut aichat_options = vec!["-r", role, "-m", model];
    if !rag.is_empty() {
        aichat_options.push("--rag");
        aichat_options.push(rag);
    }

    let tm_messages = if !tm_file.is_empty() {
        let msgs = parser.parse_messages_from_file(tm_file).with_context(|| {
            tr!("Cannot open file \"{file}\" with translation memory.").replace("{file}", tm_file)
        })?;
        eprintln!(
            "{}: {}",
            tr!("INFO"),
            tr!("Loaded {count} messages from \"{file}\" file with translation memory.")
                .replace("{count}", &msgs.len().to_string())
                .replace("{file}", tm_file)
        );
        msgs
    } else {
        Vec::new()
    };

    let mut dictionaries = Vec::new();
    for dict_file in dictionary_files {
        let dict = Dictionary::from_file(dict_file).with_context(|| {
            tr!("Cannot open dictionary file \"{file}\".").replace("{file}", dict_file)
        })?;
        eprintln!(
            "{}: {}",
            tr!("INFO"),
            tr!("Loaded dictionary from {file} file ({count} entries).")
                .replace("{file}", dict_file)
                .replace("{count}", &dict.entries.len().to_string())
        );
        dictionaries.push(dict);
    }

    for file in cmdline {
        let messages = parser
            .parse_messages_from_file(file)
            .with_context(|| tr!("Cannot open file \"{}\" for translation.").replace("{}", file))?;
        eprintln!(
            "{}: {}",
            tr!("INFO"),
            tr!("Processing file {file}, found {count} messages")
                .replace("{file}", file)
                .replace("{count}", &messages.len().to_string())
        );
        let config = TranslateConfig {
            aichat_command,
            aichat_options: &aichat_options,
            language,
            number_of_plural_cases: parser.number_of_plural_cases,
            tm_messages: &tm_messages,
            dictionaries: &dictionaries,
            debug,
        };
        translate_and_print(&config, &messages)?;
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
    aichat_command: &'a str,
    aichat_options: &'a [&'a str],
    language: &'a str,
    number_of_plural_cases: Option<usize>,
    tm_messages: &'a [PoMessage],
    dictionaries: &'a [Dictionary],
    debug: bool,
}

fn translate_and_print(config: &TranslateConfig, messages: &[PoMessage]) -> Result<()> {
    for message in messages {
        if message.is_header() {
            println!("{message}");
        } else {
            translate_single_message(config, message)?;
        }
    }

    Ok(())
}

fn translate_single_message(config: &TranslateConfig, message: &PoMessage) -> Result<()> {
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

    if !message.is_header() {
        for dict in config.dictionaries {
            for entry in dict.find_matches(&message.msgid) {
                if seen_keys.insert(&entry.key) {
                    dict_context.push_str(&format!("- {} - {}\n", entry.key, entry.translation));
                }
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
        eprintln!(
            "----{}-----------------------------------------------------------",
            tr!("Message to aichat")
        );
        eprintln!("{message_text}");
        eprintln!(
            "----{}--------------------------------------------------------------",
            tr!("End of message")
        );
    }

    // Translate
    let new_message_text =
        pipe_to_command(config.aichat_command, config.aichat_options, &message_text)?;

    if config.debug {
        eprintln!(
            "----{}-----------------------------------------------------------",
            tr!("Reply from aichat")
        );
        eprintln!("{new_message_text}");
        eprintln!(
            "----{}----------------------------------------------------------------",
            tr!("End of reply")
        );
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
    };

    match parser.parse_message_from_str(new_message_text_slice) {
        Ok(new_message) => {
            let actual_key = message.to_key();
            let result_key = new_message.to_key();

            if actual_key == result_key {
                let errors = validate_message(&new_message);
                println!(
                    "{}:\n#{errors}\n#, fuzzy\n{new_message}",
                    tr!("# Translated message")
                );
            } else {
                eprintln!(
                    "{}. {} = \"{}\"\n# {}:\n=====\n{new_message_text_slice}\n=====",
                    tr!(
                        "# WARNING: Wrong msgid field when trying to translate. Replacing wrong ID with correct id"
                    ),
                    tr!("Actual key"),
                    actual_key,
                    tr!("Raw translation text")
                );
                let fixed_message = new_message.with_key(&actual_key);
                let errors = validate_message(&fixed_message);
                println!(
                    "{}:\n#{errors}#, fuzzy\n{fixed_message}",
                    tr!("# Translated message (WARNING: wrong id after translation)")
                );
            }
        }

        Err(e) => {
            eprintln!(
                "{}: {:#}:\n{message}\n# {}:\n=====\n{new_message_text_slice}\n=====",
                tr!("# ERROR: Cannot parse translation of message"),
                e,
                tr!("# Raw translation text")
            );
            println!(
                "{}:\n#, fuzzy\n{message}",
                tr!("# UNTranslated message (cannot parse translation)")
            );
        }
    }

    Ok(())
}

fn help_translate() {
    println!(
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

  --debug               Print inputs and outputs of AI models to stderr.
"#
        )
    );
}
