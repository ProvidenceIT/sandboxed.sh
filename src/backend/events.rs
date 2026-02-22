use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Terminal state of a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionTerminalState {
    /// Session completed successfully
    Completed,
    /// Session ended but may be retryable (e.g., rate limited)
    IncompleteRetryable,
    /// Session ended with terminal failure (cannot retry)
    IncompleteTerminal,
    /// Session is idle but still alive (heartbeat present)
    IdleWithHeartbeat,
    /// Session is idle without heartbeat (may be stuck)
    IdleWithoutHeartbeat,
}

/// Backend-agnostic execution events.
#[derive(Debug, Clone)]
pub enum ExecutionEvent {
    /// Agent is thinking/reasoning.
    Thinking { content: String },
    /// Agent is calling a tool.
    ToolCall {
        id: String,
        name: String,
        args: Value,
    },
    /// Tool execution completed.
    ToolResult {
        id: String,
        name: String,
        result: Value,
    },
    /// Text content being streamed.
    TextDelta { content: String },
    /// Optional turn summary (backend-specific).
    TurnSummary { content: String },
    /// Token usage report from the backend (e.g. Codex turn.completed).
    Usage {
        input_tokens: u64,
        output_tokens: u64,
    },
    /// Message execution completed.
    MessageComplete { session_id: String },
    /// Session completed with terminal state.
    SessionComplete {
        session_id: String,
        state: SessionTerminalState,
    },
    /// Error occurred.
    Error { message: String },
}
