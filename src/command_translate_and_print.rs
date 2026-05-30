//! Command to translate PO messages using AI and translation memory.
//!
//! This is the most complex command, involving fuzzy matching from TM,
//! dictionary lookups, and interaction with an AI model.

use crate::dictionary::Dictionary;
use crate::parser::{Parser, PoMessage};
use crate::util::{AiBackend, IoContext, validate_message};
use anyhow::{Context, Result, bail};
use regex::Regex;
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
    let mut prompt: Option<String> = None;
    let mut benchmark = false;

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

            ["-p", p, ..] | ["--prompt", p, ..] => {
                prompt = Some(p.to_string());
                cmdline = &cmdline[2..];
            }

            ["--debug", ..] => {
                debug = true;
                cmdline = &cmdline[1..];
            }

            ["--benchmark", ..] => {
                benchmark = true;
                cmdline = &cmdline[1..];
            }

            ["-c", cmd, ..] | ["--ai-command", cmd, ..] => {
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

        let force_matcher = if let Some(k) = &force_keyword {
            let pattern = format!(r"(?i)\b{}s?\b", regex::escape(k));
            Some(Regex::new(&pattern).with_context(|| {
                tr!("Cannot compile regex for keyword \"{}\".").replace("{}", k)
            })?)
        } else {
            None
        };

        let config = TranslateConfig {
            backend: backend.clone(),
            language,
            number_of_plural_cases: parser.number_of_plural_cases,
            tm_messages: &tm_messages,
            dictionaries: &dictionaries,
            debug,
            copy_comments: true,
            keyword_matcher: force_matcher,
            prompt: prompt.clone(),
            benchmark,
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
    keyword_matcher: Option<Regex>,
    prompt: Option<String>,
    benchmark: bool,
}

fn translate_and_print(
    ctx: &mut IoContext,
    config: &TranslateConfig,
    messages: &[PoMessage],
) -> Result<()> {
    let mut score_sum = 0.0;
    let mut score_count = 0u32;

    for message in messages {
        let should_force = config
            .keyword_matcher
            .as_ref()
            .map(|re| re.is_match(&message.msgid))
            .unwrap_or(false);

        if message.is_header() {
            // Headers are always passed through unchanged
            writeln!(ctx.out, "{message}")?;
        } else if config.benchmark {
            // In benchmark mode, process all non-header messages
            let (score, scored) = translate_benchmark_message(ctx, config, message)?;
            if scored {
                score_sum += score;
                score_count += 1;
            }
        } else if message.is_translated() && !message.is_fuzzy() && !should_force {
            // Just copy already-translated messages
            writeln!(ctx.out, "{message}")?;
        } else {
            translate_single_message(ctx, config, message)?;
        }
    }

    // Print summary to stderr
    if score_count > 0 {
        let avg = score_sum / score_count as f64;
        writeln!(
            ctx.err,
            "BENCHMARK SUMMARY: {} messages scored, average similarity: {:.4}",
            score_count, avg
        )?;
    }

    Ok(())
}

/// Translate a single message in benchmark mode.
/// Returns (score, was_scored) — if the message was skipped (header, etc.), was_scored is false.
fn translate_benchmark_message(
    ctx: &mut IoContext,
    config: &TranslateConfig,
    message: &PoMessage,
) -> Result<(f64, bool)> {
    if !message.is_translated() {
        // Output original message unchanged and do not score
        writeln!(ctx.out, "{}", message)?;
        return Ok((0.0, false));
    }

    // Erase translated message
    let mut message_to_translate = message.clone();
    message_to_translate.msgstr = Vec::new();

    // Get AI translation
    let ai_msgstr = match translate_and_get_ai_msgstr(ctx, config, &message_to_translate) {
        Ok(ai_str) => ai_str,
        Err(e) => {
            writeln!(
                ctx.err,
                "[Benchmark] Warning: AI translation failed for msgid \"{}\": {}",
                message.msgid, e
            )?;
            // Output original message with error comment
            let mut msg = message.clone();
            msg.comments
                .insert(0, "# Benchmark: ERROR - AI translation failed".to_string());
            writeln!(ctx.out, "{}", msg)?;
            return Ok((0.0, true));
        }
    };

    // Compare human vs AI using normalized Levenshtein
    let human_msgstr = message.msgstr_first();
    let score = normalized_levenshtein(human_msgstr, &ai_msgstr);

    // Print per-message score to stderr
    writeln!(ctx.err, "[Score: {:.4}]", score)?;

    // Build benchmark comment with AI translation (escaped for PO format)
    let ai_escaped = crate::parser::escape_string(&ai_msgstr);
    let benchmark_comment = format!("# Benchmark: Score={:.4} | AI: {}", score, ai_escaped);

    // Output original message with benchmark comment prepended
    let mut msg = message.clone();
    msg.comments.insert(0, benchmark_comment);
    writeln!(ctx.out, "{}", msg)?;

    Ok((score, true))
}

/// Common logic for calling AI translator and getting the raw string slice of translation.
fn execute_ai_translation_request(
    ctx: &mut IoContext,
    config: &TranslateConfig,
    message: &PoMessage,
) -> Result<String> {
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

    let prompt_text = if let Some(p) = &config.prompt {
        format!("IMPORTANT: {p}\n")
    } else {
        "".to_string()
    };

    // Translation template
    let message_text = format!(
        r#"{dict_context}
{fuzzy_match_text}
<instruction>
IMPORTANT: Translate text in <message></message> tag only and _nothing else_.
IMPORTANT: Answers must be VALID Gettext PO messages. Msgid field must be verbatim copy of original msgid, while msgstr must be {language} translation.
IMPORTANT: Don't translate <context> and <dictionary>. They are just for reference.
IMPORTANT: Prefer translations proposed by dictionary.
You are a professional English (en_US) to {language} translator. Your goal is to accurately convey the meaning and nuances of the original English text while adhering to {language} grammar, vocabulary, and cultural sensitivities.
Produce only the {language} translation, without any additional explanations or commentary. Please translate the following English text in <message></message> into {language}.
{prompt_text}
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
            tr!("Message to translator")
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
            tr!("Reply from translator")
        )?;
        writeln!(ctx.err, "{new_message_text}")?;
        writeln!(
            ctx.err,
            "----{}----------------------------------------------------------------",
            tr!("End of reply")
        )?;
    }

    // Skip thinking/reasoning tags from reasoning models
    let mut cleaned_text = &new_message_text[..];
    for tag in &["</think>", "</reasoning>"] {
        if let Some(start) = cleaned_text.rfind(tag) {
            cleaned_text = &cleaned_text[(start + tag.len())..];
        }
    }

    let new_message_text_slice = if let Some(end) = cleaned_text.rfind("</message>") {
        // Extract text between <message> and </message>, if they are present
        let tag_open = "<message>";
        if let Some(start) = cleaned_text[..end].rfind(tag_open) {
            &cleaned_text[(start + tag_open.len())..end]
        } else {
            cleaned_text
        }
    } else if let Some(start) = cleaned_text.rfind("msgid ") {
        &cleaned_text[start..]
    } else {
        cleaned_text
    };

    Ok(new_message_text_slice.to_string())
}

