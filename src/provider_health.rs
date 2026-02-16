//! Provider health tracking and model chain definitions.
//!
//! Implements per-account cooldown tracking with exponential backoff,
//! model fallback chain definitions, and chain resolution logic.
//!
//! Used by the OpenAI-compatible proxy to route requests through fallback
//! chains, and by credential rotation in backend runners.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

// ─────────────────────────────────────────────────────────────────────────────
// Health Tracking
// ─────────────────────────────────────────────────────────────────────────────

/// Reason an account was placed into cooldown.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CooldownReason {
    /// HTTP 429 rate limit
    RateLimit,
    /// HTTP 529 overloaded
    Overloaded,
    /// Connection timeout or network error
    Timeout,
    /// Server error (5xx other than 529)
    ServerError,
}

impl std::fmt::Display for CooldownReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RateLimit => write!(f, "rate_limit"),
            Self::Overloaded => write!(f, "overloaded"),
            Self::Timeout => write!(f, "timeout"),
            Self::ServerError => write!(f, "server_error"),
        }
    }
}

/// Health state for a single provider account.
#[derive(Debug, Clone)]
pub struct AccountHealth {
    /// When the cooldown expires (None = healthy).
    pub cooldown_until: Option<std::time::Instant>,
    /// Number of consecutive failures (for exponential backoff).
    pub consecutive_failures: u32,
    /// Last failure reason.
    pub last_failure_reason: Option<CooldownReason>,
    /// Last failure timestamp (wall clock, for API responses).
    pub last_failure_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Total requests routed to this account.
    pub total_requests: u64,
    /// Total successful requests.
    pub total_successes: u64,
    /// Total rate-limited requests.
    pub total_rate_limits: u64,
    /// Total errors (non-rate-limit).
    pub total_errors: u64,
}

impl Default for AccountHealth {
    fn default() -> Self {
        Self {
            cooldown_until: None,
            consecutive_failures: 0,
            last_failure_reason: None,
            last_failure_at: None,
            total_requests: 0,
            total_successes: 0,
            total_rate_limits: 0,
            total_errors: 0,
        }
    }
}

impl AccountHealth {
    /// Whether this account is currently in cooldown.
    pub fn is_in_cooldown(&self) -> bool {
        self.cooldown_until
            .map(|until| std::time::Instant::now() < until)
            .unwrap_or(false)
    }

    /// Remaining cooldown duration, if any.
    pub fn remaining_cooldown(&self) -> Option<std::time::Duration> {
        self.cooldown_until.and_then(|until| {
            let now = std::time::Instant::now();
            if now < until {
                Some(until - now)
            } else {
                None
            }
        })
    }
}

/// Backoff configuration for a provider type.
#[derive(Debug, Clone)]
pub struct BackoffConfig {
    /// Base delay for first failure.
    pub base_delay: std::time::Duration,
    /// Maximum backoff cap.
    pub max_delay: std::time::Duration,
    /// Multiplier per consecutive failure (typically 2.0).
    pub multiplier: f64,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            base_delay: std::time::Duration::from_secs(5),
            max_delay: std::time::Duration::from_secs(300), // 5 minutes
            multiplier: 2.0,
        }
    }
}

impl BackoffConfig {
    /// Calculate the cooldown duration for a given number of consecutive failures.
    pub fn cooldown_for(&self, consecutive_failures: u32) -> std::time::Duration {
        let delay_secs =
            self.base_delay.as_secs_f64() * self.multiplier.powi(consecutive_failures as i32);
        let capped = delay_secs.min(self.max_delay.as_secs_f64());
        std::time::Duration::from_secs_f64(capped)
    }
}

/// Global health tracker for all provider accounts.
///
/// Thread-safe, shared across the proxy endpoint and all backend runners.
/// Keyed by account UUID so the same tracker works for AIProviderStore accounts
/// and for non-store accounts identified by synthetic UUIDs.
#[derive(Debug, Clone)]
pub struct ProviderHealthTracker {
    accounts: Arc<RwLock<HashMap<Uuid, AccountHealth>>>,
    backoff_config: BackoffConfig,
}

