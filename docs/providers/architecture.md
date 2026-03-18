---
summary: "Source-of-truth contract for provider runtimes, canonical content/event shapes, replay fidelity, and provider docs"
read_when:
  - You are adding a new first-class provider implementation
  - You are writing or reshaping a provider doc
  - You need the exact canonical message, content, or stream-event contract
  - You are wiring same-provider replay or cross-provider transformation behavior
---

# Provider Architecture Contract

This document is the source of truth for provider implementations in `alchemy_llm`.

It defines four things:

1. the canonical data shapes every provider must map into
2. the stream event contract exposed by `stream(...)`
3. the replay contract for same-provider and cross-provider reuse
4. the structure every provider doc should follow

If a provider-specific doc disagrees with this document, this document wins.

## The Short Version

Every provider implementation must do all of the following:

1. accept the shared `Context` input model
2. normalize provider output into canonical `Content` blocks
3. emit canonical `AssistantMessageEvent` sequences
4. preserve provider-native replay fidelity when the next turn targets the same provider/model
5. degrade only through `transform_messages(...)` when crossing providers
6. prove the mapping with focused serialization and streaming tests

## Canonical Input Contract

Top-level generation always starts from:

```rust
Context {
    system_prompt: Option<String>,
    messages: Vec<Message>,
    tools: Option<Vec<Tool>>,
}
```

The message history is provider-agnostic:

```rust
Message::User(UserMessage)
Message::Assistant(AssistantMessage)
Message::ToolResult(ToolResultMessage)
```

### User message shapes

User messages support two content forms:

```rust
UserContent::Text(String)
UserContent::Multi(Vec<UserContentBlock>)
```

The current user content blocks are:

```rust
UserContentBlock::Text(TextContent)
UserContentBlock::Image(ImageContent)
```

Provider serializers may drop image inputs when the target model does not declare `InputType::Image`.

### Tool definitions

Tool definitions come from `Context.tools` and must be serialized into the provider's native tool/function schema without changing their semantic meaning:

- tool name
- tool description
- JSON Schema parameters

## Canonical Output Contract

Every provider must normalize the final assistant result into:

```rust
AssistantMessage {
    content: Vec<Content>,
    api: Api,
    provider: Provider,
    model: String,
    usage: Usage,
    stop_reason: StopReason,
    error_message: Option<String>,
    timestamp: i64,
}
```

The canonical assistant block model is:

```rust
Content::Text { inner: TextContent }
Content::Thinking { inner: ThinkingContent }
Content::Image { inner: ImageContent }
Content::ToolCall { inner: ToolCall }
```

### Block semantics

`Content::Text`

- visible assistant answer text
- provider text should land here unless it is explicitly reasoning/thinking

`Content::Thinking`

- normalized provider reasoning/thinking
- use this whenever the provider emits internal reasoning in any supported form
- preserve provider metadata in `thinking_signature` when it matters for replay

`Content::Image`

- assistant image output in canonical binary form
- not all streaming runtimes expose image-specific delta events today

`Content::ToolCall`

- normalized tool/function call
- `id` must be stable enough for tool-result replay
- `arguments` must end as parsed JSON
- `thought_signature` is reserved for providers that attach replay-sensitive metadata to tool-call blocks

### Signature fields

The shared content model has three provider-metadata escape hatches:

- `TextContent.text_signature`
- `ThinkingContent.thinking_signature`
- `ToolCall.thought_signature`

Use them only when the provider requires extra metadata for valid same-provider replay.

If the provider returns an opaque replay token, preserve the raw token. Do not replace it with a synthetic marker.

## Canonical Stream Event Contract

The public streaming surface is `AssistantMessageEventStream`, which yields `AssistantMessageEvent`.

The canonical event families are:

```rust
Start
TextStart / TextDelta / TextEnd
ThinkingStart / ThinkingDelta / ThinkingEnd
ToolCallStart / ToolCallDelta / ToolCallEnd
Done
Error
```

### Event sequencing rules

Every successful stream must follow this shape:

1. one `Start`
2. zero or more finalized block lifecycles
3. one `Done`

Every failed stream must follow this shape:

1. zero or one `Start`
2. optional partial block activity
3. one `Error`

For each block type, events must be ordered as:

1. `*Start`
2. zero or more `*Delta` events
3. `*End`

Block boundaries matter. If the provider switches from thinking to text, or from text to tool call, the current block must be finalized before the next block starts.

### Partial snapshot semantics

`partial` snapshots in delta events are in-progress structural snapshots, not necessarily fully materialized block payloads.

In the shared helpers today:

- text/thinking/tool-call blocks are opened before their final payload is written back into `output.content`
- the authoritative finalized block content arrives at `*End` and is also present in `Done`

Provider docs should not promise stronger semantics unless that runtime actually provides them.

### Stop reason normalization

Providers must map native termination reasons into the canonical enum:

- `StopReason::Stop`
- `StopReason::Length`
- `StopReason::ToolUse`
- `StopReason::Error`
- `StopReason::Aborted`

Provider-native names differ, but the public stream/result surface must not.

### Usage normalization

Usage data must be accumulated into canonical `Usage` fields:

