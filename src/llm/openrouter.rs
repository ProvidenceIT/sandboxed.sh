//! OpenRouter API client implementation.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{ChatMessage, ChatResponse, LlmClient, ToolCall, ToolDefinition};

const OPENROUTER_API_URL: &str = "https://openrouter.ai/api/v1/chat/completions";

/// OpenRouter API client.
pub struct OpenRouterClient {
    client: Client,
    api_key: String,
}

impl OpenRouterClient {
    /// Create a new OpenRouter client.
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }
}

#[async_trait]
impl LlmClient for OpenRouterClient {
    async fn chat_completion(
        &self,
        model: &str,
        messages: &[ChatMessage],
        tools: Option<&[ToolDefinition]>,
    ) -> anyhow::Result<ChatResponse> {
        let request = OpenRouterRequest {
            model: model.to_string(),
            messages: messages.to_vec(),
            tools: tools.map(|t| t.to_vec()),
            tool_choice: tools.map(|_| "auto".to_string()),
        };

        tracing::debug!("Sending request to OpenRouter: model={}", model);

        let response = self
            .client
            .post(OPENROUTER_API_URL)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("HTTP-Referer", "https://github.com/open-agent")
            .header("X-Title", "Open Agent")
            .json(&request)
            .send()
            .await?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            tracing::error!("OpenRouter error: status={}, body={}", status, body);
            return Err(anyhow::anyhow!(
                "OpenRouter API error: {} - {}",
                status,
                body
            ));
        }

        let response: OpenRouterResponse = serde_json::from_str(&body).map_err(|e| {
            tracing::error!("Failed to parse response: {}, body: {}", e, body);
            anyhow::anyhow!("Failed to parse OpenRouter response: {}", e)
        })?;

        let choice = response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("No choices in response"))?;

        Ok(ChatResponse {
            content: choice.message.content,
            tool_calls: choice.message.tool_calls,
            finish_reason: choice.finish_reason,
        })
    }
}

/// OpenRouter API request format.
#[derive(Debug, Serialize)]
struct OpenRouterRequest {
    model: String,
    messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ToolDefinition>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
}

/// OpenRouter API response format.
#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    choices: Vec<OpenRouterChoice>,
}

/// A choice in the OpenRouter response.
#[derive(Debug, Deserialize)]
struct OpenRouterChoice {
    message: OpenRouterMessage,
    finish_reason: Option<String>,
}

/// Message in OpenRouter response.
#[derive(Debug, Deserialize)]
struct OpenRouterMessage {
    content: Option<String>,
    tool_calls: Option<Vec<ToolCall>>,
}

