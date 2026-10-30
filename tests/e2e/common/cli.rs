//! Subprocess execution helpers for opaque-box CLI invocations.

use std::path::Path;
use std::process::{Command, Output};

/// Result of executing a subprocess.
#[derive(Debug)]
pub struct CliResult {
    /// Process exit code (0 for success).
    pub exit_code: i32,
    /// Standard output text.
    pub stdout: String,
    /// Standard error text.
    pub stderr: String,
}

impl CliResult {
    /// True if command exited with code 0.
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }
}

/// Execute a command in `working_dir` and capture output.
pub fn run_command(working_dir: &Path, program: &str, args: &[&str]) -> std::io::Result<CliResult> {
    let output: Output = Command::new(program)
        .current_dir(working_dir)
        .args(args)
        .output()?;

    let exit_code = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    Ok(CliResult {
        exit_code,
        stdout,
        stderr,
    })
}
