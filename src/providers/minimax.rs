use serde_json::json;

use super::openai_completions::OpenAICompletionsOptions;
use super::shared::{
    apply_deferred_tool_calls, convert_messages, convert_tools, extract_reasoning,
    handle_reasoning_delta, handle_text_delta, map_stop_reason, prepare_openai_like_chunk,
    run_openai_like_stream, spawn_openai_like_stream, AssistantThinkingMode, CurrentBlock,
    OpenAiLikeMessageOptions, OpenAiLikeRequest, OpenAiLikeStreamChunk, OpenAiLikeStreamDelta,
    ReasoningDelta, SystemPromptRole,
};
#[cfg(test)]
use super::shared::{finish_current_block, initialize_output};
use crate::types::{
    AssistantMessage, AssistantMessageEventStream, Context, EventStreamSender, MinimaxCompletions,
    Model,
};
use crate::utils::{ThinkFragment, ThinkTagParser};

const REASONING_DETAILS_SIGNATURE: &str = "reasoning_details";
const THINK_TAG_SIGNATURE: &str = "think_tag";
const MINIMAX_MIN_TEMPERATURE: f64 = f64::MIN_POSITIVE;
const MINIMAX_MAX_TEMPERATURE: f64 = 1.0;

/// Stream completions from MiniMax chat/completions API.
pub fn stream_minimax_completions(
    model: &Model<MinimaxCompletions>,
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
    model: &Model<MinimaxCompletions>,
    context: &Context,
    options: &OpenAICompletionsOptions,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
) -> Result<(), crate::Error> {
    let params = build_params(model, context, options);
    let mut think_tag_parser = ThinkTagParser::new();

    run_openai_like_stream(
        OpenAiLikeRequest::new(model, options, &params),
        output,
        sender,
        |chunk, output, sender, current_block| match chunk {
            Some(chunk) => {
                process_chunk(&chunk, output, sender, current_block, &mut think_tag_parser);
            }
            None => {
                emit_think_fragments(think_tag_parser.flush(), output, sender, current_block);
            }
        },
    )
    .await
}

fn build_params(
    model: &Model<MinimaxCompletions>,
    context: &Context,
    options: &OpenAICompletionsOptions,
) -> serde_json::Value {
    let message_options = OpenAiLikeMessageOptions::openai_like(
        SystemPromptRole::System,
        false,
        AssistantThinkingMode::ThinkTags,
    );

    let mut params = json!({
        "model": model.id,
        "stream": true,
        "stream_options": { "include_usage": true },
        "messages": convert_messages(model, context, &message_options),
    });

    if model.reasoning {
        params["reasoning_split"] = json!(true);
    }

    if let Some(max_tokens) = options.max_tokens {
        params["max_tokens"] = json!(max_tokens);
    }

    if let Some(temperature) = options.temperature {
        params["temperature"] =
            json!(temperature.clamp(MINIMAX_MIN_TEMPERATURE, MINIMAX_MAX_TEMPERATURE));
    }

    if let Some(tools) = &context.tools {
        params["tools"] = convert_tools(tools);
    }

    if let Some(tool_choice) = &options.tool_choice {
        params["tool_choice"] = json!(tool_choice);
    }

    params
}

fn process_chunk(
    chunk: &OpenAiLikeStreamChunk,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
    current_block: &mut Option<CurrentBlock>,
    think_tag_parser: &mut ThinkTagParser,
) {
    let Some(prelude) =
        prepare_openai_like_chunk(chunk, output, sender, current_block, map_stop_reason)
    else {
        return;
    };

    let delta = prelude.delta;
    let explicit_reasoning = emit_explicit_reasoning(delta, output, sender, current_block);

    if let Some(content) = delta.content.as_deref() {
        if explicit_reasoning {
            handle_text_delta(content, output, sender, current_block);
        } else {
            // No explicit reasoning fields: reasoning may arrive inline as
            // <think> tags in the content stream.
            emit_think_fragments(
                think_tag_parser.feed(content),
                output,
                sender,
                current_block,
            );
        }
    }

    apply_deferred_tool_calls(prelude, output, sender, current_block);
}

