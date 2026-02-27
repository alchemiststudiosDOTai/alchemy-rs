use serde::Deserialize;
use serde_json::json;

use super::openai_completions::OpenAICompletionsOptions;
use super::shared::{
    build_http_client, convert_messages, convert_tools, finish_current_block,
    handle_reasoning_delta, handle_text_delta, handle_tool_calls, initialize_output,
    map_stop_reason, process_sse_stream, push_stream_done, push_stream_error,
    send_streaming_request, update_usage_from_chunk, AssistantThinkingMode, CurrentBlock,
    OpenAiLikeMessageOptions, OpenAiLikeStreamUsage, OpenAiLikeToolCallDelta, ReasoningDelta,
    SystemPromptRole,
};
use crate::stream::{AssistantMessageEventStream, EventStreamSender};
use crate::types::{
    Api, AssistantMessage, AssistantMessageEvent, Context, Model, StopReason, ZaiCompletions,
};

const STREAM_INCLUDE_USAGE_FIELD: &str = "include_usage";
const REASONING_CONTENT_SIGNATURE: &str = "reasoning_content";
const REASONING_SIGNATURE: &str = "reasoning";
const REASONING_TEXT_SIGNATURE: &str = "reasoning_text";

/// Stream completions from z.ai chat/completions API.
pub fn stream_zai_completions(
    model: &Model<ZaiCompletions>,
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
    model: Model<ZaiCompletions>,
    context: Context,
    options: OpenAICompletionsOptions,
    mut sender: EventStreamSender,
) {
    let mut output = initialize_output(
        Api::ZaiCompletions,
        model.provider.clone(),
        model.id.clone(),
    );

    if let Err(error) = run_stream_inner(&model, &context, &options, &mut output, &mut sender).await
    {
        push_stream_error(&mut output, &mut sender, error);
    }
}

async fn run_stream_inner(
    model: &Model<ZaiCompletions>,
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

    process_sse_stream::<StreamChunk, _>(response, |chunk| {
        process_chunk(&chunk, output, sender, &mut current_block);
    })
    .await?;

    finish_current_block(&mut current_block, output, sender);
    push_stream_done(output, sender);

    Ok(())
}

fn build_params(
    model: &Model<ZaiCompletions>,
    context: &Context,
    options: &OpenAICompletionsOptions,
) -> serde_json::Value {
    let message_options = OpenAiLikeMessageOptions {
        assistant_content_as_string: true,
        emit_reasoning_content_field: true,
        tool_call_arguments_as_object: false,
        ..OpenAiLikeMessageOptions::openai_like(
            SystemPromptRole::System,
            false,
            AssistantThinkingMode::Omit,
        )
    };

    let mut params = json!({
        "model": model.id,
        "stream": true,
        "messages": convert_messages(model, context, &message_options),
    });

    params["stream_options"] = json!({ STREAM_INCLUDE_USAGE_FIELD: true });

    add_max_tokens(&mut params, options);
    add_temperature(&mut params, options);
    add_tools(&mut params, context);
    add_tool_choice(&mut params, options);
    add_zai_optional_fields(&mut params, options);
    add_default_reasoning(&mut params, model, options);

    params
}

fn add_max_tokens(params: &mut serde_json::Value, options: &OpenAICompletionsOptions) {
    let max_tokens = options
        .zai
        .as_ref()
        .and_then(|zai_options| zai_options.max_tokens)
        .or(options.max_tokens);

    if let Some(value) = max_tokens {
        params["max_tokens"] = json!(value);
    }
}

fn add_temperature(params: &mut serde_json::Value, options: &OpenAICompletionsOptions) {
    if let Some(temperature) = options.temperature {
        params["temperature"] = json!(temperature);
    }
}

fn add_tools(params: &mut serde_json::Value, context: &Context) {
    if let Some(tools) = &context.tools {
        params["tools"] = convert_tools(tools);
    }
}

fn add_tool_choice(params: &mut serde_json::Value, options: &OpenAICompletionsOptions) {
    if let Some(tool_choice) = &options.tool_choice {
        params["tool_choice"] = json!(tool_choice);
    }
}

fn add_zai_optional_fields(params: &mut serde_json::Value, options: &OpenAICompletionsOptions) {
    let Some(zai_options) = options.zai.as_ref() else {
        return;
    };

    if let Some(do_sample) = zai_options.do_sample {
        params["do_sample"] = json!(do_sample);
    }

    if let Some(top_p) = zai_options.top_p {
        params["top_p"] = json!(top_p);
    }

    if let Some(stop) = &zai_options.stop {
        params["stop"] = json!(stop);
    }

    if let Some(tool_stream) = zai_options.tool_stream {
        params["tool_stream"] = json!(tool_stream);
    }

    if let Some(request_id) = &zai_options.request_id {
        params["request_id"] = json!(request_id);
    }

    if let Some(user_id) = &zai_options.user_id {
        params["user_id"] = json!(user_id);
    }

    if let Some(response_format) = &zai_options.response_format {
        params["response_format"] = json!(response_format);
    }

    if let Some(thinking) = &zai_options.thinking {
        params["thinking"] = json!(thinking);
    }
}

