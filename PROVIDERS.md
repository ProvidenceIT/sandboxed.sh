# AI Provider Configuration Guide

This guide explains how to configure AI providers in Sandboxed.sh.

## Supported Providers

### Tested & Fully Supported

#### Anthropic
- **Models**: `claude-opus-4-5`, `claude-sonnet-4-5`, `claude-haiku-4-5`
- **Auth**: OAuth (Claude Pro/Max) or API Key
- **Backends**: OpenCode, Claude Code
- **API Key Env**: `ANTHROPIC_API_KEY`
- **Example Config**:
  ```json
  {
    "provider_type": "anthropic",
    "name": "Anthropic",
    "api_key": "sk-ant-api03-...",
    "use_for_backends": ["opencode", "claudecode"]
  }
  ```

#### OpenAI
- **Models**: `gpt-5.2`, `gpt-5.2-codex`, `gpt-4o`
- **Auth**: OAuth (ChatGPT Plus/Pro) or API Key
- **Backends**: OpenCode, Codex
- **API Key Env**: `OPENAI_API_KEY`
- **Example Config**:
  ```json
  {
    "provider_type": "openai",
    "name": "OpenAI",
    "api_key": "sk-...",
    "use_for_backends": ["opencode", "codex"]
  }
  ```

#### Google
- **Models**: `gemini-3-pro`, `gemini-3-flash`, `gemini-2-flash-thinking`
- **Auth**: OAuth or API Key
- **Backends**: OpenCode
- **API Key Env**: `GOOGLE_GENERATIVE_AI_API_KEY` or `GOOGLE_API_KEY`

#### Cerebras
- **Models**: `llama-3.3-70b`, `llama-3.1-8b`
- **Auth**: API Key only
- **Backends**: OpenCode
- **API Key Env**: `CEREBRAS_API_KEY`
- **Base URL**: `https://api.cerebras.ai/v1`
- **Example Config**:
  ```json
  {
    "provider_type": "cerebras",
    "name": "Cerebras",
    "api_key": "csk-...",
    "use_for_backends": ["opencode"]
  }
  ```
- **Model Override Example**: `cerebras/llama-3.3-70b`

#### Z.AI (ZhipuAI/GLM)
- **Models**: `glm-5`, `glm-4-flash`, `glm-4-plus`
- **Auth**: API Key only
- **Backends**: OpenCode
- **API Key Env**: `ZHIPU_API_KEY`
- **Base URL**: `https://open.bigmodel.cn/api/paas/v4`
- **Example Config**:
  ```json
  {
    "provider_type": "zai",
    "name": "Z.AI",
    "api_key": "6e2476987dba4f75be9f4818ce3e1b25.xxxxx",
    "use_for_backends": ["opencode"]
  }
  ```
- **Model Override Example**: `zai/glm-5`

### Additional Supported Providers

#### Deep Infra
- **Models**: `meta-llama/Meta-Llama-3.1-70B-Instruct`, `Qwen/QwQ-32B-Preview`
- **API Key Env**: `DEEPINFRA_API_KEY`

#### Together AI
- **Models**: `meta-llama/Llama-3.3-70B-Instruct-Turbo`, `Qwen/Qwen2.5-Coder-32B-Instruct`
- **API Key Env**: `TOGETHER_API_KEY`

#### Perplexity
- **Models**: `llama-3.1-sonar-large-128k-online`, `llama-3.1-sonar-small-128k-online`
- **API Key Env**: `PERPLEXITY_API_KEY`

#### Cohere
- **API Key Env**: `COHERE_API_KEY`

#### Custom Providers
- For OpenAI-compatible endpoints
- Requires base URL and custom models configuration

## Configuration Methods

### 1. Via Dashboard UI
1. Navigate to Settings → Providers
2. Click "Add Provider"
3. Select provider type
4. Choose authentication method (OAuth or API Key)
5. Follow the prompts
6. Select which backends to use this provider for

### 2. Via API
```bash
curl -X POST https://your-backend/api/ai/providers \
  -H "Content-Type: application/json" \
  -d '{
    "provider_type": "cerebras",
    "name": "Cerebras",
    "api_key": "your-api-key",
    "enabled": true,
    "use_for_backends": ["opencode"]
  }'
```

### 3. Model Override in Missions
When creating a mission, you can override the model:
```bash
curl -X POST https://your-backend/api/missions \
  -H "Content-Type: application/json" \
  -d '{
    "prompt": "Your task",
    "workspace_id": "workspace-id",
    "backend": "opencode",
    "model_override": "cerebras/llama-3.3-70b"
  }'
```

## oh-my-opencode Configuration

For OpenCode backend, models are configured via oh-my-opencode profiles.

### Example: Cerebras Profile
```json
{
  "$schema": "https://raw.githubusercontent.com/code-yeongyu/oh-my-opencode/master/assets/oh-my-opencode.schema.json",
  "agents": {
    "atlas": {
      "model": "cerebras/llama-3.3-70b"
    },
    "explore": {
      "model": "cerebras/llama-3.1-8b"
    }
  },
  "categories": {
    "quick": {
      "model": "cerebras/llama-3.1-8b"
    },
    "deep": {
      "model": "cerebras/llama-3.3-70b"
    }
  }
}
```

### Example: Z.AI Profile
```json
{
  "$schema": "https://raw.githubusercontent.com/code-yeongyu/oh-my-opencode/master/assets/oh-my-opencode.schema.json",
  "agents": {
    "atlas": {
      "model": "zai/glm-5"
    },
    "explore": {
      "model": "zai/glm-4-flash"
    }
  },
  "categories": {
    "quick": {
      "model": "zai/glm-4-flash"
    },
    "deep": {
      "model": "zai/glm-5"
    }
  }
}
```

## Troubleshooting

### API Key Not Working
1. Verify the API key is correct
2. Check provider status: `GET /api/ai/providers`
3. Ensure provider is enabled
4. Check environment variables are set correctly

### Model Not Found
1. Verify model name matches provider's supported models
2. Check model override format: `provider/model-name`
3. For OpenCode: ensure oh-my-opencode config is correct

### Provider Auth Expires (OAuth)
OAuth tokens are automatically refreshed. If refresh fails:
1. Re-authenticate via dashboard
2. Check provider status for error messages

## Best Practices

1. **Use OAuth when available** - More secure and easier to manage
2. **Enable only needed backends** - Reduces configuration complexity
3. **Test with cheap models first** - Before using expensive models
4. **Set model overrides in profiles** - For consistent configuration
5. **Monitor provider status** - Check dashboard for auth issues