fn emit_explicit_reasoning(
    delta: &OpenAiLikeStreamDelta,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
    current_block: &mut Option<CurrentBlock>,
) -> bool {
    let mut emitted_reasoning = false;

    for detail in delta.reasoning_details.iter().flatten() {
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

    if emitted_reasoning {
        return true;
    }

    if let Some(reasoning) = extract_reasoning(delta) {
        handle_reasoning_delta(reasoning, output, sender, current_block);
        return true;
    }

    false
}

fn emit_think_fragments(
    fragments: Vec<ThinkFragment>,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
    current_block: &mut Option<CurrentBlock>,
) {
    for fragment in fragments {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{
        assert_final_message_shape, assert_multiply_tool_call, build_final_message_shape_chunks,
        concise_context, make_test_model, tool_call_continuation_chunk, tool_call_start_chunk,
        ExpectedFinalMessageShape,
    };
    use crate::types::{
        Api, AssistantMessageEvent, Content, KnownProvider, Message, Provider, StopReason, Usage,
    };
    use futures::executor::block_on;
    use futures::StreamExt;

    const TEST_BASE_URL: &str = "https://api.minimax.io/v1/chat/completions";

    fn make_model(reasoning: bool) -> Model<MinimaxCompletions> {
        make_test_model(
            MinimaxCompletions,
            KnownProvider::Minimax,
            "MiniMax-M2.5",
            TEST_BASE_URL,
            reasoning,
        )
    }

    fn process_chunks_for_test(
        chunks: Vec<OpenAiLikeStreamChunk>,
    ) -> (Vec<AssistantMessageEvent>, AssistantMessage) {
        let (mut stream, mut sender) = AssistantMessageEventStream::new();
        let mut output = initialize_output(
            Api::MinimaxCompletions,
            Provider::Known(KnownProvider::Minimax),
            "MiniMax-M2.5".to_string(),
        );
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

        emit_think_fragments(parser.flush(), &mut output, &mut sender, &mut current_block);
        finish_current_block(&mut current_block, &mut output, &mut sender);

        drop(sender);

        let events = block_on(async move { stream.by_ref().collect::<Vec<_>>().await });
        (events, output)
    }

    fn run_interleaved_tool_call_case(
        chunks: Vec<OpenAiLikeStreamChunk>,
        expected_call_id: &str,
    ) -> AssistantMessage {
        let (_events, output) = process_chunks_for_test(chunks);

        assert_eq!(output.stop_reason, StopReason::ToolUse);
        assert_eq!(output.content.len(), 2);
        assert_multiply_tool_call(&output.content[0], expected_call_id);

        output
    }

    fn assert_thinking_event_sequence(events: &[AssistantMessageEvent]) {
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
    }

    fn assert_text_event_sequence(events: &[AssistantMessageEvent], start_index: usize) {
        assert!(matches!(
            events[start_index],
            AssistantMessageEvent::TextStart { .. }
        ));
        assert!(matches!(
            events[start_index + 1],
            AssistantMessageEvent::TextDelta { .. }
        ));
        assert!(matches!(
            events[start_index + 2],
            AssistantMessageEvent::TextEnd { .. }
        ));
    }

    fn assert_thinking_content(content: &Content, expected_text: &str, expected_signature: &str) {
        match content {
            Content::Thinking { inner } => {
                assert_eq!(inner.thinking, expected_text);
                assert_eq!(
                    inner.thinking_signature.as_deref(),
                    Some(expected_signature)
                );
            }
            _ => panic!("expected thinking content"),
        }
    }

    fn assert_text_content(content: &Content, expected_text: &str) {
        match content {
            Content::Text { inner } => {
                assert_eq!(inner.text, expected_text);
            }
            _ => panic!("expected text content"),
        }
    }

    #[test]
    fn build_params_for_reasoning_model_uses_minimax_semantics() {
        let model = make_model(true);
        let context = concise_context();
        let options = OpenAICompletionsOptions {
            temperature: Some(1.2),
            max_tokens: Some(512),
            ..OpenAICompletionsOptions::default()
        };

        let params = build_params(&model, &context, &options);

        assert_eq!(params["stream"], true);
        assert_eq!(params["stream_options"]["include_usage"], true);
        assert_eq!(params["reasoning_split"], true);
        assert_eq!(params["max_tokens"], 512);
        assert_eq!(params["temperature"], MINIMAX_MAX_TEMPERATURE);
        assert_eq!(params["messages"][0]["role"], "system");
        assert_eq!(params["messages"][0]["content"], "You are concise");
        assert!(params.get("n").is_none());
        assert!(params.get("max_completion_tokens").is_none());
    }

    #[test]
    fn build_params_for_non_reasoning_model_omits_reasoning_split() {
        let model = make_model(false);
        let context = concise_context();
        let options = OpenAICompletionsOptions::default();

        let params = build_params(&model, &context, &options);

        assert!(params.get("reasoning_split").is_none());
    }

    #[test]
    fn build_params_clamps_temperature_to_positive_lower_bound() {
        let model = make_model(true);
        let context = concise_context();
        let options = OpenAICompletionsOptions {
            temperature: Some(0.0),
            ..OpenAICompletionsOptions::default()
        };

        let params = build_params(&model, &context, &options);

        assert_eq!(params["temperature"], MINIMAX_MIN_TEMPERATURE);
    }

    #[test]
    fn build_params_wraps_assistant_thinking_with_think_tags_for_replay() {
        let model = make_model(true);
        let assistant = AssistantMessage {
            content: vec![Content::thinking("step"), Content::text("answer")],
            api: Api::MinimaxCompletions,
            provider: Provider::Known(KnownProvider::Minimax),
            model: model.id.clone(),
            usage: Usage::default(),
            stop_reason: StopReason::Stop,
            error_message: None,
            timestamp: 0,
        };
        let context = Context {
            system_prompt: None,
            messages: vec![Message::Assistant(assistant)],
            tools: None,
        };

        let params = build_params(&model, &context, &OpenAICompletionsOptions::default());

        assert_eq!(params["messages"][0]["role"], "assistant");
        assert_eq!(
            params["messages"][0]["content"][0]["text"],
            "<think>step</think>"
        );
        assert_eq!(params["messages"][0]["content"][1]["text"], "answer");
    }

    #[test]
    fn process_chunk_maps_reasoning_details_to_thinking_events() {
        let chunk: OpenAiLikeStreamChunk = serde_json::from_value(serde_json::json!({
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

        assert_thinking_event_sequence(&events);
        assert_thinking_content(&output.content[0], "step one", REASONING_DETAILS_SIGNATURE);
    }

    #[test]
    fn process_chunk_skips_empty_reasoning_fields_before_think_tag_fallback() {
        let chunk: OpenAiLikeStreamChunk = serde_json::from_value(serde_json::json!({
            "choices": [{
                "delta": {
                    "reasoning_content": "",
                    "reasoning": "step one",
                    "content": "answer"
                }
            }]
        }))
        .expect("valid mixed reasoning payload");

        let (_events, output) = process_chunks_for_test(vec![chunk]);

        assert_eq!(output.content.len(), 2);
        assert_thinking_content(&output.content[0], "step one", "reasoning");
        assert_text_content(&output.content[1], "answer");
    }

    #[test]
    fn process_chunk_with_inline_think_tags_emits_expected_event_order() {
        let chunk: OpenAiLikeStreamChunk = serde_json::from_value(serde_json::json!({
            "choices": [{
                "delta": {
                    "content": "<think>reason</think>answer"
                }
            }]
        }))
        .expect("valid think-tag payload");

        let (events, output) = process_chunks_for_test(vec![chunk]);

        assert_thinking_event_sequence(&events);
        assert_text_event_sequence(&events, 3);

        assert_eq!(output.content.len(), 2);
        assert_thinking_content(&output.content[0], "reason", THINK_TAG_SIGNATURE);
        assert_text_content(&output.content[1], "answer");
    }

    #[test]
    fn process_chunk_prioritizes_tool_call_continuations_before_text_fallback() {
        let output = run_interleaved_tool_call_case(
            vec![
                tool_call_start_chunk("call_function_1"),
                tool_call_continuation_chunk(serde_json::json!({ "content": "tail" })),
            ],
            "call_function_1",
        );

        assert_text_content(&output.content[1], "tail");
    }

    #[test]
    fn process_chunk_prioritizes_tool_call_continuations_before_reasoning_details() {
        let output = run_interleaved_tool_call_case(
            vec![
                tool_call_start_chunk("call_function_2"),
                tool_call_continuation_chunk(serde_json::json!({
                    "reasoning_details": [{
                        "type": "reasoning",
                        "id": "r-1",
                        "format": "text",
                        "index": 0,
                        "text": "next step"
                    }]
                })),
            ],
            "call_function_2",
        );

        assert_thinking_content(&output.content[1], "next step", REASONING_DETAILS_SIGNATURE);
    }

    #[test]
    fn usage_only_chunk_updates_output_usage() {
        let chunk: OpenAiLikeStreamChunk = serde_json::from_value(serde_json::json!({
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

    #[test]
    fn stream_minimax_completions_final_message_shape() {
        let chunks = build_final_message_shape_chunks(serde_json::json!({
            "reasoning_details": [{"text": "reason"}]
        }));
        let (_events, output) = process_chunks_for_test(chunks);

        assert_final_message_shape(
            &output,
            ExpectedFinalMessageShape {
                api: Api::MinimaxCompletions,
                provider: Provider::Known(KnownProvider::Minimax),
                model: "MiniMax-M2.5",
                stop_reason: StopReason::Stop,
                total_tokens: 15,
            },
        );
    }
}
