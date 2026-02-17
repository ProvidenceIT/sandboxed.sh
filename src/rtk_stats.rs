//! RTK (CLI output compressor) statistics tracking.
//!
//! Tracks token savings from RTK compression of CLI command output.
//! RTK reduces token consumption by 60-90% on common dev commands.

use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RtkStats {
    pub commands_processed: u64,
    pub original_chars: u64,
    pub compressed_chars: u64,
    pub chars_saved: u64,
    pub savings_percent: f64,
}

impl RtkStats {
    pub fn token_savings(&self) -> u64 {
        self.original_chars.saturating_sub(self.compressed_chars)
    }

    pub fn calculate_savings_percent(&self) -> f64 {
        if self.original_chars == 0 {
            return 0.0;
        }
        let saved = self.token_savings() as f64;
        let original = self.original_chars as f64;
        (saved / original) * 100.0
    }
}

#[derive(Debug, Clone)]
pub struct RtkStatsTracker {
    stats: Arc<RwLock<RtkStats>>,
    enabled: bool,
}

impl Default for RtkStatsTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl RtkStatsTracker {
    pub fn new() -> Self {
        let enabled = std::path::Path::new("/usr/local/bin/rtk").exists();
        Self {
            stats: Arc::new(RwLock::new(RtkStats::default())),
            enabled,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub async fn record_command(&self, original_chars: u64, compressed_chars: u64) {
        if !self.enabled {
            return;
        }
        let mut stats = self.stats.write().await;
        stats.commands_processed += 1;
        stats.original_chars += original_chars;
        stats.compressed_chars += compressed_chars;
        stats.chars_saved = stats.original_chars.saturating_sub(stats.compressed_chars);
        stats.savings_percent = stats.calculate_savings_percent();
        
        tracing::debug!(
            original = original_chars,
            compressed = compressed_chars,
            saved = original_chars.saturating_sub(compressed_chars),
            "RTK compression recorded"
        );
    }

    pub async fn get_stats(&self) -> RtkStats {
        let mut stats = self.stats.read().await.clone();
        stats.chars_saved = stats.original_chars.saturating_sub(stats.compressed_chars);
        stats.savings_percent = stats.calculate_savings_percent();
        stats
    }

    pub async fn reset(&self) {
        let mut stats = self.stats.write().await;
        *stats = RtkStats::default();
    }

    pub async fn parse_rtk_gain_output(&self, output: &str) -> Option<(u64, u64)> {
        for line in output.lines() {
            let line = line.trim();
            if line.contains("tokens saved") || line.contains("->") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                let mut original = 0u64;
                let mut compressed = 0u64;
                
                for (i, part) in parts.iter().enumerate() {
                    if let Ok(num) = part.parse::<u64>() {
                        if i > 0 && parts[i - 1] == "original:" {
                            original = num;
                        } else if i > 0 && parts[i - 1] == "compressed:" {
                            compressed = num;
                        } else if original == 0 {
                            original = num;
                        } else {
                            compressed = num;
                        }
                    }
                }
                
                if original > 0 {
                    return Some((original, compressed));
                }
            }
        }
        None
    }
}

pub fn estimate_token_count(text: &str) -> u64 {
    let char_count = text.chars().count() as u64;
    char_count / 4
}
