# Phase 5: Cross-Provider Features

**Status:** Not Started
**Estimated Files:** 4-5 Rust modules
**Source Reference:** `ai/src/providers/transform-messages.ts`, `ai/src/utils/validation.ts`, `ai/src/utils/overflow.ts`

---

## Objective

Implement cross-provider utilities that ensure consistent behavior when:
- Switching models mid-conversation
- Validating tool calls
- Detecting context overflow
- Handling image content across providers

---

## Prerequisites

- Phase 4 complete (all providers implemented)

---

## Components to Implement

### 5.1 Message Transformation

**Source:** `ai/src/providers/transform-messages.ts` (167 lines)

**Purpose:** Transform conversation history for cross-provider compatibility when switching models.

#### Key Transformations

1. **Thinking Block Handling**
   - Same model/provider: Keep thinking with signature for replay
   - Different model: Convert thinking to plain text (without `<thinking>` tags)
   - Empty thinking: Filter out completely

2. **Tool Call ID Normalization**
   - Each provider has different ID requirements
   - Build mapping: original ID -> normalized ID
   - Apply mapping to subsequent tool results

3. **Orphaned Tool Call Handling**
   - If assistant has tool calls but no results follow, insert synthetic error results
   - This satisfies API requirements that tool calls must have results

4. **Error/Abort Filtering**
   - Skip assistant messages with `stopReason: "error"` or `"aborted"`
   - These are incomplete turns that shouldn't be replayed

**Rust Target:** `src/transform.rs`

