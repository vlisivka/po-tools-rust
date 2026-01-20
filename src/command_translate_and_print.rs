use crate::parser::{Parser, PoMessage};
use crate::util::pipe_to_command;
use anyhow::{bail, Result};
use strsim::normalized_levenshtein;

pub fn command_translate_and_print(parser: &Parser, cmdline: &[&str]) -> Result<()> {
    let dictionary = r#"
patch - латка
bug - помилка
"#;
    let mut language = "Ukrainian";
    let mut model = "ollama:phi4:latest";
    let mut role = "translate-po";
    let mut rag = "";
    let mut tm_file = "";
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
                bail!("Unknown option: \"{arg}\". Use --help for list of options.")
            }
            _ => break,
        }
    }

    if cmdline.is_empty() {
        bail!("Expected one argument at least: name of the file to translate.");
    }

    let aichat_options = ["-r", role, "-m", model];
    let aichat_options_with_rag = ["-r", role, "-m", model, "--rag", rag];

    let tm_messages = if !tm_file.is_empty() {
        let msgs = parser.parse_messages_from_file(tm_file)?;
        eprintln!(
            "INFO: Loaded {} messages from {tm_file} with translation memory.",
            msgs.len()
        );
        msgs
    } else {
        Vec::new()
    };

    for file in cmdline {
        let messages = parser.parse_messages_from_file(file)?;
        eprintln!(
            "INFO: Processing file {file}, found {} messages",
            messages.len()
        );
        translate_and_print(
            aichat_command,
            if rag.is_empty() {
                &aichat_options
            } else {
                &aichat_options_with_rag
            },
            language,
            parser.number_of_plural_cases,
            dictionary,
            &messages,
            &tm_messages,
        )?;
    }

    Ok(())
}
fn get_msgid(message: &PoMessage) -> Option<&str> {
    match message {
        PoMessage::Regular { msgid, .. } => Some(msgid),
        PoMessage::RegularWithContext { msgid, .. } => Some(msgid),
        PoMessage::Plural { msgid, .. } => Some(msgid),
        PoMessage::PluralWithContext { msgid, .. } => Some(msgid),
        PoMessage::Header { .. } => None,
    }
}

fn find_fuzzy_matches<'a>(message: &PoMessage, tm_messages: &'a [PoMessage]) -> Vec<&'a PoMessage> {
    let msgid = match get_msgid(message) {
        Some(id) => id,
        None => return Vec::new(),
    };

    let mut matches: Vec<(f64, &PoMessage)> = tm_messages
        .iter()
        .filter_map(|tm_msg| {
            get_msgid(tm_msg).map(|tm_msgid| (normalized_levenshtein(msgid, tm_msgid), tm_msg))
        })
        .collect();

    // Sort by score descending
    matches.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    // Take top 10
    matches.into_iter().take(10).map(|(_, msg)| msg).collect()
}

