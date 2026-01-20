# Type Mapping Reference: TypeScript to Rust

**Source:** `/home/fabian/alchemy/ai/src/types.ts`
**Purpose:** One-to-one type mapping for Rust port

---

## 1. API Type System

### TypeScript

```typescript
export type Api =
  | "openai-completions"
  | "openai-responses"
  | "openai-codex-responses"    // EXCLUDE: OAuth
  | "anthropic-messages"
  | "bedrock-converse-stream"
  | "google-generative-ai"
  | "google-gemini-cli"         // EXCLUDE: OAuth
  | "google-vertex";

export type KnownProvider =
  | "amazon-bedrock"
  | "anthropic"
  | "google"
  | "google-gemini-cli"         // EXCLUDE: OAuth
  | "google-antigravity"        // EXCLUDE: OAuth
  | "google-vertex"
  | "openai"
  | "openai-codex"              // EXCLUDE: OAuth
  | "github-copilot"            // EXCLUDE: OAuth
  | "xai"
  | "groq"
  | "cerebras"
  | "openrouter"
  | "vercel-ai-gateway"
  | "zai"
  | "mistral"
  | "minimax"
  | "minimax-cn"
  | "opencode";                 // EXCLUDE: OAuth

export type Provider = KnownProvider | string;
```

### Rust

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum Api {
    OpenAICompletions,
    OpenAIResponses,
    AnthropicMessages,
    BedrockConverseStream,
    GoogleGenerativeAi,
    GoogleVertex,
}

impl Display for Api {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::OpenAICompletions => write!(f, "openai-completions"),
            Self::OpenAIResponses => write!(f, "openai-responses"),
            Self::AnthropicMessages => write!(f, "anthropic-messages"),
            Self::BedrockConverseStream => write!(f, "bedrock-converse-stream"),
            Self::GoogleGenerativeAi => write!(f, "google-generative-ai"),
            Self::GoogleVertex => write!(f, "google-vertex"),
        }
    }
}

