# Phase 3: First Provider (Anthropic)

**Status:** Not Started
**Estimated Files:** 2-3 Rust modules
**Source Reference:** `ai/src/providers/anthropic.ts` (662 lines)

---

## Objective

Implement the Anthropic Messages API provider as the reference implementation. This establishes patterns for all other providers.

---

## Prerequisites

- Phase 1 complete (types, EventStream)
- Phase 2 complete (dispatch, env keys)

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                  streamAnthropic()                       │
├─────────────────────────────────────────────────────────┤
│  1. Create AssistantMessageEventStream                  │
│  2. Spawn async task:                                   │
│     a. Create Anthropic HTTP client                     │
│     b. Build request params                             │
│     c. Send request, get SSE stream                     │
│     d. Parse events, emit to EventStream                │
│     e. Handle errors                                    │
└─────────────────────────────────────────────────────────┘
```

---

## Components to Implement

### 1. Anthropic Options

**Rust Target:** `src/providers/anthropic/options.rs`

```rust
use std::collections::HashMap;
use crate::types::StreamOptions;

#[derive(Debug, Clone, Default)]
pub struct AnthropicOptions {
    pub api_key: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub thinking_enabled: bool,
    pub thinking_budget_tokens: Option<u32>,
    pub interleaved_thinking: bool,
    pub tool_choice: Option<ToolChoice>,
    pub headers: Option<HashMap<String, String>>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ToolChoice {
    Auto,
    Any,
    None,
    Tool { name: String },
}

impl StreamOptions for AnthropicOptions {
    fn temperature(&self) -> Option<f64> { self.temperature }
    fn max_tokens(&self) -> Option<u32> { self.max_tokens }
    fn api_key(&self) -> Option<&str> { self.api_key.as_deref() }
    fn session_id(&self) -> Option<&str> { self.session_id.as_deref() }
    fn headers(&self) -> Option<&HashMap<String, String>> { self.headers.as_ref() }
}
```

---

### 2. Request Types (Anthropic API Format)

**Rust Target:** `src/providers/anthropic/types.rs`

```rust
use serde::{Deserialize, Serialize};

/// POST /v1/messages request body
#[derive(Debug, Serialize)]
pub struct CreateMessageRequest {
    pub model: String,
    pub messages: Vec<MessageParam>,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<Vec<SystemBlock>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolParam>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoiceParam>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingParam>,
    pub stream: bool,
}

#[derive(Debug, Serialize)]
pub struct SystemBlock {
    #[serde(rename = "type")]
    pub block_type: String,  // "text"
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

#[derive(Debug, Serialize)]
pub struct CacheControl {
    #[serde(rename = "type")]
    pub control_type: String,  // "ephemeral"
}

#[derive(Debug, Serialize)]
#[serde(tag = "role")]
pub enum MessageParam {
    #[serde(rename = "user")]
    User { content: UserContent },
    #[serde(rename = "assistant")]
    Assistant { content: Vec<AssistantContentBlock> },
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum UserContent {
    Text(String),
    Blocks(Vec<UserContentBlock>),
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum UserContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { source: ImageSource },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: UserContent,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },
}

#[derive(Debug, Serialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String,  // "base64"
    pub media_type: String,   // "image/jpeg", etc.
    pub data: String,         // base64 encoded
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum AssistantContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "thinking")]
    Thinking { thinking: String, signature: String },
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String, input: serde_json::Value },
}

#[derive(Debug, Serialize)]
pub struct ToolParam {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct ToolChoiceParam {
    #[serde(rename = "type")]
    pub choice_type: String,  // "auto", "any", "none", "tool"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ThinkingParam {
    #[serde(rename = "type")]
    pub thinking_type: String,  // "enabled"
    pub budget_tokens: u32,
}
```

---

### 3. SSE Event Types (Response)

**Rust Target:** `src/providers/anthropic/events.rs`

```rust
use serde::Deserialize;

/// Server-Sent Event wrapper
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: MessageStartData },

    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: usize,
        content_block: ContentBlock,
    },

    #[serde(rename = "content_block_delta")]
    ContentBlockDelta {
        index: usize,
        delta: ContentDelta,
    },

    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: usize },

    #[serde(rename = "message_delta")]
    MessageDelta {
        delta: MessageDeltaData,
        usage: UsageData,
    },

    #[serde(rename = "message_stop")]
    MessageStop,

    #[serde(rename = "ping")]
    Ping,

    #[serde(rename = "error")]
    Error { error: ErrorData },
}

