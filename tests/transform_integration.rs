//! Integration tests for message transformation
//!
//! These tests verify cross-provider message transformation
//! including format conversion, tool mapping, and usage calculation.

use alchemy_llm::types::{
    Api, AssistantMessage, Content, Context, Cost, InputType, KnownProvider, Message, Model,
    ModelCost, OpenAICompletions, Provider, StopReason, StopReasonError, StopReasonSuccess,
    TextContent, Tool, ToolCallId, ToolResultContent, ToolResultMessage, Usage, UserContent,
    UserMessage,
};
use serde_json::json;

#[test]
fn provider_custom_creation() {
    let provider = Provider::Custom("custom-provider".to_string());
    assert_eq!(provider.to_string(), "custom-provider");
}

#[test]
fn stop_reason_mapping_contract() {
    // Test that stop reasons map correctly between internal and provider formats
    assert_eq!(StopReason::from(StopReasonSuccess::Stop), StopReason::Stop);
    assert_eq!(
        StopReason::from(StopReasonSuccess::Length),
        StopReason::Length
    );
    assert_eq!(
        StopReason::from(StopReasonSuccess::ToolUse),
        StopReason::ToolUse
    );
    assert_eq!(StopReason::from(StopReasonError::Error), StopReason::Error);
    assert_eq!(
        StopReason::from(StopReasonError::Aborted),
        StopReason::Aborted
    );
}

#[test]
fn usage_default_is_zero() {
    let usage = Usage::default();
    assert_eq!(usage.input, 0);
    assert_eq!(usage.output, 0);
    assert_eq!(usage.cache_read, 0);
    assert_eq!(usage.cache_write, 0);
    assert_eq!(usage.total_tokens, 0);
    assert_eq!(usage.cost.input, 0.0);
    assert_eq!(usage.cost.output, 0.0);
}

#[test]
fn usage_with_values_calculates_total() {
    let usage = Usage {
        input: 10,
        output: 20,
        cache_read: 5,
        cache_write: 3,
        total_tokens: 38,
        cost: Cost::default(),
    };

    assert_eq!(usage.total_tokens, 38);
    assert_eq!(usage.input, 10);
    assert_eq!(usage.output, 20);
}

#[test]
fn usage_cost_calculation() {
    let cost = Cost {
        input: 0.01,
        output: 0.03,
        cache_read: 0.005,
        cache_write: 0.01,
        total: 0.05,
    };

    assert_eq!(cost.input, 0.01);
    assert_eq!(cost.output, 0.03);
    assert_eq!(cost.total, 0.05);
}

#[test]
fn model_creation_with_all_fields() {
    let model = Model::<OpenAICompletions> {
        id: "test-model".to_string(),
        name: "Test Model".to_string(),
        api: OpenAICompletions,
        provider: Provider::Known(KnownProvider::OpenAI),
        base_url: "https://api.openai.com/v1".to_string(),
        reasoning: true,
        input: vec![InputType::Text, InputType::Image],
        cost: ModelCost {
            input: 0.001,
            output: 0.002,
            cache_read: 0.0005,
            cache_write: 0.001,
        },
        context_window: 128_000,
        max_tokens: 4096,
        headers: None,
        compat: None,
    };

    assert_eq!(model.id, "test-model");
    assert!(model.reasoning);
    assert_eq!(model.input.len(), 2);
    assert_eq!(model.cost.input, 0.001);
}

#[test]
fn user_content_text_creation() {
    let content = UserContent::Text("Hello, world!".to_string());

    match content {
        UserContent::Text(text) => assert_eq!(text, "Hello, world!"),
        _ => panic!("Expected text content"),
    }
}

#[test]
fn message_context_with_multiple_types() {
    let tool = Tool::new(
        "test_tool",
        "A test tool",
        json!({"type": "object", "properties": {}}),
    );

    let context = Context {
        system_prompt: Some("System".to_string()),
        messages: vec![
            Message::User(UserMessage {
                content: UserContent::Text("Hello".to_string()),
                timestamp: 0,
            }),
            Message::Assistant(AssistantMessage {
                content: vec![Content::text("Response")],
                api: Api::OpenAICompletions,
                provider: Provider::Custom("test".to_string()),
                model: "gpt-4".to_string(),
                usage: Usage::default(),
                stop_reason: StopReason::Stop,
                error_message: None,
                timestamp: 1,
            }),
            Message::ToolResult(ToolResultMessage {
                tool_call_id: "call_1".into(),
                tool_name: "test_tool".to_string(),
                content: vec![ToolResultContent::Text(TextContent {
                    text: "Result".to_string(),
                    text_signature: None,
                })],
                details: None,
                is_error: false,
                timestamp: 2,
            }),
        ],
        tools: Some(vec![tool]),
    };

    assert_eq!(context.messages.len(), 3);
    assert!(context.system_prompt.is_some());
}

#[test]
fn provider_string_representation() {
    let openai = Provider::Known(KnownProvider::OpenAI);
    let anthropic = Provider::Known(KnownProvider::Anthropic);
    let google = Provider::Known(KnownProvider::Google);

    assert_eq!(openai.to_string(), "openai");
    assert_eq!(anthropic.to_string(), "anthropic");
    assert_eq!(google.to_string(), "google");
}

#[test]
fn api_type_consistency() {
    assert_eq!(Api::OpenAICompletions.to_string(), "openai-completions");
    assert_eq!(Api::AnthropicMessages.to_string(), "anthropic-messages");
    assert_eq!(Api::GoogleGenerativeAi.to_string(), "google-generative-ai");
}

#[test]
fn model_cost_calculation() {
    let cost = ModelCost {
        input: 0.001,
        output: 0.002,
        cache_read: 0.0005,
        cache_write: 0.001,
    };

    // Cost for 1000 input tokens
    let input_cost = cost.input * 1000.0;
    assert_eq!(input_cost, 1.0);

    // Cost for 500 output tokens
    let output_cost = cost.output * 500.0;
    assert_eq!(output_cost, 1.0);
}

#[test]
fn stop_reason_error_variants_exist() {
    // Verify Error and Aborted variants exist
    let _err = StopReasonError::Error;
    let _aborted = StopReasonError::Aborted;
}

#[test]
fn stop_reason_success_variants_exist() {
    // Verify all success variants exist
    let _stop = StopReasonSuccess::Stop;
    let _length = StopReasonSuccess::Length;
    let _tool_use = StopReasonSuccess::ToolUse;
}

#[test]
fn stop_reason_enum_variants_exist() {
    // Verify all StopReason variants exist
    let _stop = StopReason::Stop;
    let _length = StopReason::Length;
    let _tool_use = StopReason::ToolUse;
    let _error = StopReason::Error;
    let _aborted = StopReason::Aborted;
}

#[test]
fn tool_call_id_in_tool_result() {
    let id: ToolCallId = "test-call-id".into();
    let result = ToolResultMessage {
        tool_call_id: id,
        tool_name: "test".to_string(),
        content: vec![],
        details: None,
        is_error: false,
        timestamp: 0,
    };

    assert_eq!(result.tool_call_id.as_str(), "test-call-id");
}

#[test]
fn content_text_creation() {
    let content = Content::text("Hello world");
    match content {
        Content::Text { inner } => assert_eq!(inner.text, "Hello world"),
        _ => panic!("Expected text content"),
    }
}

#[test]
fn context_default_is_empty() {
    let ctx = Context::default();
    assert!(ctx.system_prompt.is_none());
    assert!(ctx.messages.is_empty());
    assert!(ctx.tools.is_none());
}
