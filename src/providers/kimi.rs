use serde::Deserialize;
use serde_json::json;

use super::openai_completions::OpenAICompletionsOptions;
#[cfg(test)]
use super::shared::finish_current_block;
use super::shared::{
    apply_deferred_tool_calls, convert_messages, convert_tools, handle_reasoning_delta,
    handle_text_delta, initialize_output, map_stop_reason, prepare_openai_like_chunk,
    push_stream_error, run_openai_like_stream_without_state, AssistantThinkingMode, CurrentBlock,
    OpenAiLikeMessageOptions, OpenAiLikeRequest, OpenAiLikeStreamChunk, OpenAiLikeToolCallDelta,
    ReasoningDelta, SystemPromptRole,
};
use crate::cache::kimi_cache_capability::cache_capability_for;
use crate::cache::request::{prepare_cache_request, CacheRequestInput};
use crate::types::{
    AnthropicMessages, Api, AssistantMessage, AssistantMessageEventStream, Context,
    EventStreamSender, Model,
};

const STREAM_INCLUDE_USAGE_FIELD: &str = "include_usage";
const REASONING_CONTENT_SIGNATURE: &str = "reasoning_content";
const REASONING_SIGNATURE: &str = "reasoning";
const REASONING_TEXT_SIGNATURE: &str = "reasoning_text";

pub fn stream_kimi_messages(
    model: &Model<AnthropicMessages>,
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

    if let Err(error) = run_stream_inner(&model, &context, &options, &mut output, &mut sender).await
    {
        push_stream_error(&mut output, &mut sender, error);
    }
}

async fn run_stream_inner(
    model: &Model<AnthropicMessages>,
    context: &Context,
    options: &OpenAICompletionsOptions,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
) -> Result<(), crate::Error> {
    let request = build_request(model, context, options);

    run_openai_like_stream_without_state::<StreamChunk, _>(
        request,
        output,
        sender,
        |chunk, output, sender, current_block| {
            process_chunk(&chunk, output, sender, current_block);
        },
    )
    .await
}

fn build_request<'a>(
    model: &'a Model<AnthropicMessages>,
    context: &Context,
    options: &'a OpenAICompletionsOptions,
) -> OpenAiLikeRequest<'a> {
    let params = build_params(model, context, options);
    let cache_preparation = prepare_cache_request(
        cache_capability_for(&model.provider),
        CacheRequestInput {
            base_url: &model.base_url,
            model_id: &model.id,
            cache: options.cache.as_ref(),
        },
    )
    .expect("kimi runtime requires the kimi cache capability");

    OpenAiLikeRequest::new_with_cache(
        &model.provider,
        &model.base_url,
        &options.api_key,
        model.headers.as_ref(),
        options.headers.as_ref(),
        params,
        cache_preparation,
    )
}

fn build_params(
    model: &Model<AnthropicMessages>,
    context: &Context,
    options: &OpenAICompletionsOptions,
) -> serde_json::Value {
    let message_options = OpenAiLikeMessageOptions {
        assistant_content_as_string: true,
        emit_reasoning_content_field: true,
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
        "max_tokens": options.max_tokens.unwrap_or(model.max_tokens),
    });

    params["stream_options"] = json!({ STREAM_INCLUDE_USAGE_FIELD: true });

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
) {
    let Some(prelude) = prepare_openai_like_chunk(
        chunk,
        output,
        sender,
        current_block,
        map_stop_reason,
        delta_tool_calls,
    ) else {
        return;
    };

    let delta = prelude.delta;

    if let Some(content) = delta.content.as_deref() {
        handle_text_delta(content, output, sender, current_block);
    }

    if let Some(reasoning) = extract_reasoning(delta) {
        handle_reasoning_delta(reasoning, output, sender, current_block);
    }

    apply_deferred_tool_calls(prelude, output, sender, current_block);
}

fn delta_tool_calls(delta: &StreamDelta) -> Option<&[OpenAiLikeToolCallDelta]> {
    delta.tool_calls.as_deref()
}

