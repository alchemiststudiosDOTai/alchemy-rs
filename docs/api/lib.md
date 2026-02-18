---
summary: "Public API surface with crate modules, provider entry points, model constructors, and utility exports"
read_when:
  - You want to understand what `alchemy_llm` exports from `src/lib.rs`
  - You need the correct import path for stream functions and core types
  - You want to discover built-in MiniMax model constructors
---

# Public API (`src/lib.rs`)

`alchemy_llm` re-exports its main modules and top-level helpers from `src/lib.rs`.

## Module Layout

```text
src/
  lib.rs       # Public API re-exports
  error.rs     # Error enum + Result alias
  models/      # Built-in model constructors
  providers/   # Provider implementations
  stream/      # stream() + complete() entry points
  transform.rs # Cross-provider message transformation
  types/       # Core data structures
  utils/       # Shared helper utilities
```

## Top-Level Re-Exports

### Error

```rust
pub use error::{Error, Result};
```

### Model Constructors

```rust
pub use models::{
    minimax_cn_m2, minimax_cn_m2_1, minimax_cn_m2_1_highspeed, minimax_cn_m2_5,
    minimax_cn_m2_5_highspeed, minimax_m2, minimax_m2_1, minimax_m2_1_highspeed,
    minimax_m2_5, minimax_m2_5_highspeed,
};
```

### Provider Functions

```rust
pub use providers::{
    get_env_api_key,
    stream_minimax_completions,
    stream_openai_completions,
    OpenAICompletionsOptions,
};
```

### Stream Entry Points

```rust
pub use stream::{complete, stream, AssistantMessageEventStream};
```

### Transform

```rust
pub use transform::{transform_messages, transform_messages_simple, TargetModel};
```

### Utilities

```rust
pub use utils::{
    is_context_overflow,
    parse_streaming_json,
    parse_streaming_json_smart,
    sanitize_for_api,
    sanitize_surrogates,
    validate_tool_arguments,
    validate_tool_call,
    ThinkFragment,
    ThinkTagParser,
};
```

## Recommended Usage Pattern

Use `stream()` for provider-agnostic dispatch and built-in model constructors when available.

```rust
use alchemy_llm::{minimax_m2_5, stream, OpenAICompletionsOptions};
use alchemy_llm::types::{AssistantMessageEvent, Context, Message, UserContent, UserMessage};
use futures::StreamExt;

#[tokio::main]
async fn main() -> alchemy_llm::Result<()> {
    let model = minimax_m2_5();

    let context = Context {
        system_prompt: Some("You are concise.".to_string()),
        messages: vec![Message::User(UserMessage {
            content: UserContent::Text("Explain ownership in two bullets.".to_string()),
            timestamp: 0,
        })],
        tools: None,
    };

    let options = Some(OpenAICompletionsOptions {
        api_key: std::env::var("MINIMAX_API_KEY").ok(),
        ..OpenAICompletionsOptions::default()
    });

    let mut events = stream(&model, &context, options)?;

    while let Some(event) = events.next().await {
        if let AssistantMessageEvent::TextDelta { delta, .. } = event {
            print!("{delta}");
        }
    }

    Ok(())
}
```

## Notes

- `stream()` resolves API keys from options first, then environment variables via `get_env_api_key()`.
- `complete()` is a convenience wrapper around `stream()` that returns the final `AssistantMessage`.
- `types::*` contains the canonical cross-provider event/message contracts used by all providers.