fn translate_and_print(
    aichat_command: &str,
    aichat_options: &[&str],
    language: &str,
    number_of_plural_cases: Option<usize>,
    dictionary: &str,
    messages: &Vec<PoMessage>,
    tm_messages: &Vec<PoMessage>,
) -> Result<()> {
    let parser = Parser {
        number_of_plural_cases,
    };

    for message in messages {
        match message {
            // Pass header untranslated
            PoMessage::Header { .. } => {
                println!("{message}");
            }

            PoMessage::Regular { .. } | PoMessage::RegularWithContext { .. } => {
                let fuzzy_matches = find_fuzzy_matches(message, tm_messages);
                let fuzzy_match_text = if !fuzzy_matches.is_empty() {
                    let mut text =
                        String::from("<context>\nFuzzy matches from translation memory:\n");
                    for m in fuzzy_matches {
                        text.push_str(&format!("{}\n", m));
                    }
                    text.push_str("</context>");
                    text
                } else {
                    "".to_string()
                };

                // Translation template
                let message_text = format!(
                    r#"
<instruction>
Act as technical translator for Gettext .po files.
Translate PO message in <message></message> tag to {language} language. IMPORTANT: Copy msgid field verbatim, put translation into msgstr field.
Resulting message must be correct Gettext PO Message, wrapped in <message></message> tag.
In translated message, msgid field must be copied intact first, then msgstr field must be translation of msgid to {language} language.
IMPORTANT: Start reply with "<message> msgid ", then write translation in msgstr.
</instruction>
<message>
{message}
</message>
{fuzzy_match_text}
<dictionary>{dictionary}</dictionary>
"#
                );
                eprintln!("--------------------------------------------------------------------------------");
                eprintln!("{message_text}");
                eprintln!("--------------------------------------------------------------------------------");

                // Translate
                let new_message_text =
                    pipe_to_command(aichat_command, aichat_options, &message_text)?;

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
                        if message.to_key() == new_message.to_key() {
                            let errors = validate_message(&new_message);
                            println!("# Translated message:\n{errors}#, fuzzy\n{new_message}");
                        } else {
                            eprintln!("# WARNING: Wrong msgid field when trying to translate. Replacing wrong ID with correct id.");
                            let fixed_message = new_message.with_key(&message.to_key());
                            let errors = validate_message(&fixed_message);
                            println!("# Translated message (WARNING: wrong id after translation):\n{errors}#, fuzzy\n{fixed_message}");
                        }
                    }

                    Err(e) => {
                        eprintln!("# ERROR: Cannot parse translation of message: {:#}:\n{message}\n# Raw translation text:\n=====\n{new_message_text_slice}\n=====", e);
                        println!("# UNTranslated message (cannot parse translation):\n#, fuzzy\n{message}");
                    }
                }
            }

            PoMessage::Plural { .. } | PoMessage::PluralWithContext { .. } => {
                let number_of_plural_cases = number_of_plural_cases.unwrap_or(2);
                let fuzzy_matches = find_fuzzy_matches(message, tm_messages);
                let fuzzy_match_text = if !fuzzy_matches.is_empty() {
                    let mut text =
                        String::from("<context>\nFuzzy matches from translation memory:\n");
                    for m in fuzzy_matches {
                        text.push_str(&format!("{}\n", m));
                    }
                    text.push_str("</context>");
                    text
                } else {
                    "".to_string()
                };

                // Translation template
                let message_text = format!(
                    r#"
<instruction>
Act as technical translator for Gettext .po files.
Translate PO message in <message></message> tag to {language} language. IMPORTANT: Copy msgid and msgid_plural fields verbatim,
put translation into msgstr[] fields. Resulting message must be correct Gettext PO Message, wrapped in <message></message> tag.
In translated message, msgid and msgid_plural fields must be copied intact first, then all {number_of_plural_cases} msgstr[] fields must be translation
of msgid and msgid_plural to {language} language. IMPORTANT: Start with "<message> msgid ".
</instruction>
<message>
{message}
</message>
{fuzzy_match_text}
<example>
msgid "%s new patch,"
msgid_plural "%s new patches,"
msgstr[0] "%s нова латка,"
msgstr[1] "%s нові латки,"
msgstr[2] "%s нових латок,"
</example>
<dictionary>{dictionary}</dictionary>
"#
                );
                eprintln!("--------------------------------------------------------------------------------");
                eprintln!("{message_text}");
                eprintln!("--------------------------------------------------------------------------------");

                // Translate
                let new_message_text =
                    pipe_to_command(aichat_command, aichat_options, &message_text)?;

                let parser = Parser {
                    number_of_plural_cases: Some(number_of_plural_cases),
                };

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
                        if message.to_key() == new_message.to_key() {
                            let errors = validate_message(&new_message);
                            println!("# Translated message:\n{errors}#, fuzzy\n{new_message}");
                        } else {
                            eprintln!("# WARNING: Wrong msgid field when trying to translate. Replacing wrong ID with correct id.");
                            let fixed_message = new_message.with_key(&message.to_key());
                            let errors = validate_message(&fixed_message);
                            println!("# Translated message (WARNING: wrong id after translation):\n{errors}#, fuzzy\n{fixed_message}");
                        }
                    }

                    Err(e) => {
                        eprintln!("# ERROR: Cannot parse translation of message: {:#}:\n{message}\n# Raw translation text:\n=====\n{new_message_text_slice}\n=====", e);
                        println!("# UNTranslated message (cannot parse translation):\n#, fuzzy\n{message}");
                    }
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
            ["-m", model_name, ..] | ["--model", model_name, ..] => {
                model = model_name;
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
                help_review();
                return Ok(());
            }
            ["--", ..] => {
                cmdline = &cmdline[1..];
                break;
            }
            [arg, ..] if arg.starts_with('-') => {
                bail!("Unknown option: \"{arg}\". Use --help for list of options.")
            }
            _ => break,
        }
    }

    if cmdline.is_empty() {
        bail!("Expected one argument at least: name of the file to review.");
    }

    let mut messages = Vec::new();
    for file in cmdline {
        let file_messages = parser.parse_messages_from_file(file)?;
        messages.push(file_messages);
    }

    review_files_and_print(
        aichat_command,
        &["-r", role, "-m", model],
        language,
        parser.number_of_plural_cases,
        dictionary,
        messages,
    )?;

    Ok(())
}

