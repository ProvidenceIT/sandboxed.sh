//! Leaf agents - specialized agents that do actual work.
//!
//! # Active Leaf Agent
//! - `TaskExecutor`: Executes tasks using tools (main worker)
//!
//! # Removed Agents (superseded by SimpleAgent)
//! - `ComplexityEstimator`: Was unreliable (LLM-based estimation)
//! - `ModelSelector`: Was over-engineered (U-curve optimization)
//! - `Verifier`: Was ineffective (rubber-stamped everything)

mod executor;

pub use executor::{TaskExecutor, ExecutionLoopResult};
