---
summary: "Cross-provider message transformation for switching models/providers mid-conversation"
read_when:
  - You need to switch between different LLM providers during a conversation
  - You want to understand how thinking blocks and tool calls are transformed
  - You need to handle tool call ID normalization across providers
---

# Message Transformation

Cross-provider message transformation for conversation history compatibility. Handles thinking blocks, tool call IDs, orphaned tool calls, and error filtering.

**Source:** `src/transform.rs` (~775 lines with tests)

## Purpose

When switching between models or providers during a conversation, message formats may not be directly compatible. The `transform_messages` function normalizes conversation history for the target model.

## Key Transformations

### 1. Thinking Block Handling

Thinking blocks (`<thinking>...</thinking>`) are handled differently based on the target:

| Source | Target | Action |
|--------|--------|--------|
| Same model + signature | Same model + signature | Keep for replay |
| Same model | Same model | Keep as-is |
| Any | Different model | Convert to plain text |
| Any | Any | Empty thinking is filtered out |

### 2. Tool Call ID Normalization

Different providers have different ID requirements. The optional `normalize_tool_call_id` function builds a mapping from original IDs to normalized IDs, then applies the mapping to tool results.

### 3. Orphaned Tool Call Handling

If an assistant message contains tool calls but no results follow (e.g., user interrupted), synthetic error results are inserted. This satisfies API requirements that all tool calls must have corresponding results.

### 4. Error/Abort Filtering

Assistant messages with `stop_reason: Error` or `Aborted` are filtered out completely. These represent incomplete turns that should not be replayed.

## API

### TargetModel

Information about the target model for transformation.

```rust
pub struct TargetModel {
    pub api: Api,
    pub provider: Provider,
    pub model_id: String,
}
```

### transform_messages

Full transformation with optional tool call ID normalization.

```rust
pub fn transform_messages<F>(
    messages: &[Message],
    target: &TargetModel,
    normalize_tool_call_id: Option<F>,
) -> Vec<Message>
where
    F: Fn(&str, &TargetModel, &AssistantMessage) -> String,
```

**Parameters:**
- `messages`: Source conversation history
- `target`: Target model information
- `normalize_tool_call_id`: Optional function to normalize tool call IDs

**Returns:** Transformed message vector

### transform_messages_simple

Convenience wrapper when no ID normalization is needed.

```rust
pub fn transform_messages_simple(
    messages: &[Message],
    target: &TargetModel,
) -> Vec<Message>
```

## Usage Examples

### Basic Transformation (Same Provider)

```rust
use alchemy::transform::{transform_messages_simple, TargetModel};
use alchemy::types::{Api, Provider, KnownProvider};

let target = TargetModel {
    api: Api::AnthropicMessages,
    provider: Provider::Known(KnownProvider::Anthropic),
    model_id: "claude-sonnet-4-20250514".to_string(),
};

let transformed = transform_messages_simple(&messages, &target);
```

### Cross-Provider with ID Normalization

```rust
use alchemy::transform::{transform_messages, TargetModel};
use alchemy::types::{Api, Provider, KnownProvider};

let target = TargetModel {
    api: Api::OpenAICompletions,
    provider: Provider::Known(KnownProvider::OpenAI),
    model_id: "gpt-4o".to_string(),
};

// Normalize Anthropic-style IDs to OpenAI format
let normalize = |id: &str, _target: &TargetModel, _msg: &AssistantMessage| -> String {
    format!("call_{}", id.replace('-', "_"))
};

let transformed = transform_messages(&messages, &target, Some(normalize));
```

## Test Coverage

The module includes 15 comprehensive tests covering:

- User message passthrough
- Error message filtering
- Aborted message filtering
- Thinking block behavior (same/different model, with/without signature)
- Empty thinking filtering
- Text signature stripping
- Tool call ID normalization
- Orphaned tool call synthetic results
- Multiple tool calls with partial results
- Image content passthrough

Run tests with:

```bash
cargo test --package alchemy --lib transform
```

## Implementation Notes

### Two-Pass Algorithm

1. **First pass:** Transform each message, build tool call ID mapping
2. **Second pass:** Insert synthetic tool results for orphaned calls

### State Tracking

During transformation, the algorithm tracks:
- `tool_call_id_map`: Maps original IDs to normalized IDs
- `pending_tool_calls`: Tool calls waiting for results
- `existing_result_ids`: Tool result IDs already seen

### Signature Stripping

Signatures (`text_signature`, `thinking_signature`, `thought_signature`) are stripped when transforming to a different model. They are only preserved for exact same-model replay.

## See Also

- [`types::Message`](../types/message.md) - Message types
- [`types::Content`](../types/content.md) - Content block types
- [`types::StopReason`](../types/stop_reason.md) - Stop reason enumeration
- Phase 5 plan: `memory-bank/plan/phase-5-cross-provider-features.md`