fn review_files_and_print(
    aichat_command: &str,
    aichat_options: &[&str],
    language: &str,
    number_of_plural_cases: Option<usize>,
    dictionary: &str,
    mut messages: Vec<Vec<PoMessage>>,
) -> Result<()> {
    let parser = Parser {
        number_of_plural_cases,
    };

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
        let message_text = format!(
            r#"
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
"#
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
                    println!("# Reviewed message:\n{errors}#, fuzzy\n{new_message}");
                } else {
                    eprintln!("# ERROR: Wrong msgid field when trying to review:\n{message}\n# Review:\n=====\n{new_message_text_slice}\n=====");
                    println!("# Reviewed message (warning:wrong id after review):\n{errors}#, fuzzy\n{message}");
                }
            }

            Err(e) => {
                eprintln!("# ERROR: Cannot parse review of message: {:#}:\n{message}\n# Review:\n=====\n{new_message_text_slice}\n=====", e);
                println!("#UNReviewed message (cannot parse review):\n#, fuzzy\n{message}");
            }
        }
    }

    Ok(())
}

fn help_translate() {
    println!(
        r#"
Usage: po-tools [GLOBAL_OPTIONS] translate [OPTIONS] [--] FILE

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


"#
    );
}

fn help_review() {
    println!(
        r#"
Usage: po-tools [GLOBAL_OPTIONS] review [OPTIONS] [--] FILE1 [FILE2...]

WORK IN PROGRESS.

Review multiple different translations of same messages and select the bese one among them using AI tools (aichat, ollama).

OPTIONS:

  -l | --language LANG  Language to use. Default value: "Ukrainian".

  -m | --model MODEL    AI model to use with aichat. Default value: "ollama:phi4:14b-q8_0".
                        Additional models: "aya-expanse:32b-q3_K_S", "codestral:22b-v0.1-q5_K_S".

  -r | --role ROLE      AI role to use with aichat.  Default value: "translate-po".
                        For better reproducibility, set temperature and top_p to 0, to remove randomness.

"#
    );
}

fn validate_message(message: &PoMessage) -> String {
    use crate::command_check_symbols::check_symbols;
    use crate::command_translate_and_print::PoMessage::*;

    match message {
        Regular { msgstr, .. } | RegularWithContext { msgstr, .. } => {
            if msgstr.is_empty() {
                return "Message is not translated.".to_string();
            }
        }

        Plural { msgstr, .. } | PluralWithContext { msgstr, .. } => {
            for msgstr in msgstr {
                if msgstr.is_empty() {
                    return "Message is not translated fully.".to_string();
                }
            }
        }

        Header { .. } => {}
    }

    match check_symbols(message) {
        None => "".into(),
        Some(errors) => errors,
    }
}
