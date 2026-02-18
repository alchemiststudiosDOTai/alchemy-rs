use serde::Deserialize;
use serde_json::json;

use super::openai_completions::OpenAICompletionsOptions;
use super::shared::{
    build_http_client, convert_messages, convert_tools, finish_current_block,
    handle_reasoning_delta, handle_text_delta, handle_tool_calls, initialize_output,
    map_stop_reason, process_sse_stream, push_stream_done, push_stream_error,
    send_streaming_request, update_usage_from_chunk, CurrentBlock, OpenAiLikeMessageOptions,
    OpenAiLikeStreamUsage, OpenAiLikeToolCallDelta, ReasoningDelta, SystemPromptRole,
};
use crate::stream::{AssistantMessageEventStream, EventStreamSender};
use crate::types::{
    Api, AssistantMessage, AssistantMessageEvent, Context, MinimaxCompletions, Model,
};
use crate::utils::{ThinkFragment, ThinkTagParser};

const STREAM_INCLUDE_USAGE_FIELD: &str = "include_usage";
const MAX_TOKENS_FIELD: &str = "max_tokens";
const REASONING_SPLIT_FIELD: &str = "reasoning_split";
const REASONING_DETAILS_SIGNATURE: &str = "reasoning_details";
const REASONING_CONTENT_SIGNATURE: &str = "reasoning_content";
const REASONING_SIGNATURE: &str = "reasoning";
const REASONING_TEXT_SIGNATURE: &str = "reasoning_text";
const THINK_TAG_SIGNATURE: &str = "think_tag";

/// Stream completions from MiniMax chat/completions API.
pub fn stream_minimax_completions(
    model: &Model<MinimaxCompletions>,
    context: &Context,
    options: OpenAICompletionsOptions,
) -> AssistantMessageEventStream {
    let (stream, sender) = AssistantMessageEventStream::new();

    let model = model.clone();
    let context = context.clone();

    tokio::spawn(async move {
        run_stream(model, context, options, sender).await;
    });

    stream
}

async fn run_stream(
    model: Model<MinimaxCompletions>,
    context: Context,
    options: OpenAICompletionsOptions,
    mut sender: EventStreamSender,
) {
    let mut output = initialize_output(
        Api::MinimaxCompletions,
        model.provider.clone(),
        model.id.clone(),
    );

    if let Err(error) = run_stream_inner(&model, &context, &options, &mut output, &mut sender).await
    {
        push_stream_error(&mut output, &mut sender, error);
    }
}

async fn run_stream_inner(
    model: &Model<MinimaxCompletions>,
    context: &Context,
    options: &OpenAICompletionsOptions,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
) -> Result<(), crate::Error> {
    let api_key = options
        .api_key
        .as_ref()
        .ok_or_else(|| crate::Error::NoApiKey(model.provider.to_string()))?;

    let client = build_http_client(api_key, model.headers.as_ref(), options.headers.as_ref())?;
    let params = build_params(model, context, options);

    let response = send_streaming_request(&client, &model.base_url, &params).await?;

    sender.push(AssistantMessageEvent::Start {
        partial: output.clone(),
    });

    let mut current_block: Option<CurrentBlock> = None;
    let mut think_tag_parser = ThinkTagParser::new();

    process_sse_stream::<StreamChunk, _>(response, |chunk| {
        process_chunk(
            &chunk,
            output,
            sender,
            &mut current_block,
            &mut think_tag_parser,
        );
    })
    .await?;

    flush_think_tag_parser(&mut think_tag_parser, output, sender, &mut current_block);
    finish_current_block(&mut current_block, output, sender);
    push_stream_done(output, sender);

    Ok(())
}

fn build_params(
    model: &Model<MinimaxCompletions>,
    context: &Context,
    options: &OpenAICompletionsOptions,
) -> serde_json::Value {
    let message_options = OpenAiLikeMessageOptions {
        system_role: SystemPromptRole::System,
        requires_tool_result_name: false,
    };

    let mut params = json!({
        "model": model.id,
        "stream": true,
        "messages": convert_messages(model, context, &message_options),
    });

    let mut stream_options = serde_json::Map::new();
    stream_options.insert(STREAM_INCLUDE_USAGE_FIELD.to_string(), json!(true));
    params["stream_options"] = serde_json::Value::Object(stream_options);

    if model.reasoning {
        params[REASONING_SPLIT_FIELD] = json!(true);
    }

    if let Some(max_tokens) = options.max_tokens {
        params[MAX_TOKENS_FIELD] = json!(max_tokens);
    }

    if let Some(temperature) = options.temperature {
        params["temperature"] = json!(temperature);
    }

    if let Some(tools) = &context.tools {
        params["tools"] = convert_tools(tools);
    }

    if let Some(tool_choice) = &options.tool_choice {
        params["tool_choice"] = serde_json::to_value(tool_choice).unwrap_or(json!("auto"));
    }

    params
}

