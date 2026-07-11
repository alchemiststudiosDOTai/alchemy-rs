use serde_json::json;

use super::openai_completions::OpenAICompletionsOptions;
use super::shared::{
    apply_deferred_tool_calls, convert_messages, convert_tools, extract_reasoning,
    handle_reasoning_delta, handle_text_delta, map_stop_reason, prepare_openai_like_chunk,
    run_openai_like_stream, spawn_openai_like_stream, AssistantThinkingMode, CurrentBlock,
    OpenAiLikeMessageOptions, OpenAiLikeRequest, OpenAiLikeStreamChunk, SystemPromptRole,
};
#[cfg(test)]
use super::shared::{finish_current_block, initialize_output, REASONING_CONTENT_SIGNATURE};
use crate::types::{
    AssistantMessage, AssistantMessageEventStream, Context, EventStreamSender, Model, StopReason,
    ZaiChatCompletionsOptions, ZaiCompletions,
};

/// Stream completions from z.ai chat/completions API.
pub fn stream_zai_completions(
    model: &Model<ZaiCompletions>,
    context: &Context,
    options: OpenAICompletionsOptions,
) -> AssistantMessageEventStream {
    spawn_openai_like_stream(
        model,
        context,
        options,
        |model, context, options, output, sender| {
            Box::pin(run_stream(model, context, options, output, sender))
        },
    )
}

async fn run_stream(
    model: &Model<ZaiCompletions>,
    context: &Context,
    options: &OpenAICompletionsOptions,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
) -> Result<(), crate::Error> {
    let params = build_params(model, context, options);

    run_openai_like_stream(
        OpenAiLikeRequest::new(model, options, &params),
        output,
        sender,
        |chunk, output, sender, current_block| {
            if let Some(chunk) = chunk {
                process_chunk(&chunk, output, sender, current_block);
            }
        },
    )
    .await
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
        "stream_options": { "include_usage": true },
        "messages": convert_messages(model, context, &message_options),
    });

    let zai_options = options.zai.as_ref();

    if let Some(max_tokens) = zai_options
        .and_then(|zai_options| zai_options.max_tokens)
        .or(options.max_tokens)
    {
        params["max_tokens"] = json!(max_tokens);
    }

    if let Some(temperature) = options.temperature {
        params["temperature"] = json!(temperature);
    }

    if let Some(tools) = &context.tools {
        params["tools"] = convert_tools(tools);
    }

    if let Some(tool_choice) = &options.tool_choice {
        params["tool_choice"] = json!(tool_choice);
    }

    if let Some(zai_options) = zai_options {
        add_zai_optional_fields(&mut params, zai_options);
    }

    let has_explicit_thinking =
        zai_options.is_some_and(|zai_options| zai_options.thinking.is_some());
    if model.reasoning && !has_explicit_thinking {
        params["thinking"] = json!({ "type": "enabled" });
    }

    params
}

