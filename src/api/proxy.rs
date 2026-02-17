//! OpenAI-compatible proxy endpoint.
//!
//! Receives `POST /v1/chat/completions` requests, resolves the model name
//! to a chain of provider+account entries, and forwards the request through
//! the chain until one succeeds. Pre-stream 429/529 errors trigger instant
//! failover to the next entry in the chain.

use std::sync::Arc;

use axum::{
    body::Body,
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use futures::StreamExt;
use serde::{Deserialize, Serialize};

use crate::ai_providers::ProviderType;
use crate::provider_health::CooldownReason;

// ─────────────────────────────────────────────────────────────────────────────
// Types
// ─────────────────────────────────────────────────────────────────────────────

/// OpenAI-compatible chat completion request (subset we need for proxying).
///
/// We deserialize only the fields we inspect (model, stream); the full JSON
/// body is forwarded as-is to the upstream provider after swapping `model`.
#[derive(Debug, Deserialize)]
struct ChatCompletionRequest {
    model: String,
    #[serde(default)]
    stream: Option<bool>,
}

/// Minimal error response matching OpenAI's format.
#[derive(Serialize)]
struct ErrorResponse {
    error: ErrorBody,
}

#[derive(Serialize)]
struct ErrorBody {
    message: String,
    r#type: String,
    code: Option<String>,
}

fn error_response(status: StatusCode, message: String, code: &str) -> Response {
    let body = ErrorResponse {
        error: ErrorBody {
            message,
            r#type: "error".to_string(),
            code: Some(code.to_string()),
        },
    };
    (status, Json(body)).into_response()
}

// ─────────────────────────────────────────────────────────────────────────────
// Provider Base URLs
// ─────────────────────────────────────────────────────────────────────────────

/// Default base URL for OpenAI-compatible providers.
///
/// Returns `None` for providers that don't have an OpenAI-compatible API
/// (e.g., Google Gemini uses a different format).
fn default_base_url(provider_type: ProviderType) -> Option<&'static str> {
    match provider_type {
        ProviderType::OpenAI => Some("https://api.openai.com/v1"),
        ProviderType::Xai => Some("https://api.x.ai/v1"),
        ProviderType::Cerebras => Some("https://api.cerebras.ai/v1"),
        ProviderType::Zai => Some("https://api.z.ai/api/coding/paas/v4"),
        ProviderType::Minimax => Some("https://api.minimax.io/v1"),
        ProviderType::DeepInfra => Some("https://api.deepinfra.com/v1/openai"),
        ProviderType::Groq => Some("https://api.groq.com/openai/v1"),
        ProviderType::OpenRouter => Some("https://openrouter.ai/api/v1"),
        ProviderType::Mistral => Some("https://api.mistral.ai/v1"),
        ProviderType::TogetherAI => Some("https://api.together.xyz/v1"),
        ProviderType::Perplexity => Some("https://api.perplexity.ai"),
        ProviderType::Custom => None, // uses account's base_url
        // Non-OpenAI-compatible providers
        ProviderType::Anthropic => None,
        ProviderType::Google => None,
        ProviderType::AmazonBedrock => None,
        ProviderType::Azure => None,
        ProviderType::Cohere => None,
        ProviderType::GithubCopilot => None,
    }
}

/// Get the chat completions URL for a resolved entry.
fn completions_url(
    provider_type: ProviderType,
    account_base_url: Option<&str>,
) -> Option<String> {
    // Account-level override takes precedence
    let base = account_base_url
        .or_else(|| default_base_url(provider_type))?;
    let base = base.trim_end_matches('/');
    Some(format!("{}/chat/completions", base))
}

// ─────────────────────────────────────────────────────────────────────────────
// Routes
// ─────────────────────────────────────────────────────────────────────────────