#[derive(Debug, Deserialize)]
pub struct MessageStartData {
    pub usage: UsageData,
}

#[derive(Debug, Deserialize)]
pub struct UsageData {
    pub input_tokens: Option<u32>,
    pub output_tokens: Option<u32>,
    pub cache_read_input_tokens: Option<u32>,
    pub cache_creation_input_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "thinking")]
    Thinking { thinking: String },
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String, input: serde_json::Value },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum ContentDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },
    #[serde(rename = "thinking_delta")]
    ThinkingDelta { thinking: String },
    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },
    #[serde(rename = "signature_delta")]
    SignatureDelta { signature: String },
}

#[derive(Debug, Deserialize)]
pub struct MessageDeltaData {
    pub stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ErrorData {
    #[serde(rename = "type")]
    pub error_type: String,
    pub message: String,
}
```

---

### 4. Main Stream Function

**Rust Target:** `src/providers/anthropic/stream.rs`

```rust
use reqwest::Client;
use futures::StreamExt;
use crate::types::{
    AssistantMessage, AssistantMessageEvent, Context, Model,
    Content, TextContent, ThinkingContent, ToolCall, Usage, StopReason,
    model::AnthropicMessages,
};
use crate::stream::{AssistantMessageEventStream, AssistantMessageEventSender};
use crate::Result;

use super::options::AnthropicOptions;
use super::types::*;
use super::events::*;
use super::convert::{convert_messages, convert_tools};

const BETA_FEATURES: &[&str] = &[
    "fine-grained-tool-streaming-2025-05-14",
    "interleaved-thinking-2025-05-14",
];

pub fn stream(
    model: &Model<AnthropicMessages>,
    context: &Context,
    options: AnthropicOptions,
) -> AssistantMessageEventStream {
    let (sender, stream) = AssistantMessageEventStream::new();

    let model = model.clone();
    let context = context.clone();

    tokio::spawn(async move {
        if let Err(e) = stream_inner(&model, &context, options, sender.clone()).await {
            // Error already pushed in stream_inner
        }
    });

    stream
}

async fn stream_inner(
    model: &Model<AnthropicMessages>,
    context: &Context,
    options: AnthropicOptions,
    sender: AssistantMessageEventSender,
) -> Result<()> {
    let api_key = options.api_key.as_ref()
        .ok_or_else(|| crate::Error::NoApiKey("anthropic".to_string()))?;

    // Build output message
    let mut output = AssistantMessage {
        content: Vec::new(),
        api: crate::types::Api::AnthropicMessages,
        provider: model.provider.clone(),
        model: model.id.clone(),
        usage: Usage::default(),
        stop_reason: StopReason::Stop,
        error_message: None,
        timestamp: current_timestamp(),
    };

    // Build request
    let request = build_request(model, context, &options);

    // Create HTTP client
    let client = Client::new();
    let response = client
        .post(format!("{}/v1/messages", model.base_url))
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("anthropic-beta", BETA_FEATURES.join(","))
        .header("content-type", "application/json")
        .header("accept", "text/event-stream")
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(crate::Error::ApiError {
            status_code: status,
            message: body,
        });
    }

    sender.push(AssistantMessageEvent::Start {
        partial: output.clone(),
    });

    // Track blocks by index
    let mut blocks: Vec<BlockState> = Vec::new();

    // Parse SSE stream
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        // Process complete events
        while let Some(event) = parse_next_event(&mut buffer) {
            process_event(event, &mut output, &mut blocks, &sender)?;
        }
    }

    // Final event
    sender.push(AssistantMessageEvent::Done {
        reason: output.stop_reason.into(),
        message: output.clone(),
    });
    sender.end(output);

    Ok(())
}

#[derive(Debug)]
struct BlockState {
    index: usize,
    content_index: usize,
    partial_json: String,
}

