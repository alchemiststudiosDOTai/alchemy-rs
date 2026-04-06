---
summary: "Kimi first-class provider notes for the Anthropic-typed public surface and cache-capable chat completions runtime"
read_when:
  - You are adding or debugging Kimi support
  - You need the Kimi model helper or environment variable name
  - You want to understand how Kimi fits the unified abstraction
---

# Kimi Provider

Kimi is a **first-class provider identity** in `alchemy_llm` with an Anthropic-typed public surface and a Kimi-specific chat completions runtime underneath.

That means:

- the public abstraction stays the same: `Model<TApi>`, `stream(...)`, and `complete(...)`
- callers can target `KnownProvider::Kimi` directly
- the public helper remains `Model<AnthropicMessages>`
- the runtime uses the shared OpenAI-like stream helpers with Kimi-specific cache request shaping

## Quick Start

```rust
use alchemy_llm::{kimi_k2_5, stream};
use alchemy_llm::types::{AssistantMessageEvent, Context, Message, UserContent, UserMessage};
use futures::StreamExt;

#[tokio::main]
async fn main() -> alchemy_llm::Result<()> {
    let model = kimi_k2_5();
    let context = Context {
        system_prompt: None,
        messages: vec![Message::User(UserMessage {
            content: UserContent::Text("Hello from Kimi".to_string()),
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

Set `KIMI_API_KEY` before calling `stream(...)` or `complete(...)`.

## Constructor

Use the curated model helper:

```rust
use alchemy_llm::kimi_k2_5;

let model = kimi_k2_5();
```

The helper returns `Model<AnthropicMessages>` with:

- provider: `KnownProvider::Kimi`
- base URL: `https://api.kimi.com/coding`
- model id: `kimi-coding`
- default context window: `128_000`
- default max output tokens: `16_384`
- reasoning: `true`
- default input type: text

## Environment and Request Flow

Authentication is resolved through the normal provider environment lookup:

- `KnownProvider::Kimi`
  -> `KIMI_API_KEY`
  -> `Authorization: Bearer ...` on the Kimi chat completions transport

At the top-level API, Kimi uses the same entry points as other providers:

- `stream(&model, &context, None)`
- `complete(&model, &context, None)`

The public cache options field is:

```rust
OpenAICompletionsOptions {
    cache: Some(CacheOptions::new("stable-prefix-key")),
    ..OpenAICompletionsOptions::default()
}
```

The Kimi runtime maps that provider-neutral cache options shape onto the transport as:

- request endpoint: `https://api.kimi.com/coding/v1/chat/completions`
- request header: `User-Agent: KimiCLI/1.29.0`
- request body field: `prompt_cache_key`
- transport model id: `kimi-for-coding`

That provider translation is implemented in `src/cache/kimi_cache_capability.rs`.

The public output model identity remains `kimi-coding`, so callers keep the same crate-level replay and event contract.

## Live Validation Notes

The following behaviors were validated against the live Kimi Coding API during implementation:

- `POST /coding/v1/chat/completions` returned `403` without the Kimi CLI user agent
- the same request succeeded once `User-Agent: KimiCLI/1.29.0` was present
- the cache-capable request body accepted `prompt_cache_key`
- the transport accepted `kimi-for-coding` while the crate continued to expose `kimi-coding`
- streamed deltas exposed OpenAI-like text/reasoning/tool-call chunk shapes that the runtime normalizes back into the canonical Anthropic-typed event contract

## Files to Reference

Implementation and integration points live in:

- `src/models/kimi.rs`
- `src/providers/kimi.rs`
- `src/cache/kimi_cache_capability.rs`
- `src/cache/request.rs`
- `src/providers/shared/openai_like_runtime.rs`
- `src/providers/shared/openai_like_messages.rs`
- `src/providers/env.rs`
- `src/stream/mod.rs`

## Related Docs

- [architecture.md](./architecture.md)
- [../README.md](../README.md)
