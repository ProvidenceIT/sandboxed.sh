//! Model selection agent with U-curve cost optimization.
//!
//! # U-Curve Optimization
//! The total expected cost follows a U-shaped curve:
//! - Cheap models: Low per-token cost, but may fail/retry, use more tokens
//! - Expensive models: High per-token cost, but succeed more often
//! - Optimal: Somewhere in the middle, minimizing total expected cost
//!
//! # Cost Model
//! Expected Cost = base_cost * (1 + failure_rate * retry_multiplier) * token_efficiency
//!
//! # Benchmark Integration
//! When benchmark data is available, uses actual benchmark scores (from llm-stats.com)
//! for task-type-specific capability estimation instead of price-based heuristics.
//!
//! # Learning Integration
//! When memory is available, uses historical model statistics (actual success rates,
//! cost ratios) instead of pure heuristics.

use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;

use crate::agents::{
    Agent, AgentContext, AgentId, AgentResult, AgentType, LeafAgent, LeafCapability,
};
use crate::budget::{PricingInfo, TaskType};
use crate::memory::ModelStats;
use crate::task::Task;

/// Agent that selects the optimal model for a task.
/// 
/// # Algorithm
/// 1. Get task complexity and budget constraints
/// 2. Fetch available models and pricing
/// 3. For each model, calculate expected total cost
/// 4. Return model with minimum expected cost within budget
pub struct ModelSelector {
    id: AgentId,
    retry_multiplier: f64,
    inefficiency_scale: f64,
    max_failure_probability: f64,
}

/// Model recommendation from the selector.
#[derive(Debug, Clone)]
pub struct ModelRecommendation {
    /// Recommended model ID
    pub model_id: String,
    
    /// Expected cost in cents
    pub expected_cost_cents: u64,
    
    /// Confidence in this recommendation (0-1)
    pub confidence: f64,
    
    /// Reasoning for the selection
    pub reasoning: String,
    
    /// Alternative models if primary fails
    pub fallbacks: Vec<String>,
    
    /// Whether historical data was used for this selection
    pub used_historical_data: bool,

    /// Whether benchmark data was used for capability estimation
    pub used_benchmark_data: bool,

    /// Inferred task type
    pub task_type: Option<TaskType>,
}

impl ModelSelector {
    /// Create a new model selector.
    pub fn new() -> Self {
        Self {
            id: AgentId::new(),
            retry_multiplier: 1.5,
            inefficiency_scale: 0.5,
            max_failure_probability: 0.9,
        }
    }

    /// Create a selector with calibrated parameters.
    pub fn with_params(retry_multiplier: f64, inefficiency_scale: f64, max_failure_probability: f64) -> Self {
        Self {
            id: AgentId::new(),
            retry_multiplier: retry_multiplier.max(1.0),
            inefficiency_scale: inefficiency_scale.max(0.0),
            max_failure_probability: max_failure_probability.clamp(0.0, 0.99),
        }
    }

