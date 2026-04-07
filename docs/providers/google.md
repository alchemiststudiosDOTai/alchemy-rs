---
summary: "Google Generative AI (Gemini) first-class provider with dedicated streaming runtime"
read_when:
  - You are adding or debugging Google/Gemini support
  - You need the Gemini model helper or environment variable name
  - You want to understand how Gemini reasoning and tool calling works
  - You want to understand how Gemini fits the unified abstraction
---

# Google Generative AI Provider

Google Generative AI (Gemini) is a **first-class provider identity** in `alchemy_llm` with a dedicated streaming runtime that handles Gemini's unique REST API format.

That means:

- the public abstraction stays the same: `Model<TApi>`, `stream(...)`, and `complete(...)`
- callers target `KnownProvider::Google`
- a dedicated runtime handles Gemini's `contents`/`parts` format, `systemInstruction`, `generationConfig`, and SSE streaming with `alt=sse`
- shared block handling reuses the crate's canonical `CurrentBlock` state machine

## Quick Start

```rust
use alchemy_llm::{gemini_2_5_flash, stream};
use alchemy_llm::types::{AssistantMessageEvent, Context, Message, UserContent, UserMessage};
use futures::StreamExt;

#[tokio::main]
async fn main() -> alchemy_llm::Result<()> {
    let model = gemini_2_5_flash();
    let context = Context {
        system_prompt: None,
        messages: vec![Message::User(UserMessage {
            content: UserContent::Text("Hello from Gemini".to_string()),
            timestamp: 0,
        })],
        tools: None,
    };

    let mut stream = stream(&model, &context, None)?;

    while let Some(event) = stream.next().await {
        match event {
            AssistantMessageEvent::TextDelta { delta, .. } => print!("{}", delta),
            AssistantMessageEvent::ThinkingDelta { delta, .. } => print!("[thinking: {}]", delta),
            _ => {}
        }
    }

    Ok(())
}
```

Set `GEMINI_API_KEY` before calling `stream(...)` or `complete(...)`.

## Constructors

```rust
use alchemy_llm::{gemini_2_5_pro, gemini_2_5_flash};

let model = gemini_2_5_flash();
```

All helpers return `Model<GoogleGenerativeAi>` with:

- provider: `KnownProvider::Google`
- base URL: `https://generativelanguage.googleapis.com/v1beta`
- reasoning: `true`
- context window: `1_048_576`
- max output tokens: `65_536`
- input types: text, image

## Authentication

Set:

```bash
GEMINI_API_KEY=AIza...
```

The API key is passed as a query parameter (`?key=...`) per Google's REST API convention, not as a Bearer token header.

## API Format

Google Gemini uses a unique REST API that differs from both OpenAI and Anthropic:

- **Endpoint**: `/v1beta/models/{model}:streamGenerateContent?alt=sse`
- **Messages**: `contents` array with `role: "user" | "model"` and `parts` arrays
- **System prompt**: `systemInstruction` object (not a message role)
- **Generation config**: `generationConfig` object for `temperature`, `maxOutputTokens`, etc.
- **Tools**: `functionDeclarations` format (not OpenAI's `tools` schema)
- **Tool calls**: `functionCall` in parts; results sent as `functionResponse`
- **Thinking**: `thinkingConfig` with `thinkingBudget` (Gemini 2.5+); thoughts arrive as parts with `thought: true`

## Reasoning

For reasoning-capable models, the runtime automatically sets:

```json
{
  "thinkingConfig": { "thinkingBudget": 8192 }
}
```

Thought parts arrive with `thought: true` and are emitted as `ThinkingStart/ThinkingDelta/ThinkingEnd` events. The `thinking_signature` is set to `"google_thought"`.

## Tool Calling

Tools are converted to Google's `functionDeclarations` format:

```json
{
  "tools": [{
    "functionDeclarations": [{
      "name": "get_weather",
      "description": "Get weather",
      "parameters": { "type": "object", ... }
    }]
  }]
}
```

Tool calls arrive as `functionCall` parts and are mapped to `ToolCallStart/ToolCallDelta/ToolCallEnd` events. Tool results are sent back as `functionResponse` parts in user messages.

## Stream Event Mapping

| Gemini Chunk | Canonical Event |
|---|---|
| `parts[].text` | `TextStart/TextDelta/TextEnd` |
| `parts[].text` + `thought: true` | `ThinkingStart/ThinkingDelta/ThinkingEnd` |
| `parts[].functionCall` | `ToolCallStart/ToolCallDelta/ToolCallEnd` |
| `usageMetadata` | Accumulated into `Usage` |
| `finishReason` | Mapped to `StopReason` |

## Stop Reason Mapping

- `STOP` -> `StopReason::Stop`
- `MAX_TOKENS` -> `StopReason::Length`
- `SAFETY` / `RECITATION` / `BLOCKLIST` / `PROHIBITED_CONTENT` / `SPII` -> `StopReason::Error`

## Replay

When replaying assistant messages, thinking content is preserved with `thought: true` flag on parts. Tool calls are re-serialized as `functionCall` parts.

## Files to Reference

- `src/models/google.rs` - Model helpers (Gemini 2.5 Pro, Gemini 2.5 Flash)
- `src/providers/google.rs` - Thin provider entry point
- `src/providers/shared/google_like.rs` - Streaming runtime
- `src/providers/env.rs` - Environment variable lookup (`GEMINI_API_KEY`)
- `src/stream/mod.rs` - Top-level dispatch
