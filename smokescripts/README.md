# Smoke Scripts

Live smoke scripts for provider integrations.

## Prerequisites

- `.env` at repo root or exported environment variables
- Internet access to provider endpoints

### MiniMax-only scripts

- `MINIMAX_API_KEY`

### Cross-provider unified tool-call script

- `OPENROUTER_API_KEY`
- `MINIMAX_API_KEY`
- `CHUTES_API_KEY`

## Scripts

### MiniMax provider scripts

- `smokescripts/run_minimax_reasoning_split.sh`  
  Runs `examples/minimax_live_reasoning_split.rs`.
- `smokescripts/run_minimax_inline_think.sh`  
  Runs `examples/minimax_live_inline_think.rs`.
- `smokescripts/run_minimax_usage_chunk.sh`  
  Runs `examples/minimax_live_usage_chunk.rs`.
- `smokescripts/run_all_minimax.sh`  
  Runs all three MiniMax smoke examples in sequence.

### Unified tool-call ID script (OpenRouter + MiniMax + Chutes)

- `smokescripts/run_tool_call_unified_types.sh`  
  Runs `examples/tool_call_unified_types_smoke.rs`.

This smoke proves unified typing and emits full typed stream output per provider by default:
- full `AssistantMessageEvent` debug output (`typed_event = ...`)
- full final typed `AssistantMessage` JSON (`typed_done_message_json = ...`)
- type lines:
  - `type(tool_call)`
  - `type(tool_call.id)`
  - `type(tool_result)`
  - `type(tool_result.tool_call_id)`

Defaults in `run_tool_call_unified_types.sh`:
- `TOOL_SMOKE_TYPES_ONLY=0`
- `TOOL_SMOKE_FULL_TYPED_RESPONSE=1`

For type-lines only mode:
- `TOOL_SMOKE_TYPES_ONLY=1`

## Usage

```bash
bash smokescripts/run_minimax_reasoning_split.sh
bash smokescripts/run_minimax_inline_think.sh
bash smokescripts/run_minimax_usage_chunk.sh
bash smokescripts/run_all_minimax.sh

bash smokescripts/run_tool_call_unified_types.sh
```

### Optional prompt/model overrides

```bash
MINIMAX_PROMPT="Explain borrow checker in 4 bullets" bash smokescripts/run_minimax_reasoning_split.sh
MINIMAX_INLINE_PROMPT="Think step by step then answer: 21*19" bash smokescripts/run_minimax_inline_think.sh
MINIMAX_USAGE_PROMPT="Summarize Rust async in one stanza" bash smokescripts/run_minimax_usage_chunk.sh

TOOL_SMOKE_PROMPT="Call get_weather for Tokyo exactly once" bash smokescripts/run_tool_call_unified_types.sh
OPENROUTER_MODEL="anthropic/claude-3.5-sonnet" CHUTES_MODEL="deepseek-ai/DeepSeek-V3-0324" bash smokescripts/run_tool_call_unified_types.sh
```