```rust
use crate::types::{
    Api, AssistantMessage, Content, Message, Model, Provider,
    TextContent, ThinkingContent, ToolCall, ToolResultMessage,
};
use std::collections::{HashMap, HashSet};

/// Transform messages for cross-provider compatibility.
///
/// Handles:
/// - Thinking block conversion (signature preservation vs text conversion)
/// - Tool call ID normalization
/// - Orphaned tool call handling (synthetic error results)
/// - Error/aborted message filtering
pub fn transform_messages<TApi: crate::types::ApiType>(
    messages: &[Message],
    model: &Model<TApi>,
    normalize_tool_call_id: Option<&dyn Fn(&str, &Model<TApi>, &AssistantMessage) -> String>,
) -> Vec<Message> {
    let mut tool_call_id_map: HashMap<String, String> = HashMap::new();

    // First pass: transform messages
    let transformed: Vec<Message> = messages
        .iter()
        .filter_map(|msg| {
            transform_message(msg, model, normalize_tool_call_id, &mut tool_call_id_map)
        })
        .collect();

    // Second pass: insert synthetic tool results for orphaned calls
    insert_synthetic_tool_results(transformed)
}

fn transform_message<TApi: crate::types::ApiType>(
    msg: &Message,
    model: &Model<TApi>,
    normalize_fn: Option<&dyn Fn(&str, &Model<TApi>, &AssistantMessage) -> String>,
    id_map: &mut HashMap<String, String>,
) -> Option<Message> {
    match msg {
        Message::User(user) => Some(Message::User(user.clone())),

        Message::ToolResult(result) => {
            // Apply ID mapping if exists
            let tool_call_id = id_map
                .get(&result.tool_call_id)
                .cloned()
                .unwrap_or_else(|| result.tool_call_id.clone());

            Some(Message::ToolResult(ToolResultMessage {
                tool_call_id,
                ..result.clone()
            }))
        }

        Message::Assistant(assistant) => {
            // Skip errored/aborted messages
            if matches!(
                assistant.stop_reason,
                crate::types::StopReason::Error | crate::types::StopReason::Aborted
            ) {
                return None;
            }

            let is_same_model = is_same_model_provider(assistant, model);

            let content = assistant
                .content
                .iter()
                .filter_map(|block| {
                    transform_content_block(block, is_same_model, model, assistant, normalize_fn, id_map)
                })
                .collect();

            Some(Message::Assistant(AssistantMessage {
                content,
                ..assistant.clone()
            }))
        }
    }
}

fn is_same_model_provider<TApi: crate::types::ApiType>(
    msg: &AssistantMessage,
    model: &Model<TApi>,
) -> bool {
    msg.provider == model.provider
        && msg.api == model.api.api()
        && msg.model == model.id
}

fn transform_content_block<TApi: crate::types::ApiType>(
    block: &Content,
    is_same_model: bool,
    model: &Model<TApi>,
    assistant: &AssistantMessage,
    normalize_fn: Option<&dyn Fn(&str, &Model<TApi>, &AssistantMessage) -> String>,
    id_map: &mut HashMap<String, String>,
) -> Option<Content> {
    match block {
        Content::Thinking { inner } => {
            // Same model with signature: keep for replay
            if is_same_model && inner.thinking_signature.is_some() {
                return Some(block.clone());
            }

            // Empty thinking: filter out
            if inner.thinking.trim().is_empty() {
                return None;
            }

            // Same model without signature: keep as-is
            if is_same_model {
                return Some(block.clone());
            }

            // Different model: convert to plain text (no <thinking> tags)
            Some(Content::text(&inner.thinking))
        }

        Content::Text { inner } => {
            if is_same_model {
                Some(block.clone())
            } else {
                // Strip signature if present
                Some(Content::Text {
                    inner: TextContent {
                        text: inner.text.clone(),
                        text_signature: None,
                    },
                })
            }
        }

        Content::ToolCall { inner } => {
            let mut new_call = inner.clone();

            // Strip thought signature for different model
            if !is_same_model {
                new_call.thought_signature = None;
            }

            // Normalize ID for different model
            if !is_same_model {
                if let Some(normalize) = normalize_fn {
                    let normalized_id = normalize(&inner.id, model, assistant);
                    if normalized_id != inner.id {
                        id_map.insert(inner.id.clone(), normalized_id.clone());
                        new_call.id = normalized_id;
                    }
                }
            }

            Some(Content::ToolCall { inner: new_call })
        }

        Content::Image { .. } => Some(block.clone()),
    }
}

fn insert_synthetic_tool_results(messages: Vec<Message>) -> Vec<Message> {
    let mut result: Vec<Message> = Vec::new();
    let mut pending_tool_calls: Vec<ToolCall> = Vec::new();
    let mut existing_result_ids: HashSet<String> = HashSet::new();

    for msg in messages {
        match &msg {
            Message::Assistant(assistant) => {
                // Insert synthetic results for previous orphaned calls
                insert_orphaned_results(&mut result, &pending_tool_calls, &existing_result_ids);
                pending_tool_calls.clear();
                existing_result_ids.clear();

                // Track tool calls from this message
                for content in &assistant.content {
                    if let Content::ToolCall { inner } = content {
                        pending_tool_calls.push(inner.clone());
                    }
                }

                result.push(msg);
            }

            Message::ToolResult(tool_result) => {
                existing_result_ids.insert(tool_result.tool_call_id.clone());
                result.push(msg);
            }

            Message::User(_) => {
                // User message interrupts tool flow
                insert_orphaned_results(&mut result, &pending_tool_calls, &existing_result_ids);
                pending_tool_calls.clear();
                existing_result_ids.clear();

                result.push(msg);
            }
        }
    }

    result
}

fn insert_orphaned_results(
    result: &mut Vec<Message>,
    pending: &[ToolCall],
    existing: &HashSet<String>,
) {
    for tc in pending {
        if !existing.contains(&tc.id) {
            result.push(Message::ToolResult(ToolResultMessage {
                tool_call_id: tc.id.clone(),
                tool_name: tc.name.clone(),
                content: vec![crate::types::ToolResultContent::Text(TextContent {
                    text: "No result provided".to_string(),
                    text_signature: None,
                })],
                details: None,
                is_error: true,
                timestamp: current_timestamp(),
            }));
        }
    }
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
```

---

### 5.2 Tool Validation

**Source:** `ai/src/utils/validation.ts` (84 lines)

**Purpose:** Validate tool call arguments against JSON schema.

**Rust Target:** `src/utils/validation.rs`