fn process_event(
    event: StreamEvent,
    output: &mut AssistantMessage,
    blocks: &mut Vec<BlockState>,
    sender: &AssistantMessageEventSender,
) -> Result<()> {
    match event {
        StreamEvent::MessageStart { message } => {
            output.usage.input = message.usage.input_tokens.unwrap_or(0);
            output.usage.output = message.usage.output_tokens.unwrap_or(0);
            output.usage.cache_read = message.usage.cache_read_input_tokens.unwrap_or(0);
            output.usage.cache_write = message.usage.cache_creation_input_tokens.unwrap_or(0);
            update_total_tokens(&mut output.usage);
        }

        StreamEvent::ContentBlockStart { index, content_block } => {
            let content_index = output.content.len();
            blocks.push(BlockState {
                index,
                content_index,
                partial_json: String::new(),
            });

            match content_block {
                ContentBlock::Text { .. } => {
                    output.content.push(Content::text(""));
                    sender.push(AssistantMessageEvent::TextStart {
                        content_index,
                        partial: output.clone(),
                    });
                }
                ContentBlock::Thinking { .. } => {
                    output.content.push(Content::thinking(""));
                    sender.push(AssistantMessageEvent::ThinkingStart {
                        content_index,
                        partial: output.clone(),
                    });
                }
                ContentBlock::ToolUse { id, name, .. } => {
                    output.content.push(Content::tool_call(&id, &name, serde_json::Value::Object(Default::default())));
                    sender.push(AssistantMessageEvent::ToolCallStart {
                        content_index,
                        partial: output.clone(),
                    });
                }
            }
        }

        StreamEvent::ContentBlockDelta { index, delta } => {
            let block = blocks.iter_mut().find(|b| b.index == index);
            if let Some(block) = block {
                let content_index = block.content_index;
                match delta {
                    ContentDelta::TextDelta { text } => {
                        if let Some(Content::Text { inner }) = output.content.get_mut(content_index) {
                            inner.text.push_str(&text);
                        }
                        sender.push(AssistantMessageEvent::TextDelta {
                            content_index,
                            delta: text,
                            partial: output.clone(),
                        });
                    }
                    ContentDelta::ThinkingDelta { thinking } => {
                        if let Some(Content::Thinking { inner }) = output.content.get_mut(content_index) {
                            inner.thinking.push_str(&thinking);
                        }
                        sender.push(AssistantMessageEvent::ThinkingDelta {
                            content_index,
                            delta: thinking,
                            partial: output.clone(),
                        });
                    }
                    ContentDelta::InputJsonDelta { partial_json } => {
                        block.partial_json.push_str(&partial_json);
                        if let Some(Content::ToolCall { inner }) = output.content.get_mut(content_index) {
                            inner.arguments = parse_partial_json(&block.partial_json);
                        }
                        sender.push(AssistantMessageEvent::ToolCallDelta {
                            content_index,
                            delta: partial_json,
                            partial: output.clone(),
                        });
                    }
                    ContentDelta::SignatureDelta { signature } => {
                        if let Some(Content::Thinking { inner }) = output.content.get_mut(content_index) {
                            let sig = inner.thinking_signature.get_or_insert_with(String::new);
                            sig.push_str(&signature);
                        }
                    }
                }
            }
        }

        StreamEvent::ContentBlockStop { index } => {
            if let Some(block) = blocks.iter().find(|b| b.index == index) {
                let content_index = block.content_index;
                match &output.content[content_index] {
                    Content::Text { inner } => {
                        sender.push(AssistantMessageEvent::TextEnd {
                            content_index,
                            content: inner.text.clone(),
                            partial: output.clone(),
                        });
                    }
                    Content::Thinking { inner } => {
                        sender.push(AssistantMessageEvent::ThinkingEnd {
                            content_index,
                            content: inner.thinking.clone(),
                            partial: output.clone(),
                        });
                    }
                    Content::ToolCall { inner } => {
                        sender.push(AssistantMessageEvent::ToolCallEnd {
                            content_index,
                            tool_call: inner.clone(),
                            partial: output.clone(),
                        });
                    }
                    _ => {}
                }
            }
        }

        StreamEvent::MessageDelta { delta, usage } => {
            if let Some(reason) = delta.stop_reason {
                output.stop_reason = map_stop_reason(&reason);
            }
            output.usage.output = usage.output_tokens.unwrap_or(output.usage.output);
            update_total_tokens(&mut output.usage);
        }

        StreamEvent::MessageStop | StreamEvent::Ping => {}

        StreamEvent::Error { error } => {
            return Err(crate::Error::ApiError {
                status_code: 0,
                message: error.message,
            });
        }
    }

    Ok(())
}

