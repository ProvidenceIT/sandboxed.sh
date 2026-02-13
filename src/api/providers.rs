//! Provider catalog API.
//!
//! Provides endpoints for listing available providers and their models for UI selection.
//! Only returns providers that are actually configured and authenticated.

use std::collections::HashSet;
use std::sync::Arc;

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use super::routes::AppState;
use crate::ai_providers::ProviderType;

/// A model available from a provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderModel {
    /// Model identifier (e.g., "claude-opus-4-5-20251101")
    pub id: String,
    /// Human-readable name (e.g., "Claude Opus 4.5")
    pub name: String,
    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
}

/// A provider configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    /// Provider identifier (e.g., "anthropic")
    pub id: String,
    /// Human-readable name (e.g., "Claude (Subscription)")
    pub name: String,
    /// Billing type: "subscription" or "pay-per-token"
    pub billing: String,
    /// Description of the provider
    pub description: String,
    /// Available models from this provider
    pub models: Vec<ProviderModel>,
}

/// Query parameters for providers endpoint.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProvidersQuery {
    /// Include providers even if they are not configured/authenticated.
    #[serde(default)]
    pub include_all: bool,
}

/// Response for the providers endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersResponse {
    pub providers: Vec<Provider>,
}

/// Model option for a specific backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendModelOption {
    /// Model value to submit (raw model id or provider/model)
    pub value: String,
    /// UI label
    pub label: String,
    /// Optional description
    #[serde(default)]
    pub description: Option<String>,
    /// Provider ID (for custom providers, shows the sanitized ID used in config)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
}

/// Response for backend model options.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendModelOptionsResponse {
    pub backends: std::collections::HashMap<String, Vec<BackendModelOption>>,
}

/// Query parameters for backend models endpoint.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct BackendModelsQuery {
    /// Include providers even if they are not configured/authenticated.
    #[serde(default)]
    pub include_all: bool,
}

/// Configuration file structure for providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    pub providers: Vec<Provider>,
}

/// Load providers configuration from file.
fn load_providers_config(working_dir: &str) -> ProvidersConfig {
    let config_path = format!("{}/.sandboxed-sh/providers.json", working_dir);

    match std::fs::read_to_string(&config_path) {
        Ok(contents) => match serde_json::from_str(&contents) {
            Ok(config) => config,
            Err(e) => {
                tracing::warn!("Failed to parse providers.json: {}. Using defaults.", e);
                default_providers_config()
            }
        },
        Err(_) => {
            tracing::info!(
                "No providers.json found at {}. Using defaults.",
                config_path
            );
            default_providers_config()
        }
    }
}

fn sanitize_custom_provider_id(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .collect::<String>()
        .to_lowercase()
        .replace('-', "_")
}

