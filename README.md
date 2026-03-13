# Alchemy

[![Crates.io](https://img.shields.io/crates/v/alchemy-llm.svg)](https://crates.io/crates/alchemy-llm)
[![Documentation](https://docs.rs/alchemy-llm/badge.svg)](https://docs.rs/alchemy-llm)
[![License: MIT](https://img.shields.io/crates/l/alchemy-llm.svg)](https://opensource.org/licenses/MIT)

A unified LLM API abstraction layer in Rust that supports 13+ providers through a consistent interface.

> **Warning:** This project is in early development (v0.1.x). APIs may change without notice. Not recommended for production use yet.

![Alchemy-rs](/assets/alchemy-rs-readme.png)

**Heavily inspired by and ported from:** [pi-mono/packages/ai](https://github.com/badlogic/pi-mono/tree/main/packages/ai)

## Supported Providers

- **Anthropic** (Claude)
- **OpenAI** (GPT-4, GPT-3.5)
- **Featherless** (OpenAI-compatible catalog provider)
- **Google** (Gemini)
- **AWS Bedrock**
- **Mistral**
- **MiniMax** (Global)
- **MiniMax CN**
- **xAI** (Grok)
- **Groq**
- **Cerebras**
- **OpenRouter**
- **z.ai** (GLM)

> Current first-class streaming implementations in Rust: **OpenAI-compatible Completions** (including **OpenAI**, **OpenRouter**, and **Featherless**), **MiniMax Completions**, and **z.ai GLM Completions**. Other provider APIs are being ported incrementally.

## Features

- **Streaming-first** - All providers use async streams
- **Type-safe** - Leverages Rust's type system
- **Provider-agnostic** - Switch providers without code changes
- **Tool calling** - Function/tool support across providers
- **Message transformation** - Cross-provider message compatibility

## Installation

```bash
cargo add alchemy-llm
```

Or add to your `Cargo.toml`:

```toml
[dependencies]
alchemy-llm = "0.1"
```

## Quick Start

```rust
use alchemy_llm::stream;
use alchemy_llm::types::{
    AssistantMessageEvent, Context, InputType, KnownProvider, Message, Model, ModelCost,
    OpenAICompletions, Provider, UserContent, UserMessage,
};
use futures::StreamExt;

#[tokio::main]
async fn main() -> alchemy_llm::Result<()> {
    let model = Model::<OpenAICompletions> {
        id: "gpt-4o-mini".to_string(),
        name: "GPT-4o Mini".to_string(),
        api: OpenAICompletions,
        provider: Provider::Known(KnownProvider::OpenAI),
        base_url: "https://api.openai.com/v1".to_string(),
        reasoning: false,
        input: vec![InputType::Text],
        cost: ModelCost {
            input: 0.0,
            output: 0.0,
            cache_read: 0.0,
            cache_write: 0.0,
        },
        context_window: 128_000,
        max_tokens: 16_384,
        headers: None,
        compat: None,
    };

    let context = Context {
        messages: vec![Message::User(UserMessage {
            content: UserContent::Text("Hello!".to_string()),
            timestamp: 0,
        })],
        system_prompt: None,
        tools: None,
    };

    let mut stream = stream(&model, &context, None)?;

    while let Some(event) = stream.next().await {
        if let AssistantMessageEvent::TextDelta { delta, .. } = event {
            print!("{}", delta);
        }
    }

    Ok(())
}
```

### Featherless Quick Example

Featherless is available as a first-class provider identity while reusing the shared OpenAI-compatible runtime underneath. The public API stays the same: build a `Model<OpenAICompletions>`, then call `stream(...)` or `complete(...)`.

```rust
use alchemy_llm::{featherless_model, stream};
use alchemy_llm::types::{AssistantMessageEvent, Context, Message, UserContent, UserMessage};
use futures::StreamExt;

#[tokio::main]
async fn main() -> alchemy_llm::Result<()> {
    let model = featherless_model("moonshotai/Kimi-K2.5");
    let context = Context {
        system_prompt: None,
        messages: vec![Message::User(UserMessage {
            content: UserContent::Text("Hello from Featherless".to_string()),
            timestamp: 0,
        })],
        tools: None,
    };

    let mut stream = stream(&model, &context, None)?;

    while let Some(event) = stream.next().await {
        if let AssistantMessageEvent::TextDelta { delta, .. } = event {
            print!("{}", delta);
        }
    }

    Ok(())
}
```

Set `FEATHERLESS_API_KEY` in your environment.

The helper returns a default `Model<OpenAICompletions>` with:

- provider: `KnownProvider::Featherless`
- base URL: `https://api.featherless.ai/v1/chat/completions`
- default context window: `128_000`
- default max output tokens: `16_384`

Because Featherless exposes a dynamic catalog, you should treat those limits as safe defaults. If you fetch exact model metadata from `GET /v1/models`, override the returned `Model` fields before calling `stream(...)` or `complete(...)`.

## Latest Release

- **Crate:** [alchemy-llm on crates.io](https://crates.io/crates/alchemy-llm)
- **Docs:** [docs.rs/alchemy-llm](https://docs.rs/alchemy-llm)
- Current version: `0.1.8`
- Release notes: [CHANGELOG.md](./CHANGELOG.md#018---2026-03-13)
- Highlights:
  - Consolidated shared OpenAI-like request/stream runtime helpers across OpenAI-compatible, MiniMax, and z.ai providers
  - Deduplicated stream dispatch tests and enum string-mapping boilerplate while preserving behavior
## Setup

1. **Clone the repository**
   ```bash
   git clone https://github.com/alchemiststudiosDOTai/alchemy-rs.git
   cd alchemy-rs
   ```

2. **Configure API keys**
   ```bash
   cp .env.example .env
   # Edit .env and add your API keys
   ```

3. **Build the project**
   ```bash
   cargo build
   ```

4. **Run tests**
   ```bash
   cargo test
   ```

5. **Run the example**
   ```bash
   cargo run --example api_lifecycle
   ```

### Examples

| Example | Description |
|---------|-------------|
| `api_lifecycle` | Full API lifecycle demonstration |
| `simple_chat` | Basic chat with GPT-4o-mini |
| `tool_calling` | Tool/function calling with weather API |
| `minimax_live_reasoning_split` | Live MiniMax stream with `reasoning_split` enabled |
| `minimax_live_inline_think` | Live MiniMax stream exercising `<think>` fallback parsing |
| `minimax_live_usage_chunk` | Live MiniMax final message + usage summary |
| `zai_glm_simple_chat` | Live z.ai GLM chat with thinking/text event output |
| `zai_glm_tool_call_smoke` | Live z.ai GLM tool-call smoke for unified tool events |
| `tool_call_unified_types_smoke` | Cross-provider typed tool-call stream/output smoke |

## Documentation

- [docs/README.md](./docs/README.md) - Documentation index
- [docs/providers/architecture.md](./docs/providers/architecture.md) - Provider architecture contract for unified thinking, replay fidelity, and stream normalization
- [docs/providers/featherless.md](./docs/providers/featherless.md) - Featherless as a first-class provider on the shared OpenAI-compatible path

## Development

See [AGENTS.md](./AGENTS.md) for detailed development guidelines, architecture, and quality gates.

### Quality Checks

Pre-commit hooks automatically run:
- `cargo fmt` - Code formatting
- `cargo clippy` - Linting with complexity checks
- `cargo check` - Compilation

Run all quality checks:
```bash
make quality-full     # All checks including complexity, duplicates, and ast-rules
make quality-quick    # Fast checks (fmt, clippy, check)
make complexity       # Cyclomatic complexity analysis
make duplicates       # Duplicate code detection
make ast-rules        # Ast-grep architecture boundary checks
```

Or run individually:
```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo check --all-targets --all-features
make ast-rules
```

**Tools used:**
- **Clippy** - Cognitive complexity warnings (threshold: 20)
- **polydup** - Duplicate code detection (install: `cargo install polydup-cli`)
- **ast-grep (`sg`)** - Architecture boundary checks (`make ast-rules`)

## License

MIT
