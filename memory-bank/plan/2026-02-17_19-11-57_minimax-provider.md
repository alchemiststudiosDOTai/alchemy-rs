---
title: "First-Class MiniMax Provider -- Plan"
phase: Plan
date: "2026-02-17T19:11:57"
owner: "agent"
parent_research: "memory-bank/research/2026-02-17_minimax-provider-research.md"
git_commit_at_plan: "da2ee06"
tags: [plan, minimax, provider, thinking, streaming, coding]
---

## Goal

- Add first-class MiniMax provider support with native `<think>` tag parsing for interleaved thinking, proper compat detection, `reasoning_split` request parameter, and full model catalog.

### Non-goals

- Deployment, CI/CD, observability.
- Anthropic-compatible MiniMax API (using OpenAI-compat path only).
- Image/audio input support (MiniMax does not support it).
- Live API integration tests (no MiniMax API key assumed available in CI).

## Scope & Assumptions

### In scope

- `<think>...</think>` tag parser for inline reasoning in content stream.
- `reasoning_split: true` request parameter for MiniMax reasoning models.
- `reasoning_details` SSE delta field support.
- MiniMax compat detection in `detect_compat()`.
- Temperature clamping for MiniMax (0.0, 1.0].
- Public MiniMax model constants with pricing.
- Unit tests for the `<think>` tag parser.
- Round-trip `thinking_signature` support for multi-turn replay.

### Out of scope

- Changes to `stream/mod.rs` dispatch (MiniMax stays on `Api::OpenAICompletions`).
- Changes to `types/event.rs` or `types/message.rs` (canonical contract unchanged).
- New API types or provider modules (reuses `openai_completions.rs`).

### Assumptions

- MiniMax SSE format is standard OpenAI (verified by research doc).
- `reasoning_split: true` causes MiniMax to emit a separate reasoning field (name TBD, likely `reasoning_content` or `reasoning_details`).
- `<think>` tag parsing is the fallback for when `reasoning_split` is absent or unsupported.
- MiniMax pricing is available from public docs (gaps filled with `0.0` placeholders).

## Deliverables

1. `src/utils/think_tag_parser.rs` -- Streaming `<think>` tag parser.
2. Modified `src/providers/openai_completions.rs` -- MiniMax compat, `reasoning_split`, `<think>` integration, temperature clamping.
3. Modified `src/types/compat.rs` -- New `ThinkingFormat::ThinkTag` variant for `<think>`-based providers.
4. `src/models/minimax.rs` -- Public MiniMax model constants.
5. Modified `src/models/mod.rs` and `src/lib.rs` -- Module wiring and re-exports.

## Readiness

- [x] `KnownProvider::Minimax` and `MinimaxCn` exist in `src/types/api.rs:60-61`
- [x] Env var mapping exists in `src/providers/env.rs:41-42`
- [x] `openai_completions.rs` has interleaved thinking state machine (`CurrentBlock`)
- [x] `StreamDelta` already deserializes `reasoning_content`, `reasoning`, `reasoning_text`
- [x] Research doc complete at `memory-bank/research/2026-02-17_minimax-provider-research.md`

---

## Milestones

- **M1: Compat & Request Building** -- MiniMax detected correctly, `reasoning_split` sent, temperature clamped.
- **M2: `<think>` Tag Parser** -- Streaming parser that handles tag boundaries across SSE chunks.
- **M3: Integration into Streaming Pipeline** -- Parser wired into `run_stream`, emitting proper ThinkingStart/Delta/End events.
- **M4: Model Catalog & Tests** -- Public model constants, unit tests for parser, `cargo check`/`clippy` pass.

---

## Work Breakdown (Tasks)

### T1: Add MiniMax compat detection

**Milestone:** M1
**Estimate:** Small
**Dependencies:** None
**Files:** `src/providers/openai_completions.rs` (lines 881-930)

Add `is_minimax` boolean to `detect_compat()` matching `KnownProvider::Minimax | KnownProvider::MinimaxCn` and base URLs containing `api.minimax.io` or `api.minimax.chat`. Set:

