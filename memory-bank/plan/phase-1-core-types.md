# Phase 1: Core Types and Infrastructure

**Status:** In Progress
**Estimated Files:** 4 Rust modules
**Source Reference:** `ai/src/types.ts`, `ai/src/utils/event-stream.ts`, `ai/src/models.ts`

---

## Objective

Establish the foundational type system and streaming infrastructure that all providers will build upon.

---

## Current Progress

| Module | Status | Notes |
|--------|--------|-------|
| `types/api.rs` | DONE | Api enum, Provider, ApiType trait |
| `types/content.rs` | DONE | Content blocks with serde flatten |
| `types/message.rs` | DONE | Message types, Context |
| `types/usage.rs` | DONE | Usage, Cost, StopReason |
| `types/event.rs` | DONE | AssistantMessageEvent |
| `types/options.rs` | DONE | StreamOptions trait |
| `types/tool.rs` | DONE | Tool with schema generation |
| `types/model.rs` | DONE | Model<TApi> generic |
| `types/compat.rs` | DONE | OpenAI compat options |
| `error.rs` | DONE | Error enum |
| `stream/event_stream.rs` | TODO | Core streaming primitive |
| `models/registry.rs` | TODO | Model lookup functions |
| `models/generated.rs` | TODO | Generated model data |

---

## Remaining Work

### 1. Event Stream Implementation

**Source:** `ai/src/utils/event-stream.ts` (82 lines)

**Purpose:** Async iterator that allows pushing events and retrieving a final result.

**TypeScript Pattern:**
```typescript
export class EventStream<T, R = T> implements AsyncIterable<T> {
  private queue: T[] = [];
  private waiting: ((value: IteratorResult<T>) => void)[] = [];
  private done = false;
  private finalResultPromise: Promise<R>;

  push(event: T): void;
  end(result: R): void;
  async result(): Promise<R>;
  [Symbol.asyncIterator](): AsyncIterator<T>;
}
```

**Rust Target:** `src/stream/event_stream.rs`

```rust
use tokio::sync::{mpsc, oneshot};
use futures::Stream;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct EventStream<T, R = T> {
    rx: mpsc::UnboundedReceiver<T>,
    result_rx: Option<oneshot::Receiver<R>>,
}

pub struct EventStreamSender<T, R = T> {
    tx: mpsc::UnboundedSender<T>,
    result_tx: Option<oneshot::Sender<R>>,
}

impl<T, R> EventStream<T, R> {
    pub fn new() -> (EventStreamSender<T, R>, Self);
    pub async fn result(self) -> Result<R, oneshot::error::RecvError>;
}

impl<T, R> EventStreamSender<T, R> {
    pub fn push(&self, event: T) -> Result<(), mpsc::error::SendError<T>>;
    pub fn end(self, result: R);
}

impl<T, R> Stream for EventStream<T, R> {
    type Item = T;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>>;
}
```

**Key Decisions:**
- Split into `EventStream` (receiver) and `EventStreamSender` (sender) for ownership clarity
- Use `mpsc::unbounded_channel` for events (backpressure not needed for LLM streaming)
- Use `oneshot` for final result
- Implement `futures::Stream` trait for async iteration

---

### 2. AssistantMessageEventStream Type Alias

**Source:** `ai/src/stream.ts` (lines 1-50)

**Rust Target:** `src/stream/mod.rs`

```rust
pub type AssistantMessageEventStream = EventStream<AssistantMessageEvent, AssistantMessage>;
pub type AssistantMessageEventSender = EventStreamSender<AssistantMessageEvent, AssistantMessage>;
```

---

### 3. Model Registry

**Source:** `ai/src/models.ts` (69 lines)

**Purpose:** Functions to look up models by ID or list models by provider/API.

**TypeScript API:**
```typescript
export function getModel(modelId: string): Model | undefined;
export function getModels(): Model[];
export function getModelsByProvider(provider: Provider): Model[];
export function getModelsByApi(api: Api): Model[];
```

**Rust Target:** `src/models/registry.rs`

```rust
use std::collections::HashMap;
use once_cell::sync::Lazy;

static MODELS: Lazy<HashMap<String, AnyModel>> = Lazy::new(|| {
    // Load from generated data
});

pub fn get_model(model_id: &str) -> Option<&'static AnyModel>;
pub fn get_models() -> impl Iterator<Item = &'static AnyModel>;
pub fn get_models_by_provider(provider: &Provider) -> Vec<&'static AnyModel>;
pub fn get_models_by_api(api: Api) -> Vec<&'static AnyModel>;
```

