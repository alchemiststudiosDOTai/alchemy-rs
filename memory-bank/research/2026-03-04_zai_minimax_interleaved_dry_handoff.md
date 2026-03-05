# Handoff: DRY interleaved tool-call continuation handling (MiniMax + z.ai)

Date: 2026-03-04
Owner handoff from: current PR #40 (`fix/zai: preserve interleaved tool-call argument continuations`)

## Why this exists
We fixed a real z.ai bug where interleaved `content` / `reasoning_content` could close an active tool-call block before id-less argument continuation deltas were applied.

That fix intentionally mirrored MiniMax logic, so duplication increased. `make duplicates` now flags the shared pattern.

## Current state
- MiniMax already had interleaved-priority logic.
- z.ai now has the same pattern (PR #40) and regression tests.
- Duplicate scan highlights overlap in:
  - `src/providers/minimax.rs`
  - `src/providers/zai.rs`

## Root cause pattern
When a chunk contains both:
- text/reasoning deltas, and
- tool-call continuation deltas (often id-less)

processing text/reasoning first can finalize the active `CurrentBlock::ToolCall` too early.

Correct behavior: if currently in a tool call and `delta.tool_calls` exists, process tool calls first.

## Refactor goal (DRY)
Extract shared interleaved-priority behavior so MiniMax and z.ai do not each reimplement it.

## Recommended approach (small, safe)
### 1) Extract shared helper
Move this decision logic to shared code (likely `src/providers/shared/stream_blocks.rs` or new shared module):

- `should_prioritize_tool_calls(current_block, tool_calls) -> bool`

Current duplicated condition:

```rust
matches!(current_block, Some(CurrentBlock::ToolCall { .. })) && tool_calls.is_some()
```

### 2) Keep provider-specific chunk details local
Do **not** force full unification of `process_chunk` yet.
MiniMax and z.ai differ in reasoning-source fields and fallback semantics.

### 3) Optional second pass
If desired, add a small shared helper for the common call order skeleton:
- pre-handle tool calls when prioritized
- handle provider-specific content/reasoning
- handle tool calls in normal order when not prioritized

Only do this if diff stays small and readability improves.

## Must-keep behavior
- z.ai tool-stream interleaving should preserve tool-call JSON assembly.
- MiniMax existing behavior/tests must remain unchanged.
- No regressions for id-less continuation handling.

## Test plan
Run at minimum:

```bash
cargo test providers::zai::tests::process_chunk_prioritizes_tool_call_continuations_before_content -- --nocapture
cargo test providers::zai::tests::process_chunk_prioritizes_tool_call_continuations_before_reasoning -- --nocapture
cargo test providers::minimax::tests::process_chunk_prioritizes_tool_call_continuations_before_text_fallback -- --nocapture
cargo test providers::minimax::tests::process_chunk_prioritizes_tool_call_continuations_before_reasoning_details -- --nocapture
cargo test --all-features
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
make complexity
make duplicates
```

## Done criteria
- Duplicated interleaved-priority logic between MiniMax and z.ai is removed/reduced.
- All provider regression tests above pass.
- `make duplicates` no longer reports this specific duplicate pair for interleaved priority logic.

## Notes
- PR #40 fixed correctness first; this handoff is for cleanup/refactor.
- Keep commits small and scoped to DRYing only (no API changes).