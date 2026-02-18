# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [Unreleased]

### Added
- First-class MiniMax provider via `Api::MinimaxCompletions` and `stream_minimax_completions`
- Built-in MiniMax model constructors for global and CN endpoints
- Live MiniMax examples and smoke scripts for reasoning split, `<think>` fallback, and usage chunks
- New MiniMax provider guide: `docs/providers/minimax.md`
- Documentation index: `docs/README.md`

### Fixed
- MiniMax temperature is clamped to supported range `(0.0, 1.0]`
- Assistant thinking replay preserves `<think>...</think>` wrapping semantics

## [0.1.3] - 2026-02-12

### Fixed
- `openai_completions`: Populate `usage.cost` from OpenRouter/OpenAI-compatible streaming usage payloads (`cost` and `cost_details`)
- `docs`: Fix doctest crate paths to `alchemy_llm` so doctests compile during `cargo test`

## [0.1.2] - 2026-02-12

### Added
- `examples/simple_chat.rs` - Basic chat example using GPT-4o-mini
- `examples/tool_calling.rs` - Tool/function calling demonstration with weather API example

### Fixed
- `openai`: Align usage semantics with provider raw tokens

## [0.1.1] - 2026-02-12

### Added
- Initial crates.io release
- Deployment documentation with crate-publisher skill reference

## [0.1.0] - 2026-02-11

### Added
- Initial release
- Support for 8+ providers: Anthropic, OpenAI, Google, AWS Bedrock, Mistral, xAI, Groq, Cerebras, OpenRouter
- Streaming-first async API
- Type-safe provider abstraction
- Tool calling across providers
- Message transformation for cross-provider compatibility
