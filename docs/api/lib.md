---
summary: "Public API surface with re-exports of all crate modules and entry points"
read_when:
  - You want to understand what's available in the alchemy crate
  - You need to find the correct import for a type or function
  - You're getting started with Alchemy and need an overview
---

# Public API

The `lib.rs` file defines the public API surface for the Alchemy crate. All publicly exported types and functions are re-exported here for convenience.

## Module Structure

```
alchemy/
  src/
    lib.rs       # Public API re-exports
    error.rs     # Error types
    providers/   # Provider implementations
    stream/      # Streaming API
    transform.rs # Message transformation
    types/       # Core type definitions
    utils/       # Utility functions
```

## Public Exports

### Error Types

```rust
pub use error::{Error, Result};
```

### Provider Functions

```rust
pub use providers::{
    get_env_api_key,           // Get API key from environment
    stream_openai_completions, // OpenAI completions streaming
    OpenAICompletionsOptions,  // Options for OpenAI completions
};
```

### Stream API

```rust
pub use stream::{
    complete,                      // Non-streaming completion
    stream,                        // Streaming completion
    AssistantMessageEventStream,   // Event stream type
};
```

### Transform API

```rust
pub use transform::{
    transform_messages,        // Transform messages for target model
    transform_messages_simple, // Transform without ID normalization
    TargetModel,               // Target model information
};
```

### Utilities

```rust
pub use types::{
    is_context_overflow,     // Detect context overflow
    validate_tool_arguments, // Validate tool call arguments
    validate_tool_call,      // Validate tool against tools
};

pub use utils::{
    parse_streaming_json,       // Parse incomplete JSON
    parse_streaming_json_smart, // Smart partial JSON parsing
    sanitize_for_api,           // Sanitize strings for API
    sanitize_surrogates,        // Remove UTF-16 surrogates
};
```

## Quick Start

```rust
use alchemy::{
    stream,
    types::{Context, Message, UserContent},
};

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

## Pattern: Streaming Completion

```rust
use alchemy::{stream, types::*};
use futures::StreamExt;

async fn chat(messages: Vec<Message>) -> Result<AssistantMessage> {
    let model = get_model("gpt-4o")?;
    let context = Context { messages, ..Default::default() };
    let mut stream = stream(&model, &context, &[])?;

    let mut response = String::new();
    while let Some(event) = stream.next().await {
        if let AssistantMessageEvent::Content(delta) = event {
            response.push_str(&delta);
            print!("{}", delta);
        }
    }

    Ok(AssistantMessage {
        content: vec![Content::text(response)],
        ..Default::default()
    })
}
```

## Pattern: Cross-Provider Transformation

```rust
use alchemy::{transform_messages, types::*};

fn switch_provider(messages: Vec<Message>) -> Vec<Message> {
    let target = TargetModel {
        api: Api::OpenAICompletions,
        provider: Provider::Known(KnownProvider::OpenAI),
        model_id: "gpt-4o".to_string(),
    };

    transform_messages_simple(&messages, &target)
}
```

## Pattern: Tool Validation

```rust
use alchemy::{validate_tool_call, types::*};
use serde_json::json;

fn check_tool_call(tools: &[Tool], call: &ToolCall) -> Result<()> {
    let validated_args = validate_tool_call(tools, call)?;
    println!("Validated: {}", validated_args);
    Ok(())
}
```
