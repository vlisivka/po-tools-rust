//! Utility functions for the PO-tools project.
//!
//! This module contains common helper functions like executing external commands
//! with piped input/output.

use crate::command_check_symbols::check_symbols;
use crate::parser::PoMessage;
use anyhow::{Context, Result, bail};
use std::io::Write;

/// Context for I/O operations, allowing for testable output and error streams.
pub struct IoContext<'a> {
    pub out: &'a mut dyn Write,
    pub err: &'a mut dyn Write,
}

/// Executes an external command, piping the given text to its stdin and capturing stdout.
///
/// This is used extensively for interacting with AI tools like `aichat`.
pub fn pipe_to_command(command: &str, args: &[&str], text: &str) -> Result<String> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut stdin = child.stdin.take().expect(
        "Failed to open stdin of child process; check if Command was spawned with Stdio::piped()",
    );
    let text = text.to_string();

    let output = std::thread::scope(|s| {
        let handle = s.spawn(move || stdin.write_all(text.as_bytes()));

        let output = child.wait_with_output();

        let write_res = handle
            .join()
            .expect("Stdin writer thread panicked")
            .context(
                tr!("Failed to write to stdin of \"{command}\"").replace("{command}", command),
            );

        let output = output.context(tr!("Failed to wait for child process"));

        match (write_res, output) {
            (Ok(_), Ok(output)) if output.status.success() => Ok(output),
            (write_res, Ok(output)) => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let mut err_msg = tr!("Command \"{command}\" failed").replace("{command}", command);
                if let Err(e) = write_res {
                    err_msg.push_str(&format!(" ({e})"));
                }
                if !output.status.success() {
                    err_msg.push_str(&format!(
                        " {}",
                        tr!("with exit code {code}").replace("{code}", &output.status.to_string())
                    ));
                }
                bail!("{} {:?}\nStderr: {}", err_msg, args, stderr);
            }
            (Err(e), _) => Err(e),
            (_, Err(e)) => Err(e),
        }
    })?;

    let mut result = String::from_utf8_lossy(&output.stdout).into_owned();
    if cfg!(windows) {
        // Normalize Windows CRLF line endings to LF so output matches our expected text format.
        result = result.replace("\r\n", "\n");
    }

    Ok(result)
}
/// Validates a message and returns a string with any found issues.
///
/// This is used by AI-based commands to check if the generated translation
/// is technically sound.
pub fn validate_message(message: &PoMessage) -> String {
    if message.is_header() {
        return "".into();
    }

    if !message.is_plural() {
        if message.msgstr_first().is_empty() {
            return tr!("Message is not translated.").to_string();
        }
    } else {
        for msgstr in &message.msgstr {
            if msgstr.is_empty() {
                return tr!("Message is not translated fully.").to_string();
            }
        }
    }

    match check_symbols(message) {
        None => "".into(),
        Some(errors) => errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(not(windows))]
    fn test_pipe_to_command_cat() -> Result<()> {
        let input = "hello world\nmultiple\nlines";
        let result = pipe_to_command("cat", &[], input)?;
        assert_eq!(result, input);
        Ok(())
    }

    #[test]
    fn test_pipe_to_command_grep() -> Result<()> {
        let input = "hello\nworld\nhello world\n";
        let result = pipe_to_command("grep", &["hello"], input)?;
        assert_eq!(result, "hello\nhello world\n");
        Ok(())
    }

    #[test]
    fn test_pipe_to_command_error() {
        let result = pipe_to_command("non-existent-command-123", &[], "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_pipe_to_command_exit_failure() {
        // false command always returns 1
        let result = pipe_to_command("false", &[], "test");
        assert!(result.is_err());
    }
}
