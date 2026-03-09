---
summary: "Cross-provider message transformation behavior from src/transform.rs"
status: current
last_verified: 2026-03-09
---

# Message Transformation

`src/transform.rs` rewrites conversation history when callers switch providers or model families.

## What It Handles

- Thinking block preservation vs. conversion to plain text
- Tool-call ID normalization
- Synthetic tool-result insertion for orphaned tool calls
- Filtering of assistant messages that ended with `StopReason::Error` or `StopReason::Aborted`

## Public Surface

- `TargetModel`
- `transform_messages`
- `transform_messages_simple`

## Key Rules

- If the target model/provider matches the source assistant message exactly, reasoning blocks and signatures can be preserved.
- If the target changes, reasoning becomes plain text and text/tool-call signatures are stripped where needed.
- Tool-result IDs are rewritten when tool-call IDs are normalized.
- Orphaned tool calls are backfilled with synthetic error results so downstream providers receive a complete conversation shape.