```rust
use crate::types::{Tool, ToolCall};
use crate::error::{Error, Result};
use jsonschema::{Draft, JSONSchema};

/// Validate a tool call against available tools.
///
/// Finds the matching tool by name and validates arguments against its schema.
pub fn validate_tool_call(tools: &[Tool], tool_call: &ToolCall) -> Result<serde_json::Value> {
    let tool = tools
        .iter()
        .find(|t| t.name == tool_call.name)
        .ok_or_else(|| Error::ToolNotFound(tool_call.name.clone()))?;

    validate_tool_arguments(tool, tool_call)
}

/// Validate tool call arguments against the tool's JSON schema.
///
/// Returns the validated (and potentially coerced) arguments.
pub fn validate_tool_arguments(tool: &Tool, tool_call: &ToolCall) -> Result<serde_json::Value> {
    let schema = JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(&tool.parameters)
        .map_err(|e| Error::ToolValidationFailed(format!("Schema compile error: {}", e)))?;

    let args = &tool_call.arguments;

    let validation_result = schema.validate(args);
    if let Err(errors) = validation_result {
        let error_messages: Vec<String> = errors
            .map(|err| {
                let path = err.instance_path.to_string();
                let path = if path.is_empty() { "root".to_string() } else { path };
                format!("  - {}: {}", path, err)
            })
            .collect();

        let error_msg = format!(
            "Validation failed for tool \"{}\":\n{}\n\nReceived arguments:\n{}",
            tool_call.name,
            error_messages.join("\n"),
            serde_json::to_string_pretty(args).unwrap_or_default()
        );

        return Err(Error::ToolValidationFailed(error_msg));
    }

    // Return a clone (jsonschema doesn't do coercion like AJV)
    Ok(args.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_validate_valid_args() {
        let tool = Tool {
            name: "get_weather".to_string(),
            description: "Get weather".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "location": { "type": "string" }
                },
                "required": ["location"]
            }),
        };

        let tool_call = ToolCall {
            id: "123".to_string(),
            name: "get_weather".to_string(),
            arguments: json!({ "location": "NYC" }),
            thought_signature: None,
        };

        let result = validate_tool_arguments(&tool, &tool_call);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_missing_required() {
        let tool = Tool {
            name: "get_weather".to_string(),
            description: "Get weather".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "location": { "type": "string" }
                },
                "required": ["location"]
            }),
        };

        let tool_call = ToolCall {
            id: "123".to_string(),
            name: "get_weather".to_string(),
            arguments: json!({}),
            thought_signature: None,
        };

        let result = validate_tool_arguments(&tool, &tool_call);
        assert!(result.is_err());
    }
}
```

---

### 5.3 Context Overflow Detection

**Source:** `ai/src/utils/overflow.ts` (118 lines)

**Purpose:** Detect when input exceeds model's context window, either via error or silent acceptance.

**Rust Target:** `src/utils/overflow.rs`

```rust
use crate::types::AssistantMessage;
use once_cell::sync::Lazy;
use regex::Regex;

/// Patterns to detect context overflow errors from different providers.
static OVERFLOW_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        Regex::new(r"(?i)prompt is too long").unwrap(),                      // Anthropic
        Regex::new(r"(?i)input is too long for requested model").unwrap(),   // Amazon Bedrock
        Regex::new(r"(?i)exceeds the context window").unwrap(),              // OpenAI
        Regex::new(r"(?i)input token count.*exceeds the maximum").unwrap(),  // Google Gemini
        Regex::new(r"(?i)maximum prompt length is \d+").unwrap(),            // xAI Grok
        Regex::new(r"(?i)reduce the length of the messages").unwrap(),       // Groq
        Regex::new(r"(?i)maximum context length is \d+ tokens").unwrap(),    // OpenRouter
        Regex::new(r"(?i)exceeds the limit of \d+").unwrap(),                // GitHub Copilot
        Regex::new(r"(?i)exceeds the available context size").unwrap(),      // llama.cpp
        Regex::new(r"(?i)greater than the context length").unwrap(),         // LM Studio
        Regex::new(r"(?i)context window exceeds limit").unwrap(),            // MiniMax
        Regex::new(r"(?i)context[_ ]length[_ ]exceeded").unwrap(),           // Generic
        Regex::new(r"(?i)too many tokens").unwrap(),                         // Generic
        Regex::new(r"(?i)token limit exceeded").unwrap(),                    // Generic
    ]
});

/// Pattern for providers that return status codes without body (Cerebras, Mistral)
static STATUS_CODE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)^4(00|13|29)\s*(status code)?\s*\(no body\)").unwrap()
});

/// Check if an assistant message represents a context overflow error.
///
/// Handles two cases:
/// 1. **Error-based overflow**: Provider returns error with detectable message
/// 2. **Silent overflow**: Provider accepts but usage.input > context_window
///
/// # Arguments
///
/// * `message` - The assistant message to check
/// * `context_window` - Optional context window size for detecting silent overflow
///
/// # Provider Reliability
///
/// **Reliable detection:**
/// - Anthropic, OpenAI, Google Gemini, xAI, Groq, Cerebras, Mistral,
///   OpenRouter, llama.cpp, LM Studio
///
/// **Unreliable detection:**
/// - z.ai: Sometimes accepts silently (pass context_window to detect)
/// - Ollama: Silently truncates input (cannot detect)
///
/// # Example
///
/// ```rust
/// use alchemy::utils::overflow::is_context_overflow;
///
/// if is_context_overflow(&message, Some(200_000)) {
///     // Handle overflow - maybe summarize conversation
/// }
/// ```
pub fn is_context_overflow(message: &AssistantMessage, context_window: Option<u32>) -> bool {
    // Case 1: Check error message patterns
    if message.stop_reason == crate::types::StopReason::Error {
        if let Some(ref error_msg) = message.error_message {
            // Check known patterns
            if OVERFLOW_PATTERNS.iter().any(|p| p.is_match(error_msg)) {
                return true;
            }

            // Check for status code pattern (Cerebras, Mistral)
            if STATUS_CODE_PATTERN.is_match(error_msg) {
                return true;
            }
        }
    }

    // Case 2: Silent overflow (z.ai style)
    if let Some(window) = context_window {
        if message.stop_reason == crate::types::StopReason::Stop {
            let input_tokens = message.usage.input + message.usage.cache_read;
            if input_tokens > window {
                return true;
            }
        }
    }

    false
}