```rust
let is_minimax = matches!(
    provider,
    Provider::Known(KnownProvider::Minimax) | Provider::Known(KnownProvider::MinimaxCn)
) || base_url.contains("minimax.io")
  || base_url.contains("minimax.chat");
```

Merge `is_minimax` into the `is_non_standard` and `use_max_tokens` booleans so MiniMax gets:

| Flag | Value |
|------|-------|
| `supports_store` | `false` (via `is_non_standard`) |
| `supports_developer_role` | `false` (via `is_non_standard`) |
| `supports_reasoning_effort` | `false` |
| `supports_usage_in_streaming` | `true` |
| `max_tokens_field` | `MaxTokens` (via `use_max_tokens`) |
| `requires_tool_result_name` | `false` |
| `requires_assistant_after_tool_result` | `false` |
| `requires_thinking_as_text` | `false` |
| `requires_mistral_tool_ids` | `false` |
| `thinking_format` | `ThinkingFormat::Openai` |

**Acceptance test:** `detect_compat` for a MiniMax model returns `supports_store: false`, `max_tokens_field: MaxTokens`, `supports_reasoning_effort: false`.

---

### T2: Add `reasoning_split` to `build_params()`

**Milestone:** M1
**Estimate:** Small
**Dependencies:** T1
**Files:** `src/providers/openai_completions.rs` (lines 620-685)

In `build_params()`, after the reasoning effort block (line 682), add a new block. When the model's provider is `Minimax` or `MinimaxCn` AND `model.reasoning` is true, inject `reasoning_split: true` into the request body:

```rust
// MiniMax reasoning_split: separates thinking into dedicated SSE field
if model.reasoning && is_minimax_provider(&model.provider) {
    params["reasoning_split"] = json!(true);
}
```

Add a helper `is_minimax_provider(provider: &Provider) -> bool` in the same file.

The `compat` reference is already available in `build_params`, but we need the provider. The provider is available via `model.provider`. We need to pass nothing new -- `model` is already a parameter.

**Acceptance test:** `build_params()` for a MiniMax reasoning model includes `"reasoning_split": true` in the output JSON.

---

### T3: Add temperature clamping for MiniMax

**Milestone:** M1
**Estimate:** Small
**Dependencies:** T2
**Files:** `src/providers/openai_completions.rs` (lines 657-660)

In `build_params()`, after setting the temperature (line 660), clamp for MiniMax:

```rust
if let Some(temp) = options.temperature {
    let temp = if is_minimax_provider(&model.provider) {
        temp.clamp(f64::MIN_POSITIVE, 1.0)  // MiniMax: (0.0, 1.0]
    } else {
        temp
    };
    params["temperature"] = json!(temp);
}
```

MiniMax rejects `0.0` (exclusive lower bound), so clamp to `f64::MIN_POSITIVE` as the floor.

**Acceptance test:** Temperature `0.0` for MiniMax model is clamped to a small positive value.

---

### T4: Add `reasoning_details` to `StreamDelta`

**Milestone:** M1
**Estimate:** Small
**Dependencies:** None
**Files:** `src/providers/openai_completions.rs` (lines 957-964, 351-370)

Add `reasoning_details` as a 4th optional field on `StreamDelta`:

```rust
#[derive(Debug, Deserialize)]
struct StreamDelta {
    content: Option<String>,
    reasoning_content: Option<String>,
    reasoning: Option<String>,
    reasoning_text: Option<String>,
    reasoning_details: Option<String>,  // MiniMax with reasoning_split
    tool_calls: Option<Vec<StreamToolCall>>,
}
```

Add a new constant and extend `extract_reasoning()`:

```rust
const REASONING_DETAILS_FIELD: &str = "reasoning_details";

fn extract_reasoning(delta: &StreamDelta) -> Option<ReasoningDelta<'_>> {
    // ... existing checks ...
    if let Some(text) = delta.reasoning_details.as_deref() {
        return Some(ReasoningDelta { text, signature: REASONING_DETAILS_FIELD });
    }
    // ... existing fallback ...
}
```

