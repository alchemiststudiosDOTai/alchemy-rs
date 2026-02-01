# Research – Python Bindings for alchemy-rs

**Date:** 2026-02-01
**Owner:** Claude
**Phase:** Research

## Goal

Research how to create Python bindings for the alchemy-rs Rust crate, covering tooling, architecture patterns, and implementation strategy.

## Findings

### Public API Surface to Expose

**Main Entry Points** (`src/lib.rs`):
- `stream()` - streaming completions
- `complete()` - blocking completion helper
- `transform_messages()` / `transform_messages_simple()` - cross-provider message transformation
- `get_env_api_key()` - API key resolution
- Utility functions for validation, JSON parsing, sanitization

**Core Types** (`src/types/`):
| Type | File | Purpose |
|------|------|---------|
| `Message` | `message.rs:8-12` | Tagged enum (User/Assistant/ToolResult) |
| `Context` | `message.rs:87-93` | Conversation state (system_prompt, messages, tools) |
| `AssistantMessage` | `message.rs:54-65` | LLM response with content blocks, usage, stop_reason |
| `Content` | `content.rs:6-27` | Tagged enum (Text/Thinking/Image/ToolCall) |
| `Tool` | `tool.rs:5-9` | Function schema (name, description, parameters) |
| `Usage`/`Cost` | `usage.rs` | Token counts and pricing |
| `Api`/`Provider` | `api.rs` | Provider enumeration |

**Streaming** (`src/stream/`):
- `AssistantMessageEventStream` - implements `futures::Stream<Item = AssistantMessageEvent>`
- `AssistantMessageEvent` - 10 event variants (Start, TextDelta, Done, Error, etc.)

**Provider Implementation** (`src/providers/`):
- Currently only `OpenAICompletions` is implemented
- Other APIs return "not yet implemented" errors

### FFI Challenges Identified

1. **Generic Types**: `Model<TApi>` is generic over API type - PyO3 cannot directly expose generics
   - **Solution**: Separate Python classes per API or enum-based dispatch

2. **Async Streaming**: `AssistantMessageEventStream` implements `futures::Stream`
   - **Solution**: Custom async iterator pattern with channels (not experimental `unstable-streams`)

3. **Nested Enums**: `Message`, `Content`, `UserContent` are tagged unions
   - **Solution**: PyO3 0.18+ enum support or flatten to dict with "type" discriminator

4. **serde_json::Value Fields**: Tool.parameters, ToolCall.arguments
   - **Solution**: Use `pythonize` crate or manual dict construction

5. **Lifetimes**: `stream()` and `complete()` take `&Model`, `&Context`
   - **Solution**: Clone data into owned values (both are `Clone`)

6. **Trait Objects**: `StreamOptions`, `CompatibilityOptions` traits
   - **Solution**: Use concrete types in Python API, hide trait dispatch internally

## Key Patterns / Solutions Found

### Recommended Tooling

| Tool | Purpose | Notes |
|------|---------|-------|
| **PyO3** | Rust-Python FFI | 15,000+ stars, de facto standard |
| **Maturin** | Build tool | Handles wheels, cross-platform, PyPI publishing |
| **pyo3-async-runtimes** | Async support | Bridges tokio and asyncio |

### Streaming Pattern (Critical for LLM APIs)

```rust
#[pyclass]
struct EventStreamIterator {
    receiver: mpsc::UnboundedReceiver<AssistantMessageEvent>,
}

#[pymethods]
impl EventStreamIterator {
    fn __aiter__(slf: PyRef<'_, Self>) -> PyRef<'_, Self> { slf }

    fn __anext__<'py>(&'py mut self, py: Python<'py>) -> PyResult<Option<Py<PyAny>>> {
        future_into_py(py, async move {
            match self.receiver.recv().await {
                Some(event) => Ok(Some(event_to_python(py, event)?)),
                None => Ok(None),  // StopAsyncIteration
            }
        })
    }
}
```

### Project Structure

```
alchemy-python/
├── Cargo.toml          # crate-type = ["cdylib"]
├── pyproject.toml      # maturin build backend
├── src/
│   ├── lib.rs          # #[pymodule] entry point
│   ├── types.rs        # Python class wrappers
│   ├── streaming.rs    # Async iterator implementations
│   └── error.rs        # Error conversions
├── alchemy/
│   ├── __init__.py
│   └── __init__.pyi    # Type stubs for IDE support
└── tests/
```

### Key Dependencies

```toml
[dependencies]
pyo3 = { version = "0.27", features = ["abi3-py38", "extension-module"] }
pyo3-async-runtimes = { version = "0.26", features = ["tokio-runtime"] }
tokio = { version = "1.0", features = ["full"] }
alchemy-llm = "0.1"  # This crate
```

### Error Handling Pattern

```rust
use pyo3::create_exception;
create_exception!(alchemy, AlchemyError, PyException);
create_exception!(alchemy, ModelNotFoundError, AlchemyError);
create_exception!(alchemy, ApiError, AlchemyError);

impl From<alchemy_llm::Error> for PyErr {
    fn from(err: alchemy_llm::Error) -> PyErr {
        match err {
            Error::ModelNotFound { .. } => ModelNotFoundError::new_err(err.to_string()),
            Error::ApiError { .. } => ApiError::new_err(err.to_string()),
            _ => AlchemyError::new_err(err.to_string()),
        }
    }
}
```

## Implementation Roadmap

| Phase | Goal | Python API |
|-------|------|------------|
| **1. Basic** | Sync completion | `complete(request) -> Response` |
| **2. Async** | Async/await | `await complete_async(request)` |
| **3. Streaming** | Async iteration | `async for event in stream(request)` |
| **4. Advanced** | Tools, caching | Tool schemas, structured outputs |
| **5. Production** | Polish | Type stubs, docs, CI/CD, PyPI |

## Real-World Examples

| Project | Pattern | Relevance |
|---------|---------|-----------|
| **tiktoken** (OpenAI) | Thin Rust core + Python wrapper | Same domain (LLM tooling) |
| **tokenizers** (HuggingFace) | Full API exposure with stubs | Production-grade example |
| **llm-rs-python** | LLM inference bindings | Direct LLM use case |

## Knowledge Gaps

- Whether to expose all 4 API types (Anthropic, OpenAI, Google, Bedrock) or just OpenAI initially
- Package naming: `alchemy-llm-python` vs `alchemy-py` vs `py-alchemy`
- Whether to maintain separate Python package repo or monorepo with bindings subfolder

## References

- [PyO3 User Guide](https://pyo3.rs/)
- [Maturin Documentation](https://www.maturin.rs/)
- [pyo3-async-runtimes](https://github.com/PyO3/pyo3-async-runtimes)
- [tiktoken](https://github.com/openai/tiktoken)
- [tokenizers](https://github.com/huggingface/tokenizers)
- `src/lib.rs` - Public API exports
- `src/types/` - Core type definitions
- `src/stream/event_stream.rs` - Streaming implementation
- `src/providers/openai_completions.rs` - Provider implementation pattern
