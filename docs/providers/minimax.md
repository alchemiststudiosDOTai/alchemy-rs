---
summary: "First-class MiniMax provider guide: models, configuration, streaming behavior, and smoke testing"
read_when:
  - You want to use MiniMax with `alchemy_llm`
  - You need to understand `reasoning_split` and `<think>` fallback behavior
  - You need model constructors and environment variable setup
---

# MiniMax Provider Guide

## What was added in the latest merge to `main`

MiniMax is now a first-class provider path (not just OpenAI-compat detection):

- `Api::MinimaxCompletions` added to the API enum
- Dedicated implementation in `src/providers/minimax.rs`
- Built-in model constructors in `src/models/minimax.rs`
- Shared OpenAI-like runtime/message utilities reused for consistency
- Streaming reasoning support from both:
  - explicit reasoning fields (`reasoning_details`, `reasoning_content`, etc.)
  - inline `<think>...</think>` fallback parsing

## Environment Variables

| Provider | Env var |
|---|---|
| `KnownProvider::Minimax` | `MINIMAX_API_KEY` |
| `KnownProvider::MinimaxCn` | `MINIMAX_CN_API_KEY` |

`stream()` resolves API keys from options first, then falls back to environment variables.

## Built-in MiniMax Models

All built-in MiniMax models:

- use `Api::MinimaxCompletions`
- default `reasoning = true`
- use `InputType::Text`
- default to `context_window = 204_800`, `max_tokens = 16_384`

### Global endpoint (`https://api.minimax.io/v1/chat/completions`)

- `minimax_m2_5()`
- `minimax_m2_5_highspeed()`
- `minimax_m2_1()`
- `minimax_m2_1_highspeed()`
- `minimax_m2()`

### CN endpoint (`https://api.minimax.chat/v1/chat/completions`)

- `minimax_cn_m2_5()`
- `minimax_cn_m2_5_highspeed()`
- `minimax_cn_m2_1()`
- `minimax_cn_m2_1_highspeed()`
- `minimax_cn_m2()`

## Request Semantics

MiniMax request building in `src/providers/minimax.rs` behaves as follows:

- Always sends `stream: true`
- Always sends `stream_options.include_usage: true`
- Sends `reasoning_split: true` when `model.reasoning == true`
- Maps `OpenAICompletionsOptions.max_tokens` -> `max_tokens`
- Clamps `temperature` to `(0.0, 1.0]` using:
  - lower bound: `f64::MIN_POSITIVE`
  - upper bound: `1.0`
- For tools, reuses shared OpenAI-like tool conversion

## Streaming Reasoning Semantics

Reasoning emission order for each chunk:

1. `reasoning_details[*].text`
2. `reasoning_content`
3. `reasoning`
4. `reasoning_text`
5. Fallback parse from `content` using `<think>...</think>`

If explicit reasoning is found in a delta, `content` is treated as normal text.  
If explicit reasoning is not found, the `<think>` parser extracts reasoning/text fragments from `content`.

### Thinking signatures

`thinking_signature` on `Content::Thinking` is set to one of:

- `reasoning_details`
- `reasoning_content`
- `reasoning`
- `reasoning_text`
- `think_tag` (fallback parser path)

## Replay Behavior

When assistant history is converted back to MiniMax request messages, thinking blocks are wrapped as:

```text
<think>...</think>
```

This preserves reasoning blocks for multi-turn replay on MiniMax.

## Quick Start

```rust
use alchemy_llm::{minimax_m2_5, stream, OpenAICompletionsOptions};
use alchemy_llm::types::{AssistantMessageEvent, Context, Message, UserContent, UserMessage};
use futures::StreamExt;

#[tokio::main]
async fn main() -> alchemy_llm::Result<()> {
    let model = minimax_m2_5();

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
            api_key: std::env::var("MINIMAX_API_KEY").ok(),
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

- `smokescripts/run_minimax_reasoning_split.sh`
- `smokescripts/run_minimax_inline_think.sh`
- `smokescripts/run_minimax_usage_chunk.sh`
- `smokescripts/run_all_minimax.sh`

See: [`smokescripts/README.md`](../../smokescripts/README.md)