/// Serializable snapshot of account health for API responses.
#[derive(Debug, Clone, Serialize)]
pub struct AccountHealthSnapshot {
    pub account_id: Uuid,
    pub is_healthy: bool,
    pub cooldown_remaining_secs: Option<f64>,
    pub consecutive_failures: u32,
    pub last_failure_reason: Option<String>,
    pub last_failure_at: Option<chrono::DateTime<chrono::Utc>>,
    pub total_requests: u64,
    pub total_successes: u64,
    pub total_rate_limits: u64,
    pub total_errors: u64,
}

impl ProviderHealthTracker {
    pub fn new() -> Self {
        Self {
            accounts: Arc::new(RwLock::new(HashMap::new())),
            backoff_config: BackoffConfig::default(),
        }
    }

    pub fn with_backoff(backoff_config: BackoffConfig) -> Self {
        Self {
            accounts: Arc::new(RwLock::new(HashMap::new())),
            backoff_config,
        }
    }

    /// Check whether an account is currently healthy (not in cooldown).
    pub async fn is_healthy(&self, account_id: Uuid) -> bool {
        let accounts = self.accounts.read().await;
        accounts
            .get(&account_id)
            .map(|h| !h.is_in_cooldown())
            .unwrap_or(true) // Unknown accounts are healthy by default
    }

    /// Record a successful request for an account.
    pub async fn record_success(&self, account_id: Uuid) {
        let mut accounts = self.accounts.write().await;
        let health = accounts.entry(account_id).or_default();
        health.total_requests += 1;
        health.total_successes += 1;
        // Reset consecutive failures on success
        health.consecutive_failures = 0;
        health.cooldown_until = None;
    }

    /// Record a failure and place the account into cooldown.
    ///
    /// If `retry_after` is provided (from response headers), use that as the
    /// cooldown duration instead of exponential backoff.
    pub async fn record_failure(
        &self,
        account_id: Uuid,
        reason: CooldownReason,
        retry_after: Option<std::time::Duration>,
    ) {
        let mut accounts = self.accounts.write().await;
        let health = accounts.entry(account_id).or_default();

        health.total_requests += 1;
        match &reason {
            CooldownReason::RateLimit => health.total_rate_limits += 1,
            _ => health.total_errors += 1,
        }

        health.consecutive_failures += 1;
        health.last_failure_reason = Some(reason);
        health.last_failure_at = Some(chrono::Utc::now());

        // Use retry_after from headers if available, else exponential backoff
        let cooldown = retry_after.unwrap_or_else(|| {
            self.backoff_config
                .cooldown_for(health.consecutive_failures.saturating_sub(1))
        });

        health.cooldown_until = Some(std::time::Instant::now() + cooldown);

        tracing::info!(
            account_id = %account_id,
            consecutive_failures = health.consecutive_failures,
            cooldown_secs = cooldown.as_secs_f64(),
            "Account placed in cooldown"
        );
    }

    /// Get a snapshot of health state for an account (for API responses).
    pub async fn get_health(&self, account_id: Uuid) -> AccountHealthSnapshot {
        let accounts = self.accounts.read().await;
        match accounts.get(&account_id) {
            Some(health) => AccountHealthSnapshot {
                account_id,
                is_healthy: !health.is_in_cooldown(),
                cooldown_remaining_secs: health
                    .remaining_cooldown()
                    .map(|d| d.as_secs_f64()),
                consecutive_failures: health.consecutive_failures,
                last_failure_reason: health.last_failure_reason.as_ref().map(|r| r.to_string()),
                last_failure_at: health.last_failure_at,
                total_requests: health.total_requests,
                total_successes: health.total_successes,
                total_rate_limits: health.total_rate_limits,
                total_errors: health.total_errors,
            },
            None => AccountHealthSnapshot {
                account_id,
                is_healthy: true,
                cooldown_remaining_secs: None,
                consecutive_failures: 0,
                last_failure_reason: None,
                last_failure_at: None,
                total_requests: 0,
                total_successes: 0,
                total_rate_limits: 0,
                total_errors: 0,
            },
        }
    }

