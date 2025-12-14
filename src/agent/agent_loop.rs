//! Core agent loop implementation.

use std::path::Path;
use std::sync::Arc;

use crate::api::types::{LogEntryType, TaskLogEntry};
use crate::config::Config;
use crate::llm::{ChatMessage, LlmClient, OpenRouterClient, Role, ToolCall};
use crate::tools::ToolRegistry;

use super::prompt::build_system_prompt;

/// The autonomous agent.
pub struct Agent {
    config: Config,
    llm: Arc<dyn LlmClient>,
    tools: ToolRegistry,
}

impl Agent {
    /// Create a new agent with the given configuration.
    pub fn new(config: Config) -> Self {
        let llm = Arc::new(OpenRouterClient::new(config.api_key.clone()));
        let tools = ToolRegistry::new();

        Self { config, llm, tools }
    }

    /// Run a task and return the final response and execution log.
    pub async fn run_task(
        &self,
        task: &str,
        model: &str,
        workspace_path: &Path,
    ) -> anyhow::Result<(String, Vec<TaskLogEntry>)> {
        let mut log = Vec::new();
        let workspace_str = workspace_path.to_string_lossy().to_string();

        // Build initial messages
        let system_prompt = build_system_prompt(&workspace_str, &self.tools);
        let mut messages = vec![
            ChatMessage {
                role: Role::System,
                content: Some(system_prompt),
                tool_calls: None,
                tool_call_id: None,
            },
            ChatMessage {
                role: Role::User,
                content: Some(task.to_string()),
                tool_calls: None,
                tool_call_id: None,
            },
        ];

        // Get tool schemas for LLM
        let tool_schemas = self.tools.get_tool_schemas();

        // Agent loop
        for iteration in 0..self.config.max_iterations {
            tracing::debug!("Agent iteration {}", iteration + 1);

            // Call LLM
            let response = self
                .llm
                .chat_completion(model, &messages, Some(&tool_schemas))
                .await?;

            // Check for tool calls
            if let Some(tool_calls) = &response.tool_calls {
                if !tool_calls.is_empty() {
                    // Add assistant message with tool calls
                    messages.push(ChatMessage {
                        role: Role::Assistant,
                        content: response.content.clone(),
                        tool_calls: Some(tool_calls.clone()),
                        tool_call_id: None,
                    });

                    // Execute each tool call
                    for tool_call in tool_calls {
                        log.push(TaskLogEntry {
                            timestamp: chrono_now(),
                            entry_type: LogEntryType::ToolCall,
                            content: format!(
                                "Calling tool: {} with args: {}",
                                tool_call.function.name, tool_call.function.arguments
                            ),
                        });

                        let result = self
                            .execute_tool_call(tool_call, workspace_path)
                            .await;

                        let result_str = match &result {
                            Ok(output) => output.clone(),
                            Err(e) => format!("Error: {}", e),
                        };

                        log.push(TaskLogEntry {
                            timestamp: chrono_now(),
                            entry_type: LogEntryType::ToolResult,
                            content: truncate_for_log(&result_str, 1000),
                        });

                        // Add tool result message
                        messages.push(ChatMessage {
                            role: Role::Tool,
                            content: Some(result_str),
                            tool_calls: None,
                            tool_call_id: Some(tool_call.id.clone()),
                        });
                    }

                    continue;
                }
            }

            // No tool calls - this is the final response
            if let Some(content) = response.content {
                log.push(TaskLogEntry {
                    timestamp: chrono_now(),
                    entry_type: LogEntryType::Response,
                    content: truncate_for_log(&content, 2000),
                });
                return Ok((content, log));
            }

            // Empty response - shouldn't happen but handle gracefully
            return Err(anyhow::anyhow!("LLM returned empty response"));
        }

        Err(anyhow::anyhow!(
            "Max iterations ({}) reached without completion",
            self.config.max_iterations
        ))
    }

    /// Execute a single tool call.
    async fn execute_tool_call(
        &self,
        tool_call: &ToolCall,
        workspace_path: &Path,
    ) -> anyhow::Result<String> {
        let args: serde_json::Value = serde_json::from_str(&tool_call.function.arguments)
            .unwrap_or(serde_json::Value::Null);

        self.tools
            .execute(&tool_call.function.name, args, workspace_path)
            .await
    }
}

/// Get current timestamp as ISO 8601 string.
fn chrono_now() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    format!("{}", now)
}

/// Truncate a string for logging purposes.
fn truncate_for_log(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}... [truncated]", &s[..max_len])
    }
}