**Key Decisions:**
- Use `once_cell::Lazy` for static initialization
- Need `AnyModel` type-erased enum since `Model<TApi>` is generic
- Consider whether to use runtime or compile-time model data

---

### 4. AnyModel Type-Erased Wrapper

**Problem:** `Model<TApi>` is generic, but registry needs homogeneous storage.

**Solution:**
```rust
// src/types/model.rs (addition)

pub enum AnyModel {
    Anthropic(Model<AnthropicMessages>),
    Bedrock(Model<BedrockConverseStream>),
    OpenAICompletions(Model<OpenAICompletions>),
    OpenAIResponses(Model<OpenAIResponses>),
    GoogleGenerativeAi(Model<GoogleGenerativeAi>),
    GoogleVertex(Model<GoogleVertex>),
}

impl AnyModel {
    pub fn id(&self) -> &str;
    pub fn name(&self) -> &str;
    pub fn api(&self) -> Api;
    pub fn provider(&self) -> &Provider;
    pub fn base_url(&self) -> &str;
    pub fn cost(&self) -> &ModelCost;
    pub fn context_window(&self) -> u32;
    pub fn max_tokens(&self) -> u32;
    pub fn supports_images(&self) -> bool;
    pub fn supports_reasoning(&self) -> bool;
}
```

---

### 5. Generated Model Data

**Source:** `ai/src/models.generated.ts` (10,825 lines)

**Options:**

1. **Build Script Generation** - `build.rs` generates Rust code
2. **JSON + Serde** - Load JSON at runtime
3. **Manual Port** - Hand-write Rust (not recommended)

**Recommended:** Option 2 (JSON + Serde)

```rust
// src/models/generated.rs
use serde::Deserialize;

#[derive(Deserialize)]
struct ModelData {
    id: String,
    name: String,
    api: String,
    provider: String,
    base_url: String,
    // ...
}

const MODEL_DATA: &str = include_str!("../../data/models.json");

pub fn load_models() -> Vec<ModelData> {
    serde_json::from_str(MODEL_DATA).expect("invalid model data")
}
```

**Action Items:**
1. Extract model data from TS to JSON format
2. Write converter script or use `scripts/generate-models.ts` output
3. Create `data/models.json` in crate

---

## File Structure After Phase 1

```
alchemy/
  src/
    lib.rs
    error.rs
    types/
      mod.rs
      api.rs        [DONE]
      content.rs    [DONE]
      message.rs    [DONE]
      usage.rs      [DONE]
      event.rs      [DONE]
      options.rs    [DONE]
      tool.rs       [DONE]
      model.rs      [DONE + AnyModel]
      compat.rs     [DONE]
    stream/
      mod.rs        [NEW]
      event_stream.rs [NEW]
    models/
      mod.rs        [NEW]
      registry.rs   [NEW]
      generated.rs  [NEW]
  data/
    models.json     [NEW]
```

---

## Acceptance Criteria

- [ ] `EventStream` compiles and passes basic tests
- [ ] Can iterate over events with `while let Some(event) = stream.next().await`
- [ ] Can retrieve final result with `stream.result().await`
- [ ] `get_model("claude-sonnet-4-20250514")` returns correct model
- [ ] `get_models_by_api(Api::AnthropicMessages)` returns Anthropic models
- [ ] All types are `Send + Sync` for async compatibility
- [ ] `cargo test` passes
- [ ] `cargo clippy` clean

---

## Dependencies

```toml
[dependencies]
once_cell = "1.19"  # Add for Lazy static
# Existing deps sufficient
```

---

## Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_stream_basic() {
        let (sender, stream) = EventStream::<i32, String>::new();

        sender.push(1).unwrap();
        sender.push(2).unwrap();
        sender.end("done".to_string());

        let events: Vec<_> = stream.collect().await;
        assert_eq!(events, vec![1, 2]);
    }

    #[tokio::test]
    async fn test_event_stream_result() {
        let (sender, stream) = EventStream::<i32, String>::new();
        sender.end("result".to_string());

        assert_eq!(stream.result().await.unwrap(), "result");
    }

    #[test]
    fn test_get_model() {
        let model = get_model("claude-sonnet-4-20250514");
        assert!(model.is_some());
        assert_eq!(model.unwrap().api(), Api::AnthropicMessages);
    }
}
```
