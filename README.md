# Alchemy

[![Crates.io](https://img.shields.io/crates/v/alchemy-llm.svg)](https://crates.io/crates/alchemy-llm)
[![Documentation](https://docs.rs/alchemy-llm/badge.svg)](https://docs.rs/alchemy-llm)
[![License: MIT](https://img.shields.io/crates/l/alchemy-llm.svg)](https://opensource.org/licenses/MIT)

A unified LLM API abstraction layer in Rust that supports 8+ providers through a consistent interface.

> **Warning:** This project is in early development (v0.1.x). APIs may change without notice. Not recommended for production use yet.

![Alchemy-rs](/assets/alchemy-rs-readme.png)

**Heavily inspired by and ported from:** [pi-mono/packages/ai](https://github.com/badlogic/pi-mono/tree/main/packages/ai)

## Supported Providers

- **Anthropic** (Claude)
- **OpenAI** (GPT-4, GPT-3.5)
- **Google** (Gemini)
- **AWS Bedrock**
- **Mistral**
- **xAI** (Grok)
- **Groq**
- **Cerebras**
- **OpenRouter**

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

## Latest Release

- **Crate:** [alchemy-llm on crates.io](https://crates.io/crates/alchemy-llm)
- **Docs:** [docs.rs/alchemy-llm](https://docs.rs/alchemy-llm)
- Current version: `0.1.3`
- Release notes: [CHANGELOG.md](./CHANGELOG.md#013---2026-02-12)
- Highlights:
  - Populate `usage.cost` from OpenAI-compatible streaming payloads
  - Fix doctest crate paths to `alchemy_llm`

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

## Development

See [AGENTS.md](./AGENTS.md) for detailed development guidelines, architecture, and quality gates.

### Quality Checks

Pre-commit hooks automatically run:
- `cargo fmt` - Code formatting
- `cargo clippy` - Linting with complexity checks
- `cargo check` - Compilation

Run all quality checks:
```bash
make quality-full     # All checks including complexity and duplicates
make quality-quick    # Fast checks (fmt, clippy, check)
make complexity       # Cyclomatic complexity analysis
make duplicates       # Duplicate code detection
```

Or run individually:
```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo check --all-targets --all-features
```

**Tools used:**
- **Clippy** - Cognitive complexity warnings (threshold: 20)
- **polydup** - Duplicate code detection (install: `cargo install polydup-cli`)

## License

MIT
