---
summary: "Public API surface exported from src/lib.rs"
status: current
last_verified: 2026-03-09
---

# Public API

`src/lib.rs` re-exports the main crate surface.

## Modules

- `error`
- `models`
- `providers`
- `stream`
- `transform`
- `types`
- `utils`

## Top-Level Re-Exports

- `Error`, `Result`
- Built-in MiniMax and z.ai model constructors
- `stream_minimax_completions`, `stream_openai_completions`, `OpenAICompletionsOptions`
- `complete`, `stream`, `AssistantMessageEventStream`
- `transform_messages`, `transform_messages_simple`, `TargetModel`
- Utility helpers such as `parse_streaming_json`, `sanitize_for_api`, `validate_tool_call`, and `ThinkTagParser`

## Notes

- `providers::stream_zai_completions` is exported from `src/providers/mod.rs`, even though it is not re-exported at the crate root.
- `types::*` is the canonical shared contract used across providers and the transform layer.