    /// Calculate expected cost for a model given task complexity.
    /// 
    /// # Formula
    /// ```text
    /// expected_cost = base_cost * (1 + failure_prob * retry_cost) * inefficiency_factor
    /// ```
    /// 
    /// # Parameters
    /// - `pricing`: Model pricing info
    /// - `complexity`: Task complexity (0-1)
    /// - `estimated_tokens`: Estimated tokens needed
    /// - `capability`: Model capability score (0-1), from benchmarks or price heuristic
    /// 
    /// # Returns
    /// Expected cost in cents
    /// 
    /// # Pure Function
    /// No side effects, deterministic output.
    fn calculate_expected_cost_with_capability(
        &self,
        pricing: &PricingInfo,
        complexity: f64,
        estimated_tokens: u64,
        capability: f64,
        from_benchmarks: bool,
    ) -> ExpectedCost {
        // Failure probability: higher complexity + lower capability = more failures
        // Formula: P(fail) = complexity * (1 - capability)
        let failure_prob = (complexity * (1.0 - capability)).clamp(0.0, self.max_failure_probability);
        
        // Token inefficiency: weaker models need more tokens
        // Formula: inefficiency = 1 + (1 - capability) * 0.5
        let inefficiency = 1.0 + (1.0 - capability) * self.inefficiency_scale;
        
        // Retry cost: if it fails, we pay again (possibly with a better model)
        let retry_multiplier = self.retry_multiplier;
        
        // Base cost for estimated tokens
        let input_tokens = estimated_tokens / 2;
        let output_tokens = estimated_tokens / 2;
        let base_cost = pricing.calculate_cost_cents(input_tokens, output_tokens);
        
        // Adjusted for inefficiency (weak models use more tokens)
        let adjusted_tokens = ((estimated_tokens as f64) * inefficiency) as u64;
        let adjusted_cost = pricing.calculate_cost_cents(adjusted_tokens / 2, adjusted_tokens / 2);
        
        // Expected cost including retry probability
        let expected_cost = (adjusted_cost as f64) * (1.0 + failure_prob * retry_multiplier);
        
        ExpectedCost {
            model_id: pricing.model_id.clone(),
            base_cost_cents: base_cost,
            expected_cost_cents: expected_cost.ceil() as u64,
            failure_probability: failure_prob,
            capability,
            inefficiency,
            from_benchmarks,
        }
    }

    /// Calculate expected cost using price-based capability (fallback).
    fn calculate_expected_cost(
        &self,
        pricing: &PricingInfo,
        complexity: f64,
        estimated_tokens: u64,
    ) -> ExpectedCost {
        let avg_cost = pricing.average_cost_per_token();
        let capability = self.estimate_capability_from_price(avg_cost);
        self.calculate_expected_cost_with_capability(pricing, complexity, estimated_tokens, capability, false)
    }

    /// Estimate model capability from its cost (fallback heuristic).
    /// 
    /// # Heuristic
    /// More expensive models are generally more capable.
    /// Uses log scale to normalize across price ranges.
    /// 
    /// # Returns
    /// Capability score 0-1
    fn estimate_capability_from_price(&self, avg_cost_per_token: f64) -> f64 {
        // Cost tiers (per token):
        // < 0.0001: weak (capability ~0.3)
        // 0.0001-0.001: moderate (capability ~0.6)
        // > 0.001: strong (capability ~0.9)
        
        if avg_cost_per_token < 0.0000001 {
            return 0.3; // Free/very cheap
        }
        
        // Log scale normalization
        let log_cost = avg_cost_per_token.log10();
        // Map from ~-7 (cheap) to ~-3 (expensive) => 0.3 to 0.95
        let normalized = ((log_cost + 7.0) / 4.0).clamp(0.0, 1.0);
        
        0.3 + normalized * 0.65
    }

    /// Get model capability from benchmarks (preferred) or fall back to price heuristic.
    /// 
    /// # Benchmark-Based Capability
    /// Uses actual benchmark scores from llm-stats.com when available.
    /// This provides task-type-specific capability estimation.
    async fn get_capability(
        &self,
        model_id: &str,
        task_type: TaskType,
        avg_cost_per_token: f64,
        ctx: &AgentContext,
    ) -> (f64, bool) {
        // Try to get benchmark-based capability
        if let Some(benchmarks) = &ctx.benchmarks {
            let registry = benchmarks.read().await;
            if let Some(model) = registry.get(model_id) {
                if model.has_benchmarks() {
                    let capability = model.capability(task_type);
                    tracing::info!(
                        "Using benchmark capability for {}: {:.3} (task_type: {:?})",
                        model_id, capability, task_type
                    );
                    return (capability, true); // (capability, from_benchmarks)
                }
            }
        }
        
        // Fall back to price-based heuristic
        let capability = self.estimate_capability_from_price(avg_cost_per_token);
        tracing::debug!(
            "Using price-based capability for {}: {:.3} (avg_cost: {:.10})",
            model_id, capability, avg_cost_per_token
        );
        (capability, false)
    }