fn add_default_reasoning(
    params: &mut serde_json::Value,
    model: &Model<ZaiCompletions>,
    options: &OpenAICompletionsOptions,
) {
    let has_explicit_thinking = options
        .zai
        .as_ref()
        .and_then(|zai_options| zai_options.thinking.as_ref())
        .is_some();

    if model.reasoning && !has_explicit_thinking {
        params["thinking"] = json!({ "type": "enabled" });
    }
}

fn process_chunk(
    chunk: &StreamChunk,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
    current_block: &mut Option<CurrentBlock>,
) {
    if let Some(usage) = &chunk.usage {
        update_usage_from_chunk(usage, output);
    }

    let Some(choice) = chunk.choices.first() else {
        return;
    };

    if let Some(reason) = &choice.finish_reason {
        output.stop_reason = map_zai_stop_reason(reason);
    }

    let Some(delta) = &choice.delta else {
        return;
    };

    if let Some(content) = delta.content.as_deref() {
        handle_text_delta(content, output, sender, current_block);
    }

    if let Some(reasoning) = extract_reasoning(delta) {
        handle_reasoning_delta(reasoning, output, sender, current_block);
    }

    if let Some(tool_calls) = &delta.tool_calls {
        handle_tool_calls(tool_calls, output, sender, current_block);
    }
}

fn extract_reasoning(delta: &StreamDelta) -> Option<ReasoningDelta<'_>> {
    if let Some(text) = delta.reasoning_content.as_deref() {
        return Some(ReasoningDelta {
            text,
            signature: REASONING_CONTENT_SIGNATURE,
        });
    }

    if let Some(text) = delta.reasoning.as_deref() {
        return Some(ReasoningDelta {
            text,
            signature: REASONING_SIGNATURE,
        });
    }

    delta.reasoning_text.as_deref().map(|text| ReasoningDelta {
        text,
        signature: REASONING_TEXT_SIGNATURE,
    })
}

