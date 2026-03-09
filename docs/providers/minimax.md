---
summary: "MiniMax provider guide based on the current first-class Rust implementation"
status: current
last_verified: 2026-03-09
---

# MiniMax Provider Guide

## Source Files

- `src/providers/minimax.rs`
- `src/models/minimax.rs`

## Built-In Models

Global endpoint:

- `minimax_m2_5()`
- `minimax_m2_5_highspeed()`
- `minimax_m2_1()`
- `minimax_m2_1_highspeed()`
- `minimax_m2()`

CN endpoint:

- `minimax_cn_m2_5()`
- `minimax_cn_m2_5_highspeed()`
- `minimax_cn_m2_1()`
- `minimax_cn_m2_1_highspeed()`
- `minimax_cn_m2()`

All built-in MiniMax models:

- use `Api::MinimaxCompletions`
- default `reasoning = true`
- use `InputType::Text`
- default to `context_window = 204_800`
- default to `max_tokens = 16_384`

## Request Semantics

The provider currently:

- always sends `stream: true`
- always sends `stream_options.include_usage: true`
- sends `reasoning_split: true` when the model has reasoning enabled
- forwards `max_tokens` from `OpenAICompletionsOptions`
- clamps temperature into `(0.0, 1.0]`
- reuses shared OpenAI-like tool conversion

## Streaming Behavior

MiniMax reasoning is emitted from the first non-empty source in this order:

1. `reasoning_details[*].text`
2. `reasoning_content`
3. `reasoning`
4. `reasoning_text`
5. `<think>...</think>` fallback parsing from `content`

When explicit reasoning is present, `content` is treated as normal text. When it is absent, the fallback parser splits reasoning and text from inline think tags.

## Current Gap

This checkout does not include the live smoke scripts referenced in older docs. Validate against source and tests rather than relying on script names from historical branches.
