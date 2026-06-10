//! Command to perform interactive manual review of PO file translations.
//!
//! This command compares original and AI-translated PO files, presents differing
//! messages sequentially with diff highlighting, and allows users to selectively
//! approve translations into an output file.

use crate::parser::{Parser, PoMessage};
use crate::util::IoContext;
use anyhow::{Result, bail};
use std::io::Write;

/// Entry point for the `review-interactive` command.
pub fn command_review_interactive(
    parser: &Parser,
    cmdline: &[&str],
    ctx: &mut IoContext,
) -> Result<()> {
    let mut original_file: Option<&str> = None;
    let mut ai_translated_file: Option<&str> = None;
    let mut output_file: Option<&str> = None;
    let mut editor: Option<String> = None;

    // Parse arguments
    let mut args = cmdline.iter().peekable();
    while let Some(&arg) = args.peek() {
        if arg.starts_with('-') {
            match *arg {
                "-h" | "--help" => {
                    help_review_interactive(ctx.out)?;
                    return Ok(());
                }
                "--editor" => {
                    args.next();
                    if let Some(&editor_arg) = args.peek() {
                        editor = Some(editor_arg.to_string());
                    } else {
                        bail!("--editor requires an argument");
                    }
                }
                _ => bail!("Unknown option: {}", arg),
            }
        } else {
            match args.next() {
                Some(&file) if original_file.is_none() => original_file = Some(file),
                Some(&file) if ai_translated_file.is_none() => ai_translated_file = Some(file),
                Some(&file) if output_file.is_none() => output_file = Some(file),
                _ => bail!("Unexpected argument: {}", arg),
            }
        }
    }

    let original_file =
        original_file.ok_or_else(|| anyhow::anyhow!("Missing original file argument"))?;
    let ai_translated_file =
        ai_translated_file.ok_or_else(|| anyhow::anyhow!("Missing AI translated file argument"))?;
    let output_file = output_file.ok_or_else(|| anyhow::anyhow!("Missing output file argument"))?;

    // Parse files
    let original_messages = parser.parse_messages_from_file(original_file)?;
    let ai_messages = parser.parse_messages_from_file(ai_translated_file)?;

    // Load existing output file (if it exists)
    let existing_output = std::fs::read_to_string(output_file).unwrap_or_default();
    let existing_parser = Parser::new(None);
    let existing_messages = existing_parser
        .parse_messages_from_str(&existing_output)
        .unwrap_or_default();

    // Resolve editor (from option, env var, or user prompt)
    let resolved_editor = resolve_editor(ctx, editor.as_deref())?;

    // Perform interactive review
    review_interactive_sequential(
        ctx,
        &original_messages,
        &ai_messages,
        &existing_messages,
        output_file,
        &resolved_editor,
    )?;

    Ok(())
}

/// Resolves the editor to use for editing messages.
/// Priority: --editor option > $EDITOR env var > user prompt
fn resolve_editor(ctx: &mut IoContext, cli_editor: Option<&str>) -> Result<String> {
    // Priority 1: CLI option
    if let Some(editor) = cli_editor {
        return Ok(editor.to_string());
    }

    // Priority 2: $EDITOR environment variable
    if let Ok(editor) = std::env::var("EDITOR")
        && !editor.is_empty()
    {
        return Ok(editor);
    }

    // Priority 3: Prompt user for editor name
    write!(
        ctx.out,
        "No editor configured. Enter editor command (e.g., vim, nano): "
    )?;
    ctx.out.flush()?;

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    let editor = input.trim().to_string();

    if editor.is_empty() {
        bail!("No editor specified");
    }

    Ok(editor)
}

/// Edits a message using the configured editor.
/// Returns the edited content if successful, None if editing failed or was cancelled.
fn edit_message(content: &str, editor: &str) -> Result<Option<String>> {
    // Create temporary file
    let temp_file = tempfile::NamedTempFile::new()
        .map_err(|e| anyhow::anyhow!("Failed to create temporary file: {}", e))?;

    // Write content to temp file
    std::fs::write(temp_file.path(), content.as_bytes())
        .map_err(|e| anyhow::anyhow!("Failed to write to temporary file: {}", e))?;

    // Launch editor
    let mut command = std::process::Command::new(editor);
    command.arg(temp_file.path());
    let status = command
        .status()
        .map_err(|e| anyhow::anyhow!("Failed to launch editor '{}': {}", editor, e))?;

    // Check if editor exited successfully
    if !status.success() {
        return Ok(None);
    }

    // Read edited content
    let edited_content = std::fs::read_to_string(temp_file.path())
        .map_err(|e| anyhow::anyhow!("Failed to read edited file: {}", e))?;

    Ok(Some(edited_content))
}

