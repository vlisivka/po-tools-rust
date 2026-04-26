use predicates::prelude::*;
use std::fs;
use tempfile::NamedTempFile;

#[test]
fn test_help_command() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("po-tools");
    cmd.env("LC_ALL", "C")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("COMMANDS"));
}

#[test]
fn test_merge_command() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("po-tools");
    cmd.env("LC_ALL", "C");

    // Create temp files
    let f1 = NamedTempFile::new().unwrap();
    let f2 = NamedTempFile::new().unwrap();

    fs::write(f1.path(), "msgid \"t1\"\nmsgstr \"v1\"\n").unwrap();
    fs::write(f2.path(), "msgid \"t1\"\nmsgstr \"v2\"\n").unwrap();

    let output = cmd
        .arg("merge")
        .arg(f1.path())
        .arg(f2.path())
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("msgid \"t1\""));
    assert!(stdout.contains("msgstr \"v2\""));
}

#[test]
fn test_sort_command() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("po-tools");
    cmd.env("LC_ALL", "C");

    let f = NamedTempFile::new().unwrap();
    fs::write(
        f.path(),
        "msgid \"b\"\nmsgstr \"\"\n\nmsgid \"a\"\nmsgstr \"\"\n",
    )
    .unwrap();

    let output = cmd.arg("sort").arg(f.path()).assert().success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    let id_a_pos = stdout.find("msgid \"a\"").unwrap();
    let id_b_pos = stdout.find("msgid \"b\"").unwrap();
    assert!(id_a_pos < id_b_pos);
}

#[test]
fn test_parse_file_with_utf8_bom() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("po-tools");
    cmd.env("LC_ALL", "C");

    let output = cmd
        .arg("parse")
        .arg("test-data/test_bom.po")
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("The cat is on the mat"));
}

#[test]
fn test_parse_file_with_utf8_bom_via_sort_command() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("po-tools");
    cmd.env("LC_ALL", "C");

    let output = cmd
        .arg("sort")
        .arg("test-data/test_bom.po")
        .assert()
        .success();

    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();
    assert!(stdout.contains("The cat is on the mat"));
}