    /// Select optimal model from available options.
    /// 
    /// # Algorithm
    /// 1. Calculate expected cost for each model using benchmark capabilities when available
    /// 2. If user requested a specific model, use it as minimum capability floor
    /// 3. Filter models exceeding budget
    /// 4. Select model with minimum expected cost
    /// 5. Include fallbacks in case of failure
    /// 
    /// # Preconditions
    /// - `models` is non-empty
    /// - `budget_cents > 0`
    async fn select_optimal(
        &self,
        models: &[PricingInfo],
        complexity: f64,
        estimated_tokens: u64,
        budget_cents: u64,
        task_type: TaskType,
        historical_stats: Option<&HashMap<String, ModelStats>>,
        requested_model: Option<&str>,
        ctx: &AgentContext,
    ) -> Option<ModelRecommendation> {
        if models.is_empty() {
            return None;
        }

        // Calculate expected cost for all models, using benchmark or historical stats when available
        let mut costs: Vec<ExpectedCost> = Vec::with_capacity(models.len());
        let mut any_from_benchmarks = false;

        for m in models {
            let cost = if let Some(stats) = historical_stats.and_then(|h| h.get(&m.model_id)) {
                // Use historical data if available (highest priority)
                self.calculate_expected_cost_with_history(m, complexity, estimated_tokens, stats)
            } else {
                // Use benchmark data for capability
                let (capability, from_benchmarks) = self.get_capability(
                    &m.model_id,
                    task_type,
                    m.average_cost_per_token(),
                    ctx,
                ).await;
                
                if from_benchmarks {
                    any_from_benchmarks = true;
                }
                
                self.calculate_expected_cost_with_capability(
                    m, complexity, estimated_tokens, capability, from_benchmarks
                )
            };
            costs.push(cost);
        }

        // Sort by expected cost (ascending)
        costs.sort_by(|a, b| {
            a.expected_cost_cents
                .cmp(&b.expected_cost_cents)
        });

        // If user requested a specific model, use it as minimum capability floor
        // Filter out models with lower capability than the requested one
        let min_capability = if let Some(req_model) = requested_model {
            // Find the requested model's capability
            if let Some(req_cost) = costs.iter().find(|c| c.model_id == req_model) {
                tracing::info!(
                    "Using requested model {} as capability floor: {:.3}",
                    req_model,
                    req_cost.capability
                );
                req_cost.capability
            } else {
                // Requested model not found - fall back to looking up its price
                if let Some(req_pricing) = models.iter().find(|m| m.model_id == req_model) {
                    let cap = self.estimate_capability_from_price(req_pricing.average_cost_per_token());
                    tracing::info!(
                        "Requested model {} not in costs list, using price-based capability: {:.3}",
                        req_model,
                        cap
                    );
                    cap
                } else {
                    // Model not found at all, use a reasonable floor (0.7 = mid-tier)
                    tracing::warn!(
                        "Requested model {} not found, using default capability floor 0.7",
                        req_model
                    );
                    0.7
                }
            }
        } else {
            0.0 // No minimum
        };

        // Filter to models meeting minimum capability
        let filtered_costs: Vec<_> = if min_capability > 0.0 {
            costs.iter()
                .filter(|c| c.capability >= min_capability * 0.95) // Allow 5% tolerance
                .cloned()
                .collect()
        } else {
            costs.clone()
        };

        let costs_to_use = if filtered_costs.is_empty() {
            tracing::warn!("No models meet minimum capability {:.2}, using all models", min_capability);
            &costs
        } else {
            &filtered_costs
        };

        // Find cheapest model within budget
        let within_budget: Vec<_> = costs_to_use
            .iter()
            .filter(|c| c.expected_cost_cents <= budget_cents)
            .cloned()
            .collect();

        let selected = within_budget.first().cloned().or_else(|| costs_to_use.first().cloned())?;
        
        // Get fallback models (next best options)
        let fallbacks: Vec<String> = costs
            .iter()
            .filter(|c| c.model_id != selected.model_id)
            .take(2)
            .map(|c| c.model_id.clone())
            .collect();

        let used_history = historical_stats.and_then(|h| h.get(&selected.model_id)).is_some();

        let recommendation = ModelRecommendation {
            model_id: selected.model_id.clone(),
            expected_cost_cents: selected.expected_cost_cents,
            confidence: 1.0 - selected.failure_probability,
            reasoning: format!(
                "Selected {} for {:?} task with expected cost {} cents (capability: {:.2}, failure prob: {:.2}){}{}",
                selected.model_id,
                task_type,
                selected.expected_cost_cents,
                selected.capability,
                selected.failure_probability,
                if used_history { " [historical]" } else { "" },
                if selected.from_benchmarks { " [benchmark]" } else { "" }
            ),
            fallbacks,
            used_historical_data: used_history,
            used_benchmark_data: selected.from_benchmarks,
            task_type: Some(task_type),
        };
        
        tracing::info!(
            "Model selected: {} (task: {:?}, cost: {} cents, benchmark_data: {}, history: {})",
            recommendation.model_id,
            task_type,
            recommendation.expected_cost_cents,
            recommendation.used_benchmark_data,
            recommendation.used_historical_data
        );
        
        Some(recommendation)
    }