fn add_zai_optional_fields(
    params: &mut serde_json::Value,
    zai_options: &ZaiChatCompletionsOptions,
) {
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

fn process_chunk(
    chunk: &OpenAiLikeStreamChunk,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
    current_block: &mut Option<CurrentBlock>,
) {
    let Some(prelude) =
        prepare_openai_like_chunk(chunk, output, sender, current_block, map_zai_stop_reason)
    else {
        return;
    };

    if let Some(content) = prelude.delta.content.as_deref() {
        handle_text_delta(content, output, sender, current_block);
    }

    if let Some(reasoning) = extract_reasoning(prelude.delta) {
        handle_reasoning_delta(reasoning, output, sender, current_block);
    }

    apply_deferred_tool_calls(prelude, output, sender, current_block);
}

fn map_zai_stop_reason(reason: &str) -> StopReason {
    match reason {
        "sensitive" | "network_error" => StopReason::Error,
        _ => map_stop_reason(reason),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{
        assert_final_message_shape, assert_multiply_tool_call, build_final_message_shape_chunks,
        concise_context, make_test_model, populated_zai_chat_completions_options,
        populated_zai_chat_completions_options_json, tool_call_continuation_chunk,
        tool_call_start_chunk, ExpectedFinalMessageShape,
    };
    use crate::types::{
        Api, AssistantMessageEvent, Content, KnownProvider, Message, Provider, Usage,
    };
    use futures::executor::block_on;
    use futures::StreamExt;

    const TEST_BASE_URL: &str = "https://api.z.ai/api/paas/v4/chat/completions";

    fn make_model(reasoning: bool) -> Model<ZaiCompletions> {
        make_test_model(
            ZaiCompletions,
            KnownProvider::Zai,
            "glm-4.7",
            TEST_BASE_URL,
            reasoning,
        )
    }

    fn process_chunks_for_test(
        chunks: Vec<OpenAiLikeStreamChunk>,
    ) -> (Vec<AssistantMessageEvent>, AssistantMessage) {
        let (mut stream, mut sender) = AssistantMessageEventStream::new();
        let mut output = initialize_output(
            Api::ZaiCompletions,
            Provider::Known(KnownProvider::Zai),
            "glm-4.7".to_string(),
        );
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
        assert_eq!(params["stream_options"]["include_usage"], true);
        assert_eq!(params["max_tokens"], 1024);
        assert!(params.get("max_completion_tokens").is_none());
        assert_eq!(params["messages"][0]["content"], "final answer");
        assert_eq!(params["messages"][0]["reasoning_content"], "first reason");
        assert_eq!(params["thinking"]["type"], "enabled");
    }

    #[test]
    fn build_params_serializes_optional_zai_fields_and_explicit_thinking() {
        let model = make_model(true);
        let context = concise_context();
        let expected = populated_zai_chat_completions_options_json();
        let options = OpenAICompletionsOptions {
            zai: Some(populated_zai_chat_completions_options()),
            ..OpenAICompletionsOptions::default()
        };

        let params = build_params(&model, &context, &options);

        assert_eq!(params["do_sample"], expected["do_sample"]);
        assert_eq!(params["top_p"], expected["top_p"]);
        assert_eq!(params["stop"], expected["stop"]);
        assert_eq!(params["tool_stream"], expected["tool_stream"]);
        assert_eq!(params["request_id"], expected["request_id"]);
        assert_eq!(params["user_id"], expected["user_id"]);
        assert_eq!(params["response_format"], expected["response_format"]);
        assert_eq!(params["thinking"], expected["thinking"]);
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
        let chunk: OpenAiLikeStreamChunk = serde_json::from_value(json!({
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

    #[test]
    fn process_chunk_prioritizes_tool_call_continuations_before_content() {
        let (_events, output) = process_chunks_for_test(vec![
            tool_call_start_chunk("call_function_1"),
            tool_call_continuation_chunk(json!({ "content": "tail" })),
        ]);

        assert_eq!(output.stop_reason, StopReason::ToolUse);
        assert_eq!(output.content.len(), 2);
        assert_multiply_tool_call(&output.content[0], "call_function_1");

        match &output.content[1] {
            Content::Text { inner } => assert_eq!(inner.text, "tail"),
            _ => panic!("expected text content"),
        }
    }

    #[test]
    fn process_chunk_prioritizes_tool_call_continuations_before_reasoning() {
        let (_events, output) = process_chunks_for_test(vec![
            tool_call_start_chunk("call_function_2"),
            tool_call_continuation_chunk(json!({ "reasoning_content": "next step" })),
        ]);

        assert_eq!(output.stop_reason, StopReason::ToolUse);
        assert_eq!(output.content.len(), 2);
        assert_multiply_tool_call(&output.content[0], "call_function_2");

        match &output.content[1] {
            Content::Thinking { inner } => {
                assert_eq!(inner.thinking, "next step");
                assert_eq!(
                    inner.thinking_signature.as_deref(),
                    Some(REASONING_CONTENT_SIGNATURE)
                );
            }
            _ => panic!("expected thinking content"),
        }
    }

    #[test]
    fn stream_zai_completions_final_message_shape() {
        let chunks = build_final_message_shape_chunks(json!({
            "reasoning_content": "reason"
        }));
        let (_events, output) = process_chunks_for_test(chunks);

        assert_final_message_shape(
            &output,
            ExpectedFinalMessageShape {
                api: Api::ZaiCompletions,
                provider: Provider::Known(KnownProvider::Zai),
                model: "glm-4.7",
                stop_reason: StopReason::Stop,
                total_tokens: 15,
            },
        );
    }
}
