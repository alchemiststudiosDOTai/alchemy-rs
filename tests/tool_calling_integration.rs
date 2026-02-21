//! Integration tests for tool calling functionality
//!
//! These tests verify the complete tool calling flow including:
//! - Tool definition and serialization
//! - Tool call ID generation and handling
//! - Message context building with tools
//! - Tool result message construction

use alchemy_llm::types::{
    Api, AssistantMessage, Content, Context, Message, StopReason, TextContent, Tool, ToolCall,
    ToolCallId, ToolResultContent, ToolResultMessage, Usage, UserContent, UserMessage,
};
use serde_json::json;

fn create_test_tool(name: &str, description: &str) -> Tool {
    Tool::new(
        name,
        description,
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query"
                }
            },
            "required": ["query"]
        }),
    )
}

fn create_tool_with_complex_schema() -> Tool {
    Tool::new(
        "get_weather",
        "Get weather information for a location",
        json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "City name or coordinates"
                },
                "units": {
                    "type": "string",
                    "enum": ["celsius", "fahrenheit"],
                    "description": "Temperature units"
                },
                "days": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 7,
                    "description": "Number of forecast days"
                }
            },
            "required": ["location"]
        }),
    )
}

#[test]
fn tool_serialization_preserves_schema() {
    let tool = create_test_tool("search", "Search for documents");
    let serialized = serde_json::to_string(&tool).expect("Tool should serialize");

    assert!(serialized.contains("search"));
    assert!(serialized.contains("Search for documents"));
    assert!(serialized.contains("object"));
    assert!(serialized.contains("properties"));
}

#[test]
fn tool_with_complex_schema_serializes_correctly() {
    let tool = create_tool_with_complex_schema();
    let serialized = serde_json::to_string(&tool).expect("Tool should serialize");

    assert!(serialized.contains("get_weather"));
    assert!(serialized.contains("celsius"));
    assert!(serialized.contains("fahrenheit"));
    assert!(serialized.contains("minimum"));
    assert!(serialized.contains("maximum"));
}

#[test]
fn tool_call_id_creation_from_string() {
    let id: ToolCallId = "call_123".into();
    assert_eq!(id.as_str(), "call_123");
    assert_eq!(id.to_string(), "call_123");
}

#[test]
fn tool_call_id_creation_from_str() {
    let id = ToolCallId::from("valid_id-123");
    assert_eq!(id.as_str(), "valid_id-123");

    let another_valid = ToolCallId::from("abcABC_012");
    assert_eq!(another_valid.as_str(), "abcABC_012");
}

#[test]
fn tool_call_id_round_trip_conversion() {
    let original = "test-id-456";
    let id: ToolCallId = original.into();
    let back_to_string: String = id.into();
    assert_eq!(back_to_string, original);
}

#[test]
fn tool_call_construction_with_all_fields() {
    let tool_call = ToolCall {
        id: "call_abc123".into(),
        name: "get_weather".to_string(),
        arguments: json!({"location": "Tokyo", "units": "celsius"}),
        thought_signature: None,
    };

    assert_eq!(tool_call.id.as_str(), "call_abc123");
    assert_eq!(tool_call.name, "get_weather");
    assert_eq!(tool_call.arguments["location"], "Tokyo");
}

#[test]
fn tool_result_message_construction() {
    let tool_call_id: ToolCallId = "call_123".into();
    let content = vec![ToolResultContent::Text(TextContent {
        text: "Sunny, 22C".to_string(),
        text_signature: None,
    })];

    let result = ToolResultMessage {
        tool_call_id,
        tool_name: "get_weather".to_string(),
        content,
        details: None,
        is_error: false,
        timestamp: 1234567890,
    };

    assert_eq!(result.tool_call_id.as_str(), "call_123");
    assert_eq!(result.tool_name, "get_weather");
    assert!(!result.is_error);
}