    /// Calculate expected cost using actual historical statistics.
    /// 
    /// This uses real success rates and cost ratios from past executions
    /// instead of heuristic estimates.
    fn calculate_expected_cost_with_history(
        &self,
        pricing: &PricingInfo,
        _complexity: f64,
        estimated_tokens: u64,
        stats: &ModelStats,
    ) -> ExpectedCost {
        // Use actual failure rate from history (inverted success rate)
        let failure_prob = (1.0 - stats.success_rate).clamp(0.0, self.max_failure_probability);
        
        // Use actual token ratio from history for inefficiency
        let inefficiency = stats.avg_token_ratio.clamp(0.5, 3.0);
        
        // Base cost for estimated tokens
        let input_tokens = estimated_tokens / 2;
        let output_tokens = estimated_tokens / 2;
        let base_cost = pricing.calculate_cost_cents(input_tokens, output_tokens);
        
        // Adjust for actual inefficiency
        let adjusted_tokens = ((estimated_tokens as f64) * inefficiency) as u64;
        let adjusted_cost = pricing.calculate_cost_cents(adjusted_tokens / 2, adjusted_tokens / 2);
        
        // Apply actual cost ratio (how much more/less than predicted)
        let cost_with_ratio = (adjusted_cost as f64) * stats.avg_cost_ratio.clamp(0.5, 3.0);
        
        // Expected cost including retry probability
        let expected_cost = cost_with_ratio * (1.0 + failure_prob * self.retry_multiplier);
        
        // Capability estimated from success rate rather than price
        let capability = stats.success_rate.clamp(0.3, 0.95);
        
        ExpectedCost {
            model_id: pricing.model_id.clone(),
            base_cost_cents: base_cost,
            expected_cost_cents: expected_cost.ceil() as u64,
            failure_probability: failure_prob,
            capability,
            inefficiency,
            from_benchmarks: false, // Historical data is not benchmark data
        }
    }

    /// Query historical model stats from memory.
    async fn get_historical_model_stats(
        &self,
        complexity: f64,
        ctx: &AgentContext,
    ) -> Option<HashMap<String, ModelStats>> {
        let memory = ctx.memory.as_ref()?;
        
        // Query stats for models at similar complexity levels (+/- 0.2)
        match memory.retriever.get_model_stats(complexity, 0.2).await {
            Ok(stats) if !stats.is_empty() => {
                tracing::debug!(
                    "Found historical stats for {} models at complexity ~{:.2}",
                    stats.len(),
                    complexity
                );
                
                // Convert to HashMap for easy lookup
                Some(stats.into_iter()
                    .map(|s| (s.model_id.clone(), s))
                    .collect())
            }
            Ok(_) => {
                tracing::debug!("No historical stats found for complexity ~{:.2}", complexity);
                None
            }
            Err(e) => {
                tracing::warn!("Failed to fetch model stats: {}", e);
                None
            }
        }
    }
}