Insert the `reasoning_details` check after `reasoning_content` (highest priority) and before `reasoning` (lower priority), since it is MiniMax-specific and should take precedence over generic field names.

**Acceptance test:** An SSE delta with `"reasoning_details": "some thinking"` is extracted by `extract_reasoning()`.

---

### T5: Implement `ThinkTagParser`

**Milestone:** M2
**Estimate:** Medium
**Dependencies:** None
**Files:** `src/utils/think_tag_parser.rs` (new), `src/utils/mod.rs`

Create a streaming `<think>` tag parser that handles tag boundaries across SSE chunks.

#### Design

```rust
pub(crate) struct ThinkTagParser {
    state: TagParserState,
    buffer: String,
}

enum TagParserState {
    Text,           // Outside <think> tags, accumulating text
    PendingOpen,    // Saw '<', buffering potential "<think>" match
    Thinking,       // Inside <think>, accumulating reasoning
    PendingClose,   // Inside <think>, saw '<', buffering potential "</think>" match
}

pub(crate) enum ParsedFragment {
    Text(String),
    Thinking(String),
}

impl ThinkTagParser {
    pub fn new() -> Self;

    /// Feed a chunk of content. Returns fragments to emit.
    pub fn push(&mut self, content: &str) -> Vec<ParsedFragment>;

    /// Flush any remaining buffered content at end of stream.
    pub fn flush(&mut self) -> Vec<ParsedFragment>;
}
```

#### Key behaviors

1. **Normal text:** `"Hello world"` -> `[Text("Hello world")]`
2. **Complete think block:** `"<think>reasoning</think>answer"` -> `[Thinking("reasoning"), Text("answer")]`
3. **Tag split across chunks:** Chunk1: `"<thi"`, Chunk2: `"nk>reasoning</think>"` -> Chunk1: `[]`, Chunk2: `[Thinking("reasoning")]`
4. **Interleaved:** `"<think>t1</think>text1<think>t2</think>text2"` -> `[Thinking("t1"), Text("text1"), Thinking("t2"), Text("text2")]`
5. **Partial tag that fails to match:** `"<thx"` -> `[Text("<thx")]` (buffer flushed as text)
6. **End of stream inside think:** `flush()` emits remaining thinking as `Thinking(...)`.
7. **Empty thinking:** `"<think></think>"` -> skipped (empty fragment not emitted).

#### Tag matching

Match exactly `<think>` and `</think>` (case-sensitive, no attributes). Use byte-by-byte matching against the expected tag characters. When a partial match fails, flush the buffer as the current block type.

The `thinking_signature` for `<think>`-parsed blocks should use a new constant `THINK_TAG_FIELD: &str = "think_tag"` defined in `openai_completions.rs` alongside the other signature constants.

**Acceptance test:** Unit test in the same file covering cases 1-7 above.

---

### T6: Wire `ThinkTagParser` into streaming pipeline

**Milestone:** M3
**Estimate:** Medium
**Dependencies:** T1, T2, T4, T5
**Files:** `src/providers/openai_completions.rs` (lines 145-222, 249-282)

This is the core integration task. Modify `run_stream()` and `process_chunk()` to use the `ThinkTagParser` for MiniMax reasoning models.

#### Changes to `run_stream()`

1. After `resolve_compat()` (line 156), detect if this is a MiniMax reasoning model:

```rust
let use_think_parser = model.reasoning && is_minimax_provider(&model.provider);
let mut think_parser = use_think_parser.then(ThinkTagParser::new);
```

2. Pass `&mut think_parser` through to `process_chunk()`.

3. After the SSE loop ends (line 205) but before `finish_current_block` (line 208), flush the parser:

