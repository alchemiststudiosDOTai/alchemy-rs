---
summary: "Centralized error types with Result<T> alias and comprehensive error variants"
read_when:
  - You need to handle API errors, network failures, or validation issues
  - You want to understand what can go wrong in the alchemy_llm crate
  - You need to propagate errors using the ? operator
---

# Error Handling

Centralized error types for the `alchemy_llm` crate. All errors are represented by the `Error` enum, and `Result<T>` is a type alias for `std::result::Result<T, Error>`.

## Error Type

```rust
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("No API key provided for provider: {0}")]
    NoApiKey(String),

    #[error("HTTP request failed: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("API returned error: {status_code} - {message}")]
    ApiError { status_code: u16, message: String },

    #[error("Stream aborted")]
    Aborted,

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Invalid header: {0}")]
    InvalidHeader(String),

    #[error("Invalid JSON: {0}")]
    InvalidJson(#[from] serde_json::Error),

    #[error("Model not found: provider={provider}, model_id={model_id}")]
    ModelNotFound { provider: String, model_id: String },

    #[error("Unknown provider: {0}")]
    UnknownProvider(String),

    #[error("Unknown API: {0}")]
    UnknownApi(String),

    #[error("Tool validation failed: {0}")]
    ToolValidationFailed(String),

    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    #[error("Context overflow: model context window exceeded")]
    ContextOverflow,
}
```

## Result Type

```rust
pub type Result<T> = std::result::Result<T, Error>;
```

## Error Variants

| Variant | Description |
|---------|-------------|
| `NoApiKey` | No API key was found for the specified provider (check environment variables) |
| `RequestError` | HTTP request failed (network issues, timeout, etc.) |
| `ApiError` | Provider returned an error response with status code and message |
| `Aborted` | Stream was aborted by the client |
| `InvalidResponse` | Provider returned an unparseable response |
| `InvalidHeader` | Invalid header value in response |
| `InvalidJson` | JSON parsing failed |
| `ModelNotFound` | Model not found in registry |
| `UnknownProvider` | Provider not recognized |
| `UnknownApi` | API type not recognized |
| `ToolValidationFailed` | Tool call arguments failed schema validation |
| `ToolNotFound` | Tool name not found in available tools |
| `ContextOverflow` | Input exceeds model's context window |

## Usage Example

```rust
use alchemy_llm::{Error, Result};

fn make_request() -> Result<String> {
    // Returns Ok(String) or Err(Error)
    Ok("success".to_string())
}

match make_request() {
    Ok(response) => println!("Got: {}", response),
    Err(Error::NoApiKey(provider)) => {
        eprintln!("Set {}_API_KEY environment variable", provider);
    }
    Err(Error::ContextOverflow) => {
        eprintln!("Conversation too long, consider summarizing");
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

## Propagation

Use the `?` operator to propagate errors:

```rust
use alchemy_llm::{Error, Result};
use serde_json::Value;

fn parse_payload(payload: &str) -> Result<Value> {
    let parsed = serde_json::from_str(payload)?;
    Ok(parsed)
}
```