/// Default provider configuration.
fn default_providers_config() -> ProvidersConfig {
    ProvidersConfig {
        providers: vec![
            Provider {
                id: "anthropic".to_string(),
                name: "Claude (Subscription)".to_string(),
                billing: "subscription".to_string(),
                description: "Included in Claude Max".to_string(),
                models: vec![
                    ProviderModel {
                        id: "claude-opus-4-6".to_string(),
                        name: "Claude Opus 4.6".to_string(),
                        description: Some(
                            "Most capable, recommended for complex tasks".to_string(),
                        ),
                    },
                    ProviderModel {
                        id: "claude-sonnet-4-6".to_string(),
                        name: "Claude Sonnet 4.6".to_string(),
                        description: Some("Balanced speed and capability".to_string()),
                    },
                    ProviderModel {
                        id: "claude-opus-4-5-20251101".to_string(),
                        name: "Claude Opus 4.5".to_string(),
                        description: Some(
                            "Most capable, recommended for complex tasks".to_string(),
                        ),
                    },
                    ProviderModel {
                        id: "claude-sonnet-5".to_string(),
                        name: "Claude Sonnet 5".to_string(),
                        description: Some("Balanced speed and capability".to_string()),
                    },
                    ProviderModel {
                        id: "claude-sonnet-4-20250514".to_string(),
                        name: "Claude Sonnet 4".to_string(),
                        description: Some("Good balance of speed and capability".to_string()),
                    },
                    ProviderModel {
                        id: "claude-3-5-haiku-20241022".to_string(),
                        name: "Claude Haiku 3.5".to_string(),
                        description: Some("Fastest, most economical".to_string()),
                    },
                ],
            },
            Provider {
                id: "openai".to_string(),
                name: "OpenAI (Subscription)".to_string(),
                billing: "subscription".to_string(),
                description: "ChatGPT Plus/Pro via OAuth".to_string(),
                models: vec![
                    ProviderModel {
                        id: "gpt-5.3-spark".to_string(),
                        name: "GPT-5.3 Spark".to_string(),
                        description: Some("Fast, lightweight GPT-5.3 variant".to_string()),
                    },
                    ProviderModel {
                        id: "gpt-5.3-extra-high".to_string(),
                        name: "GPT-5.3 Extra High".to_string(),
                        description: Some("Highest quality GPT-5.3 tier".to_string()),
                    },
                    ProviderModel {
                        id: "gpt-5.3-codex".to_string(),
                        name: "GPT-5.3 Codex".to_string(),
                        description: Some("Latest Codex model".to_string()),
                    },
                    ProviderModel {
                        id: "gpt-5.2-codex".to_string(),
                        name: "GPT-5.2 Codex".to_string(),
                        description: Some("Optimized for coding workflows".to_string()),
                    },
                    ProviderModel {
                        id: "gpt-5.1-codex".to_string(),
                        name: "GPT-5.1 Codex".to_string(),
                        description: Some("Balanced capability and speed".to_string()),
                    },
                    ProviderModel {
                        id: "gpt-5.1-codex-max".to_string(),
                        name: "GPT-5.1 Codex Max".to_string(),
                        description: Some("Highest reasoning capacity".to_string()),
                    },
                    ProviderModel {
                        id: "gpt-5.1-codex-mini".to_string(),
                        name: "GPT-5.1 Codex Mini".to_string(),
                        description: Some("Fast and economical".to_string()),
                    },
                    ProviderModel {
                        id: "gpt-5.3".to_string(),
                        name: "GPT-5.3".to_string(),
                        description: Some("General-purpose GPT-5.3".to_string()),
                    },
                    ProviderModel {
                        id: "gpt-5.2".to_string(),
                        name: "GPT-5.2".to_string(),
                        description: Some("General-purpose GPT-5.2".to_string()),
                    },
                    ProviderModel {
                        id: "gpt-5.1".to_string(),
                        name: "GPT-5.1".to_string(),
                        description: Some("General-purpose GPT-5.1".to_string()),
                    },
                ],
            },
            Provider {
                id: "google".to_string(),
                name: "Google AI (OAuth)".to_string(),
                billing: "subscription".to_string(),
                description: "Gemini models via Google OAuth".to_string(),
                models: vec![
                    ProviderModel {
                        id: "gemini-2.5-pro-preview-06-05".to_string(),
                        name: "Gemini 2.5 Pro".to_string(),
                        description: Some("Most capable Gemini model".to_string()),
                    },
                    ProviderModel {
                        id: "gemini-2.5-flash-preview-05-20".to_string(),
                        name: "Gemini 2.5 Flash".to_string(),
                        description: Some("Fast and efficient".to_string()),
                    },
                    ProviderModel {
                        id: "gemini-3-flash-preview".to_string(),
                        name: "Gemini 3 Flash Preview".to_string(),
                        description: Some("Latest Gemini 3 preview".to_string()),
                    },
                ],
            },
            Provider {
                id: "xai".to_string(),
                name: "xAI (API Key)".to_string(),
                billing: "pay-per-token".to_string(),
                description: "Grok models via xAI API key".to_string(),
                models: vec![
                    ProviderModel {
                        id: "grok-2".to_string(),
                        name: "Grok 2".to_string(),
                        description: Some("Most capable Grok model".to_string()),
                    },
                    ProviderModel {
                        id: "grok-2-mini".to_string(),
                        name: "Grok 2 Mini".to_string(),
                        description: Some("Faster, lighter Grok model".to_string()),
                    },
                    ProviderModel {
                        id: "grok-2-vision".to_string(),
                        name: "Grok 2 Vision".to_string(),
                        description: Some("Vision-capable Grok model".to_string()),
                    },
                ],
            },
        ],
    }
}

