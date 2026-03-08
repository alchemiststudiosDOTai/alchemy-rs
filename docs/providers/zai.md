---
summary: "First-class z.ai GLM provider guide: models, config, streaming behavior, and tool-call support"
read_when:
  - You want to use z.ai GLM models with `alchemy_llm`
  - You need z.ai-specific options (`thinking`, `tool_stream`, `response_format`, etc.)
  - You want to verify unified thinking/tool-call event behavior
---

# z.ai Provider Guide

## What was added in PR #37

z.ai is now a first-class provider path in Rust:

- `Api::ZaiCompletions`
- Dedicated provider implementation: `src/providers/zai.rs`
- Built-in GLM model constructors: `src/models/zai.rs`
- New examples:
  - `examples/zai_glm_simple_chat.rs`
  - `examples/zai_glm_tool_call_smoke.rs`

## Environment Variable

| Provider | Env var |
|---|---|
| `KnownProvider::Zai` | `ZAI_API_KEY` |

`stream()` resolves API keys from options first, then falls back to environment variables.

## Built-in z.ai Models

All built-in GLM constructors:

- use `Api::ZaiCompletions`
- use provider `KnownProvider::Zai`
- default `reasoning = true`
- default `InputType::Text`
- use base URL: `https://api.z.ai/api/paas/v4/chat/completions`

### 200K context / 128K max output

- `glm_5()`
- `glm_4_7()`
- `glm_4_7_flash()`
- `glm_4_7_flashx()`
- `glm_4_6()`

### 128K context / 96K max output

- `glm_4_5()`
- `glm_4_5_air()`
- `glm_4_5_x()`
- `glm_4_5_airx()`
- `glm_4_5_flash()`

### 128K context / 16K max output

- `glm_4_32b_0414_128k()`

## Request Semantics

z.ai request building (`src/providers/zai.rs`) does the following:

- Always sends `stream: true`
- Always sends `stream_options.include_usage: true`
- Uses OpenAI-like message conversion with z.ai replay shape:
  - assistant content as plain string
  - assistant thinking emitted via `reasoning_content` in message payload
- `max_tokens` precedence:
  1. `options.zai.max_tokens`
  2. `options.max_tokens`

### Forwarded z.ai option fields

From `OpenAICompletionsOptions.zai`:

- `do_sample`
- `top_p`
- `stop`
- `tool_stream`
- `request_id`
- `user_id`
- `response_format`
- `thinking`

If `model.reasoning == true` and no explicit `thinking` option is provided, Alchemy sets:

```json
{"thinking":{"type":"enabled"}}
```

## Streaming Semantics (Unified Output Layer)

For each stream chunk, the z.ai provider maps to the shared event contract:

- `delta.content` -> text events
- reasoning fields -> thinking events (priority order):
  1. `reasoning_content`
  2. `reasoning`
  3. `reasoning_text`
- `delta.tool_calls` -> tool-call start/args/end events

Stop reason mapping:

- `"sensitive"` -> `StopReason::Error`
- `"network_error"` -> `StopReason::Error`
- other values use shared OpenAI-like stop mapping (`stop`, `length`, `tool_calls`, etc.)

## Quick Start

```rust
use alchemy_llm::{glm_4_7, stream, OpenAICompletionsOptions};
use alchemy_llm::types::{
    AssistantMessageEvent, Context, Message, UserContent, UserMessage, ZaiChatCompletionsOptions,
};
use futures::StreamExt;

#[tokio::main]
async fn main() -> alchemy_llm::Result<()> {
    let model = glm_4_7();

    let context = Context {
        system_prompt: Some("You are concise.".to_string()),
        messages: vec![Message::User(UserMessage {
            content: UserContent::Text("Explain ownership in two sentences.".to_string()),
            timestamp: 0,
        })],
        tools: None,
    };

    let options = Some(OpenAICompletionsOptions {
        api_key: std::env::var("ZAI_API_KEY").ok(),
        max_tokens: Some(512),
        zai: Some(ZaiChatCompletionsOptions::default()),
        ..OpenAICompletionsOptions::default()
    });

    let mut events = stream(&model, &context, options)?;

    while let Some(event) = events.next().await {
        match event {
            AssistantMessageEvent::ThinkingDelta { delta, .. } => print!("{delta}"),
            AssistantMessageEvent::TextDelta { delta, .. } => print!("{delta}"),
            AssistantMessageEvent::Done { message, .. } => {
                println!("\nstop_reason: {:?}", message.stop_reason);
                break;
            }
            _ => {}
        }
    }

    Ok(())
}
```

## Run Live Examples

```bash
cargo run --example zai_glm_simple_chat
cargo run --example zai_glm_tool_call_smoke
```