- `input`
- `output`
- `cache_read`
- `cache_write`
- `total_tokens`
- `cost`

If a provider reports only part of that data, fill what is known and leave the rest at zero/default.

## Replay Contract

Replay is where most provider integrations go wrong. The rule is simple:

### Same provider + same model

Preserve everything required for valid replay:

- thinking blocks
- provider-issued signatures
- tool-call ids
- provider-required block ordering
- provider-specific assistant serialization shape

If a provider requires reasoning signatures for replay, preserving only visible reasoning text is not enough.

### Cross-provider transform

Cross-provider reuse is allowed to degrade.

That degradation must happen through `transform_messages(...)`, not through ad hoc logic in provider runtimes. Typical degradation includes:

- dropping provider-specific signatures
- converting thinking to portable text when needed
- normalizing tool-call ids for the target provider

## Provider Families

Not every provider needs a fully custom runtime. Providers should be grouped by wire-family whenever possible.

### OpenAI-like family

Use the shared OpenAI-like helpers when the provider accepts:

- OpenAI-style `messages`
- function/tool definitions in OpenAI-style schema
- SSE chunks that look like choice/delta streams

Reference files:

- `src/providers/shared/openai_like_messages.rs`
- `src/providers/shared/stream_blocks.rs`
- `src/providers/shared/openai_like_runtime.rs`

### Anthropic-style family

Use an Anthropic-style path when the provider speaks in:

- `/messages`-style request bodies
- typed content blocks
- SSE `event:` + `data:` frames such as `message_start` and `content_block_delta`

Even when the wire format differs from OpenAI-like providers, the normalized crate output must still use the same canonical `Content` and `AssistantMessageEvent` model.

### Custom family

If the provider wire protocol is unique, a custom runtime is fine. The custom code still must end in the same canonical shapes described in this doc.

## Canonical Serialization Shapes

Provider request serializers differ, but the source material for assistant replay is always the same canonical assistant block list.

A representative assistant content sequence is:

```rust
vec![
    Content::thinking("first reason"),
    Content::text("final answer"),
    Content::tool_call("call_1", "get_weather", json!({"city": "Tokyo"})),
]
```

One existing ground-truth serialization test is:

- `src/providers/shared/openai_like_messages.rs`
  `convert_messages_zai_mode_matches_canonical_assistant_replay_shape`

That test is useful because it exercises the three most important replay block types in one assistant message:

- thinking
- text
- tool call

Every provider family should have an equivalent shape test for its own serializer.

## Test Contract

At minimum, a provider implementation should prove all of the following:

### Serialization tests

- user text is serialized correctly
- user images are either serialized correctly or intentionally dropped for text-only models
- assistant text/thinking/tool-call replay shape matches the provider's native request format
- tool-result messages serialize correctly
- provider-specific signatures survive same-provider replay

### Streaming tests

- text deltas become `Content::Text` and text events
- reasoning deltas become `Content::Thinking` and thinking events
- tool-call deltas assemble into parsed JSON arguments
- usage accounting updates canonical `Usage`
- native stop reasons map to canonical `StopReason`
- mixed/interleaved block transitions finalize the previous block before starting the next one

### End-to-end dispatch tests

- `stream(...)` routes the model to the correct runtime
- `complete(...)` returns the final canonical `AssistantMessage`

### The "all expected data types" test

For each provider family, keep one focused test case that exercises the full expected assistant surface for that family in one conversation shape:

- thinking
- text
- tool call
- tool result replay
- usage
- stop reason

If the provider supports images in a meaningful way, add image coverage explicitly rather than assuming the generic case already proves it.

## Provider Doc Template

Every provider-specific doc under `docs/providers/` should follow the same outline:

1. what family/runtime the provider uses
2. public model helpers
3. authentication and environment variables
4. request mapping from `Context` into provider-native payloads
5. canonical output mapping into `Content` and `AssistantMessageEvent`
6. replay and signature behavior
7. stop-reason and usage notes
8. tests and implementation files to reference

Provider docs should describe actual implemented behavior, not aspirational API behavior.

## Implementation Checklist

- [ ] Provider accepts canonical `Context`
- [ ] User text maps correctly
- [ ] User image handling is explicit
- [ ] Assistant text maps to `Content::Text`
- [ ] Assistant reasoning maps to `Content::Thinking`
- [ ] Assistant tool calls map to `Content::ToolCall`
- [ ] Provider replay signatures are preserved when required
- [ ] Stream event ordering matches the canonical lifecycle
- [ ] Stop reasons map into canonical `StopReason`
- [ ] Usage maps into canonical `Usage`
- [ ] Same-provider replay is valid
- [ ] Cross-provider degradation happens only via `transform_messages(...)`
- [ ] Provider doc follows the shared template above

## Files to Keep Open While Implementing

- `src/types/content.rs`
- `src/types/event.rs`
- `src/types/message.rs`
- `src/transform.rs`
- `src/providers/shared/stream_blocks.rs`
- `src/providers/shared/openai_like_messages.rs`
- `src/providers/shared/openai_like_runtime.rs`

## Related Docs

- `docs/README.md`
- `docs/providers/anthropic.md`
- `docs/providers/featherless.md`
