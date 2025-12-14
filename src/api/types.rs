//! API request and response types.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Request to submit a new task.
#[derive(Debug, Clone, Deserialize)]
pub struct CreateTaskRequest {
    /// The task description / user prompt
    pub task: String,
    
    /// Optional model override (uses default if not specified)
    pub model: Option<String>,
    
    /// Optional workspace path override
    pub workspace_path: Option<String>,
}

/// Response after creating a task.
#[derive(Debug, Clone, Serialize)]
pub struct CreateTaskResponse {
    /// Unique task identifier
    pub id: Uuid,
    
    /// Current task status
    pub status: TaskStatus,
}

/// Task status enumeration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    /// Task is queued, waiting to start
    Pending,
    /// Task is currently running
    Running,
    /// Task completed successfully
    Completed,
    /// Task failed with an error
    Failed,
    /// Task was cancelled
    Cancelled,
}

/// Full task state including results.
#[derive(Debug, Clone, Serialize)]
pub struct TaskState {
    /// Unique task identifier
    pub id: Uuid,
    
    /// Current status
    pub status: TaskStatus,
    
    /// Original task description
    pub task: String,
    
    /// Model used for this task
    pub model: String,
    
    /// Number of iterations completed
    pub iterations: usize,
    
    /// Final result or error message
    pub result: Option<String>,
    
    /// Detailed execution log
    pub log: Vec<TaskLogEntry>,
}

/// A single entry in the task execution log.
#[derive(Debug, Clone, Serialize)]
pub struct TaskLogEntry {
    /// Timestamp (ISO 8601)
    pub timestamp: String,
    
    /// Entry type
    pub entry_type: LogEntryType,
    
    /// Content of the entry
    pub content: String,
}

/// Types of log entries.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LogEntryType {
    /// Agent is thinking / planning
    Thinking,
    /// Tool is being called
    ToolCall,
    /// Tool returned a result
    ToolResult,
    /// Agent produced final response
    Response,
    /// An error occurred
    Error,
}

/// Server-Sent Event for streaming task progress.
#[derive(Debug, Clone, Serialize)]
pub struct TaskEvent {
    /// Event type
    pub event: String,
    
    /// Event data (JSON serialized)
    pub data: serde_json::Value,
}

/// Health check response.
#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    /// Service status
    pub status: String,
    
    /// Service version
    pub version: String,
}

