//! # Alchemy API Lifecycle Demo
//!
//! This example demonstrates the complete input/output flow and event lifecycle
//! of the streaming API.
//!
//! ## Flow Overview
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                           INPUT PHASE                                   │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  Model<OpenAICompletions>  ─┐                                           │
//! │  Context                   ─┼──▶  stream_openai_completions()           │
//! │  Options (api_key, etc)    ─┘                                           │
//! └─────────────────────────────────────────────────────────────────────────┘
//!                                       │
//!                                       ▼
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                         INTERNAL FLOW                                   │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  1. Build HTTP client with headers (shared/http.rs)                     │
//! │  2. Detect provider compatibility flags                                 │
//! │  3. Convert messages to provider format                                 │
//! │  4. POST to API endpoint                                                │
//! │  5. Process SSE stream chunks                                           │
//! └─────────────────────────────────────────────────────────────────────────┘
//!                                       │
//!                                       ▼
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │                        OUTPUT EVENTS                                    │
//! ├─────────────────────────────────────────────────────────────────────────┤
//! │  AssistantMessageEventStream yields:                                    │
//! │                                                                         │
//! │  1. Start        → Stream begins, partial message available             │
//! │  2. TextStart    → New text block started                               │
//! │  3. TextDelta*   → Text content chunks (main output)                    │
//! │  4. TextEnd      → Text block complete                                  │
//! │  5. Done         → Stream complete, final message with usage            │
//! │                                                                         │
//! │  Error events if something fails:                                       │
//! │  - Error         → API error, network error, etc.                       │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Event Types
//!
//! | Event          | When                        | Contains                    |
//! |----------------|-----------------------------|-----------------------------|
//! | Start          | Stream begins               | Partial AssistantMessage    |
//! | TextStart      | New text block              | content_index, partial      |
//! | TextDelta      | Each text chunk             | delta string, partial       |
//! | TextEnd        | Text block done             | full content, partial       |
//! | ThinkingStart  | Reasoning begins            | content_index, partial      |
//! | ThinkingDelta  | Reasoning chunk             | delta string, partial       |
//! | ThinkingEnd    | Reasoning done              | full thinking, partial      |
//! | ToolCallStart  | Tool call begins            | content_index, partial      |
//! | ToolCallDelta  | Arguments chunk             | delta string, partial       |
//! | ToolCallEnd    | Tool call complete          | ToolCall struct, partial    |
//! | Done           | Stream complete             | reason, final message       |
//! | Error          | Something failed            | reason, error message       |

use alchemy::providers::{stream_openai_completions, OpenAICompletionsOptions};
use alchemy::types::{
    AssistantMessageEvent, Context, InputType, Model, ModelCost, OpenAICompletions, Provider,
    UserContent, UserMessage,
};
use futures::StreamExt;

