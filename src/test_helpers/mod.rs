use crate::providers::OpenAICompletionsOptions;
use crate::types::{
    Api, ApiType, AssistantMessageEventStream, Context, Model, Provider, StopReason,
    ZaiChatCompletionsOptions, ZaiResponseFormat, ZaiResponseFormatType, ZaiThinking,
    ZaiThinkingType,
};
use serde_json::json;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

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

pub(crate) fn build_final_message_shape_sse_body(reasoning_delta: serde_json::Value) -> String {
    let reasoning_chunk = json!({
        "choices": [{
            "delta": reasoning_delta
        }]
    });
    let answer_chunk = json!({
        "choices": [{
            "delta": {"content": "answer"},
            "finish_reason": "stop"
        }]
    });
    let usage_chunk = json!({
        "choices": [],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 5,
            "total_tokens": 15
        }
    });

    format!(
        "data: {reasoning_chunk}\n\ndata: {answer_chunk}\n\ndata: {usage_chunk}\n\ndata: [DONE]\n\n"
    )
}

pub(crate) async fn spawn_sse_server(body: String, path: &str) -> String {
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

    format!("http://{address}{path}")
}

pub(crate) async fn assert_streaming_final_message_shape<TApi, FStream>(
    mut model: Model<TApi>,
    context: Context,
    options: OpenAICompletionsOptions,
    sse_body: String,
    path: &str,
    stream_fn: FStream,
    expected: ExpectedFinalMessageShape<'_>,
) where
    TApi: ApiType,
    FStream: Fn(&Model<TApi>, &Context, OpenAICompletionsOptions) -> AssistantMessageEventStream,
{
    let server_url = spawn_sse_server(sse_body, path).await;
    model.base_url = server_url;

    let stream = stream_fn(&model, &context, options);
    let result = stream.result().await.expect("stream result");

    assert_eq!(result.api, expected.api);
    assert_eq!(result.provider, expected.provider);
    assert_eq!(result.model, expected.model);
    assert_eq!(result.stop_reason, expected.stop_reason);
    assert_eq!(result.usage.total_tokens, expected.total_tokens);
}
