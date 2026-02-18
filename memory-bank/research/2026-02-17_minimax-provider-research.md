---
summary: "Research for native MiniMax stream provider matching alchemy's canonical output contract"
read_when:
  - Implementing MiniMax provider in alchemy-rs
  - Adding a new OpenAI-compatible provider
  - Understanding alchemy's streaming event schema
  - Understanding alchemy's assistant message / usage contract
date: 2026-02-17
owner: agent
phase: Research
git_commit: ef5bb6e2daa15089b94c555a7a4c240d0038f561
tags:
  - minimax
  - provider
  - streaming
  - event-schema
  - usage
  - cost
---

# Research -- Native MiniMax Stream Provider

**Date:** 2026-02-17
**Owner:** agent
**Phase:** Research

## Goal

Document everything needed to implement a native MiniMax provider that emits the exact same canonical event stream and final `AssistantMessage` contract that alchemy already uses. Covers: event schema, final message schema, usage keys, MiniMax API specifics, and provider registration steps.

---

## 1. MiniMax API Surface

MiniMax exposes **two** compatible APIs:

### 1a. OpenAI-Compatible API

| Property | Value |
|----------|-------|
| Base URL | `https://api.minimax.io/v1` |
| Auth header | `Authorization: Bearer {API_KEY}` |
| Env var | `MINIMAX_API_KEY` (custom) or reuse `OPENAI_API_KEY` with base_url override |
| Stream format | Standard OpenAI SSE (`data: {json}\n\n`, terminated by `data: [DONE]`) |

**Supported models:** MiniMax-M2.5, MiniMax-M2.5-highspeed, MiniMax-M2.1, MiniMax-M2.1-highspeed, MiniMax-M2 (all 204,800 context)

**MiniMax-specific behaviors:**
- Temperature range: (0.0, 1.0], 1.0 recommended
- `reasoning_split` (via `extra_body`): separates reasoning into `reasoning_details` field
- Native `<think>` tags in M2.5/M2.1/M2 responses must be preserved for multi-turn
- Unsupported params: `presence_penalty`, `frequency_penalty`, `logit_bias`, `function_call`
- `n` parameter: only `1` accepted
- No image/audio input

### 1b. Anthropic-Compatible API

| Property | Value |
|----------|-------|
| Base URL | `https://api.minimax.io/anthropic` |
| Auth header | `x-api-key: {API_KEY}` (standard Anthropic) |
| Stream format | Standard Anthropic SSE (`content_block_start`, `content_block_delta`, etc.) |
| Thinking support | `thinking` parameter enables reasoning content blocks |

**Limitations:** No image/document input. Ignored params: `top_k`, `stop_sequences`, `service_tier`.

### Recommendation

Use the **OpenAI-compatible** path (`Api::OpenAICompletions`). MiniMax is already listed as `KnownProvider::Minimax` and `KnownProvider::MinimaxCn` in the codebase. The existing `openai_completions.rs` provider can serve MiniMax natively -- it just needs correct compat detection and model definitions.

---

## 2. Alchemy Event Schema (Canonical Contract)

