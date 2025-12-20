//! Model compatibility tracking.
//!
//! Tracks which models support proper function calling and other features.
//! This helps avoid selecting models that will fail due to format incompatibilities.

use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Format used by the model for function/tool calling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCallFormat {
    /// Standard OpenAI-compatible JSON function calling
    OpenAI,
    /// No function calling support (text only)
    None,
    /// Uses custom format that isn't compatible
    Incompatible,
}

/// Compatibility information for a model.
#[derive(Debug, Clone)]
pub struct ModelCompatibility {
    /// Whether the model supports function calling
    pub supports_function_calling: bool,
    /// The format used for function calling
    pub tool_call_format: ToolCallFormat,
    /// Known issues with this model
    pub known_issues: Vec<String>,
    /// Whether this model has been tested
    pub tested: bool,
}

impl Default for ModelCompatibility {
    fn default() -> Self {
        Self {
            supports_function_calling: true,
            tool_call_format: ToolCallFormat::OpenAI,
            known_issues: Vec::new(),
            tested: false,
        }
    }
}

impl ModelCompatibility {
    /// Create a compatible model entry.
    pub fn compatible() -> Self {
        Self {
            supports_function_calling: true,
            tool_call_format: ToolCallFormat::OpenAI,
            known_issues: Vec::new(),
            tested: true,
        }
    }

    /// Create an incompatible model entry.
    pub fn incompatible(reason: &str) -> Self {
        Self {
            supports_function_calling: false,
            tool_call_format: ToolCallFormat::Incompatible,
            known_issues: vec![reason.to_string()],
            tested: true,
        }
    }

    /// Check if this model can be used for function calling.
    pub fn can_use_functions(&self) -> bool {
        self.supports_function_calling && self.tool_call_format == ToolCallFormat::OpenAI
    }
}

/// Registry of model compatibility information.
///
/// This is populated:
/// 1. Statically with known-bad models
/// 2. Dynamically when models fail with IncompatibleModel errors
pub struct CompatibilityRegistry {
    /// Models known to be incompatible (model ID prefixes)
    incompatible_prefixes: HashSet<String>,
    /// Models that have failed at runtime
    runtime_failures: HashSet<String>,
}

impl CompatibilityRegistry {
    /// Create a new registry with known incompatible models.
    pub fn new() -> Self {
        let mut incompatible = HashSet::new();

        // Known models with broken tool calling formats
        // These use non-standard formats like <｜tool▁calls▁begin｜>
        incompatible.insert("deepseek/deepseek-r1-distill".to_string());

        // Models that produce malformed function names (prose instead of identifiers)
        // Causes HTTP 400: "Function name must be a-z, A-Z, 0-9, underscores, dashes"
        incompatible.insert("mistralai/mistral-large-2512".to_string());

        // Models that output XML-style tool calls
        // (add as discovered)

        Self {
            incompatible_prefixes: incompatible,
            runtime_failures: HashSet::new(),
        }
    }

    /// Check if a model is known to be incompatible.
    pub fn is_incompatible(&self, model_id: &str) -> bool {
        // Check exact runtime failures
        if self.runtime_failures.contains(model_id) {
            return true;
        }

        // Check prefix matches
        self.incompatible_prefixes
            .iter()
            .any(|prefix| model_id.starts_with(prefix))
    }

    /// Mark a model as incompatible at runtime.
    /// Called when a model fails with IncompatibleModel error.
    pub fn mark_incompatible(&mut self, model_id: &str, reason: &str) {
        tracing::warn!("Marking model {} as incompatible: {}", model_id, reason);
        self.runtime_failures.insert(model_id.to_string());
    }

    /// Get compatibility info for a model.
    pub fn get(&self, model_id: &str) -> ModelCompatibility {
        if self.is_incompatible(model_id) {
            ModelCompatibility::incompatible("Known to use non-standard tool calling format")
        } else {
            // Assume compatible until proven otherwise
            ModelCompatibility::default()
        }
    }

    /// Filter a list of models to only compatible ones.
    pub fn filter_compatible<'a>(
        &self,
        models: &'a [super::PricingInfo],
    ) -> Vec<&'a super::PricingInfo> {
        models
            .iter()
            .filter(|m| !self.is_incompatible(&m.model_id))
            .collect()
    }
}

impl Default for CompatibilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared compatibility registry for concurrent access.
pub type SharedCompatibilityRegistry = Arc<RwLock<CompatibilityRegistry>>;

/// Create a new shared compatibility registry.
pub fn create_shared_registry() -> SharedCompatibilityRegistry {
    Arc::new(RwLock::new(CompatibilityRegistry::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_known_incompatible() {
        let registry = CompatibilityRegistry::new();
        assert!(registry.is_incompatible("deepseek/deepseek-r1-distill-llama-70b"));
        assert!(registry.is_incompatible("deepseek/deepseek-r1-distill-qwen-32b"));
        assert!(registry.is_incompatible("mistralai/mistral-large-2512"));
        assert!(!registry.is_incompatible("deepseek/deepseek-v3"));
        assert!(!registry.is_incompatible("openai/gpt-4o"));
    }

    #[test]
    fn test_runtime_marking() {
        let mut registry = CompatibilityRegistry::new();
        assert!(!registry.is_incompatible("some-new/model"));

        registry.mark_incompatible("some-new/model", "Uses weird format");

        assert!(registry.is_incompatible("some-new/model"));
    }

    #[test]
    fn test_compatibility_info() {
        let registry = CompatibilityRegistry::new();

        let compat = registry.get("openai/gpt-4o");
        assert!(compat.can_use_functions());

        let incompat = registry.get("deepseek/deepseek-r1-distill-llama-70b");
        assert!(!incompat.can_use_functions());
    }
}
