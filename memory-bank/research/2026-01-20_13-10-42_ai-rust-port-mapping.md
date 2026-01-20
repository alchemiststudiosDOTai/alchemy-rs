# Research – @ai to Rust Port Mapping

**Date:** 2026-01-20
**Owner:** claude
**Phase:** Research

## Goal

Map the `@mariozechner/pi-ai` TypeScript package to plan a one-to-one Rust port focused on API functionality only (no auth, no CLI).

## Summary

The `@ai` package is a unified LLM API abstraction layer supporting 8+ providers (Anthropic, OpenAI, Google, AWS Bedrock, Mistral, xAI, Groq, Cerebras, OpenRouter, etc.) through 4 main API interfaces. The codebase consists of approximately 17,215 lines of API-relevant code (excluding OAuth and CLI).

### Scope for Rust Port

**Include:**
- Core types and interfaces
- Streaming infrastructure and event system
- Model registry and discovery
- Provider implementations (API-key based only)
- Cross-provider message transformation
- Tool calling with validation
- Image/vision support
- Thinking/reasoning support

**Exclude:**
- OAuth authentication flows (anthropic, openai-codex, github-copilot, google-gemini-cli, google-antigravity)
- CLI interface (cli.ts)
- OAuth utilities (src/utils/oauth/)

---

## File Structure Map

### Core Type Definitions

| File | Lines | Purpose | Rust Equivalent |
|------|-------|---------|-----------------|
| `src/types.ts` | 272 | Core type definitions | `types.rs` module with enums and structs |
| `src/models.ts` | 69 | Model registry functions | `models.rs` with lazy_static HashMap |
| `src/models.generated.ts` | 10,825 | Auto-generated model data | `models/generated.rs` via build script or serde JSON |
| `src/index.ts` | 16 | Main exports barrel | `lib.rs` with pub use |

### Streaming Infrastructure

| File | Lines | Purpose | Rust Equivalent |
|------|-------|---------|-----------------|
| `src/stream.ts` | 554 | Main streaming orchestration | `stream.rs` with enum dispatch |
| `src/utils/event-stream.ts` | 82 | Event stream implementation | `event_stream.rs` using tokio::sync::mpsc |

### Provider Implementations

| File | Lines | Purpose | Rust Equivalent | Notes |
|------|-------|---------|-----------------|-------|
| `src/providers/anthropic.ts` | 662 | Anthropic Messages API | `providers/anthropic.rs` | Include |
| `src/providers/google.ts` | 343 | Google Generative AI | `providers/google.rs` | Include |
| `src/providers/google-vertex.ts` | 375 | Google Vertex AI | `providers/google_vertex.rs` | Include (ADC auth) |
| `src/providers/google-shared.ts` | 311 | Shared Google utilities | `providers/google_shared.rs` | Include |
| `src/providers/openai-completions.ts` | 765 | OpenAI Chat Completions | `providers/openai_completions.rs` | Include |
| `src/providers/openai-responses.ts` | 627 | OpenAI Responses API | `providers/openai_responses.rs` | Include |
| `src/providers/amazon-bedrock.ts` | 578 | AWS Bedrock Converse | `providers/bedrock.rs` | Include |
| `src/providers/google-gemini-cli.ts` | 923 | Google Gemini CLI (OAuth) | **EXCLUDE** | OAuth-based |
| `src/providers/openai-codex-responses.ts` | 722 | OpenAI Codex (OAuth) | **EXCLUDE** | OAuth-based |
| `src/providers/transform-messages.ts` | 167 | Cross-provider transforms | `providers/transform.rs` | Include |

### Utility Functions