/// Check if a JSON value contains valid auth credentials.
fn has_valid_auth(value: &serde_json::Value) -> bool {
    // Check for OAuth tokens (various field names used by different providers)
    let has_oauth = value.get("refresh").is_some()
        || value.get("refresh_token").is_some()
        || value.get("access").is_some()
        || value.get("access_token").is_some();
    // Check for API key (various field names)
    let has_api_key = value.get("key").is_some()
        || value.get("api_key").is_some()
        || value.get("apiKey").is_some();
    has_oauth || has_api_key
}

/// Get the set of configured provider IDs from OpenCode's auth files.
fn get_configured_provider_ids(working_dir: &std::path::Path) -> HashSet<String> {
    let mut configured = HashSet::new();
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());

    // 1. Read OpenCode auth.json (~/.local/share/opencode/auth.json)
    let auth_path = {
        let data_home = std::env::var("XDG_DATA_HOME").ok();
        let base = if let Some(data_home) = data_home {
            std::path::PathBuf::from(data_home).join("opencode")
        } else {
            std::path::PathBuf::from(&home).join(".local/share/opencode")
        };
        base.join("auth.json")
    };

    tracing::debug!("Checking OpenCode auth file: {:?}", auth_path);
    if let Ok(contents) = std::fs::read_to_string(&auth_path) {
        if let Ok(auth) = serde_json::from_str::<serde_json::Value>(&contents) {
            if let Some(map) = auth.as_object() {
                for (key, value) in map {
                    if has_valid_auth(value) {
                        tracing::debug!("Found valid auth for provider '{}' in auth.json", key);
                        let normalized = if key == "codex" { "openai" } else { key };
                        configured.insert(normalized.to_string());
                    }
                }
            }
        }
    }

    // 2. Check provider-specific auth files (~/.opencode/auth/{provider}.json)
    // This is where OpenAI stores its auth (separate from the main auth.json)
    let provider_auth_dir = std::path::PathBuf::from(&home).join(".opencode/auth");
    tracing::debug!("Checking provider auth dir: {:?}", provider_auth_dir);
    for provider_type in [
        ProviderType::Anthropic,
        ProviderType::OpenAI,
        ProviderType::Google,
        ProviderType::GithubCopilot,
        ProviderType::Xai,
    ] {
        let auth_file = provider_auth_dir.join(format!("{}.json", provider_type.id()));
        if let Ok(contents) = std::fs::read_to_string(&auth_file) {
            tracing::debug!(
                "Found auth file for {}: {:?}",
                provider_type.id(),
                auth_file
            );
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(&contents) {
                if has_valid_auth(&value) {
                    tracing::debug!(
                        "Found valid auth for provider '{}' in {:?}",
                        provider_type.id(),
                        auth_file
                    );
                    configured.insert(provider_type.id().to_string());
                }
            }
        }
    }

    // 3. Check Open Agent provider config (.sandboxed-sh/ai_providers.json)
    let ai_providers_path = working_dir.join(".sandboxed-sh").join("ai_providers.json");
    if let Ok(contents) = std::fs::read_to_string(&ai_providers_path) {
        if let Ok(providers) =
            serde_json::from_str::<Vec<crate::ai_providers::AIProvider>>(&contents)
        {
            for provider in providers {
                if provider.enabled && provider.has_credentials() {
                    configured.insert(provider.provider_type.id().to_string());
                }
            }
        }
    }

    tracing::debug!("Configured providers: {:?}", configured);
    configured
}

