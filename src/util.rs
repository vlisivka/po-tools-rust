use anyhow::{bail, Result};

pub fn pipe_to_command(command: &str, args: &[&str], text: &str) -> Result<String> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;

    let mut stdin = child.stdin.take().unwrap();
    let text = text.to_string();
    std::thread::spawn(move || {
        stdin
            .write_all(text.as_bytes())
            .expect("Cannot write to stdin");
    });

    let output = child.wait_with_output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        bail!(
            "Command \"{command}\" failed with non-zero exit code. Command args: {:?}",
            args
        )
    }
}