| File | Lines | Purpose | Rust Equivalent |
|------|-------|---------|-----------------|
| `src/utils/json-parse.ts` | 28 | Partial JSON parsing | `utils/json_parse.rs` |
| `src/utils/overflow.ts` | 118 | Context overflow detection | `utils/overflow.rs` |
| `src/utils/sanitize-unicode.ts` | 25 | Unicode sanitization | `utils/sanitize.rs` |
| `src/utils/validation.ts` | 84 | Tool validation | `utils/validation.rs` |
| `src/utils/typebox-helpers.ts` | 24 | TypeBox helpers | **N/A** | Use serde_json-schema |
| `src/utils/oauth/*` | 2,432 | OAuth utilities | **EXCLUDE ALL** | Out of scope |

### Scripts

| File | Purpose | Rust Equivalent |
|------|---------|-----------------|
| `scripts/generate-models.ts` | Generate model data | `build.rs` or standalone script |

---

## Key Types and Rust Translation

### Content Blocks

```typescript
// TypeScript
export interface TextContent {
  type: "text";
  text: string;
  textSignature?: string;
}

export interface ThinkingContent {
  type: "thinking";
  thinking: string;
  thinkingSignature?: string;
}

export interface ImageContent {
  type: "image";
  data: string;  // base64
  mimeType: string;
}

export interface ToolCall {
  type: "toolCall";
  id: string;
  name: string;
  arguments: Record<string, any>;
  thoughtSignature?: string;
}
```

```rust
// Rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Content {
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        text_signature: Option<String>,
    },
    Thinking {
        thinking: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        thinking_signature: Option<String>,
    },
    Image {
        data: Vec<u8>,  // base64 decoded
        mime_type: String,
    },
    ToolCall {
        id: String,
        name: String,
        arguments: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        thought_signature: Option<String>,
    },
}
```

### Message Types

```typescript
// TypeScript
export interface UserMessage {
  role: "user";
  content: string | (TextContent | ImageContent)[];
  timestamp: number;
}

export interface AssistantMessage {
  role: "assistant";
  content: (TextContent | ThinkingContent | ToolCall)[];
  api: Api;
  provider: Provider;
  model: string;
  usage: Usage;
  stopReason: StopReason;
  errorMessage?: string;
  timestamp: number;
}

export interface ToolResultMessage {
  role: "toolResult";
  toolCallId: string;
  toolName: string;
  content: (TextContent | ImageContent)[];
  isError: boolean;
  timestamp: number;
}

export type Message = UserMessage | AssistantMessage | ToolResultMessage;
```

```rust
// Rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum Message {
    User(UserMessage),
    Assistant(AssistantMessage),
    ToolResult(ToolResultMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    pub content: UserContent,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UserContent {
    Text(String),
    Multi(Vec<UserContentBlock>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UserContentBlock {
    Text(TextContent),
    Image(ImageContent),
}
```

### Stream Events

```typescript
// TypeScript
export type AssistantMessageEvent =
  | { type: "start"; partial: AssistantMessage }
  | { type: "text_start"; contentIndex: number; partial: AssistantMessage }
  | { type: "text_delta"; contentIndex: number; delta: string; partial: AssistantMessage }
  | { type: "text_end"; contentIndex: number; content: string; partial: AssistantMessage }
  | { type: "thinking_start"; contentIndex: number; partial: AssistantMessage }
  | { type: "thinking_delta"; contentIndex: number; delta: string; partial: AssistantMessage }
  | { type: "thinking_end"; contentIndex: number; content: string; partial: AssistantMessage }
  | { type: "toolcall_start"; contentIndex: number; partial: AssistantMessage }
  | { type: "toolcall_delta"; contentIndex: number; delta: string; partial: AssistantMessage }
  | { type: "toolcall_end"; contentIndex: number; toolCall: ToolCall; partial: AssistantMessage }
  | { type: "done"; reason: StopReason; message: AssistantMessage }
  | { type: "error"; reason: StopReason; error: AssistantMessage };
```

