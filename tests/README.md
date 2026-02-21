# Integration Tests

This directory contains integration tests for the Alchemy LLM library.

## Running Tests

```bash
# Run all tests
cargo test

# Run only integration tests
cargo test --test '*'

# Run a specific integration test file
cargo test --test stream_integration
cargo test --test tool_calling_integration
cargo test --test transform_integration

# Run with output
cargo test -- --nocapture
```

## Test Categories

### `stream_integration.rs`
Tests for the streaming API including:
- Stream creation with and without API keys
- Options handling (temperature, max_tokens, headers)
- Context validation
- Error handling

### `tool_calling_integration.rs`
Tests for tool/function calling including:
- Tool definition and serialization
- Tool call ID generation and handling
- Tool result message construction
- Complete tool calling flow simulation

### `transform_integration.rs`
Tests for message transformation and types including:
- Provider detection
- Stop reason mapping
- Usage calculation
- Model and message construction

## Design Principles

1. **No API Keys Required**: Integration tests use mock endpoints (127.0.0.1:1) or test fixtures
2. **Fast Execution**: Tests complete quickly without network calls
3. **Comprehensive Coverage**: Tests cover the full flow from configuration to execution
4. **Deterministic**: Tests produce consistent results across runs

## Adding New Tests

When adding new integration tests:
1. Create a new file in `tests/` with the `_integration.rs` suffix
2. Add module documentation explaining what is being tested
3. Follow the existing test patterns for consistency
4. Ensure tests can run without API keys or environment variables
5. Run `cargo test` to verify all tests pass
