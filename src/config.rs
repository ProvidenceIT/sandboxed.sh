//! Configuration management for Open Agent.
//!
//! Configuration can be set via environment variables:
//! - `OPENROUTER_API_KEY` - Required. Your OpenRouter API key.
//! - `DEFAULT_MODEL` - Optional. The default LLM model to use. Defaults to `openai/gpt-4.1-mini`.
//! - `WORKSPACE_PATH` - Optional. The workspace directory. Defaults to current directory.
//! - `HOST` - Optional. Server host. Defaults to `127.0.0.1`.
//! - `PORT` - Optional. Server port. Defaults to `3000`.
//! - `MAX_ITERATIONS` - Optional. Maximum agent loop iterations. Defaults to `50`.

use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Missing required environment variable: {0}")]
    MissingEnvVar(String),
    
    #[error("Invalid value for {0}: {1}")]
    InvalidValue(String, String),
}

/// Agent configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// OpenRouter API key
    pub api_key: String,
    
    /// Default LLM model identifier (OpenRouter format)
    pub default_model: String,
    
    /// Workspace directory for file operations
    pub workspace_path: PathBuf,
    
    /// Server host
    pub host: String,
    
    /// Server port
    pub port: u16,
    
    /// Maximum iterations for the agent loop
    pub max_iterations: usize,
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// # Errors
    ///
    /// Returns `ConfigError::MissingEnvVar` if `OPENROUTER_API_KEY` is not set.
    pub fn from_env() -> Result<Self, ConfigError> {
        let api_key = std::env::var("OPENROUTER_API_KEY")
            .map_err(|_| ConfigError::MissingEnvVar("OPENROUTER_API_KEY".to_string()))?;
        
        let default_model = std::env::var("DEFAULT_MODEL")
            .unwrap_or_else(|_| "openai/gpt-4.1-mini".to_string());
        
        let workspace_path = std::env::var("WORKSPACE_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        
        let host = std::env::var("HOST")
            .unwrap_or_else(|_| "127.0.0.1".to_string());
        
        let port = std::env::var("PORT")
            .unwrap_or_else(|_| "3000".to_string())
            .parse()
            .map_err(|e| ConfigError::InvalidValue("PORT".to_string(), format!("{}", e)))?;
        
        let max_iterations = std::env::var("MAX_ITERATIONS")
            .unwrap_or_else(|_| "50".to_string())
            .parse()
            .map_err(|e| ConfigError::InvalidValue("MAX_ITERATIONS".to_string(), format!("{}", e)))?;
        
        Ok(Self {
            api_key,
            default_model,
            workspace_path,
            host,
            port,
            max_iterations,
        })
    }
    
    /// Create a config with custom values (useful for testing).
    pub fn new(
        api_key: String,
        default_model: String,
        workspace_path: PathBuf,
    ) -> Self {
        Self {
            api_key,
            default_model,
            workspace_path,
            host: "127.0.0.1".to_string(),
            port: 3000,
            max_iterations: 50,
        }
    }
}

