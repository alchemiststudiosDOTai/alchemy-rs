# MiniMax Smoke Scripts

Live smoke scripts for the first-class MiniMax provider.

## Prerequisites

- `MINIMAX_API_KEY` must be set (or present in `.env` at repo root).
- Internet access to `https://api.minimax.io`.

## Scripts

- `smokescripts/run_minimax_reasoning_split.sh`  
  Runs `examples/minimax_live_reasoning_split.rs`.
- `smokescripts/run_minimax_inline_think.sh`  
  Runs `examples/minimax_live_inline_think.rs`.
- `smokescripts/run_minimax_usage_chunk.sh`  
  Runs `examples/minimax_live_usage_chunk.rs`.
- `smokescripts/run_all_minimax.sh`  
  Runs all three smoke examples in sequence.

## Usage

```bash
bash smokescripts/run_minimax_reasoning_split.sh
bash smokescripts/run_minimax_inline_think.sh
bash smokescripts/run_minimax_usage_chunk.sh
bash smokescripts/run_all_minimax.sh
```

### Optional prompt overrides

```bash
MINIMAX_PROMPT="Explain borrow checker in 4 bullets" bash smokescripts/run_minimax_reasoning_split.sh
MINIMAX_INLINE_PROMPT="Think step by step then answer: 21*19" bash smokescripts/run_minimax_inline_think.sh
MINIMAX_USAGE_PROMPT="Summarize Rust async in one stanza" bash smokescripts/run_minimax_usage_chunk.sh
```
