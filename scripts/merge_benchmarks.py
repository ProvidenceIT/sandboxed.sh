#!/usr/bin/env python3
"""
Merge OpenRouter models with ZeroEval benchmark scores.

This script:
1. Fetches all models from OpenRouter API
2. Fetches benchmark metadata from ZeroEval API
3. For key benchmarks in each category, fetches model scores
4. Creates a merged JSON with benchmark scores per category

Categories tracked:
- code: Coding benchmarks (SWE-bench, HumanEval, etc.)
- math: Math benchmarks (AIME, MATH, GSM8K, etc.)
- reasoning: Reasoning benchmarks (GPQA, MMLU, etc.)
- tool_calling: Tool/function calling benchmarks
- long_context: Long context benchmarks
"""

import json
import time
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Union
from urllib.request import Request, urlopen
from urllib.error import URLError, HTTPError
from collections import defaultdict

# Configuration
OPENROUTER_API = "https://openrouter.ai/api/v1/models"
ZEROEVAL_API = "https://api.zeroeval.com"
OUTPUT_DIR = Path(__file__).parent.parent  # /Users/thomas/workspace/open_agent

# Key benchmarks per category (prioritized list)
KEY_BENCHMARKS = {
    "code": [
        "swe-bench-verified", "humaneval", "livecodebench", "aider-polyglot",
        "bigcodebench", "codeforces", "mbpp"
    ],
    "math": [
        "aime-2025", "aime-2024", "math-500", "gsm8k", "minerva-math",
        "gpqa-diamond", "olympiadbench"
    ],
    "reasoning": [
        "gpqa", "mmlu-pro", "mmlu", "arc-challenge", "hellaswag",
        "winogrande", "commonsenseqa"
    ],
    "tool_calling": [
        "bfcl", "tau-bench", "acebench", "nexusraven", "gorilla-api-bench"
    ],
    "long_context": [
        "ruler", "longbench", "infinitebench", "scrolls", "loogle"
    ],
    "general": [
        "ifeval", "arena-hard", "alpaca-eval-2", "mt-bench", "chatbot-arena"
    ]
}

HEADERS = {
    "Accept": "application/json",
    "Origin": "https://llm-stats.com",
    "Referer": "https://llm-stats.com/",
    "User-Agent": "OpenAgent-BenchmarkMerger/1.0"
}


def fetch_json(url: str, retries: int = 3) -> Optional[Union[dict, list]]:
    """Fetch JSON from URL with retries."""
    for attempt in range(retries):
        try:
            req = Request(url, headers=HEADERS)
            with urlopen(req, timeout=30) as resp:
                return json.loads(resp.read().decode())
        except HTTPError as e:
            if e.code == 404:
                return None
            print(f"  HTTP error {e.code} for {url}, attempt {attempt + 1}")
        except URLError as e:
            print(f"  URL error for {url}: {e}, attempt {attempt + 1}")
        except Exception as e:
            print(f"  Error fetching {url}: {e}, attempt {attempt + 1}")
        time.sleep(1)
    return None


def fetch_openrouter_models() -> List[dict]:
    """Fetch all models from OpenRouter."""
    print("Fetching OpenRouter models...")
    data = fetch_json(OPENROUTER_API)
    if data and "data" in data:
        models = data["data"]
        print(f"  Found {len(models)} models")
        return models
    print("  Failed to fetch models!")
    return []


def fetch_all_benchmarks() -> List[dict]:
    """Fetch all benchmark metadata from ZeroEval."""
    print("Fetching ZeroEval benchmarks...")
    data = fetch_json(f"{ZEROEVAL_API}/leaderboard/benchmarks")
    if data:
        print(f"  Found {len(data)} benchmarks")
        return data
    print("  Failed to fetch benchmarks!")
    return []


def fetch_benchmark_scores(benchmark_id: str) -> Optional[dict]:
    """Fetch detailed benchmark scores for a specific benchmark."""
    data = fetch_json(f"{ZEROEVAL_API}/leaderboard/benchmarks/{benchmark_id}")
    return data


def normalize_model_id(model_id: str) -> str:
    """Normalize model ID for matching."""
    # Remove common prefixes/suffixes and normalize
    normalized = model_id.lower()
    # Remove date suffixes like -20251101
    parts = normalized.split("-")
    filtered = [p for p in parts if not (len(p) == 8 and p.isdigit())]
    return "-".join(filtered)


def build_model_score_map(benchmarks_data: Dict[str, dict]) -> Dict[str, dict]:
    """
    Build a map from normalized model names to their benchmark scores.
    
    Returns: {normalized_model_id: {category: {benchmark_id: score}}}
    """
    model_scores = defaultdict(lambda: defaultdict(dict))
    
    for category, benchmarks in benchmarks_data.items():
        for benchmark_id, benchmark_info in benchmarks.items():
            if not benchmark_info or "models" not in benchmark_info:
                continue
            
            for model in benchmark_info["models"]:
                model_id = model.get("model_id", "")
                score = model.get("score")
                if model_id and score is not None:
                    # Store both original and normalized
                    model_scores[model_id][category][benchmark_id] = score
                    
                    # Also store by normalized name for fuzzy matching
                    normalized = normalize_model_id(model_id)
                    if normalized != model_id:
                        model_scores[normalized][category][benchmark_id] = score
    
    return dict(model_scores)