#[test]
fn tool_result_with_error_flag() {
    let result = ToolResultMessage {
        tool_call_id: "call_456".into(),
        tool_name: "search".to_string(),
        content: vec![ToolResultContent::Text(TextContent {
            text: "Search failed: timeout".to_string(),
            text_signature: None,
        })],
        details: None,
        is_error: true,
        timestamp: 0,
    };

    assert!(result.is_error);
    match &result.content[0] {
        ToolResultContent::Text(t) => assert!(t.text.contains("failed")),
        _ => panic!("Expected text content"),
    }
}

#[test]
fn context_with_tools_constructs_correctly() {
    let tools = vec![
        create_test_tool("search", "Search documents"),
        create_tool_with_complex_schema(),
    ];

    let context = Context {
        system_prompt: Some("You can use tools to help.".to_string()),
        messages: vec![Message::User(UserMessage {
            content: UserContent::Text("What's the weather?".to_string()),
            timestamp: 0,
        })],
        tools: Some(tools),
    };

    assert!(context.tools.is_some());
    assert_eq!(context.tools.as_ref().unwrap().len(), 2);
}

#[test]
fn assistant_message_with_content() {
    let assistant_msg = AssistantMessage {
        content: vec![Content::text("The weather in Paris is sunny.")],
        api: Api::OpenAICompletions,
        provider: alchemy_llm::types::Provider::Custom("test-provider".to_string()),
        model: "gpt-4".to_string(),
        usage: Usage::default(),
        stop_reason: StopReason::Stop,
        error_message: None,
        timestamp: 0,
    };

    assert_eq!(assistant_msg.content.len(), 1);
    assert_eq!(assistant_msg.stop_reason, StopReason::Stop);
}

#[test]
fn message_sequence_with_user_and_assistant() {
    let messages: Vec<Message> = vec![
        Message::User(UserMessage {
            content: UserContent::Text("What's 2 + 2?".to_string()),
            timestamp: 0,
        }),
        Message::Assistant(AssistantMessage {
            content: vec![Content::text("2 + 2 = 4")],
            api: Api::OpenAICompletions,
            provider: alchemy_llm::types::Provider::Custom("test-provider".to_string()),
            model: "gpt-4".to_string(),
            usage: Usage::default(),
            stop_reason: StopReason::Stop,
            error_message: None,
            timestamp: 1,
        }),
        Message::User(UserMessage {
            content: UserContent::Text("What about 3 + 3?".to_string()),
            timestamp: 2,
        }),
    ];

    assert_eq!(messages.len(), 3);

    match &messages[0] {
        Message::User(msg) => match &msg.content {
            UserContent::Text(text) => assert_eq!(text, "What's 2 + 2?"),
            _ => panic!("Expected text content"),
        },
        _ => panic!("Expected user message"),
    }

    match &messages[1] {
        Message::Assistant(msg) => {
            assert_eq!(msg.content.len(), 1);
            assert_eq!(msg.stop_reason, StopReason::Stop);
        }
        _ => panic!("Expected assistant message"),
    }

    match &messages[2] {
        Message::User(msg) => match &msg.content {
            UserContent::Text(text) => assert_eq!(text, "What about 3 + 3?"),
            _ => panic!("Expected text content"),
        },
        _ => panic!("Expected user message"),
    }
}

#[test]
fn tool_result_in_message_sequence() {
    let tool_result = ToolResultMessage {
        tool_call_id: "call_calc".into(),
        tool_name: "calculate".to_string(),
        content: vec![ToolResultContent::Text(TextContent {
            text: "4".to_string(),
            text_signature: None,
        })],
        details: None,
        is_error: false,
        timestamp: 2,
    };

    let message = Message::ToolResult(tool_result);

    match &message {
        Message::ToolResult(msg) => {
            assert_eq!(msg.tool_call_id.as_str(), "call_calc");
            assert_eq!(msg.tool_name, "calculate");
            assert!(!msg.is_error);
        }
        _ => panic!("Expected tool result message"),
    }
}
