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

/// Backend for calling an AI model.
#[derive(Debug, Clone)]
pub struct AiBackend {
    command: String,
    args: Vec<String>,
    mock_responses: Vec<String>,
}

impl AiBackend {
    /// Create a new backend with a specific command and arguments.
    #[cfg(test)]
    pub fn new(command: String, args: Vec<String>) -> Self {
        Self {
            command,
            args,
            mock_responses: Vec::new(),
        }
    }

    /// Create a backend from a full command line string (e.g., from --ai-command).
    pub fn from_command_line(cmd: &str) -> Self {
        let parts: Vec<String> = shell_words::split(cmd).unwrap_or_else(|e| {
            // Fall back to naive split if shell_words fails
            eprintln!("warning: failed to parse command line: {e}");
            cmd.split_whitespace().map(|s| s.to_string()).collect()
        });
        if parts.is_empty() {
            // Default to aichat if empty, though this shouldn't normally happen if parsed correctly
            return Self::with_aichat_defaults("ollama:translategemma:12b", "translate-po", None);
        }
        Self {
            command: parts[0].clone(),
            args: parts[1..].to_vec(),
            mock_responses: Vec::new(),
        }
    }

    /// Create a backend for aichat with default options.
    pub fn with_aichat_defaults(model: &str, role: &str, rag: Option<&str>) -> Self {
        let mut args = vec![
            "-r".to_string(),
            role.to_string(),
            "-m".to_string(),
            model.to_string(),
        ];
        if let Some(rag_val) = rag {
            args.push("--rag".to_string());
            args.push(rag_val.to_string());
        }
        Self {
            command: "aichat".to_string(),
            args,
            mock_responses: Vec::new(),
        }
    }

    /// Create a mock backend for testing.
    #[cfg(test)]
    pub fn mock(response: &str) -> Self {
        Self {
            command: String::new(),
            args: Vec::new(),
            mock_responses: vec![response.to_string()],
        }
    }

    /// Append an additional response to the queue. When execute() is called,
    /// responses are consumed in FIFO order (last-pushed first-consumed).
    #[cfg(test)]
    pub fn with_alternate(mut self, response: &str) -> Self {
        self.mock_responses.insert(0, response.to_string());
        self
    }

    /// Executes the AI request. Consumes mock responses via clone+mut.
    pub fn execute(&self, prompt: &str) -> Result<String> {
        if !self.mock_responses.is_empty() {
            let mut owned = self.clone();
            return Ok(owned.mock_responses.pop().unwrap());
        }
        let args_ref: Vec<&str> = self.args.iter().map(|s| s.as_str()).collect();
        pipe_to_command(&self.command, &args_ref, prompt)
    }
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

    let mut stdin = child
        .stdin
        .take()
        .context("stdin was not properly configured for piped child process")?;
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
        if message.msgstr_single().is_empty() {
            return tr!("# Error: Message is not translated.\n").to_string();
        }
    } else {
        for msgstr in &message.msgstr {
            if msgstr.is_empty() {
                return tr!("# Error: Message is not translated fully.\n").to_string();
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

    #[test]
    fn test_ai_backend_mock() -> Result<()> {
        let backend = AiBackend::mock("custom response");
        let result = backend.execute("any prompt")?;
        assert_eq!(result, "custom response");
        Ok(())
    }

    #[test]
    fn test_ai_backend_new() {
        let backend = AiBackend::new(
            "aichat".to_string(),
            vec!["-m".to_string(), "gpt-4".to_string()],
        );
        assert_eq!(backend.command, "aichat");
        assert_eq!(backend.args, vec!["-m".to_string(), "gpt-4".to_string()]);
    }

    #[test]
    fn test_ai_backend_from_command_line() {
        let backend = AiBackend::from_command_line("aichat -m gpt-4");
        assert_eq!(backend.command, "aichat");
        assert_eq!(backend.args, vec!["-m".to_string(), "gpt-4".to_string()]);
    }

    #[test]
    fn test_ai_backend_with_aichat_defaults() {
        let backend = AiBackend::with_aichat_defaults("ollama:gemma", "translate", None);
        assert_eq!(backend.command, "aichat");
        assert!(backend.args.contains(&"-r".to_string()));
        assert!(backend.args.contains(&"translate".to_string()));
        assert!(backend.args.contains(&"-m".to_string()));
        assert!(backend.args.contains(&"ollama:gemma".to_string()));
    }

    #[test]
    fn test_ai_backend_with_aichat_defaults_with_rag() {
        let backend = AiBackend::with_aichat_defaults("ollama:gemma", "translate", Some("context"));
        assert_eq!(backend.command, "aichat");
        assert!(backend.args.contains(&"--rag".to_string()));
        assert!(backend.args.contains(&"context".to_string()));
    }

    #[test]
    fn test_ai_backend_from_command_line_with_quoted_args() {
        // Quoted arguments should be parsed as a single argument
        let backend = AiBackend::from_command_line("my-tool --arg 'hello world'");
        assert_eq!(backend.command, "my-tool");
        assert_eq!(
            backend.args,
            vec!["--arg".to_string(), "hello world".to_string()]
        );
    }
}
