use serde::Deserialize;
use serde_json::json;

use super::openai_completions::OpenAICompletionsOptions;
use super::shared::{
    finish_current_block, handle_reasoning_delta, handle_text_delta, initialize_output,
    push_stream_done, push_stream_error, CurrentBlock, ReasoningDelta,
};
use crate::types::{
    AnthropicMessages, Api, AssistantMessage, AssistantMessageEvent, AssistantMessageEventStream,
    Content, Context, Cost, EventStreamSender, InputType, Message, Model, StopReason,
    ToolResultContent, Usage, UserContent, UserContentBlock,
};

const ANTHROPIC_VERSION: &str = "2023-06-01";
const ANTHROPIC_BETA_VALUE: &str = "interleaved-thinking-2025-05-14";
const MESSAGES_ENDPOINT: &str = "/v1/messages";
const THINKING_SIGNATURE: &str = "thinking";

pub fn stream_anthropic_messages(
    model: &Model<AnthropicMessages>,
    context: &Context,
    options: OpenAICompletionsOptions,
) -> AssistantMessageEventStream {
    let (stream, sender) = AssistantMessageEventStream::new();
    let model = model.clone();
    let context = context.clone();
    tokio::spawn(async move { run_stream(model, context, options, sender).await });
    stream
}

async fn run_stream(
    model: Model<AnthropicMessages>,
    context: Context,
    options: OpenAICompletionsOptions,
    mut sender: EventStreamSender,
) {
    let mut output = initialize_output(
        Api::AnthropicMessages,
        model.provider.clone(),
        model.id.clone(),
    );
    if let Err(e) = run_stream_inner(&model, &context, &options, &mut output, &mut sender).await {
        push_stream_error(&mut output, &mut sender, e);
    }
}

async fn run_stream_inner(
    model: &Model<AnthropicMessages>,
    context: &Context,
    options: &OpenAICompletionsOptions,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
) -> Result<(), crate::Error> {
    let api_key = options
        .api_key
        .as_ref()
        .ok_or_else(|| crate::Error::NoApiKey(model.provider.to_string()))?;

    let client = build_client(api_key, model, options)?;
    let url = format!("{}{MESSAGES_ENDPOINT}", model.base_url);
    let response = client
        .post(&url)
        .json(&build_params(model, context, options))
        .send()
        .await?;

    if !response.status().is_success() {
        let status_code = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(crate::Error::ApiError {
            status_code,
            message: body,
        });
    }

    sender.push(AssistantMessageEvent::Start {
        partial: output.clone(),
    });

    let mut current_block: Option<CurrentBlock> = None;

    super::shared::process_sse_stream_with_event::<SseEvent, _>(response, |event_type, event| {
        process_event(&event_type, &event, output, sender, &mut current_block);
    })
    .await?;

    finish_current_block(&mut current_block, output, sender);
    push_stream_done(output, sender);
    Ok(())
}

fn build_client(
    api_key: &str,
    model: &Model<AnthropicMessages>,
    options: &OpenAICompletionsOptions,
) -> Result<reqwest::Client, crate::Error> {
    use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        HeaderName::from_static("x-api-key"),
        HeaderValue::from_str(api_key).map_err(|e| crate::Error::InvalidHeader(e.to_string()))?,
    );
    headers.insert(
        HeaderName::from_static("anthropic-version"),
        HeaderValue::from_static(ANTHROPIC_VERSION),
    );
    headers.insert(
        HeaderName::from_static("anthropic-beta"),
        HeaderValue::from_static(ANTHROPIC_BETA_VALUE),
    );

    super::shared::merge_headers(&mut headers, model.headers.as_ref());
    super::shared::merge_headers(&mut headers, options.headers.as_ref());

    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(crate::Error::from)
}

fn build_params(
    model: &Model<AnthropicMessages>,
    context: &Context,
    options: &OpenAICompletionsOptions,
) -> serde_json::Value {
    let max_tokens = options.max_tokens.unwrap_or(model.max_tokens);
    let mut params = json!({
        "model": model.id,
        "stream": true,
        "max_tokens": max_tokens,
        "messages": convert_messages(model, context),
    });

    if let Some(sys) = &context.system_prompt {
        params["system"] = json!(sys);
    }
    if let Some(t) = options.temperature {
        params["temperature"] = json!(t);
    }
    if let Some(tools) = &context.tools {
        let defs: Vec<_> = tools
            .iter()
            .map(|t| {
                json!({
                    "name": t.name, "description": t.description, "input_schema": t.parameters,
                })
            })
            .collect();
        params["tools"] = json!(defs);
    }
    if model.reasoning {
        params["thinking"] = json!({
            "type": "enabled",
            "budget_tokens": max_tokens.saturating_sub(100)
        });
    }

    params
}

