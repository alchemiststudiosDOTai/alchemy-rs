---
summary: "Research mapping for issues #4 and #5 - process_chunk and convert_messages complexity issues"
read_when:
  - Preparing to refactor process_chunk or convert_messages
  - Understanding complexity hotspots in the codebase
  - Looking for patterns to extract from large functions
---

# Research – Issues #4 & #5: Complexity Hotspots

**Date:** 2026-01-30
**Owner:** Claude Code
**Phase:** Research
**Git Commit:** 42b3ef8
**Branch:** issues-complexity

## Goal

Map the exact location, structure, and complexity characteristics of the two functions flagged for refactoring in GitHub issues #4 and #5.

## Issue Summary

| Issue | Function | Location | Problem | clippy.toml Notes |
|-------|----------|----------|---------|-------------------|
| #5 | `convert_messages` | `src/providers/openai_completions.rs:537` | Too many lines | 109 lines (threshold: 100) |
| #4 | `process_chunk` | `src/providers/openai_completions.rs:207` | Cognitive complexity | 19 complexity, 161 lines |

---

## Issue #5: convert_messages (Lines 537-667)

### Location
- **File:** `src/providers/openai_completions.rs`
- **Lines:** 537-667 (130 lines total, ~109 effective)
- **Called from:** `build_params` at line 482

### Function Signature
```rust
fn convert_messages(
    model: &Model<OpenAICompletions>,
    context: &Context,
    compat: &ResolvedCompat,
) -> serde_json::Value
```

### Structure Breakdown

```
convert_messages (130 lines)
├── System prompt handling (lines 544-555)
│   └── Role selection: "developer" vs "system" based on model.reasoning
│
└── Message conversion loop (lines 558-664)
    ├── User message branch (lines 560-591)
    │   ├── Text content -> json!(text)
    │   └── Multi-content -> array of blocks (text/image filtering)
    │
    ├── Assistant message branch (lines 592-643)
    │   ├── Collect text parts (filter_map on content)
    │   ├── Collect tool calls (filter_map on content)
    │   └── Skip empty check
    │
    └── ToolResult message branch (lines 644-662)
        ├── Base message with tool_call_id
        ├── Content text extraction/joining
        └── Optional name field for compat.requires_tool_result_name
```

### Complexity Factors

1. **Three-way match on Message enum** - each branch handles a distinct message type
2. **Nested content type matching** - UserContent::Text vs UserContent::Multi
3. **filter_map chains** - iterative collection with closures
4. **Compatibility conditionals** - supports_developer_role, requires_tool_result_name
5. **Image capability check** - model.input.contains() filtering

### Related Functions in Same File

- `build_params` (line 470) - Calls convert_messages
- `convert_tools` (line 669) - Similar pattern but simpler (15 lines)

---

## Issue #4: process_chunk (Lines 207-391)

### Location
- **File:** `src/providers/openai_completions.rs`
- **Lines:** 207-391 (184 lines total, ~161 effective)
- **Called from:** `run_stream_inner` at line 168

### Function Signature
```rust
fn process_chunk(
    chunk: &StreamChunk,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
    current_block: &mut Option<CurrentBlock>,
)
```

### Structure Breakdown

```
process_chunk (184 lines)
├── Usage handling (lines 213-236)
│   ├── cached_tokens extraction (nested option chain)
│   ├── reasoning_tokens extraction
│   └── Usage struct construction
│
├── Guard clauses (lines 239-250)
│   ├── Early return if no choices
│   └── Early return if no delta
│
├── Text content handling (lines 252-282)
│   ├── match current_block
│   │   ├── Continue Text -> push_str + TextDelta event
│   │   └── Start new Text -> finish_current_block + TextStart + TextDelta
│
├── Reasoning content handling (lines 284-329)
│   ├── Three-field fallback chain (reasoning_content / reasoning / reasoning_text)
│   ├── Signature tracking for field source
│   └── match current_block
│       ├── Continue Thinking -> push_str + ThinkingDelta
│       └── Start new Thinking -> finish_current_block + ThinkingStart + ThinkingDelta
│
└── Tool call handling (lines 331-390)
    ├── ID change detection (match + is_some_and)
    ├── Start new block condition
    ├── finish_current_block + ToolCallStart
    └── Argument accumulation (nested if-let chain)
        ├── ID update
        ├── Name update
        └── Arguments push_str + ToolCallDelta
```

### Complexity Factors

1. **State machine with 3 content types** (Text, Thinking, ToolCall)
2. **Two states per type** (continue current block vs start new block)
3. **Three different field names for reasoning** (provider compatibility)
4. **Nested option chains** for tool call ID/name/arguments
5. **Event emission pattern** - Start + Delta events for each type

### Supporting State Type

```rust
// Lines 191-205
enum CurrentBlock {
    Text { text: String },
    Thinking { thinking: String, signature: String },
    ToolCall { id: String, name: String, partial_args: String },
}
```

### Helper Function

- `finish_current_block` (lines 393-430) - Called 3 times in process_chunk

---

## File Context

### openai_completions.rs Structure

```
Lines    Content
------   -------
1-110    Imports, types, OpenAICompletionsOptions
111-189  run_stream_inner (SSE loop)
191-205  CurrentBlock enum
207-391  process_chunk (ISSUE #4)
393-430  finish_current_block
460-468  map_stop_reason
470-535  build_params
537-667  convert_messages (ISSUE #5)
669-685  convert_tools
688-775  detect_compat / resolve_compat
831-906  Tests
```

### Dependency Graph

```
stream_openai_completions (entry point)
    ↓ calls
run_stream_inner
    ↓ calls
    ├── process_chunk ← ISSUE #4
    │   ├── finish_current_block
    │   └── EventStreamSender::push
    │
    └── build_params
        ↓ calls
        ├── convert_messages ← ISSUE #5
        └── convert_tools
```

---

## Related Files

| File | Relevance |
|------|-----------|
| `src/providers/openai_completions.rs` | Contains both target functions |
| `src/stream/event_stream.rs` | EventStreamSender, event types |
| `src/types/message.rs` | Message, UserMessage, AssistantMessage types |
| `src/types/content.rs` | Content enum variants |
| `src/transform.rs` | Similar complexity patterns (transform_message, transform_content_block) |
| `clippy.toml` | Complexity thresholds and TODO notes |

---

## Refactoring Patterns in Codebase

### Pattern: Extract Per-Variant Handler
**Example:** `transform.rs` uses separate functions:
- `transform_message` (lines 53-78) - orchestrates
- `transform_content_block` (lines 162-232) - handles each variant

### Pattern: State Machine Enum
**Used in:** `CurrentBlock` enum cleanly tracks partial state

### Pattern: Guard Clauses
**Used throughout:** Early returns flatten nesting (e.g., lines 239-250 in process_chunk)

---

## GitHub Permalinks

- [process_chunk](https://github.com/alchemy-rs/alchemy/blob/42b3ef8/src/providers/openai_completions.rs#L207-L391)
- [convert_messages](https://github.com/alchemy-rs/alchemy/blob/42b3ef8/src/providers/openai_completions.rs#L537-L667)
- [clippy.toml complexity notes](https://github.com/alchemy-rs/alchemy/blob/42b3ef8/clippy.toml#L6-L12)

---

## Knowledge Gaps

- No unit tests specifically for `process_chunk` or `convert_messages` in isolation
- Integration tests in `examples/` exercise these indirectly
- Need to verify SSE chunk edge cases before refactoring process_chunk
