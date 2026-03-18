---
summary: "Anthropic first-class provider notes for the native Messages API runtime"
read_when:
  - You are adding or debugging Anthropic support
  - You need the Anthropic model helpers or environment variable names
  - You want to understand Anthropic thinking, tool use, and replay behavior in `alchemy_llm`
---

# Anthropic Provider

Anthropic is a **first-class provider** in `alchemy_llm` with a dedicated native runtime for the Anthropic Messages API.

That means:

- callers target `Model<AnthropicMessages>` directly
- top-level usage stays the same: `stream(...)` and `complete(...)`
- the wire protocol is Anthropic-native rather than routed through the shared OpenAI-compatible path

## Quick Start

```rust
use alchemy_llm::{claude_sonnet_4_6, stream};
use alchemy_llm::types::{AssistantMessageEvent, Context, Message, UserContent, UserMessage};
use futures::StreamExt;

#[tokio::main]
async fn main() -> alchemy_llm::Result<()> {
    let model = claude_sonnet_4_6();
    let context = Context {
        system_prompt: Some("You are concise".to_string()),
        messages: vec![Message::User(UserMessage {
            content: UserContent::Text("Explain Rust lifetimes briefly.".to_string()),
            timestamp: 0,
        })],
        tools: None,
    };

    let mut stream = stream(&model, &context, None)?;

    while let Some(event) = stream.next().await {
        if let AssistantMessageEvent::TextDelta { delta, .. } = event {
            print!("{}", delta);
        }
    }

    Ok(())
}
```

Set Anthropic credentials before calling `stream(...)` or `complete(...)`.

## Model Helpers

The crate exports three Anthropic helpers:

- `claude_opus_4_6()`
- `claude_sonnet_4_6()`
- `claude_haiku_4_5()`

All three return `Model<AnthropicMessages>` with:

- provider: `KnownProvider::Anthropic`
- base URL: `https://api.anthropic.com`
- reasoning: `true`
- input types: text and image

The current defaults are:

- `claude_opus_4_6()`: context window `200_000`, max output tokens `128_000`
- `claude_sonnet_4_6()`: context window `200_000`, max output tokens `64_000`
- `claude_haiku_4_5()`: context window `200_000`, max output tokens `64_000`

## Authentication

Credential lookup follows the normal provider environment path:

- `ANTHROPIC_OAUTH_TOKEN`
- `ANTHROPIC_API_KEY`

If both are set, `ANTHROPIC_OAUTH_TOKEN` wins.

You can also pass a key explicitly through `OpenAICompletionsOptions.api_key` when calling `stream(...)` or `complete(...)`.

## Runtime Family

Anthropic is dispatched through `Api::AnthropicMessages` in the top-level stream layer.

The provider runtime sends requests to:

- base URL from the model
- endpoint: `/v1/messages`

The implementation adds Anthropic-specific headers, including:

- `anthropic-version: 2023-06-01`
- `anthropic-beta: interleaved-thinking-2025-05-14`

Unlike Featherless and other OpenAI-compatible providers, Anthropic uses a dedicated SSE parser that reads `event:` and `data:` pairs from the Messages API stream.

## Request Mapping

The request builder translates `alchemy_llm` context into Anthropic-native request fields:

- `Context.system_prompt` -> top-level `system`
- user messages -> Anthropic `role: "user"` content
- assistant messages -> Anthropic assistant content blocks
- tool results -> Anthropic `tool_result` user blocks
- `Context.tools` -> Anthropic `tools` with `input_schema`
- `OpenAICompletionsOptions.temperature` -> `temperature`
- `OpenAICompletionsOptions.max_tokens` -> `max_tokens`

User images are sent only when the model declares `InputType::Image`.

## Canonical Output Mapping

Anthropic responses are normalized into the same canonical crate types used by other providers:

- visible answer text -> `Content::Text`
- reasoning/thinking -> `Content::Thinking`
- tool use -> `Content::ToolCall`
- stream lifecycle -> `AssistantMessageEvent`

- `thinking_delta` stream events become `Content::Thinking`
- Anthropic thinking signatures are preserved in `thinking_signature`
- answer text still becomes `Content::Text`
- event ordering uses the shared stream block helpers so thinking and text behave like other providers at the crate boundary

When the selected model is marked as reasoning-capable, the request enables Anthropic thinking with:

- `thinking.type = "enabled"`
- `thinking.budget_tokens = max_tokens - 100`

The runtime only sends that field when the remaining budget is at least `1024`.

## Replay and Signatures

Anthropic tool use is mapped into the crate's canonical tool-call shape:

- tool-use block start -> `Content::ToolCall`
- `input_json_delta` fragments append incremental tool arguments
- final tool-call arguments are assembled through the shared block finalization path

Assistant replay back into Anthropic request history preserves:

- text blocks
- thinking blocks
- Anthropic thinking signatures
- tool-use blocks with stable ids and arguments

That matches the provider architecture rule that same-provider replay should preserve reasoning fidelity when the provider requires it.

## Stop Reasons and Usage

The runtime currently maps Anthropic stream events as follows:

- `text_delta` -> text events and `Content::Text`
- `thinking_delta` -> thinking events and `Content::Thinking`
- `input_json_delta` -> tool-call delta events
- `message_start.usage` -> input/cache usage initialization
- `message_delta.usage` -> output token updates

Stop reasons are normalized to the crate model:

- `end_turn` and `stop_sequence` -> `StopReason::Stop`
- `max_tokens` -> `StopReason::Length`
- `tool_use` -> `StopReason::ToolUse`

## Options and Headers

Anthropic currently reuses `OpenAICompletionsOptions` as the common top-level options type. In practice, the Anthropic runtime uses:

- `api_key`
- `headers`
- `temperature`
- `max_tokens`

Provider-specific headers can be attached through:

- `Model.headers`
- `OpenAICompletionsOptions.headers`

## Tests and Files

The Anthropic implementation has focused tests around:

- model helper metadata
- request parameter construction
- thinking enablement
- tool schema serialization
- text, thinking, and tool-call stream handling
- usage accounting
- stop-reason mapping
- assistant replay preserving thinking signatures
- top-level dispatch through `stream(...)`

Implementation and integration points live in:

- `src/models/anthropic.rs`
- `src/providers/anthropic.rs`
- `src/providers/env.rs`
- `src/providers/shared/openai_like_runtime.rs`
- `src/stream/mod.rs`

## Related Docs

- [architecture.md](./architecture.md)
- [featherless.md](./featherless.md)
- [../README.md](../README.md)
