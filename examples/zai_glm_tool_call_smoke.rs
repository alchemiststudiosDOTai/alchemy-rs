use alchemy_llm::providers::openai_completions::ToolChoice;
use alchemy_llm::types::{
    AssistantMessageEvent, Context, Message, Tool, ToolCall, UserContent, UserMessage,
    ZaiChatCompletionsOptions,
};
use alchemy_llm::{glm_4_7, stream, OpenAICompletionsOptions};
use futures::StreamExt;
use serde_json::json;

fn weather_tool() -> Tool {
    Tool::new(
        "get_weather",
        "Get weather data for a city",
        json!({
            "type": "object",
            "properties": {
                "city": { "type": "string" }
            },
            "required": ["city"]
        }),
    )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("ZAI_API_KEY").map_err(|_| "ZAI_API_KEY not set")?;

    let model = glm_4_7();
    let context = Context {
        system_prompt: Some("Call tools when useful.".to_string()),
        messages: vec![Message::User(UserMessage {
            content: UserContent::Text("What is the weather in Tokyo?".to_string()),
            timestamp: 0,
        })],
        tools: Some(vec![weather_tool()]),
    };

    let options = Some(OpenAICompletionsOptions {
        api_key: Some(api_key),
        tool_choice: Some(ToolChoice::Auto),
        zai: Some(ZaiChatCompletionsOptions {
            tool_stream: Some(true),
            ..ZaiChatCompletionsOptions::default()
        }),
        ..OpenAICompletionsOptions::default()
    });

    let mut event_stream = stream(&model, &context, options)?;
    let mut tool_calls: Vec<ToolCall> = Vec::new();

    while let Some(event) = event_stream.next().await {
        match event {
            AssistantMessageEvent::TextDelta { delta, .. } => print!("{delta}"),
            AssistantMessageEvent::ToolCallEnd { tool_call, .. } => {
                println!("\n[tool_call] {}({})", tool_call.name, tool_call.arguments);
                tool_calls.push(tool_call);
            }
            AssistantMessageEvent::Done { message, .. } => {
                println!("\n== done ==");
                println!("stop_reason: {:?}", message.stop_reason);
                println!("tool_calls: {}", tool_calls.len());
                break;
            }
            AssistantMessageEvent::Error { error, .. } => {
                eprintln!("\n== error ==");
                eprintln!("{:?}", error.error_message);
                break;
            }
            _ => {}
        }
    }

    Ok(())
}