pub fn routes() -> Router<Arc<super::routes::AppState>> {
    Router::new()
        .route("/chat/completions", post(chat_completions))
        .route("/models", axum::routing::get(list_models))
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /v1/models — list chains as virtual "models"
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct ModelsResponse {
    object: &'static str,
    data: Vec<ModelObject>,
}

#[derive(Serialize)]
struct ModelObject {
    id: String,
    object: &'static str,
    created: i64,
    owned_by: &'static str,
}

/// Verify the proxy bearer token from the Authorization header.
fn verify_proxy_auth(headers: &HeaderMap, expected: &str) -> Result<(), Response> {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    match token {
        Some(t) if t == expected => Ok(()),
        _ => Err(error_response(
            StatusCode::UNAUTHORIZED,
            "Invalid or missing proxy authorization".to_string(),
            "authentication_error",
        )),
    }
}

async fn list_models(
    State(state): State<Arc<super::routes::AppState>>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = verify_proxy_auth(&headers, &state.proxy_secret) {
        return resp;
    }
    let chains = state.chain_store.list().await;
    let data = chains
        .into_iter()
        .map(|c| ModelObject {
            id: c.id,
            object: "model",
            created: c.created_at.timestamp(),
            owned_by: "sandboxed",
        })
        .collect();
    Json(ModelsResponse {
        object: "list",
        data,
    })
    .into_response()
}

// ─────────────────────────────────────────────────────────────────────────────
// Handler
// ─────────────────────────────────────────────────────────────────────────────

async fn chat_completions(
    State(state): State<Arc<super::routes::AppState>>,
    headers: HeaderMap,
    body: bytes::Bytes,
) -> Response {
    // 0. Verify proxy authorization
    if let Err(resp) = verify_proxy_auth(&headers, &state.proxy_secret) {
        return resp;
    }

    // 1. Parse the request to extract the model name
    let req: ChatCompletionRequest = match serde_json::from_slice(&body) {
        Ok(r) => r,
        Err(e) => {
            return error_response(
                StatusCode::BAD_REQUEST,
                format!("Invalid request body: {}", e),
                "invalid_request_error",
            );
        }
    };

    let is_stream = req.stream.unwrap_or(false);
    let requested_model = req.model.clone();

    // 2. Check if the model name maps to a chain ID.
    //    The @ai-sdk/openai-compatible adapter strips the provider prefix, so
    //    a model override "builtin/smart" arrives as just "smart".  We try:
    //      1. Exact match (e.g. "builtin/smart")
    //      2. "builtin/{model}" prefix (e.g. "smart" → "builtin/smart")
    //    Unknown models return an error — no silent fallback to the default
    //    chain, so typos and misconfigurations surface immediately.
    let chain_id = if state.chain_store.get(&requested_model).await.is_some() {
        requested_model.clone()
    } else {
        let prefixed = format!("builtin/{}", requested_model);
        if state.chain_store.get(&prefixed).await.is_some() {
            prefixed
        } else {
            return error_response(
                StatusCode::BAD_REQUEST,
                format!(
                    "Model '{}' is not a known chain. Available chains can be listed at /api/model-routing/chains",
                    requested_model
                ),
                "model_not_found",
            );
        }
    };

    // 3. Resolve chain → expanded entries with health filtering
    let standard_accounts =
        super::ai_providers::read_standard_accounts(&state.config.working_dir);

    let entries = state
        .chain_store
        .resolve_chain(
            &chain_id,
            &state.ai_providers,
            &standard_accounts,
            &state.health_tracker,
        )
        .await;

    if entries.is_empty() {
        return error_response(
            StatusCode::TOO_MANY_REQUESTS,
            "All providers in the chain are currently in cooldown or unconfigured".to_string(),
            "rate_limit_exceeded",
        );
    }

    // 4. Try each entry in order (waterfall)
    for entry in &entries {
        let provider_type = match ProviderType::from_id(&entry.provider_id) {
            Some(pt) => pt,
            None => continue,
        };

        let Some(api_key) = &entry.api_key else {
            continue;
        };

        let Some(url) = completions_url(provider_type, entry.base_url.as_deref()) else {
            tracing::debug!(
                provider = %entry.provider_id,
                "Skipping non-OpenAI-compatible provider in chain"
            );
            continue;
        };

        // Build the upstream request body: replace model with the real model ID
        let upstream_body = match rewrite_model(&body, &entry.model_id) {
            Ok(b) => b,
            Err(e) => {
                tracing::error!("Failed to rewrite model in request body: {}", e);
                continue;
            }
        };

        // Forward the request.
        //
        // For non-streaming requests, set a 300s timeout.  For streaming
        // requests, don't set a timeout — reqwest applies it to the full
        // response body, which would kill long-running LLM generations.
        let mut upstream_req = state
            .http_client
            .post(&url)
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", api_key))
            .body(upstream_body);
        if !is_stream {
            upstream_req = upstream_req.timeout(std::time::Duration::from_secs(300));
        }

        // Forward select client headers
        if let Some(org) = headers.get("openai-organization") {
            upstream_req = upstream_req.header("OpenAI-Organization", org);
        }

        tracing::debug!(
            provider = %entry.provider_id,
            model = %entry.model_id,
            account_id = %entry.account_id,
            url = %url,
            "Trying upstream provider"
        );

        let upstream_resp = match upstream_req.send().await {
            Ok(resp) => resp,
            Err(e) => {
                tracing::warn!(
                    provider = %entry.provider_id,
                    account_id = %entry.account_id,
                    error = %e,
                    "Upstream request failed (network error)"
                );
                let reason = if e.is_timeout() {
                    CooldownReason::Timeout
                } else {
                    CooldownReason::ServerError
                };
                state
                    .health_tracker
                    .record_failure(entry.account_id, reason, None)
                    .await;
                continue;
            }
        };

        let status = upstream_resp.status();

        // Pre-stream error handling: 429, 529, 5xx → cooldown + try next
        if status == StatusCode::TOO_MANY_REQUESTS || status.as_u16() == 529 {
            let retry_after = parse_retry_after(upstream_resp.headers());
            let reason = if status.as_u16() == 529 {
                CooldownReason::Overloaded
            } else {
                CooldownReason::RateLimit
            };
            tracing::info!(
                provider = %entry.provider_id,
                account_id = %entry.account_id,
                status = %status,
                retry_after_secs = ?retry_after.map(|d| d.as_secs_f64()),
                "Upstream rate limited, trying next entry"
            );
            state
                .health_tracker
                .record_failure(entry.account_id, reason, retry_after)
                .await;
            continue;
        }

        if status.is_server_error() {
            tracing::warn!(
                provider = %entry.provider_id,
                account_id = %entry.account_id,
                status = %status,
                "Upstream server error, trying next entry"
            );
            state
                .health_tracker
                .record_failure(entry.account_id, CooldownReason::ServerError, None)
                .await;
            continue;
        }

        // Auth errors (401/403) — bad credentials, try next account
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            tracing::warn!(
                provider = %entry.provider_id,
                account_id = %entry.account_id,
                status = %status,
                "Upstream auth error, trying next entry"
            );
            state
                .health_tracker
                .record_failure(entry.account_id, CooldownReason::AuthError, None)
                .await;
            continue;
        }

        // Only record health-tracker success on actual 2xx responses.
        // 4xx client errors (400, 422, etc.) are the caller's fault and should
        // not reset failure counters or clear cooldowns for this account.
        if status.is_success() {
            state
                .health_tracker
                .record_success(entry.account_id)
                .await;
        }

        // Stream the response back to the client
        if is_stream && status.is_success() {
            let mut response_headers = HeaderMap::new();
            response_headers.insert(
                header::CONTENT_TYPE,
                "text/event-stream".parse().unwrap(),
            );
            response_headers.insert(
                header::CACHE_CONTROL,
                "no-cache".parse().unwrap(),
            );

            let byte_stream = normalize_sse_stream(upstream_resp.bytes_stream());

            return (
                status,
                response_headers,
                Body::from_stream(byte_stream),
            )
                .into_response();
        }

        // Non-streaming: read full body and forward
        let response_headers = upstream_resp.headers().clone();
        match upstream_resp.bytes().await {
            Ok(resp_body) => {
                let mut builder = Response::builder().status(status);
                if let Some(ct) = response_headers.get(header::CONTENT_TYPE) {
                    builder = builder.header(header::CONTENT_TYPE, ct);
                }
                return builder
                    .body(Body::from(resp_body))
                    .unwrap_or_else(|_| {
                        error_response(
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Failed to build response".to_string(),
                            "internal_error",
                        )
                    });
            }
            Err(e) => {
                tracing::warn!(
                    provider = %entry.provider_id,
                    error = %e,
                    "Failed to read upstream response body"
                );
                continue;
            }
        }
    }

    // All entries exhausted
    tracing::warn!(
        chain = %chain_id,
        total_entries = entries.len(),
        "All chain entries exhausted"
    );
    error_response(
        StatusCode::TOO_MANY_REQUESTS,
        format!(
            "All {} providers in chain '{}' are rate-limited or unavailable",
            entries.len(),
            chain_id
        ),
        "rate_limit_exceeded",
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Rewrite the `model` field in the JSON request body.
fn rewrite_model(body: &[u8], new_model: &str) -> Result<bytes::Bytes, String> {
    let mut value: serde_json::Value =
        serde_json::from_slice(body).map_err(|e| format!("Invalid JSON: {}", e))?;
    value["model"] = serde_json::Value::String(new_model.to_string());
    serde_json::to_vec(&value)
        .map(bytes::Bytes::from)
        .map_err(|e| format!("Failed to serialize: {}", e))
}

/// Parse `Retry-After` header into a Duration (numeric seconds only).
fn parse_retry_after(headers: &HeaderMap) -> Option<std::time::Duration> {
    let value = headers.get("retry-after")?.to_str().ok()?;
    let secs: f64 = value.parse().ok()?;
    if secs > 0.0 {
        Some(std::time::Duration::from_secs_f64(secs))
    } else {
        None
    }
}

/// Normalize an SSE byte stream to fix provider-specific quirks.
///
/// Processes `data:` lines, parses the JSON chunk, and strips fields that
/// break OpenAI-compatible clients (e.g. MiniMax sending `delta.role: ""`).
fn normalize_sse_stream(
    inner: impl futures::Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
) -> impl futures::Stream<Item = Result<bytes::Bytes, std::io::Error>> + Send + 'static {
    futures::stream::unfold(
        (Box::pin(inner), Vec::<u8>::new()),
        |(mut stream, mut buf)| async move {
            loop {
                // Check if we have a complete line in the buffer
                if let Some(pos) = buf.iter().position(|&b| b == b'\n') {
                    let line = buf.drain(..=pos).collect::<Vec<u8>>();
                    let normalized = normalize_sse_line(&line);
                    return Some((
                        Ok(bytes::Bytes::from(normalized)),
                        (stream, buf),
                    ));
                }

                // Need more data
                match stream.next().await {
                    Some(Ok(chunk)) => {
                        buf.extend_from_slice(&chunk);
                    }
                    Some(Err(e)) => {
                        return Some((
                            Err(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())),
                            (stream, buf),
                        ));
                    }
                    None => {
                        // Stream ended — flush remaining buffer
                        if buf.is_empty() {
                            return None;
                        }
                        let remaining = std::mem::take(&mut buf);
                        let normalized = normalize_sse_line(&remaining);
                        return Some((
                            Ok(bytes::Bytes::from(normalized)),
                            (stream, buf),
                        ));
                    }
                }
            }
        },
    )
}