/// Intermediate calculation result for a model.
#[derive(Debug, Clone)]
struct ExpectedCost {
    model_id: String,
    #[allow(dead_code)]
    base_cost_cents: u64,
    expected_cost_cents: u64,
    failure_probability: f64,
    capability: f64,
    #[allow(dead_code)]
    inefficiency: f64,
    /// Whether capability was derived from benchmark data
    from_benchmarks: bool,
}

impl Default for ModelSelector {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Agent for ModelSelector {
    fn id(&self) -> &AgentId {
        &self.id
    }

    fn agent_type(&self) -> AgentType {
        AgentType::ModelSelector
    }

    fn description(&self) -> &str {
        "Selects optimal model for task based on complexity and budget (U-curve optimization)"
    }

    /// Select the optimal model for a task.
    /// 
    /// # Expected Input
    /// Task should have complexity data in its context (from ComplexityEstimator).
    /// 
    /// # Returns
    /// AgentResult with ModelRecommendation in the `data` field.
    /// 
    /// # Benchmark Integration
    /// When benchmark data is available, uses actual benchmark scores for
    /// task-type-specific capability estimation.
    /// 
    /// # Learning Integration
    /// When memory is available, queries historical model statistics and uses
    /// actual success rates/cost ratios instead of heuristics.
    async fn execute(&self, task: &mut Task, ctx: &AgentContext) -> AgentResult {
        // Get complexity + estimated tokens from task analysis (populated by ComplexityEstimator).
        let complexity = task
            .analysis()
            .complexity_score
            .unwrap_or(0.5)
            .clamp(0.0, 1.0);
        let estimated_tokens = task.analysis().estimated_total_tokens.unwrap_or(2000_u64);
        
        // Infer task type from description for benchmark-based selection
        let task_type = TaskType::infer_from_description(task.description());
        
        // Get available budget
        let budget_cents = task.budget().remaining_cents();
        
        // Query historical model stats (if memory available)
        let historical_stats = self.get_historical_model_stats(complexity, ctx).await;
        
        // Fetch pricing for tool-supporting models only
        let models = ctx.pricing.models_by_cost_filtered(true).await;
        
        if models.is_empty() {
            // Fall back to configured default model
            let default_model = ctx.config.default_model.clone();
            
            // Record on task analysis
            {
                let a = task.analysis_mut();
                a.selected_model = Some(default_model.clone());
            }
            
            return AgentResult::success(
                "Using configured default model (no other models available)",
                0,
            )
            .with_data(json!({
                "model_id": default_model,
                "expected_cost_cents": 50,
                "confidence": 0.8,
                "reasoning": "Fallback to configured default model",
                "fallbacks": [],
                "used_historical_data": false,
                "used_benchmark_data": false,
                "task_type": format!("{:?}", task_type),
            }));
        }

        // Get user-requested model - if specified, use it directly if available
        let requested_model = task.analysis().requested_model.clone();
        
        // If user explicitly requested a model and it's available, use it directly
        if let Some(ref req_model) = requested_model {
            if models.iter().any(|m| &m.model_id == req_model) {
                tracing::info!(
                    "Using user-requested model directly: {} (not optimizing)",
                    req_model
                );
                
                // Record selection in analysis
                {
                    let a = task.analysis_mut();
                    a.selected_model = Some(req_model.clone());
                    a.estimated_cost_cents = Some(50); // Default estimate
                }

                return AgentResult::success(
                    &format!("Using user-requested model: {}", req_model),
                    1,
                )
                .with_data(json!({
                    "model_id": req_model,
                    "expected_cost_cents": 50,
                    "confidence": 1.0,
                    "reasoning": format!("User explicitly requested model: {}", req_model),
                    "fallbacks": [],
                    "used_historical_data": false,
                    "used_benchmark_data": false,
                    "task_type": format!("{:?}", task_type),
                }));
            }
        }

        match self.select_optimal(
            &models,
            complexity,
            estimated_tokens,
            budget_cents,
            task_type,
            historical_stats.as_ref(),
            requested_model.as_deref(),
            ctx,
        ).await {
            Some(rec) => {
                // Record selection in analysis
                {
                    let a = task.analysis_mut();
                    a.selected_model = Some(rec.model_id.clone());
                    a.estimated_cost_cents = Some(rec.expected_cost_cents);
                }

                AgentResult::success(
                    &rec.reasoning,
                    1, // Minimal cost for selection itself
                )
                .with_data(json!({
                    "model_id": rec.model_id,
                    "expected_cost_cents": rec.expected_cost_cents,
                    "confidence": rec.confidence,
                    "reasoning": rec.reasoning,
                    "fallbacks": rec.fallbacks,
                    "used_historical_data": rec.used_historical_data,
                    "used_benchmark_data": rec.used_benchmark_data,
                    "task_type": format!("{:?}", task_type),
                    "historical_stats_available": historical_stats.as_ref().map(|h| h.len()),
                    "inputs": {
                        "complexity": complexity,
                        "estimated_tokens": estimated_tokens,
                        "budget_cents": budget_cents
                    }
                }))
            }
            None => {
                AgentResult::failure(
                    "No suitable model found within budget",
                    0,
                )
            }
        }
    }
}

impl LeafAgent for ModelSelector {
    fn capability(&self) -> LeafCapability {
        LeafCapability::ModelSelection
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_pricing(id: &str, prompt: f64, completion: f64) -> PricingInfo {
        PricingInfo {
            model_id: id.to_string(),
            prompt_cost_per_million: prompt,
            completion_cost_per_million: completion,
            context_length: 100000,
            max_output_tokens: None,
            supports_tools: true,
        }
    }

    #[test]
    fn test_expected_cost_u_curve() {
        let selector = ModelSelector::new();
        
        let cheap = make_pricing("cheap", 0.1, 0.2);
        let medium = make_pricing("medium", 1.0, 2.0);
        let expensive = make_pricing("expensive", 10.0, 20.0);
        
        let complexity = 0.7;
        let tokens = 2000;
        
        let cheap_cost = selector.calculate_expected_cost(&cheap, complexity, tokens);
        let medium_cost = selector.calculate_expected_cost(&medium, complexity, tokens);
        let expensive_cost = selector.calculate_expected_cost(&expensive, complexity, tokens);
        
        // For complex tasks, medium should be optimal (U-curve)
        // Cheap model has high failure rate
        // Expensive model has high base cost
        println!("Cheap: {} (fail: {})", cheap_cost.expected_cost_cents, cheap_cost.failure_probability);
        println!("Medium: {} (fail: {})", medium_cost.expected_cost_cents, medium_cost.failure_probability);
        println!("Expensive: {} (fail: {})", expensive_cost.expected_cost_cents, expensive_cost.failure_probability);
        
        // Basic sanity check: cheap model should have higher failure rate
        assert!(cheap_cost.failure_probability > medium_cost.failure_probability);
    }

    #[test]
    fn test_task_type_inference() {
        assert_eq!(
            TaskType::infer_from_description("Implement a function to sort arrays"),
            TaskType::Code
        );
        assert_eq!(
            TaskType::infer_from_description("Calculate the integral of x^2"),
            TaskType::Math
        );
        assert_eq!(
            TaskType::infer_from_description("Explain quantum mechanics"),
            TaskType::Reasoning
        );
    }
}