impl FromStr for Api {
    type Err = ApiError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "openai-completions" => Ok(Self::OpenAICompletions),
            "openai-responses" => Ok(Self::OpenAIResponses),
            "anthropic-messages" => Ok(Self::AnthropicMessages),
            "bedrock-converse-stream" => Ok(Self::BedrockConverseStream),
            "google-generative-ai" => Ok(Self::GoogleGenerativeAi),
            "google-vertex" => Ok(Self::GoogleVertex),
            _ => Err(ApiError::UnknownApi(s.to_string())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum KnownProvider {
    AmazonBedrock,
    Anthropic,
    Google,
    GoogleVertex,
    OpenAI,
    Xai,
    Groq,
    Cerebras,
    OpenRouter,
    VercelAiGateway,
    Zai,
    Mistral,
    Minimax,
    MinimaxCn,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum Provider {
    Known(KnownProvider),
    Custom(String),
}
```

---

## 2. Content Types

### TypeScript

```typescript
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

### Rust

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Content {
    Text(TextContent),
    Thinking(ThinkingContent),
    Image(ImageContent),
    ToolCall(ToolCall),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextContent {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingContent {
    pub thinking: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageContent {
    pub data: Vec<u8>,  // base64 decoded
    pub mime_type: String,
}

impl ImageContent {
    /// Convert from base64 string
    pub fn from_base64(data: &str, mime_type: String) -> Result<Self, base64::DecodeError> {
        Ok(Self {
            data: base64::engine::general_purpose::STANDARD.decode(data)?,
            mime_type,
        })
    }

    /// Convert to base64 string
    pub fn to_base64(&self) -> String {
        base64::engine::general_purpose::STANDARD.encode(&self.data)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thought_signature: Option<String>,
}
```

---

## 3. Message Types

### TypeScript

```typescript
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

export interface ToolResultMessage<TDetails = any> {
  role: "toolResult";
  toolCallId: string;
  toolName: string;
  content: (TextContent | ImageContent)[];
  details?: TDetails;
  isError: boolean;
  timestamp: number;
}

export type Message = UserMessage | AssistantMessage | ToolResultMessage;
```

### Rust

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "role")]
pub enum Message {
    User(UserMessage),
    Assistant(AssistantMessage),
    ToolResult(ToolResultMessage),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    #[serde(deserialize_with = "deserialize_user_content")]
    pub content: UserContent,
    #[serde(default = "current_timestamp")]
    pub timestamp: u64,
}

/// User content can be a simple string or array of blocks
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

/// Custom deserializer for user content (handles both string and array)
fn deserialize_user_content<'de, D>(deserializer: D) -> Result<UserContent, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ContentHelper {
        String(String),
        Array(Vec<UserContentBlock>),
    }

    match ContentHelper::deserialize(deserializer)? {
        ContentHelper::String(s) => Ok(UserContent::Text(s)),
        ContentHelper::Array(arr) => Ok(UserContent::Multi(arr)),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    pub content: Vec<Content>,
    pub api: Api,
    pub provider: Provider,
    pub model: String,
    pub usage: Usage,
    pub stop_reason: StopReason,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(default = "current_timestamp")]
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultMessage {
    pub tool_call_id: String,
    pub tool_name: String,
    pub content: Vec<ToolResultContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
    pub is_error: bool,
    #[serde(default = "current_timestamp")]
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolResultContent {
    Text(TextContent),
    Image(ImageContent),
}

fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}
```

---

## 4. Usage and Cost

### TypeScript

```typescript
export interface Usage {
  input: number;
  output: number;
  cacheRead: number;
  cacheWrite: number;
  totalTokens: number;
  cost: {
    input: number;
    output: number;
    cacheRead: number;
    cacheWrite: number;
    total: number;
  };
}
```

### Rust

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input: u32,
    pub output: u32,
    pub cache_read: u32,
    pub cache_write: u32,
    pub total_tokens: u32,
    pub cost: Cost,
}

impl Default for Usage {
    fn default() -> Self {
        Self {
            input: 0,
            output: 0,
            cache_read: 0,
            cache_write: 0,
            total_tokens: 0,
            cost: Cost::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Cost {
    pub input: f64,    // $/million tokens
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    pub total: f64,
}

impl Default for Cost {
    fn default() -> Self {
        Self {
            input: 0.0,
            output: 0.0,
            cache_read: 0.0,
            cache_write: 0.0,
            total: 0.0,
        }
    }
}
```

---

## 5. Stop Reason

### TypeScript

```typescript
export type StopReason = "stop" | "length" | "toolUse" | "error" | "aborted";
```

### Rust

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StopReason {
    Stop,
    Length,
    ToolUse,
    Error,
    Aborted,
}
```

---

## 6. Stream Events

### TypeScript

```typescript
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
  | { type: "done"; reason: Extract<StopReason, "stop" | "length" | "toolUse">; message: AssistantMessage }
  | { type: "error"; reason: Extract<StopReason, "aborted" | "error">; error: AssistantMessage };
```

### Rust

```rust
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
    Done { reason: StopReasonSuccess, message: AssistantMessage },
    Error { reason: StopReasonError, error: AssistantMessage },
}

/// Stop reasons that indicate successful completion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReasonSuccess {
    Stop,
    Length,
    ToolUse,
}

/// Stop reasons that indicate failure/abortion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopReasonError {
    Error,
    Aborted,
}

impl From<StopReasonSuccess> for StopReason {
    fn from(value: StopReasonSuccess) -> Self {
        match value {
            StopReasonSuccess::Stop => Self::Stop,
            StopReasonSuccess::Length => Self::Length,
            StopReasonSuccess::ToolUse => Self::ToolUse,
        }
    }
}

impl From<StopReasonError> for StopReason {
    fn from(value: StopReasonError) -> Self {
        match value {
            StopReasonError::Error => Self::Error,
            StopReasonError::Aborted => Self::Aborted,
        }
    }
}
```

---

## 7. Options and Configuration

### TypeScript

```typescript
export interface StreamOptions {
  temperature?: number;
  maxTokens?: number;
  signal?: AbortSignal;
  apiKey?: string;
  sessionId?: string;
  onPayload?: (payload: unknown) => void;
  headers?: Record<string, string>;
}

export interface SimpleStreamOptions extends StreamOptions {
  reasoning?: ThinkingLevel;
  thinkingBudgets?: ThinkingBudgets;
}

export type ThinkingLevel = "minimal" | "low" | "medium" | "high" | "xhigh";

export interface ThinkingBudgets {
  minimal?: number;
  low?: number;
  medium?: number;
  high?: number;
}
```

### Rust

```rust
use tokio_util::sync::CancellationToken;

/// Base trait for all stream options
pub trait StreamOptions: Send + Sync {
    fn temperature(&self) -> Option<f64> { None }
    fn max_tokens(&self) -> Option<u32> { None }
    fn api_key(&self) -> Option<&str> { None }
    fn session_id(&self) -> Option<&str> { None }
    fn headers(&self) -> Option<&HashMap<String, String>> { None }
}

/// Base stream options struct
#[derive(Debug, Clone)]
pub struct BaseStreamOptions {
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub cancellation_token: Option<CancellationToken>,
    pub api_key: Option<String>,
    pub session_id: Option<String>,
    pub headers: Option<HashMap<String, String>>,
}

impl StreamOptions for BaseStreamOptions {}

/// Unified options with reasoning level
#[derive(Debug, Clone)]
pub struct SimpleStreamOptions {
    pub base: BaseStreamOptions,
    pub reasoning: Option<ThinkingLevel>,
    pub thinking_budgets: Option<ThinkingBudgets>,
}

impl StreamOptions for SimpleStreamOptions {
    fn temperature(&self) -> Option<f64> { self.base.temperature }
    fn max_tokens(&self) -> Option<u32> { self.base.max_tokens }
    fn api_key(&self) -> Option<&str> { self.base.api_key.as_deref() }
    fn session_id(&self) -> Option<&str> { self.base.session_id.as_deref() }
    fn headers(&self) -> Option<&HashMap<String, String>> { self.base.headers.as_ref() }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingLevel {
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingBudgets {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimal: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medium: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high: Option<u32>,
}
```

---

## 8. Tools and Context

### TypeScript

```typescript
export interface Tool<TParameters extends TSchema = TSchema> {
  name: string;
  description: string;
  parameters: TSchema;  // TypeBox schema
}

export interface Context {
  systemPrompt?: string;
  messages: Message[];
  tools?: Tool[];
}
```

### Rust

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    /// JSON Schema for tool parameters
    pub parameters: serde_json::Value,
}

impl Tool {
    /// Create a tool from a type that implements JsonSchema
    pub fn from_schema<T: JsonSchema>(
        name: String,
        description: String,
    ) -> Self {
        let schema = serde_json::to_value(T::json_schema()).unwrap();
        Self {
            name,
            description,
            parameters: schema,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Context {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
}

impl Default for Context {
    fn default() -> Self {
        Self {
            system_prompt: None,
            messages: Vec::new(),
            tools: None,
        }
    }
}
```

---

## 9. Model Interface

### TypeScript

```typescript
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

### Rust

```rust
use std::marker::PhantomData;

/// Trait for API-specific compatibility options
pub trait ApiCompat: Send + Sync {
    fn as_any(&self) -> Option<&dyn std::any::Any>;
}

/// Generic model struct
#[derive(Debug, Clone)]
pub struct Model<TApi: ApiType> {
    pub id: String,
    pub name: String,
    pub api: TApi,
    pub provider: Provider,
    pub base_url: String,
    pub reasoning: bool,
    pub input: Vec<InputType>,
    pub cost: ModelCost,
    pub context_window: u32,
    pub max_tokens: u32,
    pub headers: Option<HashMap<String, String>>,
    pub compat: Option<TApi::Compat>,
}

/// Input type capabilities
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InputType {
    Text,
    Image,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ModelCost {
    pub input: f64,    // $/million tokens
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
}

/// Trait for API types
pub trait ApiType: Send + Sync {
    type Compat: ApiCompat;
}

// Implement for each API variant

#[derive(Debug, Clone, Copy)]
pub struct AnthropicMessages;

impl ApiType for AnthropicMessages {
    type Compat = NoCompat;
}

#[derive(Debug, Clone)]
pub struct OpenAICompletions {
    pub compat: Option<OpenAICompletionsCompat>,
}

impl ApiType for OpenAICompletions {
    type Compat = OpenAICompletionsCompat;
}

/// No compatibility options (default)
#[derive(Debug, Clone, Copy)]
pub struct NoCompat;

impl ApiCompat for NoCompat {
    fn as_any(&self) -> Option<&dyn std::any::Any> { None }
}
```

---

## 10. Compatibility Settings

### TypeScript

```typescript
export interface OpenAICompletionsCompat {
  supportsStore?: boolean;
  supportsDeveloperRole?: boolean;
  supportsReasoningEffort?: boolean;
  supportsUsageInStreaming?: boolean;
  maxTokensField?: "max_completion_tokens" | "max_tokens";
  requiresToolResultName?: boolean;
  requiresAssistantAfterToolResult?: boolean;
  requiresThinkingAsText?: boolean;
  requiresMistralToolIds?: boolean;
  thinkingFormat?: "openai" | "zai";
}

export interface OpenAIResponsesCompat {
  // Reserved for future use
}
```

### Rust

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAICompletionsCompat {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_store: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_developer_role: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_reasoning_effort: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_usage_in_streaming: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens_field: Option<MaxTokensField>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_tool_result_name: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_assistant_after_tool_result: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_thinking_as_text: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub requires_mistral_tool_ids: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_format: Option<ThinkingFormat>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MaxTokensField {
    MaxCompletionTokens,
    MaxTokens,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingFormat {
    Openai,
    Zai,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct OpenAIResponsesCompat {
    // Reserved for future use
}

impl ApiCompat for OpenAICompletionsCompat {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}

impl ApiCompat for OpenAIResponsesCompat {
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }
}
```

---

## 11. Stream Function Type

### TypeScript

```typescript
export type StreamFunction<TApi extends Api> = (
  model: Model<TApi>,
  context: Context,
  options: OptionsForApi<TApi>,
) => AssistantMessageEventStream;
```

### Rust

```rust
/// Trait for provider-specific streaming functions
pub trait StreamFunction<TApi: ApiType>: Send + Sync {
    fn stream(
        &self,
        model: &Model<TApi>,
        context: &Context,
        options: &TApi::Options,
    ) -> Result<AssistantMessageEventStream, StreamError>;
}

/// Error type for streaming operations
#[derive(Debug, thiserror::Error)]
pub enum StreamError {
    #[error("No API key provided for provider: {0}")]
    NoApiKey(String),

    #[error("HTTP request failed: {0}")]
    RequestError(#[from] reqwest::Error),

    #[error("API returned error: {0}")]
    ApiError(String),

    #[error("Stream aborted")]
    Aborted,

    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}
```

---

## Module Structure

```
src/
├── lib.rs              # Main exports
├── types/
│   ├── mod.rs          # Type exports
│   ├── api.rs          # Api, Provider, ApiType trait
│   ├── content.rs      # Content enums
│   ├── message.rs      # Message types
│   ├── usage.rs        # Usage, Cost, StopReason
│   ├── event.rs        # AssistantMessageEvent
│   ├── options.rs      # StreamOptions, SimpleStreamOptions
│   ├── tool.rs         # Tool, Context
│   ├── model.rs        # Model, InputType
│   └── compat.rs       # Compatibility settings
```

---

## Required Dependencies

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
base64 = "0.22"
tokio = { version = "1.0", features = ["full"] }
tokio-util = { version = "0.7", features = ["sync"] }
schemars = "0.8"  # for JSON schema generation
jsonschema = "0.18"
thiserror = "1.0"
reqwest = { version = "0.12", features = ["json", "stream"] }
```