fn process_chunk(
    chunk: &StreamChunk,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
    current_block: &mut Option<CurrentBlock>,
    think_tag_parser: &mut ThinkTagParser,
) {
    if let Some(usage) = &chunk.usage {
        update_usage_from_chunk(usage, output);
    }

    let Some(choice) = chunk.choices.first() else {
        return;
    };

    if let Some(reason) = &choice.finish_reason {
        output.stop_reason = map_stop_reason(reason);
    }

    let Some(delta) = &choice.delta else {
        return;
    };

    let explicit_reasoning = emit_explicit_reasoning(delta, output, sender, current_block);

    if let Some(content) = delta.content.as_deref() {
        if explicit_reasoning {
            handle_text_delta(content, output, sender, current_block);
        } else {
            process_content_with_fallback(content, think_tag_parser, output, sender, current_block);
        }
    }

    if let Some(tool_calls) = &delta.tool_calls {
        handle_tool_calls(tool_calls, output, sender, current_block);
    }
}

fn emit_explicit_reasoning(
    delta: &StreamDelta,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
    current_block: &mut Option<CurrentBlock>,
) -> bool {
    let mut emitted_reasoning = false;

    if let Some(details) = &delta.reasoning_details {
        for detail in details {
            if let Some(text) = detail.text.as_deref() {
                if text.is_empty() {
                    continue;
                }

                emitted_reasoning = true;
                handle_reasoning_delta(
                    ReasoningDelta {
                        text,
                        signature: REASONING_DETAILS_SIGNATURE,
                    },
                    output,
                    sender,
                    current_block,
                );
            }
        }
    }

    if emitted_reasoning {
        return true;
    }

    for (text, signature) in [
        (
            delta.reasoning_content.as_deref(),
            REASONING_CONTENT_SIGNATURE,
        ),
        (delta.reasoning.as_deref(), REASONING_SIGNATURE),
        (delta.reasoning_text.as_deref(), REASONING_TEXT_SIGNATURE),
    ] {
        if let Some(reasoning_text) = text {
            if reasoning_text.is_empty() {
                continue;
            }

            handle_reasoning_delta(
                ReasoningDelta {
                    text: reasoning_text,
                    signature,
                },
                output,
                sender,
                current_block,
            );
            return true;
        }
    }

    false
}

fn process_content_with_fallback(
    content: &str,
    think_tag_parser: &mut ThinkTagParser,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
    current_block: &mut Option<CurrentBlock>,
) {
    for fragment in think_tag_parser.feed(content) {
        match fragment {
            ThinkFragment::Text(text) => {
                handle_text_delta(&text, output, sender, current_block);
            }
            ThinkFragment::Thinking(thinking) => {
                handle_reasoning_delta(
                    ReasoningDelta {
                        text: &thinking,
                        signature: THINK_TAG_SIGNATURE,
                    },
                    output,
                    sender,
                    current_block,
                );
            }
        }
    }
}