#[tokio::main]
async fn main() {
    let api_key = std::env::var("CHUTES_API_KEY").expect("CHUTES_API_KEY not set");

    // ═══════════════════════════════════════════════════════════════════════
    // INPUT: Model Configuration
    // ═══════════════════════════════════════════════════════════════════════
    println!("═══ INPUT: Model ═══");
    let model: Model<OpenAICompletions> = Model {
        id: "deepseek-ai/DeepSeek-V3-0324".to_string(),
        name: "DeepSeek V3".to_string(),
        api: OpenAICompletions,
        provider: Provider::Custom("chutes".to_string()),
        base_url: "https://llm.chutes.ai/v1/chat/completions".to_string(),
        reasoning: false,
        input: vec![InputType::Text],
        cost: ModelCost {
            input: 0.0,
            output: 0.0,
            cache_read: 0.0,
            cache_write: 0.0,
        },
        context_window: 128000,
        max_tokens: 4096,
        headers: None, // Optional custom headers
        compat: None,  // Optional compatibility overrides
    };
    println!("  id:       {}", model.id);
    println!("  base_url: {}", model.base_url);
    println!("  provider: {:?}", model.provider);
    println!();

    // ═══════════════════════════════════════════════════════════════════════
    // INPUT: Context (conversation history)
    // ═══════════════════════════════════════════════════════════════════════
    println!("═══ INPUT: Context ═══");
    let context = Context {
        system_prompt: Some("You are a helpful assistant. Be concise.".to_string()),
        messages: vec![alchemy::types::Message::User(UserMessage {
            content: UserContent::Text(
                "Explain what Rust ownership is in 2 sentences.".to_string(),
            ),
            timestamp: 0,
        })],
        tools: None, // Optional: tool definitions for function calling
    };
    println!("  system: {:?}", context.system_prompt);
    println!("  messages: {} message(s)", context.messages.len());
    if let alchemy::types::Message::User(u) = &context.messages[0] {
        println!("  user: {:?}", u.content);
    }
    println!();

    // ═══════════════════════════════════════════════════════════════════════
    // INPUT: Options
    // ═══════════════════════════════════════════════════════════════════════
    println!("═══ INPUT: Options ═══");
    let options = OpenAICompletionsOptions {
        api_key: Some(api_key),
        temperature: Some(0.7),
        max_tokens: Some(200),
        tool_choice: None,      // Optional: auto, none, required, or specific
        reasoning_effort: None, // Optional: for reasoning models
        headers: None,          // Optional: extra HTTP headers
    };
    println!("  temperature: {:?}", options.temperature);
    println!("  max_tokens:  {:?}", options.max_tokens);
    println!();

    // ═══════════════════════════════════════════════════════════════════════
    // CALL: Start streaming
    // ═══════════════════════════════════════════════════════════════════════
    println!("═══ CALLING API ═══");
    println!("  stream_openai_completions(&model, &context, options)");
    println!();

    let mut stream = stream_openai_completions(&model, &context, options);

    // ═══════════════════════════════════════════════════════════════════════
    // OUTPUT: Event stream
    // ═══════════════════════════════════════════════════════════════════════
    println!("═══ OUTPUT: Events ═══");

    let mut event_count = 0;
    let mut text_output = String::new();

    while let Some(event) = stream.next().await {
        event_count += 1;
        match &event {
            AssistantMessageEvent::Start { .. } => {
                println!("[{:02}] Start         → Stream initiated", event_count);
            }
            AssistantMessageEvent::TextStart { content_index, .. } => {
                println!(
                    "[{:02}] TextStart     → Block {} started",
                    event_count, content_index
                );
            }
            AssistantMessageEvent::TextDelta {
                delta,
                content_index,
                ..
            } => {
                text_output.push_str(delta);
                println!(
                    "[{:02}] TextDelta     → Block {}: {:?}",
                    event_count,
                    content_index,
                    if delta.len() > 40 {
                        format!("{}...", &delta[..40])
                    } else {
                        delta.clone()
                    }
                );
            }
            AssistantMessageEvent::TextEnd { content_index, .. } => {
                println!(
                    "[{:02}] TextEnd       → Block {} complete",
                    event_count, content_index
                );
            }
            AssistantMessageEvent::ThinkingStart { content_index, .. } => {
                println!(
                    "[{:02}] ThinkingStart → Block {} reasoning",
                    event_count, content_index
                );
            }
            AssistantMessageEvent::ThinkingDelta { content_index, .. } => {
                println!(
                    "[{:02}] ThinkingDelta → Block {} chunk",
                    event_count, content_index
                );
            }
            AssistantMessageEvent::ThinkingEnd { content_index, .. } => {
                println!(
                    "[{:02}] ThinkingEnd   → Block {} done",
                    event_count, content_index
                );
            }
            AssistantMessageEvent::ToolCallStart { content_index, .. } => {
                println!(
                    "[{:02}] ToolCallStart → Block {} tool",
                    event_count, content_index
                );
            }
            AssistantMessageEvent::ToolCallDelta { content_index, .. } => {
                println!(
                    "[{:02}] ToolCallDelta → Block {} args",
                    event_count, content_index
                );
            }
            AssistantMessageEvent::ToolCallEnd { tool_call, .. } => {
                println!(
                    "[{:02}] ToolCallEnd   → {} complete",
                    event_count, tool_call.name
                );
            }
            AssistantMessageEvent::Done { reason, message } => {
                println!("[{:02}] Done          → {:?}", event_count, reason);
                println!();
                println!("═══ FINAL MESSAGE ═══");
                println!("  model:       {}", message.model);
                println!("  stop_reason: {:?}", message.stop_reason);
                println!("  usage:");
                println!("    input:     {} tokens", message.usage.input);
                println!("    output:    {} tokens", message.usage.output);
                println!("    cache_read: {} tokens", message.usage.cache_read);
                println!("    total:     {} tokens", message.usage.total_tokens);
            }
            AssistantMessageEvent::Error { reason, error } => {
                println!("[{:02}] Error         → {:?}", event_count, reason);
                println!("  error: {:?}", error.error_message);
            }
        }
    }

    println!();
    println!("═══ ASSEMBLED TEXT OUTPUT ═══");
    println!("{}", text_output);
    println!();
    println!("Total events: {}", event_count);
}
