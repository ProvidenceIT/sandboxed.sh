//! Git operation tools.

use std::path::Path;
use std::process::Stdio;

use async_trait::async_trait;
use serde_json::{json, Value};
use tokio::process::Command;

use super::Tool;

/// Get git status.
pub struct GitStatus;

#[async_trait]
impl Tool for GitStatus {
    fn name(&self) -> &str {
        "git_status"
    }

    fn description(&self) -> &str {
        "Get the current git status, showing modified, staged, and untracked files."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn execute(&self, _args: Value, workspace: &Path) -> anyhow::Result<String> {
        run_git_command(&["status", "--porcelain=v2", "--branch"], workspace).await
    }
}

/// Get git diff.
pub struct GitDiff;

#[async_trait]
impl Tool for GitDiff {
    fn name(&self) -> &str {
        "git_diff"
    }

    fn description(&self) -> &str {
        "Show git diff of changes. Can diff staged changes, specific files, or commits."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "staged": {
                    "type": "boolean",
                    "description": "Show staged changes instead of unstaged (default: false)"
                },
                "file": {
                    "type": "string",
                    "description": "Optional: show diff for specific file only"
                }
            }
        })
    }

    async fn execute(&self, args: Value, workspace: &Path) -> anyhow::Result<String> {
        let staged = args["staged"].as_bool().unwrap_or(false);
        let file = args["file"].as_str();

        let mut git_args = vec!["diff"];

        if staged {
            git_args.push("--staged");
        }

        if let Some(f) = file {
            git_args.push("--");
            git_args.push(f);
        }

        let result = run_git_command(&git_args, workspace).await?;

        if result.is_empty() {
            Ok("No changes".to_string())
        } else if result.len() > 10000 {
            Ok(format!(
                "{}... [diff truncated, showing first 10000 chars]",
                &result[..10000]
            ))
        } else {
            Ok(result)
        }
    }
}

/// Create a git commit.
pub struct GitCommit;

#[async_trait]
impl Tool for GitCommit {
    fn name(&self) -> &str {
        "git_commit"
    }

    fn description(&self) -> &str {
        "Stage all changes and create a git commit with the given message."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The commit message"
                },
                "files": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Optional: specific files to stage. If not provided, stages all changes."
                }
            },
            "required": ["message"]
        })
    }

    async fn execute(&self, args: Value, workspace: &Path) -> anyhow::Result<String> {
        let message = args["message"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing 'message' argument"))?;

        let files: Vec<&str> = args["files"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
            .unwrap_or_default();

        // Stage files
        if files.is_empty() {
            run_git_command(&["add", "-A"], workspace).await?;
        } else {
            let mut git_args = vec!["add", "--"];
            git_args.extend(files);
            run_git_command(&git_args, workspace).await?;
        }

        // Commit
        run_git_command(&["commit", "-m", message], workspace).await
    }
}

/// Get git log.
pub struct GitLog;

#[async_trait]
impl Tool for GitLog {
    fn name(&self) -> &str {
        "git_log"
    }

    fn description(&self) -> &str {
        "Show recent git commits."
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "num_commits": {
                    "type": "integer",
                    "description": "Number of commits to show (default: 10)"
                },
                "oneline": {
                    "type": "boolean",
                    "description": "Show condensed one-line format (default: true)"
                }
            }
        })
    }

    async fn execute(&self, args: Value, workspace: &Path) -> anyhow::Result<String> {
        let num_commits = args["num_commits"].as_u64().unwrap_or(10);
        let oneline = args["oneline"].as_bool().unwrap_or(true);

        let mut git_args = vec!["log", "-n"];
        let num_str = num_commits.to_string();
        git_args.push(&num_str);

        if oneline {
            git_args.push("--oneline");
        }

        run_git_command(&git_args, workspace).await
    }
}

/// Run a git command and return its output.
async fn run_git_command(args: &[&str], workspace: &Path) -> anyhow::Result<String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(workspace)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to run git: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !output.status.success() {
        if stderr.is_empty() {
            return Err(anyhow::anyhow!(
                "Git command failed: {}",
                stdout.trim()
            ));
        }
        return Err(anyhow::anyhow!("Git error: {}", stderr.trim()));
    }

    Ok(stdout.to_string())
}

