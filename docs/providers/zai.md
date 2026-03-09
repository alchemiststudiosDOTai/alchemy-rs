---
summary: "z.ai provider guide based on the current first-class Rust implementation"
status: current
last_verified: 2026-03-09
---

# z.ai Provider Guide

## Source Files

- `src/providers/zai.rs`
- `src/models/zai.rs`

## Built-In Models

200K context / 128K max output:

- `glm_5()`
- `glm_4_7()`
- `glm_4_7_flash()`
- `glm_4_7_flashx()`
- `glm_4_6()`

128K context / 96K max output:

- `glm_4_5()`
- `glm_4_5_air()`
- `glm_4_5_x()`
- `glm_4_5_airx()`
- `glm_4_5_flash()`

128K context / 16K max output:

- `glm_4_32b_0414_128k()`

All built-in z.ai models:

- use `Api::ZaiCompletions`
- use provider `KnownProvider::Zai`
- default `reasoning = true`
- use `InputType::Text`
- use base URL `https://api.z.ai/api/paas/v4/chat/completions`

## Request Semantics

The provider currently:

- always sends `stream: true`
- always sends `stream_options.include_usage: true`
- emits assistant history as string content with `reasoning_content` for replay
- uses `options.zai.max_tokens` before falling back to `options.max_tokens`
- forwards z.ai-specific optional fields such as `do_sample`, `top_p`, `stop`, `tool_stream`, `request_id`, `user_id`, `response_format`, and `thinking`
- enables default reasoning when the model supports reasoning and no explicit `thinking` option is provided

## Streaming Behavior

Reasoning is mapped in this order:

1. `reasoning_content`
2. `reasoning`
3. `reasoning_text`

Tool calls and stop reasons are normalized into the shared event contract alongside text deltas.

## Current Gap

This checkout does not include the z.ai example binaries referenced in older docs. Validate against source and tests rather than historical example names.