/// Normalize a single SSE line.  If it's a `data: {...}` line, parse and
/// fix known provider quirks; otherwise pass through unchanged.
fn normalize_sse_line(line: &[u8]) -> Vec<u8> {
    let trimmed = line.strip_suffix(b"\r\n").or_else(|| line.strip_suffix(b"\n")).unwrap_or(line);
    let data_prefix = b"data: ";

    if !trimmed.starts_with(data_prefix) {
        return line.to_vec();
    }

    let json_bytes = &trimmed[data_prefix.len()..];

    // "data: [DONE]" — pass through
    let json_trimmed: &[u8] = {
        let s = std::str::from_utf8(json_bytes).unwrap_or("");
        s.trim().as_bytes()
    };
    if json_trimmed == b"[DONE]" {
        return line.to_vec();
    }

    let mut chunk: serde_json::Value = match serde_json::from_slice(json_bytes) {
        Ok(v) => v,
        Err(_) => return line.to_vec(), // not valid JSON, pass through
    };

    let mut modified = false;

    // Fix MiniMax: strip empty `delta.role` field
    if let Some(choices) = chunk.get_mut("choices").and_then(|v| v.as_array_mut()) {
        for choice in choices {
            if let Some(delta) = choice.get_mut("delta").and_then(|v| v.as_object_mut()) {
                if delta.get("role").and_then(|v| v.as_str()) == Some("") {
                    delta.remove("role");
                    modified = true;
                }
            }
        }
    }

    if !modified {
        return line.to_vec();
    }

    // Re-serialize and preserve the original line ending
    let suffix = if line.ends_with(b"\r\n") { &b"\r\n"[..] } else if line.ends_with(b"\n") { &b"\n"[..] } else { &b""[..] };
    let mut out = Vec::from(&b"data: "[..]);
    let _ = serde_json::to_writer(&mut out, &chunk);
    out.extend_from_slice(suffix);
    out
}