```rust
// Rust
#[derive(Debug, Clone)]
pub enum AssistantMessageEvent {
    Start { partial: AssistantMessage },
    TextStart { content_index: usize, partial: AssistantMessage },
    TextDelta { content_index: usize, delta: String, partial: AssistantMessage },
    TextEnd { content_index: usize, content: String, partial: AssistantMessage },
    ThinkingStart { content_index: usize, partial: AssistantMessage },
    ThinkingDelta { content_index: usize, delta: String, partial: AssistantMessage },
    ThinkingEnd { content_index: usize, content: String, partial: AssistantMessage },
    ToolCallStart { content_index: usize, partial: AssistantMessage },
    ToolCallDelta { content_index: usize, delta: String, partial: AssistantMessage },
    ToolCallEnd { content_index: usize, tool_call: ToolCall, partial: AssistantMessage },
    Done { reason: StopReason, message: AssistantMessage },
    Error { reason: StopReason, error: AssistantMessage },
}
```

### Model Interface

```typescript
// TypeScript
export interface Model<TApi extends Api> {
  id: string;
  name: string;
  api: TApi;
  provider: Provider;
  baseUrl: string;
  reasoning: boolean;
  input: ("text" | "image")[];
  cost: {
    input: number;
    output: number;
    cacheRead: number;
    cacheWrite: number;
  };
  contextWindow: number;
  maxTokens: number;
  headers?: Record<string, string>;
  compat?: TApi extends "openai-completions"
    ? OpenAICompletionsCompat
    : TApi extends "openai-responses"
      ? OpenAIResponsesCompat
      : never;
}
```

```rust
// Rust
#[derive(Debug, Clone)]
pub struct Model<TApi: Api> {
    pub id: String,
    pub name: String,
    pub api: TApi,
    pub provider: String,
    pub base_url: String,
    pub reasoning: bool,
    pub input: Vec<InputType>,
    pub cost: Cost,
    pub context_window: u32,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    pub compat: Option<TApi::Compat>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum InputType {
    Text,
    Image,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Cost {
    pub input: f64,    // $/million tokens
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
}
```

### API Type System

```typescript
// TypeScript
export type Api =
  | "anthropic-messages"
  | "openai-completions"
  | "openai-responses"
  | "bedrock-converse-stream"
  | "google-generative-ai"
  | "google-vertex";

export interface ApiOptionsMap {
  "anthropic-messages": AnthropicOptions;
  "openai-completions": OpenAICompletionsOptions;
  // ...
}

export type OptionsForApi<TApi extends Api> = ApiOptionsMap[TApi];
```

```rust
// Rust - Using trait-based approach
pub trait Api: Sized {
    type Options: StreamOptions + Send + Sync;
    type Compat: Send + Sync;

    fn name(&self) -> &'static str;
}

pub enum ApiType {
    AnthropicMessages(anthropic::AnthropicApi),
    OpenAICompletions(openai::completions::CompletionsApi),
    OpenAIResponses(openai::responses::ResponsesApi),
    BedrockConverseStream(bedrock::BedrockApi),
    GoogleGenerativeAi(google::GenerativeApi),
    GoogleVertex(google::VertexApi),
}

// Or use macro-generated enum
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Api {
    AnthropicMessages,
    OpenAICompletions,
    OpenAIResponses,
    BedrockConverseStream,
    GoogleGenerativeAi,
    GoogleVertex,
}
```

---

## Streaming Architecture

### TypeScript Pattern (EventStream)

```typescript
export class EventStream<T, R = T> implements AsyncIterable<T> {
  private queue: T[] = [];
  private waiting: ((value: IteratorResult<T>) => void)[] = [];
  private done = false;
  private finalResultPromise: Promise<R>;

  async *[Symbol.asyncIterator](): AsyncIterator<T> {
    while (true) {
      if (this.queue.length > 0) {
        yield this.queue.shift()!;
      } else if (this.done) {
        return;
      } else {
        const result = await new Promise<IteratorResult<T>>((resolve) => this.waiting.push(resolve));
        if (result.done) return;
        yield result.value;
      }
    }
  }
}
```