/// Helper: run the AI translation for a single message and return the AI's msgstr[0] string.
fn translate_and_get_ai_msgstr(
    ctx: &mut IoContext,
    config: &TranslateConfig,
    message: &PoMessage,
) -> Result<String> {
    let new_message_text_slice = execute_ai_translation_request(ctx, config, message)?;

    let is_plural = message.is_plural();
    let parser = Parser {
        number_of_plural_cases: if is_plural {
            Some(config.number_of_plural_cases.unwrap_or(2))
        } else {
            config.number_of_plural_cases
        },
        ignore_garbage_after_msgstr: true,
        strip_comments: true,
    };

    match parser.parse_message_from_str(&new_message_text_slice) {
        Ok(ai_message) => {
            let result = ai_message.msgstr_first().to_string();
            if result.is_empty() {
                anyhow::bail!("AI returned an empty translation");
            }
            Ok(result)
        }
        Err(e) => {
            writeln!(
                ctx.err,
                "[Benchmark] Warning: Cannot parse AI response for msgid \"{}\": {}",
                message.msgid, e
            )?;
            anyhow::bail!("Cannot parse AI response: {e}");
        }
    }
}

fn translate_single_message(
    ctx: &mut IoContext,
    config: &TranslateConfig,
    message: &PoMessage,
) -> Result<()> {
    let new_message_text_slice = execute_ai_translation_request(ctx, config, message)?;

    let is_plural = message.is_plural();
    let parser = Parser {
        number_of_plural_cases: if is_plural {
            Some(config.number_of_plural_cases.unwrap_or(2))
        } else {
            config.number_of_plural_cases
        },
        ignore_garbage_after_msgstr: true,
        strip_comments: true,
    };

    match parser.parse_message_from_str(&new_message_text_slice) {
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
                    "{}:\n{errors}#, fuzzy\n{new_message}",
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
                    "{}:\n{errors}#, fuzzy\n{fixed_message}",
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

  -c | --ai-command COMMAND  Custom command to use for translation instead of aichat.
                        Example: --ai-command "ollama run gemma4:latest"
                        Options --model, --role, --rag will not work with this option.

  -M | --tm | --translation-memory FILE   Local Translation Memory file (PO format) to use for fuzzy matching.

  -d | --dictionary FILE  TSV dictionary file to use for context. Can be used multiple times.

  -k | --force-by-keyword KEYWORD  Force translation of messages whose msgid contains KEYWORD.

  -p | --prompt PROMPT  Additional instructions for AI models during translation.

  --debug               Print inputs and outputs of AI models to stderr.

  --benchmark           Compare AI translations with existing human translations.
                        Outputs original messages with benchmark score comments in stdout,
                        and per-message scores plus summary to stderr.
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
            keyword_matcher: None,
            prompt: None,
            benchmark: false,
        };

        let message = parser.parse_message_from_str("msgid \"a\"\nmsgstr \"\"\n")?;
        translate_and_print(&mut ctx, &config, &[message])?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("msgid \"a\""));
        assert!(result.contains("msgstr \"translated_a\""));
        assert_eq!(
            String::from_utf8_lossy(&err),
            "",
            "unexpected stderr output"
        );
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
            keyword_matcher: None,
            prompt: None,
            benchmark: false,
        };

        let message = parser.parse_message_from_str("# comment\nmsgid \"a\"\nmsgstr \"\"\n")?;
        translate_and_print(&mut ctx, &config, &[message])?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("# comment"));
        assert!(result.contains("msgid \"a\""));
        assert!(result.contains("msgstr \"translated_a\""));
        assert_eq!(
            String::from_utf8_lossy(&err),
            "",
            "unexpected stderr output"
        );
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
            keyword_matcher: None,
            prompt: None,
            benchmark: false,
        };

        // already translated message
        let message = parser.parse_message_from_str("msgid \"a\"\nmsgstr \"existing_a\"\n")?;
        translate_and_print(&mut ctx, &config, &[message])?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("msgid \"a\""));
        assert!(result.contains("msgstr \"existing_a\""));
        assert!(!result.contains("Translated message"));
        assert!(!result.contains("SHOULD NOT BE CALLED"));
        assert_eq!(
            String::from_utf8_lossy(&err),
            "",
            "unexpected stderr output"
        );
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
            keyword_matcher: None,
            prompt: None,
            benchmark: false,
        };

        // fuzzy message
        let message =
            parser.parse_message_from_str("#, fuzzy\nmsgid \"a\"\nmsgstr \"old_fuzzy_a\"\n")?;
        translate_and_print(&mut ctx, &config, &[message])?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("msgid \"a\""));
        assert!(result.contains("msgstr \"translated_fuzzy_a\""));
        assert!(result.contains("Translated message"));
        assert_eq!(
            String::from_utf8_lossy(&err),
            "",
            "unexpected stderr output"
        );
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
            // Backend should not be called
            backend: AiBackend::mock("msgid \"a %d\"\nmsgstr \"translated_a\""),
            language: "Ukrainian",
            number_of_plural_cases: None,
            tm_messages: &[],
            dictionaries: &[],
            debug: false,
            copy_comments: true,
            keyword_matcher: None,
            prompt: None,
            benchmark: false,
        };

        let message = parser.parse_message_from_str("msgid \"a %d\"\nmsgstr \"\"\n")?;
        translate_and_print(&mut ctx, &config, &[message])?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("Warning: Incorrect symbols"));
        assert!(result.contains("#, fuzzy"));
        assert_eq!(
            String::from_utf8_lossy(&err),
            "",
            "unexpected stderr output"
        );
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
            // Backend should not be called
            backend: AiBackend::mock("msgid \"a \"\nmsgstr \"translated_a\""),
            language: "Ukrainian",
            number_of_plural_cases: None,
            tm_messages: &[],
            dictionaries: &[],
            debug: false,
            copy_comments: true,
            keyword_matcher: None,
            prompt: None,
            benchmark: false,
        };

        let message = parser.parse_message_from_str("msgid \"a \"\nmsgstr \"\"\n")?;
        translate_and_print(&mut ctx, &config, &[message])?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("Warning: Whitespace mismatch"));
        assert!(result.contains("#, fuzzy"));
        assert_eq!(
            String::from_utf8_lossy(&err),
            "",
            "unexpected stderr output"
        );
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
            keyword_matcher: Some(Regex::new(r"(?i)\bkeywords?\b").unwrap()),
            prompt: None,
            benchmark: false,
        };

        // already translated message with keyword in msgid
        let message = parser
            .parse_message_from_str("msgid \"keyword message\"\nmsgstr \"old_translation\"\n")?;
        translate_and_print(&mut ctx, &config, &[message])?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("msgid \"keyword message\""));
        assert!(result.contains("msgstr \"forced_translation\""));
        assert!(result.contains("Translated message"));
        assert_eq!(
            String::from_utf8_lossy(&err),
            "",
            "unexpected stderr output"
        );
        Ok(())
    }

    #[test]
    fn test_translate_skip_translated_with_keyword_no_match() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        // Backend should not be called because the message is translated
        // and the keyword "tag" is NOT in the msgid.
        // Word "percenTAGe" contains "tag", but must not trigger the translation.
        let config = TranslateConfig {
            backend: AiBackend::mock("msgid \"percentage\"\nmsgstr \"у відсотках\"\n"),
            language: "Ukrainian",
            number_of_plural_cases: None,
            tm_messages: &[],
            dictionaries: &[],
            debug: false,
            copy_comments: false,
            keyword_matcher: Some(Regex::new(r"(?i)\btags?\b").unwrap()),
            prompt: None,
            benchmark: false,
        };

        let message =
            parser.parse_message_from_str("msgid \"percentage\"\nmsgstr \"відсоток\"\n")?;

        translate_and_print(&mut ctx, &config, &[message])?;

        let result = String::from_utf8(out)?;
        assert!(!result.contains("у відсотках"));
        assert!(result.contains("msgid \"percentage\""));
        assert!(result.contains("msgstr \"відсоток\""));
        Ok(())
    }

    #[test]
    fn test_translate_custom_prompt() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let config = TranslateConfig {
            backend: AiBackend::mock("msgid \"a\"\nmsgstr \"b\""),
            language: "Ukrainian",
            number_of_plural_cases: None,
            tm_messages: &[],
            dictionaries: &[],
            // Use debug mode to see message sent to AI
            debug: true,
            copy_comments: true,
            keyword_matcher: None,
            prompt: Some("USE VERY FORMAL STYLE".to_string()),
            benchmark: false,
        };

        let message = parser.parse_message_from_str("msgid \"a\"\nmsgstr \"\"\n")?;
        translate_single_message(&mut ctx, &config, &message)?;

        let result = String::from_utf8(err)?;
        assert!(result.contains("USE VERY FORMAL STYLE"));
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
        assert_eq!(
            String::from_utf8_lossy(&err),
            "",
            "unexpected stderr output"
        );
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
        assert_eq!(
            String::from_utf8_lossy(&err),
            "",
            "unexpected stderr output"
        );
        Ok(())
    }

    #[test]
    fn test_translate_force_keyword_comprehensive() -> Result<()> {
        let parser = Parser::new(None);

        // Test matches for "tag"
        // Expected: tag, tags, Tag match; percentage, tagging do NOT match.
        let test_cases = vec![
            ("tag", true),
            ("tags", true),
            ("Tag", true),
            ("Two tags in a row", true),
            ("percentage", false),
            ("tagging", false),
        ];

        let tag_regex = Regex::new(r"(?i)\btags?\b").unwrap();

        for (msgid, should_match) in test_cases {
            let mut out = Vec::new();
            let mut err = Vec::new();
            let mut ctx = IoContext {
                out: &mut out,
                err: &mut err,
            };
            let config = TranslateConfig {
                backend: AiBackend::mock("msgid \"...\"\nmsgstr \"forced_translation\""),
                language: "Ukrainian",
                number_of_plural_cases: None,
                tm_messages: &[],
                dictionaries: &[],
                debug: false,
                copy_comments: false,
                keyword_matcher: Some(tag_regex.clone()),
                prompt: None,
                benchmark: false,
            };

            let message =
                parser.parse_message_from_str(&format!("msgid \"{msgid}\"\nmsgstr \"old\"\n"))?;
            translate_and_print(&mut ctx, &config, &[message])?;
            let result = String::from_utf8(out)?;
            if should_match {
                assert!(
                    result.contains("forced_translation"),
                    "Should have matched '{}'",
                    msgid
                );
            } else {
                assert!(
                    result.contains("old"),
                    "Should NOT have matched '{}'",
                    msgid
                );
                assert!(
                    !result.contains("forced_translation"),
                    "Should NOT have matched '{}'",
                    msgid
                );
            }
        }

        // Test multi-word keyword
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let config_big_endian = TranslateConfig {
            backend: AiBackend::mock("msgid \"...\"\nmsgstr \"forced_translation\""),
            language: "Ukrainian",
            number_of_plural_cases: None,
            tm_messages: &[],
            dictionaries: &[],
            debug: false,
            copy_comments: false,
            keyword_matcher: Some(Regex::new(r"(?i)\bbig endians?\b").unwrap()),
            prompt: None,
            benchmark: false,
        };

        let message =
            parser.parse_message_from_str("msgid \"This is big endian\"\nmsgstr \"old\"\n")?;
        translate_and_print(&mut ctx, &config_big_endian, &[message])?;
        let result = String::from_utf8(out)?;
        assert!(
            result.contains("forced_translation"),
            "Should match 'big endian'"
        );

        Ok(())
    }

    #[test]
    fn test_translate_broken_tags() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let broken_output = "<message>msgid \"a\"\nmsgstr \"translated_a\"</message>\n\
                             Some extra text mentioning <message> tag but not closing it.";

        let config = TranslateConfig {
            backend: AiBackend::mock(broken_output),
            language: "Ukrainian",
            number_of_plural_cases: None,
            tm_messages: &[],
            dictionaries: &[],
            debug: false,
            copy_comments: true,
            keyword_matcher: None,
            prompt: None,
            benchmark: false,
        };

        let message = parser.parse_message_from_str("msgid \"a\"\nmsgstr \"\"\n")?;
        // This should NOT panic
        translate_and_print(&mut ctx, &config, &[message])?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("msgid \"a\""));
        assert!(result.contains("msgstr \"translated_a\""));
        assert_eq!(
            String::from_utf8_lossy(&err),
            "",
            "unexpected stderr output"
        );
        Ok(())
    }

    // ---------------------------------------------------------------------------
    // Plural message translation tests
    // ---------------------------------------------------------------------------

    /// A plural message with empty msgstr entries triggers AI translation.
    #[test]
    fn test_translate_plural_message() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };

        // AI returns a fully-translated plural message with 3 forms.
        let ai_response = "msgid \"%d new patch,\"\nmsgid_plural \"%d new patches,\"\nmsgstr[0] \"%d нова латка,\"\nmsgstr[1] \"%d нові латки,\"\nmsgstr[2] \"%d нових латок,\"";

        let config = TranslateConfig {
            backend: AiBackend::mock(ai_response),
            language: "Ukrainian",
            number_of_plural_cases: Some(3),
            tm_messages: &[],
            dictionaries: &[],
            debug: false,
            copy_comments: true,
            keyword_matcher: None,
            prompt: None,
            benchmark: false,
        };

        let parser = Parser::new(Some(3));
        let message = parser.parse_message_from_str("msgid \"%d new patch,\"\nmsgid_plural \"%d new patches,\"\nmsgstr[0] \"\"\nmsgstr[1] \"\"")?;

        assert!(message.is_plural());
        translate_and_print(&mut ctx, &config, &[message])?;

        let result = String::from_utf8(out)?;
        // The output must contain the translated plural forms.
        assert!(result.contains("msgstr[0] \"%d нова латка,\""));
        assert!(result.contains("msgstr[1] \"%d нові латки,\""));
        assert!(result.contains("msgstr[2] \"%d нових латок,\""));
        // Must be marked as fuzzy (AI-generated).
        assert!(result.contains("#, fuzzy"));

        let stderr = String::from_utf8(err)?;
        assert_eq!(stderr, "", "unexpected stderr output: {stderr}");
        Ok(())
    }

    /// A plural message that is already fully translated should be passed through without calling AI.
    #[test]
    fn test_translate_skip_translated_plural() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };

        let config = TranslateConfig {
            // Backend should NOT be called because the message is already translated.
            backend: AiBackend::mock("SHOULD_NOT_APPEAR"),
            language: "Ukrainian",
            number_of_plural_cases: Some(3),
            tm_messages: &[],
            dictionaries: &[],
            debug: false,
            copy_comments: true,
            keyword_matcher: None,
            prompt: None,
            benchmark: false,
        };

        let parser = Parser::new(Some(3));
        // Fully translated plural message (all 3 forms).
        let message = parser.parse_message_from_str("msgid \"%d new patch,\"\nmsgid_plural \"%d new patches,\"\nmsgstr[0] \"%d нова латка,\"\nmsgstr[1] \"%d нові латки,\"\nmsgstr[2] \"%d нових латок,\"")?;

        translate_and_print(&mut ctx, &config, &[message])?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("msgstr[0] \"%d нова латка,\""));
        // The mock response must NOT appear in the output.
        assert!(!result.contains("SHOULD_NOT_APPEAR"));
        // No translation comment - it was copied verbatim.
        assert!(!result.contains("msgid \"translated_a\""));

        let stderr = String::from_utf8(err)?;
        assert_eq!(stderr, "", "unexpected stderr output: {stderr}");
        Ok(())
    }

    /// A plural message where only msgstr[0] is translated but others are empty should still trigger AI.
    #[test]
    fn test_translate_partially_translated_plural() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };

        // AI returns a fully-translated plural message.
        let ai_response = "msgid \"%d new patch,\"\nmsgid_plural \"%d new patches,\"\nmsgstr[0] \"%d нова латка,\"\nmsgstr[1] \"%d нові латки,\"\nmsgstr[2] \"%d нових латок,\"";

        let config = TranslateConfig {
            backend: AiBackend::mock(ai_response),
            language: "Ukrainian",
            number_of_plural_cases: Some(3),
            tm_messages: &[],
            dictionaries: &[],
            debug: false,
            copy_comments: true,
            keyword_matcher: None,
            prompt: None,
            benchmark: false,
        };

        let parser = Parser::new(Some(3));
        // Partially translated: msgstr[0] is filled but others are empty.
        let message = parser.parse_message_from_str("msgid \"%d new patch,\"\nmsgid_plural \"%d new patches,\"\nmsgstr[0] \"%s нова латка\"")?;

        // is_translated() returns false because not all msgstr entries are non-empty.
        assert!(!message.is_translated());

        translate_and_print(&mut ctx, &config, &[message])?;

        let result = String::from_utf8(out)?;
        // The AI\'s new translations should appear in the output.
        assert!(result.contains("msgstr[0] \"%d нова латка,\""));
        assert!(result.contains("#, fuzzy"));

        let stderr = String::from_utf8(err)?;
        assert_eq!(stderr, "", "unexpected stderr output: {stderr}");
        Ok(())
    }

    /// The AI prompt for a plural message must include the example template.
    #[test]
    fn test_translate_plural_prompt_contains_example() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };

        // We don't care about the response content; we only check debug stderr.
        let ai_response = "msgid \"%d new patch,\"\nmsgid_plural \"%d new patches,\"\nmsgstr[0] \"%d нова латка,\"\nmsgstr[1] \"%d нові латки,\"";

        let config = TranslateConfig {
            backend: AiBackend::mock(ai_response),
            language: "Ukrainian",
            number_of_plural_cases: Some(3),
            tm_messages: &[],
            dictionaries: &[],
            debug: true, // <-- enables printing the prompt to stderr
            copy_comments: true,
            keyword_matcher: None,
            prompt: None,
            benchmark: false,
        };

        let parser = Parser::new(Some(3));
        let message = parser.parse_message_from_str(
            "msgid \"%d new patch,\"\nmsgid_plural \"%d new patches,\"\nmsgstr[0] \"\"",
        )?;

        translate_and_print(&mut ctx, &config, &[message])?;

        let stderr = String::from_utf8(err)?;
        // The plural example template is only included when `is_plural` is true.
        assert!(
            stderr.contains("<example>"),
            "plural prompt must contain <example>"
        );
        assert!(
            stderr.contains("msgid_plural"),
            "<example> must contain msgid_plural"
        );
        Ok(())
    }

    /// When the AI returns a plural message with fewer msgstr entries than expected,
    /// the parser pads them to `number_of_plural_cases`. The validation should flag
    /// this as not-fully-translated.
    #[test]
    fn test_translate_plural_incomplete_ai_response() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };

        // AI only returned 2 forms instead of the expected 3.
        let ai_response = "msgid \"%d new patch,\"\nmsgid_plural \"%d new patches,\"\nmsgstr[0] \"%d нова латка,\"\nmsgstr[1] \"%d нові латки,\"";

        let config = TranslateConfig {
            backend: AiBackend::mock(ai_response),
            language: "Ukrainian",
            number_of_plural_cases: Some(3), // We expect 3 plural forms.
            tm_messages: &[],
            dictionaries: &[],
            debug: false,
            copy_comments: true,
            keyword_matcher: None,
            prompt: None,
            benchmark: false,
        };

        let parser = Parser::new(Some(3));
        let message = parser.parse_message_from_str(
            "msgid \"%d new patch,\"\nmsgid_plural \"%d new patches,\"\nmsgstr[0] \"\"",
        )?;

        translate_and_print(&mut ctx, &config, &[message])?;

        let result = String::from_utf8(out)?;
        // The parser pads to 3 forms - so msgstr[2] should be empty.
        assert!(
            result.contains("msgstr[2]"),
            "expected padded msgstr[2], got:\n{result}"
        );
        // validate_message() detects incomplete plural translation and adds an error comment.
        assert!(
            result.contains("Error: Message is not translated fully"),
            "incomplete plural msgstr should trigger validation warning, got:\n{result}"
        );

        let stderr = String::from_utf8(err)?;
        assert_eq!(stderr, "", "unexpected stderr output: {stderr}");
        Ok(())
    }

    // ---------------------------------------------------------------------------
    // Benchmark mode tests
    // ---------------------------------------------------------------------------

    /// In benchmark mode, the original human translation is preserved and a score comment is added.
    #[test]
    fn test_benchmark_basic() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let config = TranslateConfig {
            backend: AiBackend::mock("msgid \"hello world\"\nmsgstr \"привіт світ\""),
            language: "Ukrainian",
            number_of_plural_cases: None,
            tm_messages: &[],
            dictionaries: &[],
            debug: false,
            copy_comments: true,
            keyword_matcher: None,
            prompt: None,
            benchmark: true,
        };

        // Message already has a human translation
        let message =
            parser.parse_message_from_str("msgid \"hello world\"\nmsgstr \"привіт, світ\"")?;
        translate_and_print(&mut ctx, &config, &[message])?;

        let result = String::from_utf8(out)?;
        // Human translation must be preserved
        assert!(result.contains("msgstr \"привіт, світ\""));
        // Benchmark comment with score should be present
        assert!(
            result.contains("Benchmark:"),
            "expected benchmark comment in:\n{result}"
        );
        assert!(result.contains("Score="), "expected Score= in:\n{result}");
        assert!(
            result.contains("AI: привіт світ"),
            "expected AI translation in comments:\n{result}"
        );

        let stderr = String::from_utf8(err)?;
        // Per-message score and summary should be in stderr
        assert!(
            stderr.contains("[Score:"),
            "expected per-message score in stderr: {stderr}"
        );
        assert!(
            stderr.contains("BENCHMARK SUMMARY"),
            "expected summary in stderr: {stderr}"
        );
        Ok(())
    }

    /// Benchmark mode outputs headers unchanged.
    #[test]
    fn test_benchmark_header_passthrough() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let config = TranslateConfig {
            backend: AiBackend::mock("SHOULD_NOT_APPEAR"),
            language: "Ukrainian",
            number_of_plural_cases: None,
            tm_messages: &[],
            dictionaries: &[],
            debug: false,
            copy_comments: true,
            keyword_matcher: None,
            prompt: None,
            benchmark: true,
        };

        let header = parser.parse_message_from_str("msgid \"\"\nmsgstr \"header content\"")?;
        translate_and_print(&mut ctx, &config, &[header])?;

        let result = String::from_utf8(out)?;
        assert!(result.contains("msgstr \"header content\""));
        assert!(!result.contains("Benchmark:"));
        assert!(!result.contains("SHOULD_NOT_APPEAR"));
        Ok(())
    }

    /// Benchmark mode handles AI translation failure gracefully.
    #[test]
    fn test_benchmark_ai_failure() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let config = TranslateConfig {
            backend: AiBackend::mock("INVALID AI OUTPUT THAT CANNOT BE PARSED"),
            language: "Ukrainian",
            number_of_plural_cases: None,
            tm_messages: &[],
            dictionaries: &[],
            debug: false,
            copy_comments: true,
            keyword_matcher: None,
            prompt: None,
            benchmark: true,
        };

        let message =
            parser.parse_message_from_str("msgid \"test msg\"\nmsgstr \"human translation\"")?;
        translate_and_print(&mut ctx, &config, &[message])?;

        let result = String::from_utf8(out)?;
        // Original human translation should still be preserved
        assert!(result.contains("msgstr \"human translation\""));
        // Error comment should indicate failure
        assert!(
            result.contains("Benchmark:"),
            "expected benchmark comment:\n{result}"
        );

        let stderr = String::from_utf8(err)?;
        assert!(
            stderr.contains("Warning"),
            "expected warning in stderr: {stderr}"
        );
        Ok(())
    }

    /// Benchmark mode skips untranslated messages (since there is no human translation to compare against).
    #[test]
    fn test_benchmark_untranslated_skipped() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };
        let parser = Parser::new(None);

        let config = TranslateConfig {
            // Backend should not be called because the message is untranslated and should be skipped from scoring
            backend: AiBackend::mock("SHOULD_NOT_BE_CALLED"),
            language: "Ukrainian",
            number_of_plural_cases: None,
            tm_messages: &[],
            dictionaries: &[],
            debug: false,
            copy_comments: true,
            keyword_matcher: None,
            prompt: None,
            benchmark: true,
        };

        let message = parser.parse_message_from_str("msgid \"hello world\"\nmsgstr \"\"")?;
        translate_and_print(&mut ctx, &config, &[message])?;

        let result = String::from_utf8(out)?;
        // Original message should be output unchanged
        assert!(result.contains("msgstr \"\""));
        assert!(!result.contains("Benchmark:"));

        let stderr = String::from_utf8(err)?;
        // Summary should not count the skipped message
        assert!(!stderr.contains("BENCHMARK SUMMARY"));
        Ok(())
    }

    /// Benchmark mode can score plural messages by comparing the first plural form.
    #[test]
    fn test_benchmark_plural() -> Result<()> {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut ctx = IoContext {
            out: &mut out,
            err: &mut err,
        };

        // Mock AI returns a plural message
        let ai_response = "msgid \"%d new patch,\"\nmsgid_plural \"%d new patches,\"\nmsgstr[0] \"%d нова латка,\"\nmsgstr[1] \"%d нові латки,\"";

        let config = TranslateConfig {
            backend: AiBackend::mock(ai_response),
            language: "Ukrainian",
            number_of_plural_cases: Some(2),
            tm_messages: &[],
            dictionaries: &[],
            debug: false,
            copy_comments: true,
            keyword_matcher: None,
            prompt: None,
            benchmark: true,
        };

        let parser = Parser::new(Some(2));
        // Already translated human message
        let message = parser.parse_message_from_str("msgid \"%d new patch,\"\nmsgid_plural \"%d new patches,\"\nmsgstr[0] \"%d нова латка,\"\nmsgstr[1] \"%d нові латки,\"")?;
        translate_and_print(&mut ctx, &config, &[message])?;

        let result = String::from_utf8(out)?;
        // Original human translation is preserved
        assert!(result.contains("msgstr[0] \"%d нова латка,\""));
        assert!(result.contains("msgstr[1] \"%d нові латки,\""));
        // Benchmark comment with high/perfect score (1.0000)
        assert!(result.contains("Benchmark: Score=1.0000"));

        let stderr = String::from_utf8(err)?;
        assert!(stderr.contains("[Score: 1.0000]"));
        assert!(stderr.contains("BENCHMARK SUMMARY: 1 messages scored"));
        Ok(())
    }
}
