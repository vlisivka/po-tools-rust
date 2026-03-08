//! Utility functions for the PO-tools project.
//!
//! This module contains common helper functions like executing external commands
//! with piped input/output.

use anyhow::{Context, Result, bail};

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

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
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

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