fn extract_reasoning(delta: &StreamDelta) -> Option<ReasoningDelta<'_>> {
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

            return Some(ReasoningDelta {
                text: reasoning_text,
                signature,
            });
        }
    }

    None
}

type StreamChunk = OpenAiLikeStreamChunk<StreamDelta>;

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
    use crate::models::kimi::kimi_k2_5;
    use crate::test_helpers::{assert_final_message_shape, ExpectedFinalMessageShape};
    use crate::types::{
        Api, Content, KnownProvider, Message, Provider, StopReason, Tool, UserContent, UserMessage,
    };
    use futures::executor::block_on;
    use futures::StreamExt;
    use std::env;

    fn require_live_api_key() {
        assert!(
            env::var("KIMI_API_KEY").is_ok(),
            "KIMI_API_KEY must be set to run live Kimi tests"
        );
    }

    fn text_context(prompt: &str) -> Context {
        Context {
            system_prompt: Some("You are concise.".to_string()),
            messages: vec![Message::User(UserMessage {
                content: UserContent::Text(prompt.to_string()),
                timestamp: 0,
            })],
            tools: None,
        }
    }

    fn process_chunks_for_test(chunks: Vec<StreamChunk>) -> AssistantMessage {
        let model = kimi_k2_5();
        let (mut stream, mut sender) = AssistantMessageEventStream::new();
        let mut output = initialize_output(
            Api::AnthropicMessages,
            model.provider.clone(),
            model.id.clone(),
        );
        let mut current_block = None;

        for chunk in chunks {
            process_chunk(&chunk, &mut output, &mut sender, &mut current_block);
        }

        finish_current_block(&mut current_block, &mut output, &mut sender);
        drop(sender);

        let _events = block_on(async move { stream.by_ref().collect::<Vec<_>>().await });
        output
    }

    #[test]
    fn kimi_runtime_builds_chat_completions_request_with_cache_contract() {
        let model = kimi_k2_5();
        let options = OpenAICompletionsOptions {
            api_key: Some("test-key".to_string()),
            max_tokens: Some(16),
            cache: Some(crate::types::CacheOptions {
                key: "cache-key".to_string(),
            }),
            ..OpenAICompletionsOptions::default()
        };
        let request = build_request(
            &model,
            &text_context("Reply with exactly: kimi ok"),
            &options,
        );

        assert_eq!(
            request.base_url.as_ref(),
            "https://api.kimi.com/coding/v1/chat/completions"
        );
        assert_eq!(request.params.as_ref()["model"], json!("kimi-for-coding"));
        assert_eq!(
            request.params.as_ref()["prompt_cache_key"],
            json!("cache-key")
        );
        assert_eq!(request.params.as_ref()["max_tokens"], json!(16));
        assert_eq!(
            request
                .cache_headers
                .as_ref()
                .and_then(|headers| headers.get("User-Agent"))
                .map(String::as_str),
            Some("KimiCLI/1.29.0")
        );
        assert_eq!(model.id, "kimi-coding");
    }

    #[test]
    fn kimi_process_chunk_preserves_text_reasoning_and_tool_calls() {
        let chunks = vec![
            serde_json::from_value(json!({
                "choices": [{
                    "delta": {
                        "reasoning_content": "thinking"
                    }
                }]
            }))
            .expect("valid reasoning chunk"),
            serde_json::from_value(json!({
                "choices": [{
                    "delta": {
                        "content": "done"
                    }
                }]
            }))
            .expect("valid text chunk"),
            serde_json::from_value(json!({
                "choices": [{
                    "delta": {
                        "tool_calls": [
                            {
                                "id": "call_1",
                                "function": {
                                    "name": "get_weather",
                                    "arguments": "{\"city\":\"Austin\"}"
                                }
                            }
                        ]
                    },
                    "finish_reason": "tool_calls"
                }]
            }))
            .expect("valid tool call chunk"),
            serde_json::from_value(json!({
                "choices": [],
                "usage": {
                    "prompt_tokens": 10,
                    "completion_tokens": 5,
                    "total_tokens": 15
                }
            }))
            .expect("valid usage chunk"),
        ];

        let output = process_chunks_for_test(chunks);

        assert_final_message_shape(
            &output,
            ExpectedFinalMessageShape {
                provider: Provider::Known(KnownProvider::Kimi),
                api: Api::AnthropicMessages,
                model: "kimi-coding",
                stop_reason: StopReason::ToolUse,
                total_tokens: 15,
            },
        );
        assert!(matches!(
            output.content.first(),
            Some(Content::Thinking { .. })
        ));
        assert!(matches!(output.content.get(1), Some(Content::Text { .. })));
        assert!(matches!(
            output.content.get(2),
            Some(Content::ToolCall { .. })
        ));
    }

    #[tokio::test]
    #[ignore = "live API test"]
    async fn live_kimi_complete_returns_typed_text_message() {
        require_live_api_key();

        let model = kimi_k2_5();
        let result = crate::complete(
            &model,
            &text_context("Reply with exactly: hello from kimi"),
            Some(OpenAICompletionsOptions {
                max_tokens: Some(64),
                ..Default::default()
            }),
        )
        .await
        .expect("kimi complete should succeed");

        assert_eq!(result.api, Api::AnthropicMessages);
        assert_eq!(result.provider, Provider::Known(KnownProvider::Kimi));
        assert_eq!(result.model, "kimi-coding");
        assert!(matches!(
            result.stop_reason,
            StopReason::Stop | StopReason::Length
        ));
        assert!(matches!(result.content.first(), Some(Content::Text { .. })));
    }

    #[tokio::test]
    #[ignore = "live API test"]
    async fn live_kimi_complete_accepts_reasoning_enabled_requests() {
        require_live_api_key();

        let model = kimi_k2_5();
        let result = crate::complete(
            &model,
            &text_context(
                "Think step by step, but keep the final answer to one short sentence: what is 27 times 14?",
            ),
            Some(OpenAICompletionsOptions {
                max_tokens: Some(256),
                ..Default::default()
            }),
        )
        .await
        .expect("kimi reasoning request should succeed");

        assert_eq!(result.api, Api::AnthropicMessages);
        assert_eq!(result.provider, Provider::Known(KnownProvider::Kimi));
        assert!(matches!(
            result.stop_reason,
            StopReason::Stop | StopReason::Length
        ));
        assert!(!result.content.is_empty());

        if let Some(Content::Thinking { inner }) = result
            .content
            .iter()
            .find(|content| matches!(content, Content::Thinking { .. }))
        {
            assert!(!inner.thinking.is_empty());
        }
    }

    #[tokio::test]
    #[ignore = "live API test"]
    async fn live_kimi_complete_returns_typed_tool_call() {
        require_live_api_key();

        let model = kimi_k2_5();
        let context = Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: UserContent::Text(
                    "Use the get_weather tool for Austin, TX. Do not answer directly.".to_string(),
                ),
                timestamp: 0,
            })],
            tools: Some(vec![Tool::new(
                "get_weather",
                "Get the weather for a city",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "city": {"type": "string"}
                    },
                    "required": ["city"]
                }),
            )]),
        };

        let result = crate::complete(
            &model,
            &context,
            Some(OpenAICompletionsOptions {
                max_tokens: Some(256),
                ..Default::default()
            }),
        )
        .await
        .expect("kimi tool call should succeed");

        assert_eq!(result.api, Api::AnthropicMessages);
        assert_eq!(result.provider, Provider::Known(KnownProvider::Kimi));
        assert_eq!(result.stop_reason, StopReason::ToolUse);
        assert!(matches!(
            result.content.first(),
            Some(Content::ToolCall { inner })
                if inner.name == "get_weather"
                    && inner.arguments["city"] == serde_json::json!("Austin, TX")
        ));
    }
}
