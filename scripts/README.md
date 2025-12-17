# Open Agent Scripts

Reusable Python scripts for data processing tasks that are too large for LLM context.

## Available Scripts

### merge_benchmarks.py

Merges OpenRouter models with ZeroEval benchmark scores.

**Usage:**
```bash
python3 scripts/merge_benchmarks.py
```

**What it does:**
1. Fetches all models from OpenRouter API (~350 models)
2. Fetches benchmark metadata from ZeroEval API (~383 benchmarks)
3. Fetches scores for key benchmarks in each category:
   - **code**: SWE-bench, HumanEval, LiveCodeBench, Aider-Polyglot, etc.
   - **math**: AIME 2025/2024, MATH-500, GSM8K, etc.
   - **reasoning**: GPQA, MMLU-Pro, MMLU, ARC, HellaSwag, etc.
   - **tool_calling**: BFCL, Tau-Bench, ACEBench, etc.
   - **long_context**: RULER, LongBench, InfiniteBench, etc.
   - **general**: IFEval, Arena-Hard, MT-Bench, etc.
4. Merges models with benchmark data
5. Outputs `models_with_benchmarks.json`

**Output files:**
- `models_with_benchmarks.json` - Main output with merged data
- `openrouter_models_raw.json` - Raw OpenRouter API response
- `llm_stats_benchmarks.json` - Benchmark metadata from ZeroEval

**Output format:**
```json
{
  "generated_at": "2025-12-17T03:37:04Z",
  "total_models": 349,
  "models_with_benchmarks": 156,
  "categories": ["code", "math", "reasoning", "tool_calling", "long_context", "general"],
  "models": [
    {
      "id": "openai/gpt-5.2",
      "name": "GPT-5.2",
      "context_length": 400000,
      "pricing": {...},
      "benchmarks": {
        "code": {"swe-bench-verified": 0.731},
        "math": {"aime-2025": 0.96},
        "reasoning": {"gpqa": 0.924}
      },
      "category_scores": {
        "code": 0.731,
        "math": 0.96,
        "reasoning": 0.924
      }
    }
  ]
}
```

## Best Practices for Large Data Tasks

When dealing with data too large for the LLM context (>10KB):

1. **Use scripts**: Run Python/bash scripts with `run_command`
2. **Write to files**: Save intermediate results to files
3. **Read summaries**: Read only summaries or specific sections
4. **Process in chunks**: Break large tasks into smaller pieces

Example:
```bash
# Run the merge script
python3 scripts/merge_benchmarks.py

# Check summary
python3 -c "import json; d=json.load(open('models_with_benchmarks.json')); print(f'Models: {d[\"total_models\"]}, With benchmarks: {d[\"models_with_benchmarks\"]}')"

# Look up specific model
python3 -c "import json; d=json.load(open('models_with_benchmarks.json')); m=[x for x in d['models'] if 'gpt-5' in x['id'].lower()]; print(json.dumps(m[:3], indent=2))"
```