fn map_zai_stop_reason(reason: &str) -> StopReason {
    match reason {
        "sensitive" | "network_error" => StopReason::Error,
        _ => map_stop_reason(reason),
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
    reasoning_content: Option<String>,
    reasoning: Option<String>,
    reasoning_text: Option<String>,
    tool_calls: Option<Vec<OpenAiLikeToolCallDelta>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        AssistantMessageEvent, Content, InputType, KnownProvider, Message, ModelCost, Provider,
        Usage, UserContent, UserMessage, ZaiChatCompletionsOptions, ZaiResponseFormat,
        ZaiResponseFormatType, ZaiThinking, ZaiThinkingType,
    };
    use futures::executor::block_on;
    use futures::StreamExt;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    const TEST_BASE_URL: &str = "https://api.z.ai/api/paas/v4/chat/completions";

    fn make_model(reasoning: bool) -> Model<ZaiCompletions> {
        Model {
            id: "glm-4.7".to_string(),
            name: "GLM 4.7".to_string(),
            api: ZaiCompletions,
            provider: Provider::Known(KnownProvider::Zai),
            base_url: TEST_BASE_URL.to_string(),
            reasoning,
            input: vec![InputType::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 200_000,
            max_tokens: 128_000,
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

    fn make_output_message() -> AssistantMessage {
        AssistantMessage {
            content: vec![],
            api: Api::ZaiCompletions,
            provider: Provider::Known(KnownProvider::Zai),
            model: "glm-4.7".to_string(),
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

        for chunk in chunks {
            process_chunk(&chunk, &mut output, &mut sender, &mut current_block);
        }

        finish_current_block(&mut current_block, &mut output, &mut sender);
        drop(sender);

        let events = block_on(async move { stream.by_ref().collect::<Vec<_>>().await });
        (events, output)
    }

    #[test]
    fn build_params_uses_zai_message_format_and_max_tokens_precedence() {
        let model = make_model(true);
        let context = Context {
            system_prompt: None,
            messages: vec![Message::Assistant(AssistantMessage {
                content: vec![
                    Content::thinking("first reason"),
                    Content::text("final answer"),
                ],
                api: Api::ZaiCompletions,
                provider: Provider::Known(KnownProvider::Zai),
                model: model.id.clone(),
                usage: Usage::default(),
                stop_reason: StopReason::Stop,
                error_message: None,
                timestamp: 0,
            })],
            tools: None,
        };
        let options = OpenAICompletionsOptions {
            max_tokens: Some(256),
            zai: Some(ZaiChatCompletionsOptions {
                max_tokens: Some(1024),
                ..ZaiChatCompletionsOptions::default()
            }),
            ..OpenAICompletionsOptions::default()
        };

        let params = build_params(&model, &context, &options);

        assert_eq!(params["stream"], true);
        assert_eq!(params["stream_options"][STREAM_INCLUDE_USAGE_FIELD], true);
        assert_eq!(params["max_tokens"], 1024);
        assert!(params.get("max_completion_tokens").is_none());
        assert_eq!(params["messages"][0]["content"], "final answer");
        assert_eq!(params["messages"][0]["reasoning_content"], "first reason");
        assert_eq!(params["thinking"]["type"], "enabled");
    }

    #[test]
    fn build_params_serializes_optional_zai_fields_and_explicit_thinking() {
        let model = make_model(true);
        let context = make_context();
        let options = OpenAICompletionsOptions {
            zai: Some(ZaiChatCompletionsOptions {
                do_sample: Some(true),
                top_p: Some(0.75),
                max_tokens: Some(4096),
                stop: Some(["stop-here".to_string()]),
                tool_stream: Some(true),
                request_id: Some("request-1".to_string()),
                user_id: Some("user-2".to_string()),
                response_format: Some(ZaiResponseFormat {
                    kind: ZaiResponseFormatType::JsonSchema,
                    json_schema: Some(json!({"type": "object"})),
                }),
                thinking: Some(ZaiThinking {
                    kind: ZaiThinkingType::Disabled,
                    clear_thinking: Some(true),
                }),
            }),
            ..OpenAICompletionsOptions::default()
        };

        let params = build_params(&model, &context, &options);

        assert_eq!(params["do_sample"], true);
        assert_eq!(params["top_p"], 0.75);
        assert_eq!(params["stop"], json!(["stop-here"]));
        assert_eq!(params["tool_stream"], true);
        assert_eq!(params["request_id"], "request-1");
        assert_eq!(params["user_id"], "user-2");
        assert_eq!(params["response_format"]["type"], "json_schema");
        assert_eq!(params["thinking"]["type"], "disabled");
        assert_eq!(params["thinking"]["clear_thinking"], true);
    }

    #[test]
    fn map_zai_stop_reason_overrides_sensitive_and_network_error() {
        assert_eq!(map_zai_stop_reason("sensitive"), StopReason::Error);
        assert_eq!(map_zai_stop_reason("network_error"), StopReason::Error);
        assert_eq!(map_zai_stop_reason("tool_calls"), StopReason::ToolUse);
        assert_eq!(map_zai_stop_reason("stop"), StopReason::Stop);
    }

    #[test]
    fn process_chunk_maps_usage_and_reasoning() {
        let chunk: StreamChunk = serde_json::from_value(json!({
            "choices": [{
                "finish_reason": "stop",
                "delta": {
                    "reasoning_content": "step one",
                    "content": "answer"
                }
            }],
            "usage": {
                "prompt_tokens": 9,
                "completion_tokens": 4,
                "total_tokens": 13
            }
        }))
        .expect("valid chunk payload");

        let (_events, output) = process_chunks_for_test(vec![chunk]);

        assert_eq!(output.stop_reason, StopReason::Stop);
        assert_eq!(output.usage.total_tokens, 13);
        assert_eq!(output.content.len(), 2);

        match &output.content[0] {
            Content::Text { inner } => assert_eq!(inner.text, "answer"),
            _ => panic!("expected text content first"),
        }

        match &output.content[1] {
            Content::Thinking { inner } => {
                assert_eq!(inner.thinking, "step one");
                assert_eq!(
                    inner.thinking_signature.as_deref(),
                    Some(REASONING_CONTENT_SIGNATURE)
                );
            }
            _ => panic!("expected thinking content second"),
        }
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

        format!("http://{}/api/paas/v4/chat/completions", address)
    }

    #[tokio::test]
    async fn stream_zai_completions_final_message_shape() {
        let sse_body = concat!(
            "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"reason\"}}]}\n\n",
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

        let stream = stream_zai_completions(&model, &make_context(), options);
        let result = stream.result().await.expect("stream result");

        assert_eq!(result.api, Api::ZaiCompletions);
        assert_eq!(result.provider, Provider::Known(KnownProvider::Zai));
        assert_eq!(result.model, "glm-4.7");
        assert_eq!(result.stop_reason, StopReason::Stop);
        assert_eq!(result.usage.total_tokens, 15);
    }
}