fn map_stop_reason(reason: &str) -> StopReason {
    match reason {
        "end_turn" | "pause_turn" | "stop_sequence" => StopReason::Stop,
        "max_tokens" => StopReason::Length,
        "tool_use" => StopReason::ToolUse,
        _ => StopReason::Error,
    }
}

fn update_total_tokens(usage: &mut Usage) {
    usage.total_tokens = usage.input + usage.output + usage.cache_read + usage.cache_write;
}

fn parse_partial_json(s: &str) -> serde_json::Value {
    // Try parsing as complete JSON first
    if let Ok(v) = serde_json::from_str(s) {
        return v;
    }
    // Fall back to partial JSON parsing
    // TODO: Implement proper partial JSON parser
    serde_json::Value::Object(Default::default())
}

fn parse_next_event(buffer: &mut String) -> Option<StreamEvent> {
    // SSE format: "event: type\ndata: json\n\n"
    if let Some(end) = buffer.find("\n\n") {
        let event_str = buffer[..end].to_string();
        *buffer = buffer[end + 2..].to_string();

        // Parse event type and data
        let mut data = None;
        for line in event_str.lines() {
            if let Some(json) = line.strip_prefix("data: ") {
                data = Some(json);
            }
        }

        if let Some(json) = data {
            if let Ok(event) = serde_json::from_str(json) {
                return Some(event);
            }
        }
    }
    None
}

fn current_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
```

---

### 5. Message Conversion

**Rust Target:** `src/providers/anthropic/convert.rs`

```rust
use crate::types::{Context, Message, Tool};
use super::types::*;

/// Normalize tool call IDs to Anthropic format
pub fn normalize_tool_call_id(id: &str) -> String {
    id.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .take(64)
        .collect()
}

pub fn convert_messages(messages: &[Message]) -> Vec<MessageParam> {
    let mut params = Vec::new();

    for msg in messages {
        match msg {
            Message::User(user) => {
                // Convert user message
                let content = match &user.content {
                    crate::types::UserContent::Text(s) => UserContent::Text(s.clone()),
                    crate::types::UserContent::Multi(blocks) => {
                        UserContent::Blocks(blocks.iter().map(convert_user_block).collect())
                    }
                };
                params.push(MessageParam::User { content });
            }

            Message::Assistant(assistant) => {
                let blocks = assistant.content.iter()
                    .filter_map(convert_assistant_block)
                    .collect();
                params.push(MessageParam::Assistant { content: blocks });
            }

            Message::ToolResult(tool_result) => {
                params.push(MessageParam::User {
                    content: UserContent::Blocks(vec![
                        UserContentBlock::ToolResult {
                            tool_use_id: tool_result.tool_call_id.clone(),
                            content: UserContent::Text(
                                tool_result.content.iter()
                                    .filter_map(|c| match c {
                                        crate::types::ToolResultContent::Text(t) => Some(t.text.clone()),
                                        _ => None,
                                    })
                                    .collect::<Vec<_>>()
                                    .join("\n")
                            ),
                            is_error: Some(tool_result.is_error),
                            cache_control: None,
                        }
                    ]),
                });
            }
        }
    }

    params
}

fn convert_user_block(block: &crate::types::UserContentBlock) -> UserContentBlock {
    match block {
        crate::types::UserContentBlock::Text(t) => {
            UserContentBlock::Text { text: t.text.clone() }
        }
        crate::types::UserContentBlock::Image(img) => {
            UserContentBlock::Image {
                source: ImageSource {
                    source_type: "base64".to_string(),
                    media_type: img.mime_type.clone(),
                    data: img.to_base64(),
                },
            }
        }
    }
}

fn convert_assistant_block(content: &crate::types::Content) -> Option<AssistantContentBlock> {
    match content {
        crate::types::Content::Text { inner } => {
            Some(AssistantContentBlock::Text { text: inner.text.clone() })
        }
        crate::types::Content::Thinking { inner } => {
            // Only include if has signature
            inner.thinking_signature.as_ref().map(|sig| {
                AssistantContentBlock::Thinking {
                    thinking: inner.thinking.clone(),
                    signature: sig.clone(),
                }
            })
        }
        crate::types::Content::ToolCall { inner } => {
            Some(AssistantContentBlock::ToolUse {
                id: inner.id.clone(),
                name: inner.name.clone(),
                input: inner.arguments.clone(),
            })
        }
        _ => None,
    }
}

