---
summary: "Provider implementation contract for unified thinking, replay fidelity, and shared stream block handling"
read_when:
  - You are adding a new first-class provider implementation
  - You need to understand how reasoning/thinking is normalized in `alchemy_llm`
  - You are wiring same-provider replay or cross-provider transformation behavior
---

# Provider Architecture: Unified Thinking and Replay Contract

This document describes the provider-side contract for reasoning/thinking support in `alchemy_llm`.

The short version:

1. **Normalize provider reasoning into the shared content model**
2. **Preserve same-provider replay fidelity**
3. **Degrade safely for cross-provider transforms**
4. **Use the shared stream block helpers unless a provider has a strong reason not to**

This is the architecture already followed by the OpenAI-like, MiniMax, and z.ai providers.

## Canonical Internal Model

All assistant output is normalized into `types::Content` blocks:

- `Content::Text`
- `Content::Thinking`
- `Content::ToolCall`
- `Content::Image`

Relevant types live in `src/types/content.rs`.

### Thinking and signature fields

The unified reasoning model is:

- `ThinkingContent { thinking, thinking_signature }`
- `ToolCall { id, name, arguments, thought_signature }`

These fields are intentionally provider-agnostic:

- `thinking` is the normalized reasoning text
- `thinking_signature` is provider metadata needed to preserve same-model replay
- `thought_signature` on tool calls is reserved for providers that attach replay-sensitive reasoning metadata to tool-call blocks

## Shared Pipeline

Most providers should follow this path:

```text
provider stream payload
  -> provider-specific field extraction
  -> handle_reasoning_delta / handle_text_delta / handle_tool_calls
  -> Content::{Thinking, Text, ToolCall}
  -> transform_messages(...) for cross-provider reuse
  -> provider-specific request serializer for replay
```

The key shared files are:

- `src/providers/shared/stream_blocks.rs`
- `src/providers/shared/openai_like_messages.rs`
- `src/transform.rs`

## Stream Normalization Contract

### Reasoning must become `Content::Thinking`

If a provider emits reasoning in any form, normalize it into `Content::Thinking`.

Examples from current providers:

- OpenAI-like providers map `reasoning_content`, `reasoning`, or `reasoning_text`
- MiniMax maps explicit reasoning fields or parses `<think>...</think>` fallback text
- z.ai maps its reasoning fields into the same shared helper path

The preferred entry point is `handle_reasoning_delta(...)` in `src/providers/shared/stream_blocks.rs`.

That helper is responsible for:

- starting/appending a shared thinking block
- emitting unified thinking events
- finalizing `Content::Thinking`
- storing `thinking_signature`

### Text must become `Content::Text`

Provider-visible answer text should go through `handle_text_delta(...)` so that event ordering and block boundaries stay consistent across providers.

### Tool calls must become `Content::ToolCall`

Tool-call deltas should use the shared tool-call helpers where possible.

If a provider cannot use the incremental OpenAI-like tool-call path, its custom implementation must still end in the same internal shape:

- `Content::ToolCall`
- stable `ToolCall.id`
- parsed `ToolCall.arguments`
- `thought_signature` preserved when required by the provider

## Replay Contract

### Same provider + same model should preserve replay fidelity

If the next turn targets the same provider/model, the provider should preserve all metadata required for valid replay.

This includes, when applicable:

- thinking blocks
- provider-native reasoning signatures
- tool-call thought signatures
- provider-required block ordering

The transform layer already expresses this rule.

From `src/transform.rs`:

- same model + signature -> keep for replay
- different provider/model -> degrade to portable representation

### Cross-provider transform may degrade reasoning

When transforming to a different provider/model, the system may convert thinking to plain text and strip provider-specific signatures.

That behavior is intentional and is documented in `docs/utils/transform.md`.

The replay-preservation burden therefore falls on the provider implementation for **same-provider replay**, not on the cross-provider transform path.

## Provider Serializer Contract