/// List available providers and their models.
///
/// Returns a list of providers with their available models, billing type,
/// and descriptions. Only includes providers that are actually configured
/// and authenticated. This endpoint is used by the frontend to render
/// a grouped model selector.
pub async fn list_providers(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ProvidersQuery>,
) -> Json<ProvidersResponse> {
    let working_dir = state.config.working_dir.to_string_lossy().to_string();
    let config = load_providers_config(&working_dir);

    // Get the set of configured provider IDs
    let configured = get_configured_provider_ids(state.config.working_dir.as_path());

    let providers = if query.include_all {
        config.providers
    } else {
        // Filter providers to only include those that are configured
        config
            .providers
            .into_iter()
            .filter(|p| configured.contains(&p.id))
            .collect()
    };

    Json(ProvidersResponse { providers })
}

/// List model options grouped by backend (claudecode, codex, opencode).
///
/// This is used by the frontend to power per-harness model override pickers.
pub async fn list_backend_model_options(
    State(state): State<Arc<AppState>>,
    Query(query): Query<BackendModelsQuery>,
) -> Json<BackendModelOptionsResponse> {
    let working_dir = state.config.working_dir.to_string_lossy().to_string();
    let config = load_providers_config(&working_dir);

    let configured = get_configured_provider_ids(state.config.working_dir.as_path());
    let mut providers = if query.include_all {
        config.providers
    } else {
        config
            .providers
            .into_iter()
            .filter(|p| configured.contains(&p.id))
            .collect()
    };

    // Add custom providers from AIProviderStore (for OpenCode)
    let custom_providers = state.ai_providers.list().await;
    for provider in custom_providers {
        if provider.provider_type != ProviderType::Custom || !provider.enabled {
            continue;
        }
        if !query.include_all && !provider.has_credentials() {
            continue;
        }
        let id = sanitize_custom_provider_id(&provider.name);
        let models = provider
            .custom_models
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|model| ProviderModel {
                id: model.id,
                name: model.name.unwrap_or_else(|| "Custom model".to_string()),
                description: None,
            })
            .collect();
        providers.push(Provider {
            id,
            name: provider.name.clone(),
            billing: "custom".to_string(),
            description: "Custom provider".to_string(),
            models,
        });
    }

    let mut backends: std::collections::HashMap<String, Vec<BackendModelOption>> =
        std::collections::HashMap::new();

    let mut push_options =
        |backend: &str, allowlist: Option<&[&str]>, use_provider_prefix: bool| {
            let mut options = Vec::new();
            for provider in &providers {
                if let Some(allowed) = allowlist {
                    if !allowed.iter().any(|id| *id == provider.id) {
                        continue;
                    }
                }
                // Determine if this is a custom provider (billing type "custom")
                let is_custom = provider.billing == "custom";
                for model in &provider.models {
                    let value = if use_provider_prefix {
                        format!("{}/{}", provider.id, model.id)
                    } else {
                        model.id.clone()
                    };
                    options.push(BackendModelOption {
                        value,
                        label: format!("{} — {}", provider.name, model.name),
                        description: model.description.clone(),
                        // Include provider_id for custom providers to show the resolved ID
                        provider_id: if is_custom { Some(provider.id.clone()) } else { None },
                    });
                }
            }
            backends.insert(backend.to_string(), options);
        };

    push_options("claudecode", Some(&["anthropic"]), false);
    push_options("codex", Some(&["openai"]), false);
    push_options("opencode", None, true);
    backends.entry("amp".to_string()).or_default();

    Json(BackendModelOptionsResponse { backends })
}

