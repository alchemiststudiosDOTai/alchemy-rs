---
summary: "Featherless first-class provider notes for the shared OpenAI-compatible runtime"
read_when:
  - You are adding or debugging Featherless support
  - You need the Featherless model helper or environment variable name
  - You want to understand how Featherless fits the unified abstraction
---

# Featherless Provider

Featherless is a **first-class provider identity** in `alchemy_llm` that runs on the crate's shared `OpenAICompletions` runtime.

That means:

- the public abstraction stays the same: `Model<TApi>`, `stream(...)`, and `complete(...)`
- callers can target `KnownProvider::Featherless` directly
- the implementation reuses the shared OpenAI-compatible request/stream path underneath

## Quick Start

```rust
use alchemy_llm::{featherless_model, stream};
use alchemy_llm::types::{AssistantMessageEvent, Context, Message, UserContent, UserMessage};
use futures::StreamExt;

#[tokio::main]
async fn main() -> alchemy_llm::Result<()> {
    let model = featherless_model("moonshotai/Kimi-K2.5");
    let context = Context {
        system_prompt: None,
        messages: vec![Message::User(UserMessage {
            content: UserContent::Text("Hello from Featherless".to_string()),
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

Set `FEATHERLESS_API_KEY` before calling `stream(...)` or `complete(...)`.

## Constructor

Use the generic model helper:

```rust
use alchemy_llm::featherless_model;

let model = featherless_model("moonshotai/Kimi-K2.5");
```

The helper returns `Model<OpenAICompletions>` with:

- provider: `KnownProvider::Featherless`
- base URL: `https://api.featherless.ai/v1/chat/completions`
- default context window: `128_000`
- default max output tokens: `16_384`
- reasoning: `false`
- default input type: text

Because Featherless exposes a large dynamic catalog, the helper accepts the model id directly instead of providing one function per catalog entry.

## Environment and Request Flow

Authentication is resolved through the normal provider environment lookup:

- `KnownProvider::Featherless`
  -> `FEATHERLESS_API_KEY`
  -> `Authorization: Bearer ...` on the shared OpenAI-compatible request path

At the top-level API, Featherless uses the same entry points as other providers:

- `stream(&model, &context, None)`
- `complete(&model, &context, None)`

## Authentication

Set:

```bash
FEATHERLESS_API_KEY=rc_...
```

Top-level `stream(...)` and `complete(...)` resolve that key through the normal provider environment lookup.

## Unified Runtime Behavior

Featherless is routed through the shared OpenAI-compatible runtime rather than a dedicated Featherless-only runtime.

Current compatibility defaults are:

- request path stays on `Api::OpenAICompletions`
- `max_tokens` is used by default for output-token limits
- `stream_options.include_usage` remains enabled
- `store: false` remains enabled
- developer-role system prompts remain enabled
- reasoning-effort remains enabled when the model is marked as reasoning-capable

## Reasoning and Streaming Notes

Featherless stays on the shared OpenAI-compatible streaming implementation, including the common normalization path for text, tool calls, and reasoning blocks.

If a model emits reasoning fields, the runtime currently recognizes the same OpenAI-like signatures already handled by the shared path:

- `reasoning_content`
- `reasoning`
- `reasoning_text`

Those are normalized into the crate's shared thinking content/events without adding a Featherless-only public API.

## Live Validation Notes

The following behaviors were validated against the live Featherless API during implementation:

- chat completions succeeded at `POST /v1/chat/completions`
- `max_tokens` was accepted
- `max_completion_tokens` was also accepted, but the runtime defaults to `max_tokens`
- `store: false` was accepted
- `role: "developer"` messages were accepted
- streaming SSE with `stream_options.include_usage` produced a final usage chunk
- some models emit a `reasoning` field alongside normal assistant `content`
- at least one model returned assistant `content` with leading whitespace; the crate preserves provider output as-is and does not trim it

## Model-Dependent Capabilities

Featherless is a catalog provider, so some capabilities vary by model family.

In particular:

- tool use may be model-dependent
- context length and max completion limits vary by model
- reasoning availability varies by model

If you need exact limits, fetch the Featherless model catalog and override the returned `Model` metadata accordingly. A typical override looks like:

```rust
use alchemy_llm::featherless_model;

let mut model = featherless_model("moonshotai/Kimi-K2.5");
model.context_window = 262_144;
model.max_tokens = 32_768;
model.reasoning = true;
```

## Headers

If you need provider-specific headers, use the existing extension points:

- `Model.headers`
- `OpenAICompletionsOptions.headers`

No Featherless-specific public abstraction is required for this.

## Files to Reference

Implementation and integration points live in:

- `src/models/featherless.rs`
- `src/providers/openai_completions.rs`
- `src/providers/env.rs`
- `src/stream/mod.rs`

## Related Docs

- [architecture.md](./architecture.md)
- [../README.md](../README.md)