/// Get the overflow patterns (for testing).
pub fn get_overflow_patterns() -> &'static [Regex] {
    &OVERFLOW_PATTERNS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Api, Provider, StopReason, Usage, Cost};

    fn make_message(stop_reason: StopReason, error_message: Option<&str>, input: u32) -> AssistantMessage {
        AssistantMessage {
            content: vec![],
            api: Api::AnthropicMessages,
            provider: Provider::Known(crate::types::KnownProvider::Anthropic),
            model: "test".to_string(),
            usage: Usage {
                input,
                output: 0,
                cache_read: 0,
                cache_write: 0,
                total_tokens: input,
                cost: Cost::default(),
            },
            stop_reason,
            error_message: error_message.map(String::from),
            timestamp: 0,
        }
    }

    #[test]
    fn test_anthropic_overflow() {
        let msg = make_message(
            StopReason::Error,
            Some("prompt is too long: 213462 tokens > 200000 maximum"),
            213462,
        );
        assert!(is_context_overflow(&msg, None));
    }

    #[test]
    fn test_openai_overflow() {
        let msg = make_message(
            StopReason::Error,
            Some("Your input exceeds the context window of this model"),
            100000,
        );
        assert!(is_context_overflow(&msg, None));
    }

    #[test]
    fn test_silent_overflow() {
        let msg = make_message(StopReason::Stop, None, 250000);
        assert!(is_context_overflow(&msg, Some(200000)));
        assert!(!is_context_overflow(&msg, Some(300000)));
    }

    #[test]
    fn test_no_overflow() {
        let msg = make_message(StopReason::Stop, None, 50000);
        assert!(!is_context_overflow(&msg, Some(200000)));
    }
}
```

---

### 5.4 Unicode Sanitization

**Source:** `ai/src/utils/sanitize-unicode.ts` (25 lines)

**Purpose:** Remove unpaired surrogates that can cause API errors.

**Rust Target:** `src/utils/sanitize.rs`

```rust
/// Remove unpaired UTF-16 surrogates from a string.
///
/// Some APIs reject strings containing unpaired surrogates (0xD800-0xDFFF).
/// Rust strings are valid UTF-8, so this mainly handles edge cases from
/// external data.
///
/// Note: Rust's `String` type guarantees valid UTF-8, so unpaired surrogates
/// shouldn't normally occur. This is primarily for defensive sanitization
/// of external input.
pub fn sanitize_surrogates(s: &str) -> String {
    // Rust strings are always valid UTF-8, so unpaired surrogates
    // can only exist if we're dealing with potentially malformed input.
    // For safety, we filter any replacement characters that might indicate
    // surrogate issues.
    s.chars()
        .filter(|c| *c != '\u{FFFD}')  // Remove replacement characters
        .collect()
}

