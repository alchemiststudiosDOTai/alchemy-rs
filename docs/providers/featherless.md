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

## Constructor

Use the generic model helper:

```rust
use alchemy_llm::featherless_model;

let model = featherless_model("moonshotai/Kimi-K2.5");
```

The helper returns `Model<OpenAICompletions>` with:

- provider: `KnownProvider::Featherless`
- base URL: `https://api.featherless.ai/v1/chat/completions`
- default input type: text

Because Featherless exposes a large dynamic catalog, the helper accepts the model id directly instead of providing one function per catalog entry.

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

If you need exact limits, fetch the Featherless model catalog and override the returned `Model` metadata accordingly.

## Headers

If you need provider-specific headers, use the existing extension points:

- `Model.headers`
- `OpenAICompletionsOptions.headers`

No Featherless-specific public abstraction is required for this.

## Related Docs

- [architecture.md](./architecture.md)
- [../README.md](../README.md)
