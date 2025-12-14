//! # Open Agent
//!
//! A minimal autonomous coding agent with full machine access.
//!
//! This library provides:
//! - An HTTP API for task submission and monitoring
//! - A tool-based agent loop for autonomous code editing
//! - Integration with OpenRouter for LLM access
//!
//! ## Architecture
//!
//! The agent follows the "tools in a loop" pattern:
//! 1. Receive a task via the API
//! 2. Build context with system prompt and available tools
//! 3. Call LLM, parse response, execute any tool calls
//! 4. Feed results back to LLM, repeat until task complete
//!
//! ## Example
//!
//! ```rust,ignore
//! use open_agent::{config::Config, agent::Agent};
//!
//! let config = Config::from_env()?;
//! let agent = Agent::new(config);
//! let result = agent.run_task("Create a hello world script").await?;
//! ```

pub mod api;
pub mod agent;
pub mod config;
pub mod llm;
pub mod tools;

pub use config::Config;

