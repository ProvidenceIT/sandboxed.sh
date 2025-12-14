//! Terminal/shell command execution tool.

use std::path::Path;
use std::process::Stdio;

use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::process::Command;

use super::Tool;

/// Run a shell command.
pub struct RunCommand;

#[async_trait]
impl Tool for RunCommand {
    fn name(&self) -> &str {
        "run_command"
    }

    fn description(&self) -> &str {
        "Execute a shell command in the workspace directory. Returns stdout and stderr. Use for running tests, installing dependencies, compiling code, etc."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "timeout_secs": {
                    "type": "integer",
                    "description": "Timeout in seconds (default: 60)"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, args: Value, workspace: &Path) -> anyhow::Result<String> {
        let command = args["command"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'command' argument"))?;
        let timeout_secs = args["timeout_secs"].as_u64().unwrap_or(60);

        tracing::info!("Executing command: {}", command);

        // Determine shell based on OS
        let (shell, shell_arg) = if cfg!(target_os = "windows") {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };

        let output = tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            Command::new(shell)
                .arg(shell_arg)
                .arg(command)
                .current_dir(workspace)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output(),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Command timed out after {} seconds", timeout_secs))?
        .map_err(|e| anyhow::anyhow!("Failed to execute command: {}", e))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        let exit_code = output.status.code().unwrap_or(-1);

        let mut result = String::new();

        result.push_str(&format!("Exit code: {}\n", exit_code));

        if !stdout.is_empty() {
            result.push_str("\n--- stdout ---\n");
            result.push_str(&stdout);
        }

        if !stderr.is_empty() {
            result.push_str("\n--- stderr ---\n");
            result.push_str(&stderr);
        }

        // Truncate if too long
        if result.len() > 10000 {
            result.truncate(10000);
            result.push_str("\n... [output truncated]");
        }

        Ok(result)
    }
}