fn flush_think_tag_parser(
    think_tag_parser: &mut ThinkTagParser,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
    current_block: &mut Option<CurrentBlock>,
) {
    for fragment in think_tag_parser.flush() {
        match fragment {
            ThinkFragment::Text(text) => {
                handle_text_delta(&text, output, sender, current_block);
            }
            ThinkFragment::Thinking(thinking) => {
                handle_reasoning_delta(
                    ReasoningDelta {
                        text: &thinking,
                        signature: THINK_TAG_SIGNATURE,
                    },
                    output,
                    sender,
                    current_block,
                );
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct StreamChunk {
    #[serde(default)]
    choices: Vec<StreamChoice>,
    usage: Option<OpenAiLikeStreamUsage>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: Option<StreamDelta>,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamDelta {
    content: Option<String>,
    reasoning_details: Option<Vec<ReasoningDetail>>,
    reasoning_content: Option<String>,
    reasoning: Option<String>,
    reasoning_text: Option<String>,
    tool_calls: Option<Vec<OpenAiLikeToolCallDelta>>,
}

#[derive(Debug, Deserialize)]
struct ReasoningDetail {
    #[serde(rename = "type")]
    _detail_type: Option<String>,
    _id: Option<String>,
    _format: Option<String>,
    _index: Option<u32>,
    text: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        AssistantMessageEvent, InputType, KnownProvider, ModelCost, Provider, StopReason, Usage,
        UserContent, UserMessage,
    };
    use futures::executor::block_on;
    use futures::StreamExt;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    const TEST_BASE_URL: &str = "https://api.minimax.io/v1/chat/completions";

    fn make_model(reasoning: bool) -> Model<MinimaxCompletions> {
        Model {
            id: "MiniMax-M2.5".to_string(),
            name: "MiniMax M2.5".to_string(),
            api: MinimaxCompletions,
            provider: Provider::Known(KnownProvider::Minimax),
            base_url: TEST_BASE_URL.to_string(),
            reasoning,
            input: vec![InputType::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 204_800,
            max_tokens: 16_384,
            headers: None,
            compat: None,
        }
    }

    fn make_context() -> Context {
        Context {
            system_prompt: Some("You are concise".to_string()),
            messages: vec![crate::types::Message::User(UserMessage {
                content: UserContent::Text("Hello".to_string()),
                timestamp: 0,
            })],
            tools: None,
        }
    }

    fn make_output_message() -> AssistantMessage {
        AssistantMessage {
            content: vec![],
            api: Api::MinimaxCompletions,
            provider: Provider::Known(KnownProvider::Minimax),
            model: "MiniMax-M2.5".to_string(),
            usage: Usage::default(),
            stop_reason: StopReason::Stop,
            error_message: None,
            timestamp: 0,
        }
    }

    fn process_chunks_for_test(
        chunks: Vec<StreamChunk>,
    ) -> (Vec<AssistantMessageEvent>, AssistantMessage) {
        let (mut stream, mut sender) = AssistantMessageEventStream::new();
        let mut output = make_output_message();
        let mut current_block = None;
        let mut parser = ThinkTagParser::new();

        for chunk in chunks {
            process_chunk(
                &chunk,
                &mut output,
                &mut sender,
                &mut current_block,
                &mut parser,
            );
        }

        flush_think_tag_parser(&mut parser, &mut output, &mut sender, &mut current_block);
        finish_current_block(&mut current_block, &mut output, &mut sender);

        drop(sender);

        let events = block_on(async move { stream.by_ref().collect::<Vec<_>>().await });
        (events, output)
    }

    #[test]
    fn build_params_for_reasoning_model_uses_minimax_semantics() {
        let model = make_model(true);
        let context = make_context();
        let options = OpenAICompletionsOptions {
            temperature: Some(1.2),
            max_tokens: Some(512),
            ..OpenAICompletionsOptions::default()
        };

        let params = build_params(&model, &context, &options);

        assert_eq!(params["stream"], true);
        assert_eq!(params["stream_options"][STREAM_INCLUDE_USAGE_FIELD], true);
        assert_eq!(params[REASONING_SPLIT_FIELD], true);
        assert_eq!(params[MAX_TOKENS_FIELD], 512);
        assert_eq!(params["temperature"], 1.2);
        assert_eq!(params["messages"][0]["role"], "system");
        assert_eq!(params["messages"][0]["content"], "You are concise");
        assert!(params.get("n").is_none());
        assert!(params.get("max_completion_tokens").is_none());
    }

    #[test]
    fn build_params_for_non_reasoning_model_omits_reasoning_split() {
        let model = make_model(false);
        let context = make_context();
        let options = OpenAICompletionsOptions::default();

        let params = build_params(&model, &context, &options);

        assert!(params.get(REASONING_SPLIT_FIELD).is_none());
    }

    #[test]
    fn process_chunk_maps_reasoning_details_to_thinking_events() {
        let chunk: StreamChunk = serde_json::from_value(serde_json::json!({
            "choices": [{
                "delta": {
                    "reasoning_details": [
                        { "type": "reasoning", "id": "r-1", "format": "text", "index": 0, "text": "step one" }
                    ]
                }
            }]
        }))
        .expect("valid reasoning details payload");

        let (events, output) = process_chunks_for_test(vec![chunk]);

        assert!(matches!(
            events[0],
            AssistantMessageEvent::ThinkingStart { .. }
        ));
        assert!(matches!(
            events[1],
            AssistantMessageEvent::ThinkingDelta { .. }
        ));
        assert!(matches!(
            events[2],
            AssistantMessageEvent::ThinkingEnd { .. }
        ));

        match &output.content[0] {
            crate::types::Content::Thinking { inner } => {
                assert_eq!(inner.thinking, "step one");
                assert_eq!(
                    inner.thinking_signature.as_deref(),
                    Some(REASONING_DETAILS_SIGNATURE)
                );
            }
            _ => panic!("expected thinking content"),
        }
    }

    #[test]
    fn process_chunk_with_inline_think_tags_emits_expected_event_order() {
        let chunk: StreamChunk = serde_json::from_value(serde_json::json!({
            "choices": [{
                "delta": {
                    "content": "<think>reason</think>answer"
                }
            }]
        }))
        .expect("valid think-tag payload");

        let (events, output) = process_chunks_for_test(vec![chunk]);

        assert!(matches!(
            events[0],
            AssistantMessageEvent::ThinkingStart { .. }
        ));
        assert!(matches!(
            events[1],
            AssistantMessageEvent::ThinkingDelta { .. }
        ));
        assert!(matches!(
            events[2],
            AssistantMessageEvent::ThinkingEnd { .. }
        ));
        assert!(matches!(events[3], AssistantMessageEvent::TextStart { .. }));
        assert!(matches!(events[4], AssistantMessageEvent::TextDelta { .. }));
        assert!(matches!(events[5], AssistantMessageEvent::TextEnd { .. }));

        assert_eq!(output.content.len(), 2);
        match &output.content[0] {
            crate::types::Content::Thinking { inner } => {
                assert_eq!(inner.thinking, "reason");
                assert_eq!(
                    inner.thinking_signature.as_deref(),
                    Some(THINK_TAG_SIGNATURE)
                );
            }
            _ => panic!("expected thinking content"),
        }
        match &output.content[1] {
            crate::types::Content::Text { inner } => {
                assert_eq!(inner.text, "answer");
            }
            _ => panic!("expected text content"),
        }
    }

    #[test]
    fn usage_only_chunk_updates_output_usage() {
        let chunk: StreamChunk = serde_json::from_value(serde_json::json!({
            "choices": [],
            "usage": {
                "prompt_tokens": 12,
                "completion_tokens": 8,
                "total_tokens": 20
            }
        }))
        .expect("valid usage-only payload");

        let (_events, output) = process_chunks_for_test(vec![chunk]);

        assert_eq!(output.usage.input, 12);
        assert_eq!(output.usage.output, 8);
        assert_eq!(output.usage.total_tokens, 20);
    }

    async fn spawn_sse_server(body: String) -> String {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test listener");
        let address = listener.local_addr().expect("listener address");

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept request");

            let mut request_buffer = [0_u8; 4096];
            let _ = socket.read(&mut request_buffer).await;

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body,
            );

            socket
                .write_all(response.as_bytes())
                .await
                .expect("write response");
        });

        format!("http://{}/v1/chat/completions", address)
    }

    #[tokio::test]
    async fn stream_minimax_completions_final_message_shape() {
        let sse_body = concat!(
            "data: {\"choices\":[{\"delta\":{\"reasoning_details\":[{\"text\":\"reason\"}]}}]}\n\n",
            "data: {\"choices\":[{\"delta\":{\"content\":\"answer\"},\"finish_reason\":\"stop\"}]}\n\n",
            "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":10,\"completion_tokens\":5,\"total_tokens\":15}}\n\n",
            "data: [DONE]\n\n"
        )
        .to_string();

        let server_url = spawn_sse_server(sse_body).await;

        let mut model = make_model(true);
        model.base_url = server_url;

        let options = OpenAICompletionsOptions {
            api_key: Some("test-key".to_string()),
            ..OpenAICompletionsOptions::default()
        };

        let stream = stream_minimax_completions(&model, &make_context(), options);
        let result = stream.result().await.expect("stream result");

        assert_eq!(result.api, Api::MinimaxCompletions);
        assert_eq!(result.provider, Provider::Known(KnownProvider::Minimax));
        assert_eq!(result.model, "MiniMax-M2.5");
        assert_eq!(result.stop_reason, StopReason::Stop);
        assert_eq!(result.usage.total_tokens, 15);
    }
}
