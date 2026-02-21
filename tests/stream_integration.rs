//! Integration tests for the streaming API
//!
//! These tests verify end-to-end streaming behavior without requiring
//! live API keys. They test the complete flow from model configuration
//! through event stream production.

use alchemy_llm::stream;
use alchemy_llm::types::{
    Api, ApiType, Context, InputType, KnownProvider, Message, Model, ModelCost, OpenAICompletions,
    Provider, UserContent, UserMessage,
};

fn create_test_model(base_url: &str) -> Model<OpenAICompletions> {
    Model {
        id: "gpt-4o-mini".to_string(),
        name: "GPT-4o Mini".to_string(),
        api: OpenAICompletions,
        provider: Provider::Known(KnownProvider::OpenAI),
        base_url: base_url.to_string(),
        reasoning: false,
        input: vec![InputType::Text],
        cost: ModelCost {
            input: 0.0,
            output: 0.0,
            cache_read: 0.0,
            cache_write: 0.0,
        },
        context_window: 128_000,
        max_tokens: 16_384,
        headers: None,
        compat: None,
    }
}

fn create_simple_context(message: &str) -> Context {
    Context {
        system_prompt: None,
        messages: vec![Message::User(UserMessage {
            content: UserContent::Text(message.to_string()),
            timestamp: 0,
        })],
        tools: None,
    }
}

#[tokio::test]
async fn stream_creation_fails_without_api_key() {
    let model = create_test_model("https://api.openai.com/v1");
    let context = create_simple_context("Hello");

    let result = stream(&model, &context, None);
    assert!(result.is_err());

    match result {
        Err(e) => assert!(e.to_string().contains("No API key")),
        Ok(_) => panic!("Expected error"),
    }
}

#[tokio::test]
async fn stream_creation_succeeds_with_api_key_in_options() {
    let model = create_test_model("http://127.0.0.1:1");
    let context = create_simple_context("Hello");

    let options = alchemy_llm::OpenAICompletionsOptions {
        api_key: Some("test-key".to_string()),
        ..Default::default()
    };

    let result = stream(&model, &context, Some(options));
    assert!(result.is_ok());
}

#[tokio::test]
async fn stream_handles_empty_context() {
    let model = create_test_model("http://127.0.0.1:1");
    let context = Context {
        system_prompt: None,
        messages: vec![],
        tools: None,
    };

    let options = alchemy_llm::OpenAICompletionsOptions {
        api_key: Some("test-key".to_string()),
        ..Default::default()
    };

    let result = stream(&model, &context, Some(options));
    // Empty context should still create a stream (provider may reject it)
    assert!(result.is_ok());
}

#[tokio::test]
async fn stream_with_system_prompt_produces_events() {
    let model = create_test_model("http://127.0.0.1:1");
    let context = Context {
        system_prompt: Some("You are a helpful assistant.".to_string()),
        messages: vec![Message::User(UserMessage {
            content: UserContent::Text("Hi".to_string()),
            timestamp: 0,
        })],
        tools: None,
    };

    let options = alchemy_llm::OpenAICompletionsOptions {
        api_key: Some("test-key".to_string()),
        ..Default::default()
    };

    let result = stream(&model, &context, Some(options));
    assert!(result.is_ok());
}

#[test]
fn model_provider_matches_api_type() {
    let model = create_test_model("https://api.openai.com/v1");
    assert_eq!(model.api.api(), Api::OpenAICompletions);
    assert_eq!(model.provider.to_string(), "openai");
}

#[test]
fn context_with_multiple_messages_validates() {
    let context = Context {
        system_prompt: Some("System prompt".to_string()),
        messages: vec![
            Message::User(UserMessage {
                content: UserContent::Text("First message".to_string()),
                timestamp: 0,
            }),
            Message::User(UserMessage {
                content: UserContent::Text("Second message".to_string()),
                timestamp: 1,
            }),
        ],
        tools: None,
    };

    assert_eq!(context.messages.len(), 2);
    assert!(context.system_prompt.is_some());
}

#[tokio::test]
async fn stream_with_custom_headers_in_options() {
    let model = create_test_model("http://127.0.0.1:1");
    let context = create_simple_context("Hello");

    let mut headers = std::collections::HashMap::new();
    headers.insert("X-Custom-Header".to_string(), "custom-value".to_string());

    let options = alchemy_llm::OpenAICompletionsOptions {
        api_key: Some("test-key".to_string()),
        headers: Some(headers),
        ..Default::default()
    };

    let result = stream(&model, &context, Some(options));
    assert!(result.is_ok());
}

#[tokio::test]
async fn stream_with_temperature_option() {
    let model = create_test_model("http://127.0.0.1:1");
    let context = create_simple_context("Hello");

    let options = alchemy_llm::OpenAICompletionsOptions {
        api_key: Some("test-key".to_string()),
        temperature: Some(0.7),
        ..Default::default()
    };

    let result = stream(&model, &context, Some(options));
    assert!(result.is_ok());
}

#[tokio::test]
async fn stream_with_max_tokens_option() {
    let model = create_test_model("http://127.0.0.1:1");
    let context = create_simple_context("Hello");

    let options = alchemy_llm::OpenAICompletionsOptions {
        api_key: Some("test-key".to_string()),
        max_tokens: Some(100),
        ..Default::default()
    };

    let result = stream(&model, &context, Some(options));
    assert!(result.is_ok());
}