    /// Get health snapshots for all tracked accounts.
    pub async fn get_all_health(&self) -> Vec<AccountHealthSnapshot> {
        let accounts = self.accounts.read().await;
        accounts
            .iter()
            .map(|(&id, health)| AccountHealthSnapshot {
                account_id: id,
                is_healthy: !health.is_in_cooldown(),
                cooldown_remaining_secs: health
                    .remaining_cooldown()
                    .map(|d| d.as_secs_f64()),
                consecutive_failures: health.consecutive_failures,
                last_failure_reason: health.last_failure_reason.as_ref().map(|r| r.to_string()),
                last_failure_at: health.last_failure_at,
                total_requests: health.total_requests,
                total_successes: health.total_successes,
                total_rate_limits: health.total_rate_limits,
                total_errors: health.total_errors,
            })
            .collect()
    }

    /// Clear cooldown for an account (e.g., after manual recovery).
    pub async fn clear_cooldown(&self, account_id: Uuid) {
        let mut accounts = self.accounts.write().await;
        if let Some(health) = accounts.get_mut(&account_id) {
            health.cooldown_until = None;
            health.consecutive_failures = 0;
        }
    }
}

/// Shared tracker type.
pub type SharedProviderHealthTracker = Arc<ProviderHealthTracker>;

// ─────────────────────────────────────────────────────────────────────────────
// Model Chain Definitions
// ─────────────────────────────────────────────────────────────────────────────

/// A single entry in a model chain: a provider + model pair.
///
/// When the chain is resolved, each entry is expanded into N entries —
/// one per configured account for that provider, ordered by account priority.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainEntry {
    /// Provider type ID (e.g., "zai", "minimax", "anthropic").
    pub provider_id: String,
    /// Model ID to use with this provider (e.g., "glm-5", "minimax-2.5").
    pub model_id: String,
}

/// A named model chain (fallback sequence).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelChain {
    /// Unique chain ID (e.g., "builtin/smart", "user/fast").
    pub id: String,
    /// Human-readable name.
    pub name: String,
    /// Ordered list of provider+model entries (first = highest priority).
    pub entries: Vec<ChainEntry>,
    /// Whether this is the default chain.
    #[serde(default)]
    pub is_default: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// A standard (non-custom) provider account read from OpenCode's config.
///
/// Standard providers live in `opencode.json` + `auth.json`, not in
/// `AIProviderStore`. This struct lets the chain resolver include them
/// without coupling to OpenCode's config format.
#[derive(Debug, Clone)]
pub struct StandardAccount {
    /// Stable UUID for health tracking (derived from provider type ID).
    pub account_id: Uuid,
    /// Which provider type this account belongs to.
    pub provider_type: crate::ai_providers::ProviderType,
    /// API key from auth.json (None if OAuth-only or unconfigured).
    pub api_key: Option<String>,
    /// Base URL override from opencode.json (if any).
    pub base_url: Option<String>,
}

/// Derive a deterministic UUID from a provider type ID string.
///
/// Uses a simple hash-to-UUID scheme so each standard provider always gets the
/// same UUID across restarts, which lets the health tracker persist state.
pub fn stable_provider_uuid(provider_id: &str) -> Uuid {
    // Use a fixed namespace prefix + provider_id bytes to build a deterministic UUID.
    let mut bytes = [0u8; 16];
    // Simple hash: XOR provider_id bytes into the 16-byte buffer
    for (i, b) in provider_id.bytes().enumerate() {
        bytes[i % 16] ^= b;
    }
    // Set version 4 bits (to be a valid UUID) but deterministic
    bytes[6] = (bytes[6] & 0x0f) | 0x40; // version 4
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // variant 1
    Uuid::from_bytes(bytes)
}