pub fn convert_tools(tools: &[Tool]) -> Vec<ToolParam> {
    tools.iter().map(|tool| {
        ToolParam {
            name: tool.name.clone(),
            description: tool.description.clone(),
            input_schema: tool.parameters.clone(),
        }
    }).collect()
}
```

---

## File Structure After Phase 3

```
alchemy/
  src/
    providers/
      mod.rs
      env.rs
      anthropic/
        mod.rs          [NEW: re-exports]
        options.rs      [NEW: AnthropicOptions]
        types.rs        [NEW: request types]
        events.rs       [NEW: SSE event types]
        stream.rs       [NEW: main stream function]
        convert.rs      [NEW: message conversion]
```

---

## Key Implementation Details

### 1. Tool Call ID Normalization

Anthropic requires: `^[a-zA-Z0-9_-]{1,64}$`

```rust
fn normalize_tool_call_id(id: &str) -> String {
    id.chars()
        .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
        .take(64)
        .collect()
}
```

### 2. Thinking Signature Handling

- Thinking blocks without signatures get converted to plain text
- This prevents API rejection and Claude mimicking `<thinking>` tags

### 3. Prompt Caching

- System prompt gets `cache_control: { type: "ephemeral" }`
- Last user message content block gets cache_control
- Reduces token costs on repeated conversations

### 4. Unicode Sanitization

Port `sanitize-unicode.ts` for surrogate pair handling:

```rust
/// Remove unpaired surrogates that can cause API errors
pub fn sanitize_surrogates(s: &str) -> String {
    s.chars()
        .filter(|c| !is_unpaired_surrogate(*c))
        .collect()
}

fn is_unpaired_surrogate(c: char) -> bool {
    let code = c as u32;
    (0xD800..=0xDFFF).contains(&code)
}
```

### 5. Partial JSON Parsing

For streaming tool calls, need incremental JSON parser:

```rust
// Use a crate like `partial-json` or implement simple heuristics
fn parse_partial_json(s: &str) -> serde_json::Value {
    // Try complete parse first
    if let Ok(v) = serde_json::from_str(s) {
        return v;
    }

    // Try adding closing braces
    for suffix in &["}", "}}", "\"}", "\"}"] {
        if let Ok(v) = serde_json::from_str(&format!("{}{}", s, suffix)) {
            return v;
        }
    }

    serde_json::Value::Object(Default::default())
}
```

---

## Excluded Features (OAuth)

The TypeScript has "stealth mode" for OAuth tokens to mimic Claude Code. This is **excluded** from the port:

- No `isOAuthToken` detection
- No Claude Code version headers
- No tool name remapping (toClaudeCodeName/fromClaudeCodeName)
- No special system prompt injection

---

## Dependencies

```toml
[dependencies]
reqwest = { version = "0.12", features = ["json", "stream"] }
futures = "0.3"
# Already have these
```

---

## Acceptance Criteria

- [ ] `stream()` returns valid EventStream
- [ ] SSE parsing handles all event types
- [ ] Text streaming works (text_start, text_delta, text_end)
- [ ] Thinking streaming works with signatures
- [ ] Tool call streaming works with partial JSON
- [ ] Usage/cost calculation matches TypeScript
- [ ] Stop reasons mapped correctly
- [ ] Errors produce `AssistantMessageEvent::Error`
- [ ] Prompt caching applied correctly
- [ ] `cargo test` with mock server passes

---

## Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_tool_call_id() {
        assert_eq!(normalize_tool_call_id("abc_123-def"), "abc_123-def");
        assert_eq!(normalize_tool_call_id("a@b#c"), "abc");
        assert_eq!(normalize_tool_call_id(&"a".repeat(100)), "a".repeat(64));
    }

    #[test]
    fn test_parse_sse_event() {
        let mut buffer = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":10}}}\n\n".to_string();
        let event = parse_next_event(&mut buffer);
        assert!(matches!(event, Some(StreamEvent::MessageStart { .. })));
    }

    #[tokio::test]
    async fn test_stream_text_only() {
        // Mock server that returns simple text stream
        // ...
    }
}
```

---

## Notes

- Consider using `anthropic-rs` crate instead of raw HTTP, but may need streaming support
- The TypeScript SDK (`@anthropic-ai/sdk`) handles SSE parsing internally
- For testing, use `wiremock` or `httpmock` crate
