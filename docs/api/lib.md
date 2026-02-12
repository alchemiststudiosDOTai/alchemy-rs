---
summary: "Public API surface with re-exports of all crate modules and entry points"
read_when:
  - You want to understand what's available in the alchemy_llm crate
  - You need to find the correct import for a type or function
  - You're getting started with Alchemy and need an overview
---

# Public API

The `src/lib.rs` file defines the public API surface for the `alchemy_llm` crate. All publicly exported types and functions are re-exported here for convenience.

## Module Structure

```
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
pub use utils::{
    is_context_overflow,         // Detect context overflow
    parse_streaming_json,        // Parse incomplete JSON
    parse_streaming_json_smart,  // Smart partial JSON parsing
    sanitize_for_api,            // Sanitize strings for API
    sanitize_surrogates,         // Remove UTF-16 surrogates
    validate_tool_arguments,     // Validate tool call arguments
    validate_tool_call,          // Validate tool against tools
};
```

## Quick Start

```rust
use alchemy_llm::{
    stream,
    types::{
        AssistantMessageEvent, Context, InputType, KnownProvider, Message, Model, ModelCost,
        OpenAICompletions, Provider, UserContent, UserMessage,
    },
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
        system_prompt: None,
        messages: vec![Message::User(UserMessage {
            content: UserContent::Text("Hello!".to_string()),
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

## Pattern: Streaming Completion

```rust
use alchemy_llm::{stream, types::*};
use futures::StreamExt;

async fn chat(model: &Model<OpenAICompletions>, messages: Vec<Message>) -> Result<String> {
    let context = Context {
        system_prompt: None,
        messages,
        tools: None,
    };
    let mut stream = stream(model, &context, None)?;

    let mut response = String::new();
    while let Some(event) = stream.next().await {
        if let AssistantMessageEvent::TextDelta { delta, .. } = event {
            response.push_str(&delta);
            print!("{}", delta);
        }
    }

    Ok(response)
}
```

## Pattern: Cross-Provider Transformation

```rust
use alchemy_llm::transform_messages_simple;
use alchemy_llm::types::{Api, KnownProvider, Message, Provider};
use alchemy_llm::TargetModel;

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
use alchemy_llm::validate_tool_call;
use alchemy_llm::types::{Tool, ToolCall};
use alchemy_llm::Result;

fn check_tool_call(tools: &[Tool], call: &ToolCall) -> Result<()> {
    let validated_args = validate_tool_call(tools, call)?;
    println!("Validated: {}", validated_args);
    Ok(())
}
```
