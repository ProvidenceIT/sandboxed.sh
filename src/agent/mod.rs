//! Agent module - the core autonomous agent logic.
//!
//! The agent follows a "tools in a loop" pattern:
//! 1. Build context with system prompt and user task
//! 2. Call LLM with available tools
//! 3. If LLM requests tool call, execute it and feed result back
//! 4. Repeat until LLM produces final response or max iterations reached

mod agent_loop;
mod prompt;

pub use agent_loop::Agent;
pub use prompt::build_system_prompt;

