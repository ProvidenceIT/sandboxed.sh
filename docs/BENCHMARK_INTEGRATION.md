# Benchmark Integration for Smart Model Selection

This document describes how benchmark data from llm-stats.com is integrated into open_agent for task-aware model selection.

## Overview

The open_agent now uses actual benchmark scores (from ZeroEval/llm-stats.com) to select models based on task type, instead of using price as a proxy for capability.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        Task Received                             │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                    ComplexityEstimator                           │
│  - Estimates task complexity (0-1)                               │
│  - Estimates token usage                                         │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      ModelSelector                               │
│  1. Infer TaskType from description                              │
│  2. Look up benchmark scores for each model                      │
│  3. Calculate expected cost using U-curve optimization           │
│  4. Select optimal model for task type + complexity              │
└─────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│                      TaskExecutor                                │
│  - Executes task with selected model                             │
│  - Uses tools as needed                                          │
└─────────────────────────────────────────────────────────────────┘
```

## Task Types

Tasks are classified into 6 categories:

| Task Type | Indicators | Key Benchmarks |
|-----------|-----------|----------------|
| `Code` | implement, function, bug, debug, refactor | SWE-bench, HumanEval, LiveCodeBench |
| `Math` | calculate, equation, formula, prove | AIME 2025, MATH-500, GSM8K |
| `Reasoning` | explain, analyze, why, how | GPQA, MMLU-Pro, MMLU |
| `ToolCalling` | fetch, search, file, command | BFCL, Tau-Bench, ACEBench |
| `LongContext` | document, summarize, multiple files | RULER, LongBench |
| `General` | (default) | IFEval, Arena-Hard, MT-Bench |

## Capability Lookup

When selecting a model, the system:

1. **Infers task type** from the task description using keyword matching
2. **Looks up benchmark scores** for each available model
3. **Uses benchmark-based capability** if data is available
4. **Falls back to price-based heuristic** if no benchmark data

```rust
// Example: Model capability for a coding task
let task_type = TaskType::infer_from_description("Fix the bug in user authentication");
// → TaskType::Code

// Lookup capability from benchmarks
let capability = benchmarks.capability("openai/gpt-5.2", TaskType::Code);
// → 0.731 (from SWE-bench-verified score)
```

## Data Files

| File | Description |
|------|-------------|
| `models_with_benchmarks.json` | Main data file with all models and benchmark scores |
| `openrouter_models_raw.json` | Raw OpenRouter API response |
| `llm_stats_benchmarks.json` | Benchmark metadata from ZeroEval |

## Updating Benchmark Data

Run the merge script to refresh benchmark data:

```bash
python3 scripts/merge_benchmarks.py
```

This fetches:
- 349 models from OpenRouter
- 383 benchmarks from ZeroEval API
- Matches ~156 models with benchmark scores

## U-Curve Cost Optimization

The model selection uses a U-curve cost model:

```
Expected Cost = base_cost × (1 + failure_prob × retry_multiplier) × inefficiency
```

Where:
- `failure_prob = complexity × (1 - capability)`
- `inefficiency = 1 + (1 - capability) × 0.5`
- `capability` = benchmark score for task type (or price-based fallback)

This means:
- **Cheap models**: Low base cost, but high failure rate for complex tasks
- **Expensive models**: High base cost, but reliable
- **Optimal**: Somewhere in the middle, minimizing expected total cost

## Model Selection Priority

1. **Historical data** (if memory system has past execution stats)
2. **Benchmark data** (from models_with_benchmarks.json)
3. **Price-based heuristic** (fallback)

## Example Selection

For a coding task with complexity 0.7:

| Model | Benchmark Score | Failure Prob | Expected Cost | Selected |
|-------|----------------|--------------|---------------|----------|
| gpt-5.2 | 0.731 (code) | 0.19 | $0.08 | |
| claude-opus-4.5 | 0.87 (code) | 0.09 | $0.12 | |
| deepseek-v3.2 | 0.731 (code) | 0.19 | $0.03 | ✓ |
| gpt-4o-mini | 0.43 (code) | 0.40 | $0.05 | |

DeepSeek is selected because it has good benchmark scores AND low cost.

## Testing

1. Start the agent: `cargo run`
2. Send a coding task via the control API
3. Check the model selection in the response:

```json
{
  "model_id": "deepseek/deepseek-v3.2",
  "task_type": "Code",
  "used_benchmark_data": true,
  "confidence": 0.81,
  "reasoning": "Selected deepseek/deepseek-v3.2 for Code task..."
}
```

## Files Modified

- `src/budget/benchmarks.rs` - New module for benchmark data types and loading
- `src/budget/mod.rs` - Added benchmarks export
- `src/agents/context.rs` - Added benchmarks field to AgentContext
- `src/agents/leaf/model_select.rs` - Updated to use benchmark-based capability
- `src/api/routes.rs` - Load benchmarks at startup
- `src/api/control.rs` - Pass benchmarks through control session

## Future Improvements

1. **LLM-based task classification**: Use a small model to classify task type more accurately
2. **Category-specific weights**: Weight different benchmarks within a category
3. **Cost-performance Pareto frontier**: Show users the tradeoff curve
4. **Automatic benchmark refresh**: Periodically update benchmark data