When converting an `AssistantMessage` back into request history for a provider, the serializer must respect the provider's replay semantics.

Examples from existing providers:

- MiniMax replays thinking as `<think>...</think>` blocks
- z.ai can emit thinking via `reasoning_content`
- OpenAI-like serializers may omit or flatten thinking depending on compatibility settings

This logic belongs in the provider-specific request builder or shared message serializer.

### Important distinction: normalized signatures vs opaque provider signatures

Some providers only need lightweight internal provenance markers such as:

- `reasoning_content`
- `reasoning_text`
- `think_tag`

Those are fine as `thinking_signature` values when they are sufficient for replay.

Other providers may return **opaque provider-issued replay tokens** that must be passed back exactly as received. In those cases:

- the raw provider token must be captured
- stored in the canonical content model
- serialized back in the same block position the provider expects

If a provider has hard replay requirements for thought signatures, it is not enough to preserve only the visible reasoning text.

## Provider Checklist

When adding or reviewing a provider, verify all of the following:

### Inbound stream handling

- [ ] Reasoning fields are mapped to `Content::Thinking`
- [ ] Text fields are mapped to `Content::Text`
- [ ] Tool calls are mapped to `Content::ToolCall`
- [ ] Provider-specific replay signatures are captured when present
- [ ] Unified event ordering is preserved

### Outbound replay handling

- [ ] Same-provider replay includes the provider's required thinking representation
- [ ] Same-provider replay preserves provider-issued signatures when required
- [ ] Tool-call replay preserves provider-required IDs and signature metadata
- [ ] Cross-provider replay intentionally degrades only through `transform_messages(...)`

### Tests

- [ ] Stream chunks map to unified thinking/text/tool-call content correctly
- [ ] Same-provider replay round trip works for a multi-turn conversation
- [ ] Tool-call round trip works across at least one tool execution cycle
- [ ] Provider-specific reasoning signatures are preserved when required
- [ ] Cross-provider transformation strips only the signatures it is supposed to strip

## Current Implementations to Reference

### OpenAI-like

Reference files:

- `src/providers/openai_completions.rs`
- `src/providers/shared/openai_like_messages.rs`
- `src/providers/shared/stream_blocks.rs`

Pattern:

- provider field extraction stays thin
- shared helpers own block assembly
- serializer behavior is controlled by `OpenAiLikeMessageOptions`

### MiniMax

Reference files:

- `src/providers/minimax.rs`
- `docs/providers/minimax.md`

Pattern:

- explicit reasoning fields map into `handle_reasoning_delta(...)`
- `<think>` fallback is parsed and still normalized into the same internal model
- replay wraps thinking back into `<think>...</think>` blocks

### z.ai

Reference files:

- `src/providers/zai.rs`
- `docs/providers/zai.md`

Pattern:

- reasoning fields normalize into unified thinking blocks
- replay emits provider-specific reasoning fields from the same internal content

## Guidance for Non-OpenAI-like Providers

A provider does **not** need to be OpenAI-compatible to follow this contract.

If a provider has a custom protocol, it may still:

1. parse its native response shape itself
2. map reasoning/text/tool calls into the canonical `Content` blocks
3. preserve provider-native replay signatures in `thinking_signature` / `thought_signature`
4. serialize assistant history back into the provider's native request format

What matters is the shared internal shape and replay fidelity, not whether the wire protocol looks OpenAI-like.

## Anti-Patterns

Avoid these when implementing a provider:

- Treating provider reasoning as text-only when the provider requires replay signatures
- Emitting `Content::Thinking` during streaming but dropping it during assistant-history replay
- Storing placeholder signatures when the provider returned an opaque signature that must be replayed exactly
- Implementing same-provider replay semantics only in tests or examples rather than in the serializer path

## Related Docs

- `docs/utils/transform.md`
- `docs/providers/minimax.md`
- `docs/providers/zai.md`
- `docs/api/lib.md`