Source: [`src/types/event.rs`](https://github.com/alchemiststudiosDOTai/alchemy-rs/blob/ef5bb6e/src/types/event.rs)

### 2a. AssistantMessageEvent -- 11 Variants

```rust
pub enum AssistantMessageEvent {
    // Stream lifecycle
    Start { partial: AssistantMessage },

    // Text content block
    TextStart    { content_index: usize, partial: AssistantMessage },
    TextDelta    { content_index: usize, delta: String, partial: AssistantMessage },
    TextEnd      { content_index: usize, content: String, partial: AssistantMessage },

    // Thinking/reasoning content block
    ThinkingStart { content_index: usize, partial: AssistantMessage },
    ThinkingDelta { content_index: usize, delta: String, partial: AssistantMessage },
    ThinkingEnd   { content_index: usize, content: String, partial: AssistantMessage },

    // Tool call content block
    ToolCallStart { content_index: usize, partial: AssistantMessage },
    ToolCallDelta { content_index: usize, delta: String, partial: AssistantMessage },
    ToolCallEnd   { content_index: usize, tool_call: ToolCall, partial: AssistantMessage },

    // Terminal events (exactly one emitted per stream)
    Done  { reason: StopReasonSuccess, message: AssistantMessage },
    Error { reason: StopReasonError,   error: AssistantMessage },
}
```

### 2b. Required Emission Order

```
Start
  -> (ThinkingStart -> N x ThinkingDelta -> ThinkingEnd)?
  -> (TextStart -> N x TextDelta -> TextEnd)?
  -> (ToolCallStart -> N x ToolCallDelta -> ToolCallEnd)*
  -> Done | Error
```

Every non-terminal event carries `partial: AssistantMessage` -- a clone of the accumulated output at that moment. Terminal events carry the final/failed message.

### 2c. Stop Reasons

Source: [`src/types/event.rs:62-92`](https://github.com/alchemiststudiosDOTai/alchemy-rs/blob/ef5bb6e/src/types/event.rs#L62-L92), [`src/types/usage.rs:42-50`](https://github.com/alchemiststudiosDOTai/alchemy-rs/blob/ef5bb6e/src/types/usage.rs#L42-L50)

| Variant | Serialized | Used in |
|---------|-----------|---------|
| `Stop` | `"stop"` | `Done` (via `StopReasonSuccess`) |
| `Length` | `"length"` | `Done` |
| `ToolUse` | `"tooluse"` | `Done` |
| `Error` | `"error"` | `Error` (via `StopReasonError`) |
| `Aborted` | `"aborted"` | `Error` |

**OpenAI finish_reason mapping** (used for MiniMax):

```
"stop"           -> StopReason::Stop
"length"         -> StopReason::Length
"function_call"  -> StopReason::ToolUse
"tool_calls"     -> StopReason::ToolUse
"content_filter" -> StopReason::Error
(anything else)  -> StopReason::Stop
```

---

## 3. Final AssistantMessage Schema (Canonical Contract)

Source: [`src/types/message.rs:53-65`](https://github.com/alchemiststudiosDOTai/alchemy-rs/blob/ef5bb6e/src/types/message.rs#L53-L65)

```rust
pub struct AssistantMessage {
    pub content: Vec<Content>,              // Polymorphic content blocks
    pub api: Api,                           // e.g. Api::OpenAICompletions
    pub provider: Provider,                 // e.g. Provider::Known(KnownProvider::Minimax)
    pub model: String,                      // e.g. "MiniMax-M2.5"
    pub usage: Usage,                       // Token counts + cost breakdown
    pub stop_reason: StopReason,            // Why the response ended
    pub error_message: Option<String>,      // Present only on error
    pub timestamp: i64,                     // Epoch millis
}
```

### 3a. Content Blocks

Source: [`src/types/content.rs:4-104`](https://github.com/alchemiststudiosDOTai/alchemy-rs/blob/ef5bb6e/src/types/content.rs)

```rust
#[serde(tag = "type")]
pub enum Content {
    Text     { inner: TextContent },      // {"type":"text", "text":"...", "text_signature":...}
    Thinking { inner: ThinkingContent },   // {"type":"thinking", "thinking":"...", "thinking_signature":...}
    Image    { inner: ImageContent },      // {"type":"image", "data":[...], "mime_type":"..."}
    ToolCall { inner: ToolCall },          // {"type":"toolCall", "id":"...", "name":"...", "arguments":{...}}
}
```

**ToolCall fields:** `id: String`, `name: String`, `arguments: serde_json::Value`, `thought_signature: Option<String>`

---

## 4. Usage Keys (Canonical Contract)

Source: [`src/types/usage.rs:3-32`](https://github.com/alchemiststudiosDOTai/alchemy-rs/blob/ef5bb6e/src/types/usage.rs)

### 4a. Usage struct (token counts)

| Field | Type | JSON key | Default |
|-------|------|----------|---------|
| `input` | `u32` | `usage.input` | `0` |
| `output` | `u32` | `usage.output` | `0` |
| `cache_read` | `u32` | `usage.cache_read` | `0` |
| `cache_write` | `u32` | `usage.cache_write` | `0` |
| `total_tokens` | `u32` | `usage.total_tokens` | `0` |
| `cost` | `Cost` | `usage.cost` | (see below) |

### 4b. Cost struct (dollar amounts)

| Field | Type | JSON key | Default |
|-------|------|----------|---------|
| `input` | `f64` | `usage.cost.input` | `0.0` |
| `output` | `f64` | `usage.cost.output` | `0.0` |
| `cache_read` | `f64` | `usage.cost.cache_read` | `0.0` |
| `cache_write` | `f64` | `usage.cost.cache_write` | `0.0` |
| `total` | `f64` | `usage.cost.total` | `0.0` |

### 4c. Usage Mapping from OpenAI SSE (applies to MiniMax)

Source: [`src/providers/openai_completions.rs:284-349`](https://github.com/alchemiststudiosDOTai/alchemy-rs/blob/ef5bb6e/src/providers/openai_completions.rs#L284-L349)

```
cache_read:  chunk.cache_read_input_tokens
          -> chunk.prompt_tokens_details.cached_tokens
          -> 0

cache_write: chunk.cache_creation_input_tokens
          -> chunk.prompt_tokens_details.cache_write_tokens
          -> 0

total:       chunk.total_tokens
          -> input + output

cost.total:  chunk.cost_details.upstream_inference_cost
          -> chunk.cost (top-level)
          -> sum of component costs
          -> 0.0
```

---

## 5. MiniMax-Specific Compat Defaults

### 5a. Already Registered

- `KnownProvider::Minimax` -- [`src/types/api.rs:59`](https://github.com/alchemiststudiosDOTai/alchemy-rs/blob/ef5bb6e/src/types/api.rs#L59)
- `KnownProvider::MinimaxCn` -- [`src/types/api.rs:60`](https://github.com/alchemiststudiosDOTai/alchemy-rs/blob/ef5bb6e/src/types/api.rs#L60)
- Env var: `MINIMAX_API_KEY` -- [`src/providers/env.rs`](https://github.com/alchemiststudiosDOTai/alchemy-rs/blob/ef5bb6e/src/providers/env.rs) (needs verification/addition)

### 5b. Recommended Compat Settings for `detect_compat()`

Based on MiniMax API docs + OpenAI compatibility:

| Compat field | Value | Rationale |
|---|---|---|
| `supports_store` | `false` | MiniMax does not support OpenAI `store` param |
| `supports_developer_role` | `false` | MiniMax docs do not mention developer role |
| `supports_reasoning_effort` | `false` | MiniMax uses `reasoning_split` instead of `reasoning_effort` |
| `supports_usage_in_streaming` | `true` | Standard OpenAI SSE with `stream_options.include_usage` |
| `max_tokens_field` | `MaxTokens` | MiniMax uses `max_tokens` (standard OpenAI, not `max_completion_tokens`) |
| `requires_tool_result_name` | `false` | Standard OpenAI tool result format |
| `requires_assistant_after_tool_result` | `false` | No indication of this requirement |
| `requires_thinking_as_text` | `false` | MiniMax supports native `<think>` tags |
| `requires_mistral_tool_ids` | `false` | Not Mistral |
| `thinking_format` | `Openai` | MiniMax uses `reasoning_content` field in streaming deltas |

### 5c. Reasoning / Thinking Support

MiniMax M2.5/M2.1/M2 models emit `<think>...</think>` tags inline in the response content. With `reasoning_split: true` (via `extra_body`), thinking is separated into a `reasoning_details` field in the streaming delta. This maps to alchemy's `reasoning_content` field in `StreamDelta`, which is already handled by `extract_reasoning()` at [`openai_completions.rs:351-370`](https://github.com/alchemiststudiosDOTai/alchemy-rs/blob/ef5bb6e/src/providers/openai_completions.rs#L351-L370).

---

## 6. Provider Registration Checklist

Since MiniMax uses the OpenAI-compatible API, it does **not** need a new provider module. It routes through the existing `openai_completions.rs`. The work is:

### Already Done
- [x] `KnownProvider::Minimax` and `KnownProvider::MinimaxCn` exist in [`src/types/api.rs:59-60`](https://github.com/alchemiststudiosDOTai/alchemy-rs/blob/ef5bb6e/src/types/api.rs#L59-L60)
- [x] `Api::OpenAICompletions` dispatch branch exists in [`src/stream/mod.rs:48-58`](https://github.com/alchemiststudiosDOTai/alchemy-rs/blob/ef5bb6e/src/stream/mod.rs#L48-L58)
- [x] OpenAI SSE parsing handles `reasoning_content` field for thinking

### Needs Implementation
1. **Add `detect_compat` branch** for `Minimax`/`MinimaxCn` in [`src/providers/openai_completions.rs:881-930`](https://github.com/alchemiststudiosDOTai/alchemy-rs/blob/ef5bb6e/src/providers/openai_completions.rs#L881-L930) with settings from section 5b above
2. **Add env var mapping** for `Minimax` -> `MINIMAX_API_KEY` and `MinimaxCn` -> `MINIMAX_API_KEY` in [`src/providers/env.rs`](https://github.com/alchemiststudiosDOTai/alchemy-rs/blob/ef5bb6e/src/providers/env.rs)
3. **Define model constants** (e.g., `MINIMAX_M2_5`, `MINIMAX_M2_5_HIGHSPEED`, etc.) with:
   - `api: OpenAICompletions`
   - `provider: Provider::Known(KnownProvider::Minimax)`
   - `base_url: "https://api.minimax.io/v1/chat/completions"`
   - `reasoning: true` (for M2.5, M2.1, M2)
   - `input: vec![InputType::Text]` (no image support)
   - `context_window: 204_800`
   - Cost rates per model (needs MiniMax pricing data)
4. **Handle `reasoning_split` parameter** in `build_params()` when the model is MiniMax and reasoning is enabled -- add `"extra_body": { "reasoning_split": true }` or use `reasoning_content` field detection that already exists
5. **Temperature clamping** -- MiniMax requires (0.0, 1.0]; alchemy should clamp or warn

### Base URLs

| Provider | Chat completions URL |
|----------|---------------------|
| `Minimax` | `https://api.minimax.io/v1/chat/completions` |
| `MinimaxCn` | `https://api.minimax.chat/v1/chat/completions` (CN mirror) |

---

## 7. Architecture Reference

### Dependency Direction (for MiniMax work)

```
stream/mod.rs       (dispatch)
    |
    v
providers/openai_completions.rs   (stream implementation)
    |
    v
types/              (event.rs, message.rs, usage.rs, content.rs, api.rs, model.rs, compat.rs)
    |
    v
providers/shared/   (http.rs, timestamp.rs)
providers/env.rs    (API key resolution)
```

MiniMax touches only the middle and bottom layers -- no changes to `stream/` or `types/` needed.

### Key Files

| File | Role | Permalink |
|------|------|-----------|
| `src/types/event.rs` | Event enum (11 variants) | [Link](https://github.com/alchemiststudiosDOTai/alchemy-rs/blob/ef5bb6e/src/types/event.rs) |
| `src/types/message.rs` | AssistantMessage struct | [Link](https://github.com/alchemiststudiosDOTai/alchemy-rs/blob/ef5bb6e/src/types/message.rs) |
| `src/types/usage.rs` | Usage + Cost + StopReason | [Link](https://github.com/alchemiststudiosDOTai/alchemy-rs/blob/ef5bb6e/src/types/usage.rs) |
| `src/types/content.rs` | Content enum + inner types | [Link](https://github.com/alchemiststudiosDOTai/alchemy-rs/blob/ef5bb6e/src/types/content.rs) |
| `src/types/api.rs` | Api + Provider + KnownProvider enums | [Link](https://github.com/alchemiststudiosDOTai/alchemy-rs/blob/ef5bb6e/src/types/api.rs) |
| `src/types/compat.rs` | OpenAICompletionsCompat flags | [Link](https://github.com/alchemiststudiosDOTai/alchemy-rs/blob/ef5bb6e/src/types/compat.rs) |
| `src/providers/openai_completions.rs` | Full OpenAI-compat provider | [Link](https://github.com/alchemiststudiosDOTai/alchemy-rs/blob/ef5bb6e/src/providers/openai_completions.rs) |
| `src/providers/env.rs` | Env var -> API key mapping | [Link](https://github.com/alchemiststudiosDOTai/alchemy-rs/blob/ef5bb6e/src/providers/env.rs) |
| `src/stream/mod.rs` | Dispatch + stream()/complete() | [Link](https://github.com/alchemiststudiosDOTai/alchemy-rs/blob/ef5bb6e/src/stream/mod.rs) |
| `src/stream/event_stream.rs` | Channel-based stream infra | [Link](https://github.com/alchemiststudiosDOTai/alchemy-rs/blob/ef5bb6e/src/stream/event_stream.rs) |

---

## Knowledge Gaps

1. **MiniMax pricing** -- per-token cost rates for each model needed for `ModelCost` definitions
2. **`reasoning_split` wire format** -- exact SSE delta field name when `reasoning_split: true` is set (likely `reasoning_details` or `reasoning_content`, needs live testing)
3. **MiniMax `max_tokens` field** -- needs verification whether MiniMax uses `max_tokens` or `max_completion_tokens` in request body
4. **MiniMax streaming usage** -- does MiniMax include usage in the final SSE chunk when `stream_options.include_usage: true`? Needs testing
5. **MiniMax CN base URL** -- the exact China-region endpoint for `MinimaxCn` needs confirmation
6. **Tool calling support** -- MiniMax docs show OpenAI-compatible tool calling but specific edge cases (parallel tool calls, partial arguments) need testing

## References

- MiniMax OpenAI API docs: https://platform.minimax.io/docs/api-reference/text-openai-api
- MiniMax Anthropic API docs: https://platform.minimax.io/docs/api-reference/text-anthropic-api
- Alchemy codebase: https://github.com/alchemiststudiosDOTai/alchemy-rs/tree/ef5bb6e
