//! End-to-end test of the OpenAI-like streaming pipeline over a real socket:
//! request building, SSE parsing, chunk processing, and stream finalization.

use alchemy_llm::types::{
    AssistantMessageEvent, Content, InputType, KnownProvider, Model, ModelCost, OpenAICompletions,
    Provider, StopReason, UserContent, UserMessage,
};
use alchemy_llm::{stream_openai_completions, OpenAICompletionsOptions};
use futures::StreamExt;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

const SSE_BODY: &str = "data: {\"choices\":[{\"delta\":{\"reasoning_content\":\"thinking hard\"}}]}\n\n\
data: {\"choices\":[{\"delta\":{\"content\":\"hello \"}}]}\n\n\
data: {\"choices\":[{\"delta\":{\"content\":\"world\"},\"finish_reason\":\"stop\"}]}\n\n\
data: {\"choices\":[],\"usage\":{\"prompt_tokens\":7,\"completion_tokens\":3,\"total_tokens\":10}}\n\n\
data: [DONE]\n\n";

async fn spawn_sse_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("local addr");

    tokio::spawn(async move {
        let (mut socket, _) = listener.accept().await.expect("accept");

        // Drain the request before responding.
        let mut buffer = [0u8; 4096];
        let _ = socket.read(&mut buffer).await;

        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            SSE_BODY.len(),
            SSE_BODY
        );
        socket.write_all(response.as_bytes()).await.expect("write");
        socket.shutdown().await.ok();
    });

    format!("http://{address}/v1/chat/completions")
}

fn make_model(base_url: String) -> Model<OpenAICompletions> {
    Model {
        id: "test-model".to_string(),
        name: "Test Model".to_string(),
        api: OpenAICompletions,
        provider: Provider::Known(KnownProvider::OpenAI),
        base_url,
        reasoning: false,
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

fn make_context() -> alchemy_llm::types::Context {
    alchemy_llm::types::Context {
        system_prompt: None,
        messages: vec![alchemy_llm::types::Message::User(UserMessage {
            content: UserContent::Text("Hi".to_string()),
            timestamp: 0,
        })],
        tools: None,
    }
}

#[tokio::test]
async fn streams_reasoning_text_and_usage_end_to_end() {
    let base_url = spawn_sse_server().await;
    let model = make_model(base_url);

    let stream = stream_openai_completions(
        &model,
        &make_context(),
        OpenAICompletionsOptions {
            api_key: Some("test-key".to_string()),
            ..OpenAICompletionsOptions::default()
        },
    );

    let events: Vec<AssistantMessageEvent> = stream.collect().await;

    assert!(matches!(
        events.first(),
        Some(AssistantMessageEvent::Start { .. })
    ));

    let message = match events.last() {
        Some(AssistantMessageEvent::Done { message, .. }) => message,
        other => panic!("expected Done event, got {other:?}"),
    };

    assert_eq!(message.stop_reason, StopReason::Stop);
    assert_eq!(message.usage.input, 7);
    assert_eq!(message.usage.output, 3);
    assert_eq!(message.usage.total_tokens, 10);

    assert_eq!(message.content.len(), 2);
    match &message.content[0] {
        Content::Thinking { inner } => assert_eq!(inner.thinking, "thinking hard"),
        other => panic!("expected thinking content, got {other:?}"),
    }
    match &message.content[1] {
        Content::Text { inner } => assert_eq!(inner.text, "hello world"),
        other => panic!("expected text content, got {other:?}"),
    }
}

#[tokio::test]
async fn missing_api_key_yields_error_event() {
    let model = make_model("http://127.0.0.1:9/v1/chat/completions".to_string());

    let stream =
        stream_openai_completions(&model, &make_context(), OpenAICompletionsOptions::default());

    let events: Vec<AssistantMessageEvent> = stream.collect().await;

    assert_eq!(events.len(), 1);
    match &events[0] {
        AssistantMessageEvent::Error { error, .. } => {
            assert_eq!(error.stop_reason, StopReason::Error);
            assert!(error
                .error_message
                .as_deref()
                .is_some_and(|message| message.contains("No API key")));
        }
        other => panic!("expected Error event, got {other:?}"),
    }
}