fn convert_messages(model: &Model<AnthropicMessages>, context: &Context) -> Vec<serde_json::Value> {
    context.messages.iter().filter_map(|m| match m {
        Message::User(u) => Some(json!({"role": "user", "content": convert_user_content(model, &u.content)})),
        Message::Assistant(a) => {
            let content = convert_assistant_content(a);
            (!content.is_empty()).then(|| json!({"role": "assistant", "content": content}))
        }
        Message::ToolResult(r) => {
            let text = r.content.iter().filter_map(|c| match c {
                ToolResultContent::Text(t) => Some(t.text.clone()),
                ToolResultContent::Image(_) => None,
            }).collect::<Vec<_>>().join("\n");
            Some(json!({"role": "user", "content": [{"type": "tool_result", "tool_use_id": r.tool_call_id.as_str(), "content": text, "is_error": r.is_error}]}))
        }
    }).collect()
}

fn convert_user_content(
    model: &Model<AnthropicMessages>,
    content: &UserContent,
) -> serde_json::Value {
    match content {
        UserContent::Text(text) => json!(text),
        UserContent::Multi(blocks) => json!(blocks.iter().filter_map(|b| match b {
            UserContentBlock::Text(t) => Some(json!({"type": "text", "text": t.text})),
            UserContentBlock::Image(img) if model.input.contains(&InputType::Image) => Some(json!({
                "type": "image", "source": {"type": "base64", "media_type": img.mime_type, "data": img.to_base64()}
            })),
            UserContentBlock::Image(_) => None,
        }).collect::<Vec<_>>()),
    }
}

fn convert_assistant_content(assistant: &AssistantMessage) -> Vec<serde_json::Value> {
    assistant.content.iter().filter_map(|c| match c {
        Content::Text { inner } if !inner.text.is_empty() => Some(json!({"type": "text", "text": inner.text})),
        Content::Thinking { inner } if !inner.thinking.is_empty() => {
            let mut block = json!({"type": "thinking", "thinking": inner.thinking});
            if let Some(sig) = &inner.thinking_signature {
                block["signature"] = json!(sig);
            }
            Some(block)
        }
        Content::ToolCall { inner } => Some(json!({"type": "tool_use", "id": inner.id.as_str(), "name": inner.name, "input": inner.arguments})),
        _ => None,
    }).collect()
}

fn process_event(
    event_type: &str,
    event: &SseEvent,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
    current_block: &mut Option<CurrentBlock>,
) {
    match event_type {
        "content_block_start" => handle_block_start(event, output, sender, current_block),
        "content_block_delta" => handle_block_delta(event, output, sender, current_block),
        "content_block_stop" => finish_current_block(current_block, output, sender),
        "message_start" => handle_message_start(event, output),
        "message_delta" => handle_message_delta(event, output),
        _ => {}
    }
}

fn handle_block_start(
    event: &SseEvent,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
    current_block: &mut Option<CurrentBlock>,
) {
    finish_current_block(current_block, output, sender);
    let Some(block) = &event.content_block else {
        return;
    };
    if block.block_type != "tool_use" {
        return;
    }
    let id = block.id.clone().unwrap_or_default();
    let name = block.name.clone().unwrap_or_default();
    *current_block = Some(CurrentBlock::ToolCall {
        id: id.clone(),
        name: name.clone(),
        partial_args: String::new(),
    });
    output.content.push(Content::tool_call(
        id,
        name,
        serde_json::Value::Object(serde_json::Map::new()),
    ));
    sender.push(AssistantMessageEvent::ToolCallStart {
        content_index: output.content.len() - 1,
        partial: output.clone(),
    });
}

fn handle_block_delta(
    event: &SseEvent,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
    current_block: &mut Option<CurrentBlock>,
) {
    let Some(delta) = &event.delta else { return };
    match delta.delta_type.as_deref() {
        Some("text_delta") => {
            if let Some(text) = &delta.text {
                handle_text_delta(text, output, sender, current_block);
            }
        }
        Some("thinking_delta") => {
            if let Some(thinking) = &delta.thinking {
                handle_reasoning_delta(
                    ReasoningDelta {
                        text: thinking,
                        signature: THINKING_SIGNATURE,
                    },
                    output,
                    sender,
                    current_block,
                );
            }
        }
        Some("signature_delta") => {
            if let Some(sig) = &delta.signature {
                if let Some(CurrentBlock::Thinking { signature, .. }) = current_block {
                    *signature = sig.clone();
                }
            }
        }
        Some("input_json_delta") => {
            if let Some(partial) = &delta.partial_json {
                if let Some(CurrentBlock::ToolCall { partial_args, .. }) = current_block {
                    partial_args.push_str(partial);
                    sender.push(AssistantMessageEvent::ToolCallDelta {
                        content_index: output.content.len().saturating_sub(1),
                        delta: partial.clone(),
                        partial: output.clone(),
                    });
                }
            }
        }
        _ => {}
    }
}