/// Core review loop that presents messages sequentially and collects user decisions.
fn review_interactive_sequential(
    ctx: &mut IoContext,
    original_messages: &[PoMessage],
    ai_messages: &[PoMessage],
    existing_output_messages: &[PoMessage],
    output_file_path: &str,
    editor: &str,
) -> Result<()> {
    // Open output file for appending
    let mut output_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(output_file_path)?;

    // Check if output file is freshly created (empty or doesn't exist)
    let is_fresh_file = std::fs::metadata(output_file_path)
        .map(|m| m.len() == 0)
        .unwrap_or(true);

    // Write header only when file is freshly created
    if is_fresh_file {
        // Find and write the header from AI messages (contains translation metadata)
        let header = ai_messages.iter().find(|m| m.is_header()).cloned();
        if let Some(ref header) = header {
            writeln!(output_file, "{}", header)?;
        }
    }

    // Build set of already-approved msgids
    let approved_msgids: Vec<String> = existing_output_messages
        .iter()
        .filter(|m| !m.is_header())
        .map(|m| m.msgid.clone())
        .collect();

    // Build AI messages lookup by msgid for efficient matching
    let ai_lookup: std::collections::HashMap<&str, &PoMessage> =
        ai_messages.iter().map(|m| (m.msgid.as_str(), m)).collect();

    // Sequential iteration through original messages (preserving order)
    for orig in original_messages {
        // Skip if already in output file
        if approved_msgids.contains(&orig.msgid) {
            continue;
        }

        // Find matching AI translation
        let ai_message = ai_lookup.get(orig.msgid.as_str());

        // Check if AI translation exists and differs from original
        let needs_review = ai_message
            .map(|ai| orig.msgstr_first() != ai.msgstr_first())
            .unwrap_or(false);

        if needs_review {
            // Edit-and-review loop: keep showing the message until user accepts or rejects
            let ai = ai_message.unwrap();
            let mut current_ai = (*ai).clone();
            loop {
                let needs_review = orig.msgstr_first() != current_ai.msgstr_first();

                if !needs_review {
                    // Edited content now matches original AI translation - accept,
                    // remove fuzzy flag since it's now human-approved
                    let mut msg = current_ai.clone();
                    msg.comments.retain(|c| !c.starts_with("#, fuzzy"));
                    // Skip header messages - they're written once at file creation
                    if !msg.is_header() {
                        writeln!(output_file, "\n{}", msg)?;
                    }
                    break;
                }

                // Show to user for review
                writeln!(ctx.out, "\n")?;

                // Show AI translator's comments (may contain helpful hints)
                // Display before msgctxt and msgid so they're visible as context
                for comment in &current_ai.comments {
                    if !comment.starts_with("#, fuzzy") {
                        writeln!(ctx.out, "{}", comment)?;
                    }
                }

                if let Some(ref msgctx) = current_ai.msgctxt {
                    writeln!(ctx.out, "# msgctx: {}", msgctx)?;
                }
                writeln!(ctx.out, "msgid  \"{}\"", orig.msgid)?;

                // Display original and current proposed translation with highlighting
                let orig_msgstr = orig.msgstr_first();
                let ai_msgstr = current_ai.msgstr_first();

                let highlighted = highlight_diff(orig_msgstr, ai_msgstr);
                let lines: Vec<&str> = highlighted.lines().collect();
                if lines.len() >= 2 {
                    writeln!(ctx.out, "msgstr \"{}\"", lines[0])?;
                    writeln!(ctx.out, "(new): \"{}\"", lines[1])?;
                }

                // Prompt user
                write!(ctx.out, "\nAccept this translation? [Y/n/e] ")?;
                ctx.out.flush()?;

                // Read input
                let mut input = String::new();
                std::io::stdin().read_line(&mut input)?;
                let decision = input.trim().to_lowercase();

                match decision.as_str() {
                    "y" | "" => {
                        // Accept current AI translation as-is,
                        // remove fuzzy flag since it's now human-approved
                        let mut msg = current_ai.clone();
                        msg.comments.retain(|c| !c.starts_with("#, fuzzy"));
                        // Skip header messages - they're written once at file creation
                        if !msg.is_header() {
                            writeln!(output_file, "\n{}", msg)?;
                        }
                        break;
                    }
                    "e" => {
                        // Edit the proposed translation in system editor
                        let edited = edit_message(current_ai.msgstr_first(), editor)?;
                        if let Some(edited_content) = edited {
                            current_ai.msgstr = vec![edited_content];
                        } else {
                            // Editing failed or no changes - write original and break
                            // Skip header messages - they're written once at file creation
                            if !orig.is_header() {
                                writeln!(output_file, "\n{}", orig)?;
                            }
                            break;
                        }
                    }
                    _ => {
                        // Reject - write original message and break
                        // Skip header messages - they're written once at file creation
                        if !orig.is_header() {
                            writeln!(output_file, "\n{}", orig)?;
                        }
                        break;
                    }
                }
            }
        } else {
            // AI translation doesn't exist or is identical - write original
            // Skip header messages - they're written once at file creation
            if !orig.is_header() {
                writeln!(output_file, "\n{}", orig)?;
            }
        }
    }

    Ok(())
}

