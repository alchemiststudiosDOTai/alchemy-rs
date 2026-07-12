use crate::providers::shared::OpenAiLikeStreamChunk;
use crate::types::{
    Api, ApiType, AssistantMessage, Content, Context, InputType, KnownProvider, Message, Model,
    ModelCost, Provider, StopReason, UserContent, UserMessage, ZaiChatCompletionsOptions,
    ZaiResponseFormat, ZaiResponseFormatType, ZaiThinking, ZaiThinkingType,
};
use serde_json::json;

pub(crate) struct ExpectedFinalMessageShape<'a> {
    pub api: Api,
    pub provider: Provider,
    pub model: &'a str,
    pub stop_reason: StopReason,
    pub total_tokens: u32,
}

pub(crate) fn populated_zai_chat_completions_options() -> ZaiChatCompletionsOptions {
    ZaiChatCompletionsOptions {
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
    }
}

pub(crate) fn populated_zai_chat_completions_options_json() -> serde_json::Value {
    json!({
        "do_sample": true,
        "top_p": 0.75,
        "max_tokens": 4096,
        "stop": ["stop-here"],
        "tool_stream": true,
        "request_id": "request-1",
        "user_id": "user-2",
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "type": "object"
            }
        },
        "thinking": {
            "type": "disabled",
            "clear_thinking": true
        }
    })
}

/// A model with sensible defaults for exercising provider streaming code.
pub(crate) fn make_test_model<TApi: ApiType>(
    api: TApi,
    provider: KnownProvider,
    id: &str,
    base_url: &str,
    reasoning: bool,
) -> Model<TApi> {
    Model {
        id: id.to_string(),
        name: id.to_string(),
        api,
        provider: Provider::Known(provider),
        base_url: base_url.to_string(),
        reasoning,
        input: vec![InputType::Text],
        cost: ModelCost {
            input: 0.0,
            output: 0.0,
            cache_read: 0.0,
            cache_write: 0.0,
        },
        context_window: 128_000,
        max_tokens: 4_096,
        headers: None,
        compat: None,
    }
}

pub(crate) fn concise_context() -> Context {
    Context {
        system_prompt: Some("You are concise".to_string()),
        messages: vec![Message::User(UserMessage {
            content: UserContent::Text("Hello".to_string()),
            timestamp: 0,
        })],
        tools: None,
    }
}

/// First chunk of a streamed `multiply(a: 15, b: 3)` tool call.
pub(crate) fn tool_call_start_chunk(call_id: &str) -> OpenAiLikeStreamChunk {
    serde_json::from_value(json!({
        "choices": [{
            "delta": {
                "tool_calls": [{
                    "id": call_id,
                    "type": "function",
                    "function": {
                        "name": "multiply",
                        "arguments": "{\"a\": 15, \"b\": "
                    },
                    "index": 0
                }]
            }
        }]
    }))
    .expect("valid first tool-call chunk")
}

/// Final chunk of the `multiply` tool call started by [`tool_call_start_chunk`],
/// with `extra_delta_fields` (e.g. `content` or `reasoning_content`) merged
/// into the delta to simulate interleaved output.
pub(crate) fn tool_call_continuation_chunk(
    extra_delta_fields: serde_json::Value,
) -> OpenAiLikeStreamChunk {
    let mut delta = json!({
        "tool_calls": [{
            "type": "function",
            "function": {
                "arguments": "3}"
            },
            "index": 0
        }]
    });

    for (key, value) in extra_delta_fields
        .as_object()
        .expect("extra delta fields must be a JSON object")
    {
        delta[key] = value.clone();
    }

    serde_json::from_value(json!({
        "choices": [{
            "finish_reason": "tool_calls",
            "delta": delta
        }]
    }))
    .expect("valid tool-call continuation chunk")
}

pub(crate) fn assert_multiply_tool_call(content: &Content, expected_call_id: &str) {
    match content {
        Content::ToolCall { inner } => {
            assert_eq!(inner.id.as_str(), expected_call_id);
            assert_eq!(inner.name, "multiply");
            assert_eq!(inner.arguments, json!({"a": 15, "b": 3}));
        }
        _ => panic!("expected tool call content"),
    }
}

pub(crate) fn build_final_message_shape_chunks(
    reasoning_delta: serde_json::Value,
) -> Vec<OpenAiLikeStreamChunk> {
    vec![
        serde_json::from_value(json!({
            "choices": [{
                "delta": reasoning_delta
            }]
        }))
        .expect("valid reasoning chunk"),
        serde_json::from_value(json!({
            "choices": [{
                "delta": {"content": "answer"},
                "finish_reason": "stop"
            }]
        }))
        .expect("valid answer chunk"),
        serde_json::from_value(json!({
            "choices": [],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 5,
                "total_tokens": 15
            }
        }))
        .expect("valid usage chunk"),
    ]
}

pub(crate) fn assert_final_message_shape(
    result: &AssistantMessage,
    expected: ExpectedFinalMessageShape<'_>,
) {
    assert_eq!(result.api, expected.api);
    assert_eq!(result.provider, expected.provider);
    assert_eq!(result.model, expected.model);
    assert_eq!(result.stop_reason, expected.stop_reason);
    assert_eq!(result.usage.total_tokens, expected.total_tokens);
}
