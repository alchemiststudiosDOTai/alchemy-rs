use alchemy::providers::openai_completions::ToolChoice;
use alchemy::types::{
    AssistantMessage, AssistantMessageEvent, Content, Context, InputType, KnownProvider, Message,
    Model, ModelCost, OpenAICompletions, Provider, TextContent, Tool, ToolResultContent,
    ToolResultMessage, UserContent, UserMessage,
};
use alchemy::{stream, OpenAICompletionsOptions};
use futures::StreamExt;
use serde_json::json;

fn get_model() -> Model<OpenAICompletions> {
    Model {
        id: "gpt-4o-mini".to_string(),
        name: "GPT-4o Mini".to_string(),
        api: OpenAICompletions,
        provider: Provider::Known(KnownProvider::OpenAI),
        base_url: "https://api.openai.com/v1".to_string(),
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

fn get_weather_tool() -> Tool {
    Tool::new(
        "get_weather",
        "Get the current weather for a location",
        json!({
            "type": "object",
            "properties": { "location": { "type": "string", "description": "City name" } },
            "required": ["location"]
        }),
    )
}

async fn stream_once(
    model: &Model<OpenAICompletions>,
    context: &Context,
    options: Option<OpenAICompletionsOptions>,
) -> Result<AssistantMessage, Box<dyn std::error::Error>> {
    let mut stream = stream(model, context, options)?;
    let mut result: Option<AssistantMessage> = None;
    while let Some(event) = stream.next().await {
        match event {
            AssistantMessageEvent::TextDelta { delta, .. } => print!("{}", delta),
            AssistantMessageEvent::ToolCallEnd { tool_call, .. } => {
                println!("\n[Tool call: {}({})]", tool_call.name, tool_call.arguments);
            }
            AssistantMessageEvent::Done { message, .. } => result = Some(message),
            AssistantMessageEvent::Error { error, .. } => {
                eprintln!("Error: {:?}", error.error_message)
            }
            _ => {}
        }
    }
    result.ok_or_else(|| "Stream ended without result".into())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = get_model();
    let tool = get_weather_tool();
    let context = Context {
        system_prompt: Some("You are a helpful assistant.".to_string()),
        messages: vec![Message::User(UserMessage {
            content: UserContent::Text("What's the weather in Tokyo?".to_string()),
            timestamp: 0,
        })],
        tools: Some(vec![tool]),
    };
    let options = OpenAICompletionsOptions {
        tool_choice: Some(ToolChoice::Auto),
        ..Default::default()
    };
    println!("First request (expecting tool call)...\n");
    let assistant_msg = stream_once(&model, &context, Some(options)).await?;
    let tool_call_id = assistant_msg
        .content
        .iter()
        .find_map(|c| match c {
            Content::ToolCall { inner } => Some(inner.id.clone()),
            _ => None,
        })
        .unwrap_or_default();
    let tool_result = ToolResultMessage {
        tool_call_id,
        tool_name: "get_weather".to_string(),
        content: vec![ToolResultContent::Text(TextContent {
            text: "Sunny, 22°C".to_string(),
            text_signature: None,
        })],
        details: None,
        is_error: false,
        timestamp: 0,
    };
    let mut context_with_result = context;
    context_with_result
        .messages
        .push(Message::Assistant(assistant_msg));
    context_with_result
        .messages
        .push(Message::ToolResult(tool_result));
    println!("\n\nSecond request (final response)...\n");
    let final_msg = stream_once(&model, &context_with_result, None).await?;
    println!("\n\nTotal tokens used: {}", final_msg.usage.total_tokens);
    Ok(())
}