```rust
if let Some(parser) = &mut think_parser {
    let fragments = parser.flush();
    for fragment in fragments {
        match fragment {
            ParsedFragment::Text(text) => {
                handle_text_delta(&text, &mut output, &mut sender, &mut current_block);
            }
            ParsedFragment::Thinking(thinking) => {
                let reasoning = ReasoningDelta { text: &thinking, signature: THINK_TAG_FIELD };
                handle_reasoning_delta(reasoning, &mut output, &mut sender, &mut current_block);
            }
        }
    }
}
```

#### Changes to `process_chunk()`

Add `think_parser: &mut Option<ThinkTagParser>` parameter. In the content handling (line 271-273):

```rust
if let Some(content) = delta.content.as_deref() {
    if let Some(parser) = think_parser.as_mut() {
        // MiniMax: parse <think> tags from inline content
        let fragments = parser.push(content);
        for fragment in fragments {
            match fragment {
                ParsedFragment::Text(text) => {
                    handle_text_delta(&text, output, sender, current_block);
                }
                ParsedFragment::Thinking(thinking) => {
                    let reasoning = ReasoningDelta { text: &thinking, signature: THINK_TAG_FIELD };
                    handle_reasoning_delta(reasoning, output, sender, current_block);
                }
            }
        }
    } else {
        handle_text_delta(content, output, sender, current_block);
    }
}
```

The reasoning extraction (line 275-277) remains unchanged -- if `reasoning_split` works and MiniMax sends `reasoning_content` or `reasoning_details`, those paths fire normally and the `<think>` parser won't see them (since they're in a different field, not in `content`).

This means `reasoning_split` is the primary path, and `<think>` parsing is the fallback for when MiniMax sends thinking inline.

**Acceptance test:** A simulated MiniMax SSE stream with `<think>reasoning</think>answer` in the `content` field produces `ThinkingStart -> ThinkingDelta -> ThinkingEnd -> TextStart -> TextDelta -> TextEnd` events.

---

### T7: Define MiniMax model constants

**Milestone:** M4
**Estimate:** Small
**Dependencies:** T1
**Files:** `src/models/minimax.rs` (new), `src/models/mod.rs` (new), `src/lib.rs`

Create `src/models/mod.rs` with `pub mod minimax;` and create `src/models/minimax.rs` with public model-builder functions.

Define models as public functions (not `const` -- `String` fields prevent const):

```rust
use crate::types::{
    InputType, KnownProvider, MaxTokensField, Model, ModelCost,
    OpenAICompletions, OpenAICompletionsCompat, Provider,
};

const MINIMAX_BASE_URL: &str = "https://api.minimax.io/v1/chat/completions";
const MINIMAX_CN_BASE_URL: &str = "https://api.minimax.chat/v1/chat/completions";
const MINIMAX_CONTEXT_WINDOW: u32 = 204_800;

pub fn minimax_m2_5() -> Model<OpenAICompletions> { ... }
pub fn minimax_m2_5_highspeed() -> Model<OpenAICompletions> { ... }
pub fn minimax_m2_1() -> Model<OpenAICompletions> { ... }
pub fn minimax_m2_1_highspeed() -> Model<OpenAICompletions> { ... }
pub fn minimax_m2() -> Model<OpenAICompletions> { ... }
```

Each model sets:
- `api: OpenAICompletions`
- `provider: Provider::Known(KnownProvider::Minimax)`
- `base_url: MINIMAX_BASE_URL`
- `reasoning: true` (all M2.x models support reasoning)
- `input: vec![InputType::Text]`
- `context_window: 204_800`
- `max_tokens: 16_384` (default, adjust per model)
- `cost: ModelCost { ... }` -- fill from MiniMax pricing or `0.0` placeholders
- `compat: None` (compat auto-detected via T1)

Add CN variants (`minimax_cn_m2_5()`, etc.) using `MinimaxCn` provider and `MINIMAX_CN_BASE_URL`.

Wire into `src/lib.rs`:
```rust
pub mod models;
```

**Acceptance test:** `minimax_m2_5()` compiles and produces a valid `Model<OpenAICompletions>` with correct base_url and provider.

