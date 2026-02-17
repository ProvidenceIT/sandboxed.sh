/**
 * Model Routing API - Chain management and provider health tracking.
 */

import { apiGet, apiPost, apiPut, apiDel } from "./core";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface ChainEntry {
  provider_id: string;
  model_id: string;
}

export interface ModelChain {
  id: string;
  name: string;
  entries: ChainEntry[];
  is_default: boolean;
  created_at: string;
  updated_at: string;
}

export interface ResolvedEntry {
  provider_id: string;
  model_id: string;
  account_id: string;
  has_api_key: boolean;
  has_base_url: boolean;
}

export interface AccountHealthSnapshot {
  account_id: string;
  is_healthy: boolean;
  cooldown_remaining_secs: number | null;
  consecutive_failures: number;
  last_failure_reason: string | null;
  last_failure_at: string | null;
  total_requests: number;
  total_successes: number;
  total_rate_limits: number;
  total_errors: number;
  avg_latency_ms: number | null;
}

export interface FallbackEvent {
  timestamp: string;
  chain_id: string;
  from_provider: string;
  from_account_id: string;
  reason: string;
  cooldown_secs: number | null;
  to_provider: string | null;
  latency_ms: number | null;
}

// ---------------------------------------------------------------------------
// Chain Management
// ---------------------------------------------------------------------------

export async function listModelChains(): Promise<ModelChain[]> {
  return apiGet("/api/model-routing/chains", "Failed to list model chains");
}

export async function createModelChain(data: {
  id: string;
  name: string;
  entries: ChainEntry[];
  is_default?: boolean;
}): Promise<ModelChain> {
  return apiPost("/api/model-routing/chains", data, "Failed to create model chain");
}

export async function updateModelChain(
  id: string,
  data: {
    name?: string;
    entries?: ChainEntry[];
    is_default?: boolean;
  }
): Promise<ModelChain> {
  return apiPut(
    `/api/model-routing/chains/${encodeURIComponent(id)}`,
    data,
    "Failed to update model chain"
  );
}

export async function deleteModelChain(id: string): Promise<void> {
  return apiDel(
    `/api/model-routing/chains/${encodeURIComponent(id)}`,
    "Failed to delete model chain"
  );
}

export async function resolveModelChain(id: string): Promise<ResolvedEntry[]> {
  return apiGet(
    `/api/model-routing/chains/${encodeURIComponent(id)}/resolve`,
    "Failed to resolve model chain"
  );
}

// ---------------------------------------------------------------------------
// Health Tracking
// ---------------------------------------------------------------------------

export async function listAccountHealth(): Promise<AccountHealthSnapshot[]> {
  return apiGet("/api/model-routing/health", "Failed to list account health");
}

export async function clearAccountCooldown(accountId: string): Promise<{ cleared: boolean }> {
  return apiPost(
    `/api/model-routing/health/${accountId}/clear`,
    undefined,
    "Failed to clear account cooldown"
  );
}

// ---------------------------------------------------------------------------
// Observability
// ---------------------------------------------------------------------------

export async function listFallbackEvents(): Promise<FallbackEvent[]> {
  return apiGet("/api/model-routing/events", "Failed to list fallback events");
}