/// Sanitize a string for use in API requests.
///
/// Combines surrogate sanitization with other common transformations.
pub fn sanitize_for_api(s: &str) -> String {
    sanitize_surrogates(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_string() {
        assert_eq!(sanitize_surrogates("Hello, world!"), "Hello, world!");
    }

    #[test]
    fn test_unicode_string() {
        assert_eq!(sanitize_surrogates("Hello, \u{1F600}!"), "Hello, \u{1F600}!");
    }

    #[test]
    fn test_replacement_character() {
        // Replacement character gets filtered
        assert_eq!(sanitize_surrogates("Hello\u{FFFD}World"), "HelloWorld");
    }
}
```

---

### 5.5 Partial JSON Parsing

**Source:** `ai/src/utils/json-parse.ts` (28 lines)

**Purpose:** Parse incomplete JSON during streaming tool call arguments.

**Rust Target:** `src/utils/json_parse.rs`

```rust
use serde_json::Value;

/// Parse potentially incomplete JSON from streaming tool calls.
///
/// During streaming, tool call arguments arrive incrementally. This function
/// attempts to parse the partial JSON, returning an empty object if parsing fails.
///
/// # Strategy
///
/// 1. Try parsing as complete JSON
/// 2. Try adding various closing brackets/braces
/// 3. Fall back to empty object
///
/// # Example
///
/// ```rust
/// use alchemy::utils::json_parse::parse_streaming_json;
///
/// let partial = r#"{"name": "test", "value": 42"#;
/// let result = parse_streaming_json(partial);
/// // Returns {"name": "test", "value": 42} after adding closing brace
/// ```
pub fn parse_streaming_json(s: &str) -> Value {
    // Try complete parse first
    if let Ok(v) = serde_json::from_str(s) {
        return v;
    }

    // Try common completions
    let completions = [
        "}",
        "}}",
        "\"}",
        "\"}}",
        "null}",
        "null}}",
        "]}",
        "]}}",
    ];

    for suffix in completions {
        let attempt = format!("{}{}", s, suffix);
        if let Ok(v) = serde_json::from_str(&attempt) {
            return v;
        }
    }

    // Fall back to empty object
    Value::Object(Default::default())
}

/// More sophisticated partial JSON parsing using bracket counting.
pub fn parse_streaming_json_smart(s: &str) -> Value {
    // Try complete parse first
    if let Ok(v) = serde_json::from_str(s) {
        return v;
    }

    // Count unclosed brackets
    let mut brace_count = 0i32;
    let mut bracket_count = 0i32;
    let mut in_string = false;
    let mut escape_next = false;

    for c in s.chars() {
        if escape_next {
            escape_next = false;
            continue;
        }

        match c {
            '\\' if in_string => escape_next = true,
            '"' => in_string = !in_string,
            '{' if !in_string => brace_count += 1,
            '}' if !in_string => brace_count -= 1,
            '[' if !in_string => bracket_count += 1,
            ']' if !in_string => bracket_count -= 1,
            _ => {}
        }
    }

    // Build completion string
    let mut completion = String::new();

    // Close any unclosed string
    if in_string {
        completion.push('"');
    }

    // Close brackets
    for _ in 0..bracket_count.max(0) {
        completion.push(']');
    }

    // Close braces
    for _ in 0..brace_count.max(0) {
        completion.push('}');
    }

    let attempt = format!("{}{}", s, completion);
    serde_json::from_str(&attempt).unwrap_or_else(|_| Value::Object(Default::default()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_complete_json() {
        let result = parse_streaming_json(r#"{"name": "test"}"#);
        assert_eq!(result, json!({"name": "test"}));
    }

    #[test]
    fn test_missing_closing_brace() {
        let result = parse_streaming_json(r#"{"name": "test""#);
        assert_eq!(result, json!({"name": "test"}));
    }

    #[test]
    fn test_nested_missing_braces() {
        let result = parse_streaming_json(r#"{"outer": {"inner": 1"#);
        assert_eq!(result, json!({"outer": {"inner": 1}}));
    }

    #[test]
    fn test_empty_input() {
        let result = parse_streaming_json("");
        assert_eq!(result, json!({}));
    }

    #[test]
    fn test_smart_parser() {
        let result = parse_streaming_json_smart(r#"{"items": [1, 2, 3"#);
        assert_eq!(result, json!({"items": [1, 2, 3]}));
    }
}
```

---

## File Structure After Phase 5

```
alchemy/
  src/
    lib.rs              [UPDATE: add transform, utils]
    transform.rs        [NEW: message transformation]
    utils/
      mod.rs            [NEW: utility re-exports]
      validation.rs     [NEW: tool validation]
      overflow.rs       [NEW: context overflow detection]
      sanitize.rs       [NEW: unicode sanitization]
      json_parse.rs     [NEW: partial JSON parsing]
```

---

## Dependencies to Add

```toml
[dependencies]
regex = "1.10"    # For overflow pattern matching
# jsonschema already added in Cargo.toml
```

---

## Public API After Phase 5

```rust
// Main streaming API
pub fn stream(model: &AnyModel, context: &Context, options: impl StreamOptions) -> Result<AssistantMessageEventStream>;
pub async fn complete(model: &AnyModel, context: &Context, options: impl StreamOptions) -> Result<AssistantMessage>;

// Model registry
pub fn get_model(id: &str) -> Option<&'static AnyModel>;
pub fn get_models() -> impl Iterator<Item = &'static AnyModel>;
pub fn get_models_by_provider(provider: &Provider) -> Vec<&'static AnyModel>;
pub fn get_models_by_api(api: Api) -> Vec<&'static AnyModel>;

// Utilities
pub fn transform_messages<TApi>(messages: &[Message], model: &Model<TApi>, ...) -> Vec<Message>;
pub fn validate_tool_call(tools: &[Tool], tool_call: &ToolCall) -> Result<Value>;
pub fn is_context_overflow(message: &AssistantMessage, context_window: Option<u32>) -> bool;
pub fn parse_streaming_json(s: &str) -> Value;
```

---

## Acceptance Criteria

- [ ] `transform_messages` correctly handles thinking blocks
- [ ] `transform_messages` normalizes tool call IDs
- [ ] `transform_messages` inserts synthetic results for orphans
- [ ] `transform_messages` filters error/aborted messages
- [ ] `validate_tool_call` validates against JSON schema
- [ ] `validate_tool_call` returns useful error messages
- [ ] `is_context_overflow` detects all provider patterns
- [ ] `is_context_overflow` detects silent overflow
- [ ] `parse_streaming_json` handles partial JSON
- [ ] All utilities are `Send + Sync`
- [ ] `cargo test` passes
- [ ] `cargo clippy` clean
- [ ] Documentation complete with examples

---

## Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Test cross-provider conversation replay
    #[test]
    fn test_anthropic_to_openai_transform() {
        // Create conversation with Anthropic thinking blocks
        let messages = vec![
            Message::User(UserMessage {
                content: UserContent::Text("Hello".to_string()),
                timestamp: 0,
            }),
            Message::Assistant(AssistantMessage {
                content: vec![
                    Content::thinking("Let me think..."),
                    Content::text("Hi there!"),
                ],
                api: Api::AnthropicMessages,
                provider: Provider::Known(KnownProvider::Anthropic),
                model: "claude-sonnet-4-20250514".to_string(),
                // ...
            }),
        ];

        // Transform for OpenAI
        let openai_model = get_model("gpt-4o").unwrap();
        let transformed = transform_messages(&messages, openai_model, None);

        // Thinking should be converted to text
        if let Message::Assistant(assistant) = &transformed[1] {
            assert!(matches!(assistant.content[0], Content::Text { .. }));
        }
    }

    /// Test orphaned tool call handling
    #[test]
    fn test_orphaned_tool_calls() {
        let messages = vec![
            Message::Assistant(AssistantMessage {
                content: vec![Content::tool_call("123", "search", json!({"q": "test"}))],
                // ...
            }),
            Message::User(UserMessage {
                content: UserContent::Text("Never mind".to_string()),
                timestamp: 0,
            }),
        ];

        let transformed = transform_messages(&messages, &model, None);

        // Should have synthetic tool result inserted
        assert_eq!(transformed.len(), 3);
        assert!(matches!(transformed[1], Message::ToolResult(_)));
    }
}
```

---

## Notes

- Message transformation is the most complex part - test extensively
- Consider caching compiled JSON schemas for frequently-used tools
- Overflow detection patterns may need updates as providers change
- The `transform_messages` function is called by each provider, not by users directly