def match_model(openrouter_id: str, zeroeval_scores: dict) -> Optional[dict]:
    """Try to match an OpenRouter model ID to ZeroEval scores."""
    # Try exact match first
    if openrouter_id in zeroeval_scores:
        return zeroeval_scores[openrouter_id]
    
    # Try normalized match
    normalized = normalize_model_id(openrouter_id)
    if normalized in zeroeval_scores:
        return zeroeval_scores[normalized]
    
    # Try partial match (model name without provider)
    if "/" in openrouter_id:
        model_name = openrouter_id.split("/")[-1]
        model_name_normalized = normalize_model_id(model_name)
        
        for ze_id, scores in zeroeval_scores.items():
            if model_name_normalized in ze_id or ze_id in model_name_normalized:
                return scores
    
    return None


def calculate_category_averages(scores: dict) -> dict:
    """Calculate average score per category."""
    averages = {}
    for category, benchmarks in scores.items():
        if benchmarks:
            avg = sum(benchmarks.values()) / len(benchmarks)
            averages[category] = round(avg, 4)
    return averages


def main():
    print("=" * 60)
    print("OpenRouter + ZeroEval Benchmark Merger")
    print("=" * 60)
    
    # Step 1: Fetch OpenRouter models
    openrouter_models = fetch_openrouter_models()
    if not openrouter_models:
        print("Failed to fetch OpenRouter models, exiting.")
        sys.exit(1)
    
    # Save raw OpenRouter models
    or_path = OUTPUT_DIR / "openrouter_models_raw.json"
    with open(or_path, "w") as f:
        json.dump({"data": openrouter_models}, f)
    print(f"Saved raw OpenRouter models to {or_path}")
    
    # Step 2: Fetch all benchmark metadata
    all_benchmarks = fetch_all_benchmarks()
    if not all_benchmarks:
        print("Failed to fetch benchmarks, exiting.")
        sys.exit(1)
    
    # Save benchmarks metadata
    bench_path = OUTPUT_DIR / "llm_stats_benchmarks.json"
    with open(bench_path, "w") as f:
        json.dump(all_benchmarks, f)
    print(f"Saved benchmarks metadata to {bench_path}")
    
    # Build benchmark ID lookup
    benchmark_lookup = {b["benchmark_id"]: b for b in all_benchmarks}
    
    # Step 3: Fetch scores for key benchmarks in each category
    print("\nFetching benchmark scores by category...")
    benchmarks_data = {}
    
    for category, benchmark_ids in KEY_BENCHMARKS.items():
        print(f"\n  Category: {category}")
        benchmarks_data[category] = {}
        
        for bench_id in benchmark_ids:
            # Try the exact ID first
            data = fetch_benchmark_scores(bench_id)
            
            # If not found, try finding a matching benchmark
            if data is None:
                # Search for similar benchmark IDs
                for full_id in benchmark_lookup.keys():
                    if bench_id in full_id or full_id in bench_id:
                        data = fetch_benchmark_scores(full_id)
                        if data:
                            bench_id = full_id
                            break
            
            if data:
                model_count = len(data.get("models", []))
                print(f"    ✓ {bench_id}: {model_count} models")
                benchmarks_data[category][bench_id] = data
            else:
                print(f"    ✗ {bench_id}: not found")
            
            time.sleep(0.2)  # Rate limiting
    
    # Step 4: Build model score map
    print("\nBuilding model score map...")
    model_scores = build_model_score_map(benchmarks_data)
    print(f"  Found scores for {len(model_scores)} unique model IDs")
    
    # Step 5: Merge with OpenRouter models
    print("\nMerging with OpenRouter models...")
    merged_models = []
    matched_count = 0
    
    for model in openrouter_models:
        model_id = model.get("id", "")
        
        # Try to find matching benchmark scores
        scores = match_model(model_id, model_scores)
        
        # Build merged model entry
        merged = {
            "id": model_id,
            "name": model.get("name", ""),
            "context_length": model.get("context_length"),
            "architecture": model.get("architecture", {}),
            "pricing": model.get("pricing", {}),
            "benchmarks": None,
            "category_scores": None
        }
        
        if scores:
            merged["benchmarks"] = scores
            merged["category_scores"] = calculate_category_averages(scores)
            matched_count += 1
        
        merged_models.append(merged)
    
    print(f"  Matched {matched_count}/{len(openrouter_models)} models with benchmarks")
    
    # Step 6: Save merged data
    output = {
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
        "total_models": len(merged_models),
        "models_with_benchmarks": matched_count,
        "categories": list(KEY_BENCHMARKS.keys()),
        "models": merged_models
    }
    
    output_path = OUTPUT_DIR / "models_with_benchmarks.json"
    with open(output_path, "w") as f:
        json.dump(output, f, indent=2)
    print(f"\n✓ Saved merged data to {output_path}")
    
    # Step 7: Create summary
    print("\n" + "=" * 60)
    print("Summary")
    print("=" * 60)
    print(f"Total OpenRouter models: {len(openrouter_models)}")
    print(f"Models with benchmark data: {matched_count}")
    print(f"Categories tracked: {', '.join(KEY_BENCHMARKS.keys())}")
    
    # Show some example matches
    print("\nExample matched models:")
    for m in merged_models[:10]:
        if m["benchmarks"]:
            cats = list(m["category_scores"].keys()) if m["category_scores"] else []
            print(f"  - {m['id']}: {', '.join(cats)}")


if __name__ == "__main__":
    main()
