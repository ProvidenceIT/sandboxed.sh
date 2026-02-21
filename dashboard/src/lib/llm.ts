/**
 * Lightweight client for calling an OpenAI-compatible chat completions API.
 * Used by dashboard UX features (auto-title generation, etc.).
 */

import { readLLMConfig } from "./llm-settings";

interface ChatMessage {
  role: "system" | "user" | "assistant";
  content: string;
}

interface ChatCompletionChoice {
  message: { content: string };
}

interface ChatCompletionResponse {
  choices: ChatCompletionChoice[];
}

/**
 * Send a chat completion request to the configured LLM provider.
 * Returns the assistant's response text, or null on failure.
 */
async function chatCompletion(
  messages: ChatMessage[],
  options?: { maxTokens?: number }
): Promise<string | null> {
  const cfg = readLLMConfig();
  if (!cfg.enabled || !cfg.apiKey) return null;

  const url = `${cfg.baseUrl.replace(/\/+$/, "")}/chat/completions`;

  try {
    const res = await fetch(url, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Authorization: `Bearer ${cfg.apiKey}`,
      },
      body: JSON.stringify({
        model: cfg.model,
        messages,
        max_tokens: options?.maxTokens ?? 60,
        temperature: 0.3,
      }),
    });

    if (!res.ok) return null;

    const data = (await res.json()) as ChatCompletionResponse;
    return data.choices?.[0]?.message?.content?.trim() ?? null;
  } catch {
    return null;
  }
}

/**
 * Generate a concise mission title from the first user message and assistant reply.
 * Returns null if the LLM is not configured or the request fails.
 */
export async function generateMissionTitle(
  userMessage: string,
  assistantReply: string
): Promise<string | null> {
  const trimmedUser = userMessage.slice(0, 800);
  const trimmedAssistant = assistantReply.slice(0, 800);

  return chatCompletion(
    [
      {
        role: "system",
        content:
          "Generate a short, descriptive title (3-8 words) for this coding mission. " +
          "Return ONLY the title text, no quotes, no prefix, no explanation.",
      },
      {
        role: "user",
        content: `User request:\n${trimmedUser}\n\nAssistant response:\n${trimmedAssistant}`,
      },
    ],
    { maxTokens: 30 }
  );
}
