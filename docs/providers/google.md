---
summary: "First-class Google Generative AI (Gemini) provider guide: models, configuration, streaming behavior, and smoke testing"
read_when:
  - You want to use Google Gemini with `alchemy_llm`
  - You need to understand Gemini thinking parts and tool calling behavior
  - You need model constructors and environment variable setup
---

# Google Generative AI (Gemini) Provider Guide

## What was added

Google Generative AI (Gemini) is now a first-class provider path:

- `Api::GoogleGenerativeAi` added to the API enum
- Dedicated implementation in `src/providers/google.rs`
- Built-in model constructors in `src/models/google.rs`
- Custom message/tool conversion for Google's unique request format
- Native thinking support via `thought` field in parts (Gemini 2.5+)
- Complete tool call handling (Google sends full calls, not deltas)

## Environment Variables

| Env var (priority order) | Description |
|---|---|
| `GOOGLE_API_KEY` | Primary API key (official Google precedent) |
| `GEMINI_API_KEY` | Fallback API key |

`stream()` resolves API keys from options first, then falls back to environment variables.

## Built-in Gemini Models

All built-in Gemini models:

- use `Api::GoogleGenerativeAi`
- use `InputType::Text` and `InputType::Image` (all are multimodal)
- base URL: `https://generativelanguage.googleapis.com/v1beta`

| Constructor | Model ID | Reasoning | Context | Max Output |
|---|---|---|---|---|
| `gemini_2_5_pro()` | `gemini-2.5-pro-latest` | yes | 1,048,576 | 65,536 |
| `gemini_2_5_flash()` | `gemini-2.5-flash-latest` | yes | 1,048,576 | 65,536 |
| `gemini_2_5_flash_lite()` | `gemini-2.5-flash-lite-latest` | no | 1,048,576 | 65,536 |
| `gemini_2_0_flash()` | `gemini-2.0-flash` | no | 1,048,576 | 8,192 |

## Request Semantics

Google's API format differs from OpenAI-compatible APIs:

- **Messages** are sent as `contents` array (not `messages`)
- **System prompt** is a separate `systemInstruction` field
- **Temperature/max tokens** go inside `generationConfig`
- **Tools** are wrapped in `functionDeclarations`
- **Thinking** is enabled via `thinkingConfig: { includeThoughts: true }`

### Authentication

Google uses `x-goog-api-key` header (not Bearer token).

### URL Construction

Streaming URL is dynamic per model:
```
{base_url}/models/{model_id}:streamGenerateContent?alt=sse
```

### Tool Choice Mapping

| Alchemy | Google |
|---|---|
| `ToolChoice::Auto` | `AUTO` |
| `ToolChoice::None` | `NONE` |
| `ToolChoice::Required` | `ANY` |

## Streaming Behavior

### SSE Format

Google uses `alt=sse` which produces standard SSE `data: ` lines. Unlike OpenAI, Google does **not** send `data: [DONE]` — the stream ends via connection close.

### Thinking Parts

Gemini 2.5+ models provide thinking via explicit `thought: true` field in parts (not via `<think>` tags). Thinking signature is set to `"thought"`.

### Tool Calls

Google sends complete tool calls in a single part (name + full args), not as incremental deltas like OpenAI. Tool call IDs are generated as `google_tc_{index}` since Google doesn't provide IDs.

### Stop Reason Mapping

| Google | Alchemy |
|---|---|
| `STOP` | `StopReason::Stop` |
| `MAX_TOKENS` | `StopReason::Length` |
| `SAFETY` | `StopReason::Error` |
| `RECITATION` | `StopReason::Error` |
| `OTHER` | `StopReason::Error` |

### Usage Metadata

| Google field | Alchemy field |
|---|---|
| `promptTokenCount` | `usage.input` |
| `candidatesTokenCount` | `usage.output` |
| `totalTokenCount` | `usage.total_tokens` |
| `cachedContentTokenCount` | `usage.cache_read` |

## Replay Behavior

When assistant history is converted back to Google request messages:
- Text content is sent as `{ text: "..." }` parts
- Tool calls are sent as `{ functionCall: { name, args } }` parts
- Thinking content is **omitted** (Google manages thinking internally)

## Quick Start

```rust
use alchemy_llm::{gemini_2_5_flash, stream, OpenAICompletionsOptions};
use alchemy_llm::types::{AssistantMessageEvent, Context, Message, UserContent, UserMessage};
use futures::StreamExt;

#[tokio::main]
async fn main() -> alchemy_llm::Result<()> {
    let model = gemini_2_5_flash();

    let context = Context {
        system_prompt: Some("You are concise.".to_string()),
        messages: vec![Message::User(UserMessage {
            content: UserContent::Text("Explain Rust ownership briefly.".to_string()),
            timestamp: 0,
        })],
        tools: None,
    };

    let mut events = stream(
        &model,
        &context,
        Some(OpenAICompletionsOptions {
            api_key: std::env::var("GOOGLE_API_KEY").ok(),
            ..OpenAICompletionsOptions::default()
        }),
    )?;

    while let Some(event) = events.next().await {
        match event {
            AssistantMessageEvent::ThinkingDelta { delta, .. } => print!("{delta}"),
            AssistantMessageEvent::TextDelta { delta, .. } => print!("{delta}"),
            _ => {}
        }
    }

    Ok(())
}
```

## Live Smoke Scripts

- `smokescripts/run_google_streaming.sh`
- `smokescripts/run_google_tool_call.sh`
- `smokescripts/run_all_google.sh`

See: [`smokescripts/README.md`](../../smokescripts/README.md)
