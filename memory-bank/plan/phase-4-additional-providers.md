# Phase 4: Additional Providers

**Status:** Not Started
**Estimated Files:** 10-12 Rust modules
**Source Reference:** Multiple provider files (~3000 lines total)

---

## Objective

Implement remaining provider integrations following the patterns established in Phase 3 (Anthropic).

---

## Prerequisites

- Phase 3 complete (Anthropic provider working)

---

## Provider Summary

| Provider | Source File | Lines | API Protocol | Complexity |
|----------|-------------|-------|--------------|------------|
| OpenAI Completions | `openai-completions.ts` | 765 | SSE (SDK) | High (many compat flags) |
| OpenAI Responses | `openai-responses.ts` | 627 | SSE (SDK) | Medium |
| Google Generative AI | `google.ts` + `google-shared.ts` | 654 | Async Generator | Medium |
| Google Vertex | `google-vertex.ts` + `google-shared.ts` | 686 | Async Generator | Medium (ADC auth) |
| AWS Bedrock | `amazon-bedrock.ts` | 578 | SDK Stream | High (AWS auth) |

---

## Implementation Order

1. **OpenAI Completions** - Most providers use this API (Groq, Mistral, xAI, etc.)
2. **OpenAI Responses** - Newer OpenAI API (o1, o3, gpt-5)
3. **Google Generative AI** - Direct Gemini API
4. **Google Vertex** - Enterprise Gemini (shares code with #3)
5. **AWS Bedrock** - Requires AWS SDK

---

## 4.1 OpenAI Completions Provider

**Source:** `ai/src/providers/openai-completions.ts` (765 lines)

### Used By Providers
- OpenAI (gpt-4o, gpt-4-turbo, etc.)
- Groq
- Cerebras
- xAI (Grok)
- Mistral
- OpenRouter
- Minimax
- Vercel AI Gateway

### Options

```rust
// src/providers/openai/completions/options.rs

#[derive(Debug, Clone, Default)]
pub struct OpenAICompletionsOptions {
    pub api_key: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub reasoning_effort: Option<ThinkingLevel>,
    pub tool_choice: Option<ToolChoice>,
    pub headers: Option<HashMap<String, String>>,
    pub session_id: Option<String>,
}
```

### Compatibility Flags

The TypeScript has extensive compat handling per-provider:

```rust
// src/types/compat.rs (already done)

pub struct OpenAICompletionsCompat {
    pub supports_store: Option<bool>,
    pub supports_developer_role: Option<bool>,
    pub supports_reasoning_effort: Option<bool>,
    pub supports_usage_in_streaming: Option<bool>,
    pub max_tokens_field: Option<MaxTokensField>,
    pub requires_tool_result_name: Option<bool>,
    pub requires_assistant_after_tool_result: Option<bool>,
    pub requires_thinking_as_text: Option<bool>,
    pub requires_mistral_tool_ids: Option<bool>,
    pub thinking_format: Option<ThinkingFormat>,
}
```

### Key Implementation Details

1. **Tool Call ID Formats**
   - Default: truncate to 40 chars
   - Mistral: exactly 9 alphanumeric chars (special handling)

2. **Max Tokens Field**
   - Some use `max_tokens`, others use `max_completion_tokens`

3. **Thinking Format**
   - OpenAI: `reasoning` field with `reasoning_effort`
   - Zai: Different format

4. **Role Handling**
   - Some require `developer` role instead of `system`
   - Some require assistant message after tool results

### File Structure

```
src/providers/openai/
  mod.rs
  completions/
    mod.rs
    options.rs
    types.rs      # OpenAI Chat Completion types
    events.rs     # SSE delta events
    stream.rs     # Main streaming logic
    convert.rs    # Message conversion with compat flags
```

### SSE Event Types

```rust
#[derive(Debug, Deserialize)]
pub struct ChatCompletionChunk {
    pub id: String,
    pub choices: Vec<ChunkChoice>,
    pub usage: Option<UsageData>,
}

#[derive(Debug, Deserialize)]
pub struct ChunkChoice {
    pub index: usize,
    pub delta: Delta,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Delta {
    pub role: Option<String>,
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCallDelta>>,
    pub reasoning: Option<String>,  // For reasoning models
}
```

---

## 4.2 OpenAI Responses Provider

**Source:** `ai/src/providers/openai-responses.ts` (627 lines)

### Purpose

Newer OpenAI "Responses" API for o1, o3, gpt-5 models. Different event structure.

### Key Differences from Completions

1. **Tool Call ID Format**: `call_id|item_id` compound format
2. **Reasoning**: Separate `ResponseReasoningItem` blocks
3. **Encrypted Reasoning**: `include: ["reasoning.encrypted_content"]`

### File Structure

```
src/providers/openai/
  responses/
    mod.rs
    options.rs
    types.rs
    events.rs
    stream.rs
    convert.rs
```

---

## 4.3 Google Generative AI Provider

**Source:** `ai/src/providers/google.ts` (343 lines) + `google-shared.ts` (311 lines)

### Authentication

Simple API key via `GEMINI_API_KEY`

### SDK

Uses `@google/generative-ai` package. In Rust, options:
- Use `google-generative-ai-rs` crate (if available)
- Implement raw HTTP calls

### Thinking Support

```rust
pub struct GoogleThinkingConfig {
    pub enabled: bool,
    pub budget_tokens: Option<u32>,  // For Gemini 2.x
    pub level: Option<GoogleThinkingLevel>,  // For Gemini 3.x
}

pub enum GoogleThinkingLevel {
    Minimal,
    Low,
    Medium,
    High,
}
```

### Key Details

1. **Tool Call IDs**: Optional for most models, required for claude-*/gpt-oss-*
2. **Thought Signatures**: Base64 in `thoughtSignature` field
3. **Multimodal Function Responses**: Gemini 3 supports images in tool results

### File Structure

```
src/providers/google/
  mod.rs
  shared.rs       # Shared types/conversion (from google-shared.ts)
  generative/
    mod.rs
    options.rs
    types.rs
    stream.rs
    convert.rs
```

---

## 4.4 Google Vertex Provider

**Source:** `ai/src/providers/google-vertex.ts` (375 lines)

### Authentication

Uses Application Default Credentials (ADC), not API keys:
- `GOOGLE_APPLICATION_CREDENTIALS` env var
- Or `~/.config/gcloud/application_default_credentials.json`
- Plus `GOOGLE_CLOUD_PROJECT` and `GOOGLE_CLOUD_LOCATION`

### Implementation

Shares most logic with Google Generative AI via `google-shared.ts`.

```rust
// src/providers/google/vertex/options.rs

#[derive(Debug, Clone, Default)]
pub struct GoogleVertexOptions {
    pub project: Option<String>,
    pub location: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub thinking: Option<GoogleThinkingConfig>,
    pub headers: Option<HashMap<String, String>>,
}
```

### ADC Auth in Rust

```rust
use google_cloud_auth::credentials::CredentialsFile;

async fn get_access_token() -> Result<String> {
    // Check GOOGLE_APPLICATION_CREDENTIALS first
    let creds = if let Ok(path) = std::env::var("GOOGLE_APPLICATION_CREDENTIALS") {
        CredentialsFile::new_from_file(path).await?
    } else {
        CredentialsFile::new_from_application_default().await?
    };

    let token = creds.access_token().await?;
    Ok(token.value)
}
```

---

## 4.5 AWS Bedrock Provider

**Source:** `ai/src/providers/amazon-bedrock.ts` (578 lines)

### Authentication

Uses AWS credential chain (no API key):
- `AWS_PROFILE`
- `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY`
- `AWS_BEARER_TOKEN_BEDROCK`
- Container credentials (ECS)
- Web identity (IRSA)

### SDK

Use `aws-sdk-bedrockruntime` crate:

```rust
use aws_sdk_bedrockruntime::{Client, Config};
use aws_config::BehaviorVersion;

async fn create_client() -> Result<Client> {
    let config = aws_config::defaults(BehaviorVersion::latest())
        .load()
        .await;
    Ok(Client::new(&config))
}
```

### Converse Stream API

```rust
use aws_sdk_bedrockruntime::types::{
    ConversationRole, ContentBlock, Message as BedrockMessage,
    ToolConfiguration, ToolUseBlock,
};

async fn stream_bedrock(
    client: &Client,
    model_id: &str,
    messages: Vec<BedrockMessage>,
) -> Result<impl Stream<Item = StreamEvent>> {
    let response = client
        .converse_stream()
        .model_id(model_id)
        .set_messages(Some(messages))
        .send()
        .await?;

    Ok(response.stream)
}
```

### Key Details

1. **Tool Call IDs**: Alphanumeric only (special chars cause errors)
2. **Prompt Caching**: Supported for Claude models
3. **Signature Field**: Only supported by Anthropic models on Bedrock

### File Structure

```
src/providers/bedrock/
  mod.rs
  options.rs
  types.rs
  stream.rs
  convert.rs
```

---

## Dependencies to Add

```toml
[dependencies]
# For Google providers
google-cloud-auth = "0.16"

# For Bedrock
aws-config = "1.5"
aws-sdk-bedrockruntime = "1.50"

# For OpenAI (optional - could use raw HTTP)
async-openai = "0.24"  # Or implement manually
```

---

## File Structure After Phase 4

```
alchemy/
  src/
    providers/
      mod.rs
      env.rs
      anthropic/
        ...
      openai/
        mod.rs
        completions/
          mod.rs
          options.rs
          types.rs
          events.rs
          stream.rs
          convert.rs
        responses/
          mod.rs
          options.rs
          types.rs
          events.rs
          stream.rs
          convert.rs
      google/
        mod.rs
        shared.rs
        generative/
          mod.rs
          options.rs
          types.rs
          stream.rs
          convert.rs
        vertex/
          mod.rs
          options.rs
          stream.rs
      bedrock/
        mod.rs
        options.rs
        types.rs
        stream.rs
        convert.rs
```

---

## Acceptance Criteria

### OpenAI Completions
- [ ] Works with OpenAI API
- [ ] Works with Groq (test different compat)
- [ ] Works with Mistral (9-char tool IDs)
- [ ] Reasoning effort passed correctly
- [ ] Streaming usage works

### OpenAI Responses
- [ ] Works with o1/o3 models
- [ ] Compound tool call IDs handled
- [ ] Encrypted reasoning supported

### Google Generative AI
- [ ] Works with Gemini 2.x (budget tokens)
- [ ] Works with Gemini 3.x (thinking levels)
- [ ] Thought signatures preserved
- [ ] Images in requests work

### Google Vertex
- [ ] ADC auth works
- [ ] Project/location from env
- [ ] Shares conversion with generative

### AWS Bedrock
- [ ] AWS credential chain works
- [ ] Claude models stream correctly
- [ ] Prompt caching works
- [ ] Tool call IDs sanitized

---

## Testing Strategy

Each provider needs:
1. Unit tests for message conversion
2. Unit tests for event parsing
3. Integration tests with mock server (wiremock)
4. Optional: Live API tests (feature-gated)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};

    #[tokio::test]
    async fn test_openai_completions_stream() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(ResponseTemplate::new(200)
                .set_body_string(mock_sse_response()))
            .mount(&mock_server)
            .await;

        // Test streaming...
    }
}
```

---

## Notes

- Consider implementing OpenAI first since it covers the most providers
- Google providers share significant code - implement shared module first
- Bedrock requires AWS SDK which adds ~10MB to binary size
- May want feature flags for each provider to reduce binary size