/// A resolved chain entry: a specific account + model ready for routing.
#[derive(Debug, Clone)]
pub struct ResolvedEntry {
    /// The provider type.
    pub provider_id: String,
    /// The model ID.
    pub model_id: String,
    /// The specific account UUID.
    pub account_id: Uuid,
    /// The account's API key (if available).
    pub api_key: Option<String>,
    /// The account's base URL (if custom).
    pub base_url: Option<String>,
}

/// In-memory store for model chains, persisted to disk as JSON.
#[derive(Debug, Clone)]
pub struct ModelChainStore {
    chains: Arc<RwLock<Vec<ModelChain>>>,
    storage_path: PathBuf,
}

impl ModelChainStore {
    pub async fn new(storage_path: PathBuf) -> Self {
        let store = Self {
            chains: Arc::new(RwLock::new(Vec::new())),
            storage_path,
        };

        if let Ok(loaded) = store.load_from_disk() {
            let mut chains = store.chains.write().await;
            *chains = loaded;
        }

        // Ensure default chain exists
        {
            let chains = store.chains.read().await;
            if chains.is_empty() {
                drop(chains);
                store.ensure_default_chain().await;
            }
        }

        store
    }

    /// Ensure the builtin/smart default chain exists.
    async fn ensure_default_chain(&self) {
        let now = chrono::Utc::now();
        let default_chain = ModelChain {
            id: "builtin/smart".to_string(),
            name: "Smart (Default)".to_string(),
            entries: vec![
                ChainEntry {
                    provider_id: "zai".to_string(),
                    model_id: "glm-4-plus".to_string(),
                },
                ChainEntry {
                    provider_id: "minimax".to_string(),
                    model_id: "MiniMax-M1".to_string(),
                },
                ChainEntry {
                    provider_id: "cerebras".to_string(),
                    model_id: "llama-4-scout-17b-16e-instruct".to_string(),
                },
            ],
            is_default: true,
            created_at: now,
            updated_at: now,
        };

        let mut chains = self.chains.write().await;
        chains.push(default_chain);
        drop(chains);

        if let Err(e) = self.save_to_disk().await {
            tracing::error!("Failed to save default model chain: {}", e);
        }
    }

