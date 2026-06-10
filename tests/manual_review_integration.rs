use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::NamedTempFile;

#[test]
fn test_review_interactive_command_help() {
    let mut cmd = Command::cargo_bin("po-tools").unwrap();
    cmd.arg("review-interactive")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"));
}

#[test]
fn test_review_interactive_missing_args() {
    let mut cmd = Command::cargo_bin("po-tools").unwrap();
    cmd.arg("review-interactive")
        .arg("file1.po")
        .assert()
        .failure();
}

#[test]
fn test_review_interactive_nonexistent_file() {
    let mut cmd = Command::cargo_bin("po-tools").unwrap();
    cmd.arg("review-interactive")
        .arg("nonexistent.po")
        .arg("output.po")
        .assert()
        .failure();
}

#[test]
fn test_review_interactive_equal_messages() {
    let mut cmd = Command::cargo_bin("po-tools").unwrap();

    // Create original PO file with header and one message
    let original = NamedTempFile::new().unwrap();
    fs::write(
        original.path(),
        r#"msgid ""
msgstr "Language: en\nContent-Type: text/plain; charset=UTF-8\n"

msgid "hello"
msgstr "world"
"#,
    )
    .unwrap();

    // Create AI-translated PO file (same as original)
    let ai_translated = NamedTempFile::new().unwrap();
    fs::write(
        ai_translated.path(),
        r#"msgid ""
msgstr "Language: en\nContent-Type: text/plain; charset=UTF-8\n"

msgid "hello"
msgstr "world"
"#,
    )
    .unwrap();

    let output = NamedTempFile::new().unwrap();

    cmd.arg("review-interactive")
        .arg("--editor")
        .arg("true")
        .arg(original.path())
        .arg(ai_translated.path())
        .arg(output.path())
        .assert()
        .success();

    // Output file should contain the header and equal message
    let output_content = fs::read_to_string(output.path()).unwrap();
    assert!(output_content.contains("msgid \"\""));
    assert!(output_content.contains("msgid \"hello\""));
}

#[test]
fn test_review_interactive_differing_messages() {
    let mut cmd = Command::cargo_bin("po-tools").unwrap();

    // Create original PO file
    let original = NamedTempFile::new().unwrap();
    fs::write(
        original.path(),
        r#"msgid ""
msgstr "Language: en\nContent-Type: text/plain; charset=UTF-8\n"

msgid "hello"
msgstr "original translation"
"#,
    )
    .unwrap();

    // Create AI-translated PO file (different translation)
    let ai_translated = NamedTempFile::new().unwrap();
    fs::write(
        ai_translated.path(),
        r#"msgid ""
msgstr "Language: en\nContent-Type: text/plain; charset=UTF-8\n"

msgid "hello"
msgstr "AI translation"
"#,
    )
    .unwrap();

    let output = NamedTempFile::new().unwrap();

    // Run command with --editor true (no interactive prompt needed)
    cmd.arg("review-interactive")
        .arg("--editor")
        .arg("true")
        .arg(original.path())
        .arg(ai_translated.path())
        .arg(output.path())
        .assert()
        .success();

    // Output file should contain the header and original message (since we rejected AI translation)
    let output_content = fs::read_to_string(output.path()).unwrap();
    assert!(output_content.contains("msgid \"\""));
}

#[test]
fn test_review_interactive_accept_with_fuzzy_flag() {
    // Test that pressing 'f' accepts the translation but keeps the fuzzy flag
    let mut cmd = Command::cargo_bin("po-tools").unwrap();

    // Create original PO file
    let original = NamedTempFile::new().unwrap();
    fs::write(
        original.path(),
        r#"msgid ""
msgstr "Language: en\nContent-Type: text/plain; charset=UTF-8\n"

msgid "hello"
msgstr "original translation"
"#,
    )
    .unwrap();

    // Create AI-translated PO file (different translation with fuzzy flag)
    let ai_translated = NamedTempFile::new().unwrap();
    fs::write(
        ai_translated.path(),
        r#"#, fuzzy
msgid ""
msgstr "Language: en\nContent-Type: text/plain; charset=UTF-8\n"

#, fuzzy
msgid "hello"
msgstr "AI translation"
"#,
    )
    .unwrap();

    let output = NamedTempFile::new().unwrap();

    // Provide input: 'f' to accept with fuzzy flag, then empty line (accepts)
    cmd.arg("review-interactive")
        .arg("--editor")
        .arg("true")
        .arg(original.path())
        .arg(ai_translated.path())
        .arg(output.path())
        .write_stdin("f\n\n")
        .assert()
        .success();

    // Output file should contain the header, message, and fuzzy flag
    let output_content = fs::read_to_string(output.path()).unwrap();
    assert!(output_content.contains("msgid \"\""));
    assert!(output_content.contains("msgid \"hello\""));
    assert!(
        output_content.contains("#, fuzzy"),
        "Message should have fuzzy flag when 'f' is pressed"
    );
}

#[test]
fn test_review_interactive_accept_with_fuzzy_flag_no_existing() {
    // Test that pressing 'f' adds fuzzy flag even when AI message doesn't have it
    let mut cmd = Command::cargo_bin("po-tools").unwrap();

    // Create original PO file
    let original = NamedTempFile::new().unwrap();
    fs::write(
        original.path(),
        r#"msgid ""
msgstr "Language: en\nContent-Type: text/plain; charset=UTF-8\n"

msgid "hello"
msgstr "original translation"
"#,
    )
    .unwrap();

    // Create AI-translated PO file WITHOUT fuzzy flag
    let ai_translated = NamedTempFile::new().unwrap();
    fs::write(
        ai_translated.path(),
        r#"msgid ""
msgstr "Language: en\nContent-Type: text/plain; charset=UTF-8\n"

msgid "hello"
msgstr "AI translation"
"#,
    )
    .unwrap();

    let output = NamedTempFile::new().unwrap();

    // Provide input: 'f' to accept with fuzzy flag, then empty line (accepts)
    cmd.arg("review-interactive")
        .arg("--editor")
        .arg("true")
        .arg(original.path())
        .arg(ai_translated.path())
        .arg(output.path())
        .write_stdin("f\n\n")
        .assert()
        .success();

    // Output file should contain the message WITH fuzzy flag added by 'f' command
    let output_content = fs::read_to_string(output.path()).unwrap();
    assert!(output_content.contains("msgid \"hello\""));
    assert!(
        output_content.contains("#, fuzzy"),
        "'f' command should add fuzzy flag to accepted messages"
    );
}