### Rust Translation (Stream trait)

```rust
use tokio::sync::{mpsc, oneshot};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct EventStream<T, R> {
    tx: mpsc::UnboundedSender<T>,
    rx: mpsc::UnboundedReceiver<T>,
    done: Arc<AtomicBool>,
    result_tx: Option<oneshot::Sender<R>>,
    result_rx: oneshot::Receiver<R>,
}

impl<T, R> EventStream<T, R> {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let (result_tx, result_rx) = oneshot::channel();
        Self {
            tx,
            rx,
            done: Arc::new(AtomicBool::new(false)),
            result_tx: Some(result_tx),
            result_rx,
        }
    }

    pub fn push(&self, event: T) {
        if self.done.load(Ordering::SeqCst) {
            return;
        }
        let _ = self.tx.send(event);
    }

    pub fn end(self, result: R) {
        self.done.store(true, Ordering::SeqCst);
        if let Some(tx) = self.result_tx {
            let _ = tx.send(result);
        }
    }

    pub async fn result(self) -> Result<R, oneshot::error::RecvError> {
        self.result_rx.await
    }
}

impl<T, R> Stream for EventStream<T, R> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
        self.rx.poll_recv(cx)
    }
}
```

### Fire-and-Forget Pattern

```typescript
// TypeScript
export function streamAnthropic(...): AssistantMessageEventStream {
  const stream = new AssistantMessageEventStream();

  (async () => {
    try {
      for await (const event of anthropicStream) {
        stream.push(processEvent(event));
      }
      stream.push({ type: "done", ... });
    } catch (error) {
      stream.push({ type: "error", ... });
    }
  })();

  return stream;
}
```

```rust
// Rust
pub fn stream_anthropic(...) -> AssistantMessageEventStream {
    let stream = AssistantMessageEventStream::new();
    let stream_clone = stream.clone();

    tokio::spawn(async move {
        match call_anthropic_api(...).await {
            Ok(mut api_stream) => {
                while let Some(event) = api_stream.next().await {
                    stream_clone.push(process_event(event));
                }
                stream_clone.end(final_message);
            }
            Err(e) => {
                stream_clone.push(AssistantMessageEvent::Error { ... });
            }
        }
    });

    stream
}
```

---

## Provider Implementation Mapping

### API Interfaces Summary

| API | Providers | Streaming Protocol | Tool Support | Image Support | Thinking Support |
|-----|-----------|-------------------|--------------|---------------|------------------|
| `anthropic-messages` | Anthropic | SSE (SDK) | Full | Full | Full (beta) |
| `openai-completions` | OpenAI, Mistral, xAI, Groq, Cerebras, OpenRouter, etc. | SSE (SDK) | Full | Full | Partial |
| `openai-responses` | OpenAI (o1, o3, gpt-5) | SSE (SDK) | Full | Full | Full |
| `google-generative-ai` | Google Gemini | Async Generator | Full | Full | Full |
| `google-vertex` | Google Vertex AI | Async Generator | Full | Full | Full |
| `bedrock-converse-stream` | AWS Bedrock | SDK Stream | Full | Full | Full |

### Provider-Specific Quirks

#### Anthropic
- Tool call ID: `^[a-zA-Z0-9_-]{1,64}$`
- Thinking signature required for replay
- Prompt caching via `cache_control` breakpoints
- "Stealth mode" headers for OAuth (exclude from port)

#### OpenAI Completions
- Tool call ID: truncated to 40 chars
- Mistral requires exactly 9 alphanumeric tool call IDs
- GitHub Copilot requires special headers (`X-Initiator`, `Copilot-Vision-Request`)
- OpenRouter Anthropic caching uses `cache_control` on last text part
- Reasoning via `reasoning_effort` field