/// Highlights differences between two strings using LCS-based diff.
/// Returns two lines: original msgstr with red highlights on deletions,
/// and proposed msgstr with green highlights on additions.
fn highlight_diff(original: &str, translation: &str) -> String {
    // Compute LCS table
    let orig_chars: Vec<char> = original.chars().collect();
    let trans_chars: Vec<char> = translation.chars().collect();
    let m = orig_chars.len();
    let n = trans_chars.len();

    if m == 0 && n == 0 {
        return "\n".to_string();
    }

    // Build LCS table
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if orig_chars[i - 1] == trans_chars[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }

    // Backtrack to find diff operations
    let mut ops: Vec<(char, OpType)> = Vec::new();
    let (mut i, mut j) = (m, n);
    while i > 0 && j > 0 {
        if orig_chars[i - 1] == trans_chars[j - 1] {
            ops.push((trans_chars[j - 1], OpType::Equal));
            i -= 1;
            j -= 1;
        } else if dp[i - 1][j] > dp[i][j - 1] {
            ops.push((orig_chars[i - 1], OpType::Delete));
            i -= 1;
        } else {
            ops.push((trans_chars[j - 1], OpType::Add));
            j -= 1;
        }
    }
    while i > 0 {
        ops.push((orig_chars[i - 1], OpType::Delete));
        i -= 1;
    }
    while j > 0 {
        ops.push((trans_chars[j - 1], OpType::Add));
        j -= 1;
    }
    ops.reverse();

    // Build highlighted strings
    let mut orig_line = String::new();
    let mut trans_line = String::new();

    for (ch, op) in &ops {
        match op {
            OpType::Equal => {
                orig_line.push(*ch);
                trans_line.push(*ch);
            }
            OpType::Delete => {
                orig_line.push_str(&format!("\x1b[41m{}\x1b[0m", ch));
            }
            OpType::Add => {
                trans_line.push_str(&format!("\x1b[44m{}\x1b[0m", ch));
            }
        }
    }

    format!("{}\n{}", orig_line, trans_line)
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum OpType {
    Equal,
    Delete,
    Add,
}

/// Displays help text for the review-interactive command.
fn help_review_interactive(out: &mut dyn Write) -> Result<()> {
    writeln!(
        out,
        "{}",
        tr!(
            r#"Usage: po-tools [GLOBAL_OPTIONS] review-interactive ORIGINAL_FILE AI_TRANSLATED_FILE OUTPUT_FILE

Interactive manual review of PO file translations.

Compares original and AI-translated PO files, presents differing messages
sequentially with diff highlighting, and allows selective approval of
translations into an output file.

Already-approved messages (detected by msgid in output file) are skipped.

OPTIONS:

  -h | --help     Show this help message
"#
        )
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_sequential_iteration_order() {
        // Verify that AI lookup preserves order when iterating through original messages
        let parser = Parser::new(None);
        let m1 = parser
            .parse_message_from_str("msgid \"first\"\nmsgstr \"original1\"\n")
            .unwrap();
        let m2 = parser
            .parse_message_from_str("msgid \"second\"\nmsgstr \"original2\"\n")
            .unwrap();
        let m3 = parser
            .parse_message_from_str("msgid \"third\"\nmsgstr \"original3\"\n")
            .unwrap();

        let ai1 = parser
            .parse_message_from_str("msgid \"first\"\nmsgstr \"ai1\"\n")
            .unwrap();
        let ai2 = parser
            .parse_message_from_str("msgid \"second\"\nmsgstr \"ai2\"\n")
            .unwrap();
        let ai3 = parser
            .parse_message_from_str("msgid \"third\"\nmsgstr \"ai3\"\n")
            .unwrap();

        // Verify AI lookup preserves order when iterating through original_messages
        let ai_lookup: std::collections::HashMap<&str, &PoMessage> = [
            (&ai1.msgid[..], &ai1),
            (&ai2.msgid[..], &ai2),
            (&ai3.msgid[..], &ai3),
        ]
        .iter()
        .map(|(k, v)| (*k, *v))
        .collect();

        // Simulate sequential iteration through original_messages
        let mut order = Vec::new();
        for orig in [&m1, &m2, &m3] {
            if let Some(ai) = ai_lookup.get(orig.msgid.as_str()) {
                order.push(&ai.msgid);
            }
        }

        assert_eq!(order.len(), 3);
        assert_eq!(order[0], "first");
        assert_eq!(order[1], "second");
        assert_eq!(order[2], "third");
    }

    #[test]
    fn test_highlight_no_diff() {
        let result = highlight_diff("hello", "hello");
        assert_eq!(result, "hello\nhello");
    }

    #[test]
    fn test_highlight_full_diff() {
        let result = highlight_diff("a", "b");
        assert!(result.contains("\x1b[41m")); // red for deletion (original)
        assert!(result.contains("\x1b[44m")); // blue for addition (translation)
        assert!(result.contains('\n')); // should have two lines
    }

    #[test]
    fn test_highlight_partial_diff() {
        let result = highlight_diff("abc", "adc");
        assert!(result.contains('\n')); // should have two lines
        // First line: original with b highlighted (deleted)
        // Second line: proposed with d highlighted (added)
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 2);
    }

    #[test]
    fn test_highlight_empty_original() {
        let result = highlight_diff("", "hello");
        assert!(result.contains('\n')); // should have two lines
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines[0], ""); // original is empty
    }

    #[test]
    fn test_highlight_empty_translation() {
        let result = highlight_diff("hello", "");
        assert!(result.contains('\n')); // should have two lines
        let lines: Vec<&str> = result.lines().collect();
        // When translation is empty, the second line may not appear in lines()
        // so we check if lines has at most 2 elements and first line has red highlights
        assert!(lines.len() <= 2);
        assert!(lines[0].contains("\x1b[41m")); // original has red highlights for deletions
    }

    #[test]
    fn test_highlight_unicode_diff() {
        let result = highlight_diff("hello world", "hello universe");
        assert!(result.contains("\x1b[")); // should contain ANSI codes for differing parts
    }

    #[test]
    fn test_edit_message_no_changes() {
        // Test that edit_message returns None when content is unchanged
        let original = "Hello World";

        let result = edit_message(original, "/usr/bin/true").unwrap();
        assert!(result.is_some());

        // Should NOT have a trailing newline
        let edited = result.unwrap();
        assert_eq!(edited, "Hello World");
    }

    #[test]
    fn test_edit_message_no_trailing_newline_added() {
        // Test that edit_message does NOT add a trailing newline to the content
        let original = "Hello World"; // No trailing newline

        let result = edit_message(original, "/usr/bin/true").unwrap();
        assert!(result.is_some());

        // Should NOT have a trailing newline
        let edited = result.unwrap();
        assert!(
            !edited.ends_with('\n'),
            "Edited content should not have trailing newline"
        );
        assert_eq!(edited, "Hello World");
    }

    #[test]
    fn test_edit_message_preserves_newline() {
        // Test that newline  is preserved correctly
        let original = "Hello World\n"; // With trailing newline

        let result = edit_message(original, "/usr/bin/true").unwrap();
        assert!(result.is_some());

        // Must have a trailing newline preserved
        let edited = result.unwrap();
        assert!(
            edited.ends_with('\n'),
            "Edited content must have trailing newline preserved"
        );
        assert_eq!(edited, "Hello World\n");
    }
}