fn handle_message_start(event: &SseEvent, output: &mut AssistantMessage) {
    let Some(msg) = &event.message else { return };
    let Some(u) = &msg.usage else { return };
    output.usage = Usage {
        input: u.input_tokens.unwrap_or(0),
        output: 0,
        cache_read: u.cache_read_input_tokens.unwrap_or(0),
        cache_write: u.cache_creation_input_tokens.unwrap_or(0),
        total_tokens: u.input_tokens.unwrap_or(0),
        cost: Cost::default(),
    };
}

fn handle_message_delta(event: &SseEvent, output: &mut AssistantMessage) {
    if let Some(delta) = &event.delta {
        if let Some(reason) = &delta.stop_reason {
            output.stop_reason = map_stop_reason(reason);
        }
    }
    if let Some(u) = &event.usage {
        output.usage.output = u.output_tokens.unwrap_or(0);
        output.usage.total_tokens = output.usage.input + output.usage.output;
    }
}

fn map_stop_reason(reason: &str) -> StopReason {
    match reason {
        "end_turn" | "stop_sequence" => StopReason::Stop,
        "max_tokens" => StopReason::Length,
        "tool_use" => StopReason::ToolUse,
        _ => StopReason::Stop,
    }
}

#[derive(Debug, Deserialize)]
struct SseEvent {
    delta: Option<SseDelta>,
    content_block: Option<ContentBlock>,
    message: Option<MessageStart>,
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct SseDelta {
    #[serde(rename = "type")]
    delta_type: Option<String>,
    text: Option<String>,
    thinking: Option<String>,
    signature: Option<String>,
    partial_json: Option<String>,
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    id: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessageStart {
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
    cache_read_input_tokens: Option<u32>,
    cache_creation_input_tokens: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        AssistantMessageEvent, KnownProvider, ModelCost, Provider, Tool, UserMessage,
    };
    use futures::executor::block_on;
    use futures::StreamExt;

    fn make_model(reasoning: bool) -> Model<AnthropicMessages> {
        Model {
            id: "claude-sonnet-4-6".to_string(),
            name: "Claude Sonnet 4.6".to_string(),
            api: AnthropicMessages,
            provider: Provider::Known(KnownProvider::Anthropic),
            base_url: "https://api.anthropic.com".to_string(),
            reasoning,
            input: vec![InputType::Text, InputType::Image],
            cost: ModelCost {
                input: 0.003,
                output: 0.015,
                cache_read: 0.0003,
                cache_write: 0.00375,
            },
            context_window: 200_000,
            max_tokens: 8_192,
            headers: None,
            compat: None,
        }
    }

    fn make_context() -> Context {
        Context {
            system_prompt: Some("You are concise".to_string()),
            messages: vec![Message::User(UserMessage {
                content: UserContent::Text("Hello".to_string()),
                timestamp: 0,
            })],
            tools: None,
        }
    }

    fn make_output() -> AssistantMessage {
        AssistantMessage {
            content: vec![],
            api: Api::AnthropicMessages,
            provider: Provider::Known(KnownProvider::Anthropic),
            model: "claude-sonnet-4-6".to_string(),
            usage: Usage::default(),
            stop_reason: StopReason::Stop,
            error_message: None,
            timestamp: 0,
        }
    }

    fn run_events(events: Vec<(&str, SseEvent)>) -> (Vec<AssistantMessageEvent>, AssistantMessage) {
        let (mut stream, mut sender) = AssistantMessageEventStream::new();
        let mut output = make_output();
        let mut current_block = None;
        for (t, e) in events {
            process_event(t, &e, &mut output, &mut sender, &mut current_block);
        }
        finish_current_block(&mut current_block, &mut output, &mut sender);
        drop(sender);
        let collected = block_on(async move { stream.by_ref().collect::<Vec<_>>().await });
        (collected, output)
    }

    #[test]
    fn build_params_constructs_anthropic_format() {
        let params = build_params(
            &make_model(false),
            &make_context(),
            &OpenAICompletionsOptions {
                temperature: Some(0.7),
                max_tokens: Some(1024),
                ..Default::default()
            },
        );
        assert_eq!(params["model"], "claude-sonnet-4-6");
        assert_eq!(params["max_tokens"], 1024);
        assert_eq!(params["system"], "You are concise");
        assert_eq!(params["temperature"], 0.7);
        assert!(params.get("thinking").is_none());
    }

    #[test]
    fn build_params_includes_thinking_for_reasoning() {
        let params = build_params(
            &make_model(true),
            &make_context(),
            &OpenAICompletionsOptions {
                max_tokens: Some(4096),
                ..Default::default()
            },
        );
        assert_eq!(params["thinking"]["type"], "enabled");
        assert_eq!(params["thinking"]["budget_tokens"], 3996);
    }

    #[test]
    fn build_params_uses_input_schema_for_tools() {
        let ctx = Context {
            system_prompt: None,
            tools: Some(vec![Tool::new(
                "get_weather",
                "Get weather",
                json!({"type": "object"}),
            )]),
            messages: vec![Message::User(UserMessage {
                content: UserContent::Text("t".into()),
                timestamp: 0,
            })],
        };
        let params = build_params(&make_model(false), &ctx, &Default::default());
        assert_eq!(params["tools"][0]["name"], "get_weather");
        assert_eq!(params["tools"][0]["input_schema"]["type"], "object");
    }

    #[test]
    fn text_delta_emits_text_events() {
        let e: SseEvent =
            serde_json::from_value(json!({"delta": {"type": "text_delta", "text": "Hello"}}))
                .unwrap();
        let (events, output) = run_events(vec![("content_block_delta", e)]);
        assert!(matches!(events[0], AssistantMessageEvent::TextStart { .. }));
        assert!(matches!(&output.content[0], Content::Text { inner } if inner.text == "Hello"));
    }

    #[test]
    fn thinking_delta_emits_thinking_events() {
        let e: SseEvent =
            serde_json::from_value(json!({"delta": {"type": "thinking_delta", "thinking": "hmm"}}))
                .unwrap();
        let (events, output) = run_events(vec![("content_block_delta", e)]);
        assert!(matches!(
            events[0],
            AssistantMessageEvent::ThinkingStart { .. }
        ));
        assert!(
            matches!(&output.content[0], Content::Thinking { inner } if inner.thinking == "hmm")
        );
    }

    #[test]
    fn tool_use_block_emits_tool_call() {
        let start: SseEvent = serde_json::from_value(
            json!({"content_block": {"type": "tool_use", "id": "toolu_1", "name": "calc"}}),
        )
        .unwrap();
        let delta: SseEvent = serde_json::from_value(
            json!({"delta": {"type": "input_json_delta", "partial_json": "{\"x\":1}"}}),
        )
        .unwrap();
        let stop: SseEvent = serde_json::from_value(json!({})).unwrap();
        let (_events, output) = run_events(vec![
            ("content_block_start", start),
            ("content_block_delta", delta),
            ("content_block_stop", stop),
        ]);
        assert!(
            matches!(&output.content[0], Content::ToolCall { inner } if inner.id.as_str() == "toolu_1" && inner.name == "calc")
        );
    }

    #[test]
    fn stop_reason_mapping() {
        assert_eq!(map_stop_reason("end_turn"), StopReason::Stop);
        assert_eq!(map_stop_reason("max_tokens"), StopReason::Length);
        assert_eq!(map_stop_reason("tool_use"), StopReason::ToolUse);
    }

    #[test]
    fn usage_from_message_start_and_delta() {
        let start: SseEvent = serde_json::from_value(
            json!({"message": {"usage": {"input_tokens": 100, "cache_read_input_tokens": 20}}}),
        )
        .unwrap();
        let delta: SseEvent = serde_json::from_value(
            json!({"delta": {"stop_reason": "end_turn"}, "usage": {"output_tokens": 50}}),
        )
        .unwrap();
        let (_, output) = run_events(vec![("message_start", start), ("message_delta", delta)]);
        assert_eq!(output.usage.input, 100);
        assert_eq!(output.usage.output, 50);
        assert_eq!(output.usage.cache_read, 20);
        assert_eq!(output.usage.total_tokens, 150);
    }

    #[test]
    fn assistant_replay_preserves_thinking_signature() {
        let assistant = AssistantMessage {
            content: vec![
                Content::Thinking {
                    inner: crate::types::ThinkingContent {
                        thinking: "reasoning".into(),
                        thinking_signature: Some("sig123".into()),
                    },
                },
                Content::text("answer"),
            ],
            api: Api::AnthropicMessages,
            provider: Provider::Known(KnownProvider::Anthropic),
            model: "claude-sonnet-4-6".into(),
            usage: Usage::default(),
            stop_reason: StopReason::Stop,
            error_message: None,
            timestamp: 0,
        };
        let converted = convert_assistant_content(&assistant);
        assert_eq!(converted[0]["signature"], "sig123");
        assert_eq!(converted[1]["text"], "answer");
    }
}