#### OpenAI Responses
- Tool call ID format: `call_id|item_id`
- Separate `ResponseReasoningItem` for thinking
- `include: ["reasoning.encrypted_content"]` for encrypted reasoning
- Service tier pricing multipliers

#### Google (Generative AI & Vertex)
- Tool call IDs optional for most models, required for claude-*/gpt-oss-*
- Thought signatures in `thoughtSignature` field (base64)
- Multimodal function responses for Gemini 3
- Thinking via `thinkingConfig` object

#### AWS Bedrock
- Uses AWS SDK credential chain
- Tool call ID: alphanumeric only (special chars cause errors)
- Prompt caching for Claude models
- Signature field only supported by Anthropic models

### Cross-Provider Message Transformation

The `transform-messages.ts` file handles:
1. Tool call ID normalization between provider formats
2. Thinking block filtering/conversion for cross-provider replay
3. Empty assistant message filtering
4. Orphaned tool call handling (inserts synthetic error results)

Key transformations:
- Same model/provider: Keep thinking with signature for replay
- Different model: Convert thinking blocks to text with `<thinking>` tags
- Empty thinking: Filter out (prevents model mimicking tags)

---

## Tool Calling and Validation

### Tool Definition

```typescript
// TypeScript - TypeBox based
export interface Tool<TParameters = TSchema> {
  name: string;
  description: string;
  parameters: TParameters;
}

// Usage
const weatherTool: Tool = {
  name: 'get_weather',
  description: 'Get current weather',
  parameters: Type.Object({
    location: Type.String(),
    units: StringEnum(['celsius', 'fahrenheit'], { default: 'celsius' })
  })
};
```

```rust
// Rust - JSON Schema based
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,  // JSON Schema
}

// Usage with json_schema crate
use schemars::JsonSchema;

#[derive(JsonSchema)]
struct GetWeatherParams {
    location: String,
    #[serde(default)]
    units: Option<String>,
}

let weather_tool = Tool {
    name: "get_weather".to_string(),
    description: "Get current weather".to_string(),
    parameters: serde_json::to_value(GetWeatherParams::json_schema()).unwrap(),
};
```

### Validation

```typescript
// TypeScript - AJV
import Ajv from 'ajv';

export function validateToolCall(toolCall: ToolCall, tools: Tool[]): boolean {
  const tool = tools.find(t => t.name === toolCall.name);
  if (!tool) return false;

  const ajv = new Ajv();
  const validate = ajv.compile(tool.parameters);
  return validate(toolCall.arguments);
}
```

```rust
// Rust - jsonschema crate
use jsonschema::{JSONSchema, Draft};

pub fn validate_tool_call(
    tool_call: &ToolCall,
    tools: &[Tool],
) -> Result<(), ValidationError> {
    let tool = tools.iter()
        .find(|t| t.name == tool_call.name)
        .ok_or(ValidationError::ToolNotFound)?;

    let schema = JSONSchema::compile(&tool.parameters, Some(Draft::Draft7))
        .map_err(ValidationError::SchemaCompile)?;

    let args = &tool_call.arguments;
    if let Err(errors) = schema.validate(args) {
        return Err(ValidationError::InvalidArguments(errors.collect()));
    }

    Ok(())
}
```

---

## Cost Calculation

```typescript
// TypeScript
export function calculateCost(model: Model, usage: Usage): Usage["cost"] {
  usage.cost.input = (model.cost.input / 1000000) * usage.input;
  usage.cost.output = (model.cost.output / 1000000) * usage.output;
  usage.cost.cacheRead = (model.cost.cacheRead / 1000000) * usage.cacheRead;
  usage.cost.cacheWrite = (model.cost.cacheWrite / 1000000) * usage.cacheWrite;
  usage.cost.total = usage.cost.input + usage.cost.output + usage.cost.cacheRead + usage.cost.cacheWrite;
  return usage.cost;
}
```