---

### T8: Unit tests for `ThinkTagParser`

**Milestone:** M4
**Estimate:** Small
**Dependencies:** T5
**Files:** `src/utils/think_tag_parser.rs` (test module)

Add `#[cfg(test)] mod tests` in `think_tag_parser.rs` with the following cases:

1. **Plain text (no tags):** `push("hello")` -> `[Text("hello")]`
2. **Single complete block:** `push("<think>reason</think>answer")` -> `[Thinking("reason"), Text("answer")]`
3. **Tag split across chunks:** `push("<thi")` -> `[]`, `push("nk>reason</think>")` -> `[Thinking("reason")]`
4. **Interleaved blocks:** `push("<think>t1</think>text1<think>t2</think>text2")` -> 4 fragments
5. **Close tag split across chunks:** `push("<think>reason</thi")` -> `[]`, `push("nk>answer")` -> `[Thinking("reason"), Text("answer")]`
6. **Flush at end of stream:** `push("<think>unfinished")` -> `[]`, `flush()` -> `[Thinking("unfinished")]`
7. **False tag start:** `push("<thx>text")` -> `[Text("<thx>text")]`
8. **Empty thinking:** `push("<think></think>text")` -> `[Text("text")]` (empty thinking skipped)

**Acceptance test:** All 8 cases pass.

---

### T9: Final validation

**Milestone:** M4
**Estimate:** Small
**Dependencies:** T1-T8
**Files:** None (validation only)

Run `cargo check`, `cargo clippy`, and `cargo test`. Fix any warnings or errors. Ensure no `#[allow(...)]` suppressions are introduced.

**Acceptance test:** Clean `cargo check && cargo clippy && cargo test` pass.

---

## Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| `reasoning_details` is not the correct MiniMax field name | `<think>` parser fallback handles it; field can be renamed later | Both paths implemented: field-based (T4) and tag-based (T5/T6) |
| `<think>` tag parsing across chunk boundaries has edge cases | Malformed thinking blocks | Extensive unit tests (T8), flush() ensures no data loss |
| MiniMax pricing not publicly available | Model costs show `0.0` | Placeholder costs clearly documented; easy to update |
| MiniMax SSE format has undocumented quirks | Stream parsing failures | Uses battle-tested `openai_completions.rs` SSE parser unchanged |
| `reasoning_split` parameter name is wrong | Thinking arrives inline via `<think>` tags | Parser handles both paths; `reasoning_split` is additive |

## Test Strategy

- **T5/T8:** Unit tests for `ThinkTagParser` (8 cases covering all edge cases).
- **T9:** `cargo check` + `cargo clippy` + `cargo test` as gate.
- No live API tests (no MiniMax API key in CI).

## References

- Research doc: `memory-bank/research/2026-02-17_minimax-provider-research.md`
- `src/providers/openai_completions.rs` -- Lines 145-222 (run_stream), 249-282 (process_chunk), 351-370 (extract_reasoning), 620-685 (build_params), 881-930 (detect_compat), 957-964 (StreamDelta)
- `src/types/compat.rs` -- `OpenAICompletionsCompat`, `ThinkingFormat`
- `src/types/model.rs` -- `Model<TApi>` struct
- `src/providers/env.rs:41-42` -- MiniMax env var mapping (already done)
- MiniMax API docs: https://platform.minimax.io/docs/api-reference/text-openai-api

---

## Final Gate

- **Plan path:** `memory-bank/plan/2026-02-17_19-11-57_minimax-provider.md`
- **Milestone count:** 4
- **Task count:** 9 (all ready for coding)
- **New files:** 3 (`src/utils/think_tag_parser.rs`, `src/models/minimax.rs`, `src/models/mod.rs`)
- **Modified files:** 3 (`src/providers/openai_completions.rs`, `src/lib.rs`, `src/utils/mod.rs`)
- **Next command:** `/context-engineer:execute "memory-bank/plan/2026-02-17_19-11-57_minimax-provider.md"`