    fn load_from_disk(&self) -> Result<Vec<ModelChain>, std::io::Error> {
        if !self.storage_path.exists() {
            return Ok(Vec::new());
        }
        let contents = std::fs::read_to_string(&self.storage_path)?;
        serde_json::from_str(&contents)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    async fn save_to_disk(&self) -> Result<(), std::io::Error> {
        let chains = self.chains.read().await;
        if let Some(parent) = self.storage_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = serde_json::to_string_pretty(&*chains)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(&self.storage_path, contents)?;
        Ok(())
    }

    /// List all chains.
    pub async fn list(&self) -> Vec<ModelChain> {
        self.chains.read().await.clone()
    }

    /// Get a chain by ID.
    pub async fn get(&self, id: &str) -> Option<ModelChain> {
        self.chains
            .read()
            .await
            .iter()
            .find(|c| c.id == id)
            .cloned()
    }

    /// Get the default chain.
    pub async fn get_default(&self) -> Option<ModelChain> {
        let chains = self.chains.read().await;
        chains
            .iter()
            .find(|c| c.is_default)
            .or_else(|| chains.first())
            .cloned()
    }

    /// Add or update a chain.
    pub async fn upsert(&self, mut chain: ModelChain) {
        chain.updated_at = chrono::Utc::now();
        let mut chains = self.chains.write().await;

        // If setting as default, clear others
        if chain.is_default {
            for c in chains.iter_mut() {
                c.is_default = false;
            }
        }

        if let Some(existing) = chains.iter_mut().find(|c| c.id == chain.id) {
            *existing = chain;
        } else {
            chains.push(chain);
        }
        drop(chains);

        if let Err(e) = self.save_to_disk().await {
            tracing::error!("Failed to save model chains: {}", e);
        }
    }

    /// Delete a chain by ID. Cannot delete the last chain.
    pub async fn delete(&self, id: &str) -> bool {
        let mut chains = self.chains.write().await;
        if chains.len() <= 1 {
            return false;
        }
        let len_before = chains.len();
        chains.retain(|c| c.id != id);
        let deleted = chains.len() < len_before;
        drop(chains);

        if deleted {
            if let Err(e) = self.save_to_disk().await {
                tracing::error!("Failed to save model chains after delete: {}", e);
            }
        }
        deleted
    }

    // ─────────────────────────────────────────────────────────────────────
    // Chain Resolution
    // ─────────────────────────────────────────────────────────────────────

    /// Resolve a chain into an ordered list of (account, model) entries,
    /// expanding each chain entry across all configured accounts for that
    /// provider and filtering out accounts currently in cooldown.
    ///
    /// Accounts come from two sources:
    /// 1. `AIProviderStore` — custom providers and future multi-account standard providers
    /// 2. `standard_accounts` — standard providers from OpenCode's config files
    ///
    /// Returns entries in priority order, ready for waterfall routing.
    pub async fn resolve_chain(
        &self,
        chain_id: &str,
        ai_providers: &crate::ai_providers::AIProviderStore,
        standard_accounts: &[StandardAccount],
        health_tracker: &ProviderHealthTracker,
    ) -> Vec<ResolvedEntry> {
        let chain = match self.get(chain_id).await {
            Some(c) => c,
            None => return Vec::new(),
        };

        let mut resolved = Vec::new();

        for entry in &chain.entries {
            let provider_type = match crate::ai_providers::ProviderType::from_id(&entry.provider_id)
            {
                Some(pt) => pt,
                None => {
                    tracing::warn!(
                        provider_id = %entry.provider_id,
                        "Unknown provider type in chain, skipping"
                    );
                    continue;
                }
            };

            // 1. Check AIProviderStore (custom providers, multi-account)
            let store_accounts = ai_providers.get_all_by_type(provider_type).await;

            for account in &store_accounts {
                if !health_tracker.is_healthy(account.id).await {
                    tracing::debug!(
                        account_id = %account.id,
                        provider = %entry.provider_id,
                        "Skipping account in cooldown"
                    );
                    continue;
                }
                if !account.has_credentials() {
                    continue;
                }
                resolved.push(ResolvedEntry {
                    provider_id: entry.provider_id.clone(),
                    model_id: entry.model_id.clone(),
                    account_id: account.id,
                    api_key: account.api_key.clone(),
                    base_url: account.base_url.clone(),
                });
            }

            // 2. Check standard accounts from OpenCode config (if none found in store)
            // Standard providers only appear here; custom providers only appear above.
            if store_accounts.is_empty() {
                for sa in standard_accounts {
                    if sa.provider_type != provider_type {
                        continue;
                    }
                    if !health_tracker.is_healthy(sa.account_id).await {
                        tracing::debug!(
                            account_id = %sa.account_id,
                            provider = %entry.provider_id,
                            "Skipping standard account in cooldown"
                        );
                        continue;
                    }
                    // Standard accounts must have an API key to be usable
                    if sa.api_key.is_none() {
                        continue;
                    }
                    resolved.push(ResolvedEntry {
                        provider_id: entry.provider_id.clone(),
                        model_id: entry.model_id.clone(),
                        account_id: sa.account_id,
                        api_key: sa.api_key.clone(),
                        base_url: sa.base_url.clone(),
                    });
                }
            }
        }

        resolved
    }
}

/// Shared chain store type.
pub type SharedModelChainStore = Arc<ModelChainStore>;