```rust
// Rust
pub fn calculate_cost(model: &Model, usage: &mut Usage) -> &Cost {
    usage.cost.input = (model.cost.input / 1_000_000.0) * usage.input as f64;
    usage.cost.output = (model.cost.output / 1_000_000.0) * usage.output as f64;
    usage.cost.cache_read = (model.cost.cache_read / 1_000_000.0) * usage.cache_read as f64;
    usage.cost.cache_write = (model.cost.cache_write / 1_000_000.0) * usage.cache_write as f64;
    usage.cost.total = usage.cost.input + usage.cost.output
        + usage.cost.cache_read + usage.cost.cache_write;
    &usage.cost
}
```

---

## Key Rust Crates Needed

| Category | Crate | Purpose |
|----------|-------|---------|
| Async Runtime | `tokio` | Async/await, spawn, channels |
| HTTP Client | `reqwest` | Provider API calls |
| Serialization | `serde`, `serde_json` | JSON (de)serialization |
| Schema Validation | `jsonschema` | Tool argument validation |
| Streaming | `futures`, `tokio-stream` | Stream trait, utilities |
| AWS SDK | `aws-sdk-bedrockruntime` | Bedrock provider |
| Google SDK | `google-generative-ai` (or custom) | Google provider |
| Anthropic SDK | Custom or `anthropic-rs` | Anthropic provider |
| OpenAI SDK | `async-openai` or custom | OpenAI providers |
| Error Handling | `thiserror`, `anyhow` | Error types |
| Tracing | `tracing` | Logging/debugging |

---

## Implementation Phases

### Phase 1: Core Types and Infrastructure
1. Define all core types (`types.rs`)
2. Implement event streaming (`event_stream.rs`)
3. Create model registry structure (`models.rs`)

### Phase 2: Base Provider Abstraction
1. Define `Api` trait
2. Create `StreamOptions` trait
3. Implement main `stream()` dispatch function

### Phase 3: First Provider (Anthropic)
1. Implement Anthropic provider
2. Add message conversion
3. Add tool conversion
4. Implement SSE parsing
5. Test streaming

### Phase 4: Additional Providers
1. OpenAI Completions
2. OpenAI Responses
3. Google Generative AI
4. AWS Bedrock
5. Google Vertex

### Phase 5: Cross-Provider Features
1. Message transformation
2. Tool call ID normalization
3. Thinking block handling
4. Image support
5. Validation

---

## Knowledge Gaps

1. **Model Data Source**: Need to determine if models.generated.ts should be ported directly or regenerated from source
2. **OAuth Providers**: Decision needed on whether to support OAuth providers in future
3. **SDK Availability**: Need to verify availability of Rust SDKs for each provider
4. **TypeBox Replacement**: Confirm `schemars` or `jsonschema` as TypeBox equivalent
5. **Partial JSON**: Need Rust crate for partial JSON parsing during streaming

---

## References

### Key Files to Review

- `/home/fabian/alchemy/ai/src/types.ts` - Core type definitions
- `/home/fabian/alchemy/ai/src/stream.ts` - Main streaming API
- `/home/fabian/alchemy/ai/src/utils/event-stream.ts` - Event stream implementation
- `/home/fabian/alchemy/ai/src/models.ts` - Model registry
- `/home/fabian/alchemy/ai/src/providers/anthropic.ts` - Example provider
- `/home/fabian/alchemy/ai/src/providers/transform-messages.ts` - Cross-provider transforms

### External Documentation

- Anthropic Messages API: https://docs.anthropic.com/en/api/messages
- OpenAI Chat Completions: https://platform.openai.com/docs/api-reference/chat/create
- OpenAI Responses API: https://platform.openai.com/docs/api-reference/responses/create
- Google Generative AI: https://ai.google.dev/gemini-api/docs
- AWS Bedrock: https://docs.aws.amazon.com/bedrock/latest/userguide/conversation-inference.html