/// Validate a model override for a specific backend.
/// Returns Ok(()) if valid, Err with user-friendly error message if invalid.
/// Allows custom/unknown models (escape hatch) but validates known providers.
pub async fn validate_model_override(
    state: &AppState,
    backend: &str,
    model_override: &str,
) -> Result<(), String> {
    // Amp ignores model overrides, so no validation needed
    if backend == "amp" {
        return Ok(());
    }

    let working_dir = state.config.working_dir.to_string_lossy().to_string();
    let config = load_providers_config(&working_dir);

    // Load all providers (including configured and custom)
    let mut providers = config.providers;
    let custom_providers = state.ai_providers.list().await;
    for provider in custom_providers {
        if provider.provider_type != ProviderType::Custom || !provider.enabled {
            continue;
        }
        let id = sanitize_custom_provider_id(&provider.name);
        let models = provider
            .custom_models
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(|model| ProviderModel {
                id: model.id,
                name: model.name.unwrap_or_else(|| "Custom model".to_string()),
                description: None,
            })
            .collect();
        providers.push(Provider {
            id,
            name: provider.name.clone(),
            billing: "custom".to_string(),
            description: "Custom provider".to_string(),
            models,
        });
    }

    match backend {
        "opencode" => {
            // OpenCode expects "provider/model" format
            if let Some((provider_id, model_id)) = model_override.split_once('/') {
                // Check if this is a known provider
                if let Some(provider) = providers.iter().find(|p| p.id == provider_id) {
                    // Known provider - validate model exists
                    if !provider.models.iter().any(|m| m.id == model_id) {
                        return Err(format!(
                            "Model '{}' not found for provider '{}'. Available models: {}",
                            model_id,
                            provider_id,
                            provider
                                .models
                                .iter()
                                .map(|m| &m.id)
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }
                }
                // Unknown provider - allow as custom (escape hatch)
                Ok(())
            } else {
                Err(format!(
                    "Invalid format for OpenCode model override. Expected 'provider/model' (e.g., 'openai/gpt-4'), got '{}'",
                    model_override
                ))
            }
        }
        "claudecode" => {
            // Claude Code expects raw model IDs from Anthropic
            let anthropic = providers.iter().find(|p| p.id == "anthropic");
            if let Some(provider) = anthropic {
                if !provider.models.iter().any(|m| m.id == model_override) {
                    // Check if it looks like a Claude model (starts with "claude-")
                    if model_override.starts_with("claude-") {
                        // Allow unknown Claude models (escape hatch for new models)
                        Ok(())
                    } else {
                        return Err(format!(
                            "Model '{}' not found in Anthropic catalog. Available models: {}. For custom Claude models, use format 'claude-*'",
                            model_override,
                            provider
                                .models
                                .iter()
                                .map(|m| &m.id)
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }
                } else {
                    Ok(())
                }
            } else {
                // Anthropic not configured, but allow if it looks like a Claude model
                if model_override.starts_with("claude-") {
                    Ok(())
                } else {
                    Err(format!(
                        "Anthropic provider not configured. Expected a Claude model ID (e.g., 'claude-opus-4-6'), got '{}'",
                        model_override
                    ))
                }
            }
        }
        "codex" => {
            // Codex expects raw model IDs from OpenAI
            let openai = providers.iter().find(|p| p.id == "openai");
            if let Some(provider) = openai {
                if !provider.models.iter().any(|m| m.id == model_override) {
                    // Check if it looks like an OpenAI model (common prefixes)
                    if model_override.starts_with("gpt-") || model_override.starts_with("o1-") {
                        // Allow unknown OpenAI models (escape hatch for new models)
                        Ok(())
                    } else {
                        return Err(format!(
                            "Model '{}' not found in OpenAI catalog. Available models: {}. For custom OpenAI models, use format 'gpt-*' or 'o1-*'",
                            model_override,
                            provider
                                .models
                                .iter()
                                .map(|m| &m.id)
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }
                } else {
                    Ok(())
                }
            } else {
                // OpenAI not configured, but allow if it looks like an OpenAI model
                if model_override.starts_with("gpt-") || model_override.starts_with("o1-") {
                    Ok(())
                } else {
                    Err(format!(
                        "OpenAI provider not configured. Expected an OpenAI model ID (e.g., 'gpt-4'), got '{}'",
                        model_override
                    ))
                }
            }
        }
        _ => {
            // Unknown backend - skip validation
            Ok(())
        }
    }
}
