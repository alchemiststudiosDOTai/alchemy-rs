# Alchemy

A unified LLM API abstraction layer in Rust that supports 8+ providers through a consistent interface.

**Heavily inspired by and ported from:** [pi-mono/packages/ai](https://github.com/badlogic/pi-mono/tree/main/packages/ai) by [@badlogic](https://github.com/badlogic)

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

## Quick Start

```rust
use alchemy::{stream, types::*};
use futures::StreamExt;

#[tokio::main]
async fn main() -> alchemy::Result<()> {
    let model = alchemy::get_model("claude-sonnet-4-20250514")
        .ok_or("Model not found")?;

    let context = Context {
        messages: vec![Message::user("Hello, Claude!")],
        ..Default::default()
    };

    let mut stream = stream(&model, &context, &[])?;

    while let Some(event) = stream.next().await {
        println!("{:?}", event);
    }

    Ok(())
}
```

## Setup

1. **Clone the repository**
   ```bash
   git clone https://github.com/tunahorse/alchemy-rs.git
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
