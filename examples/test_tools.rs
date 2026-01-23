//! Test tool calling with OpenRouter

use alchemy::providers::openai_completions::{
    stream_openai_completions, OpenAICompletionsOptions, ToolChoice,
};
use alchemy::types::{
    AssistantMessageEvent, Context, InputType, Model, ModelCost, OpenAICompletions, Provider, Tool,
    UserContent, UserMessage,
};
use futures::StreamExt;
use serde_json::json;

#[tokio::main]
async fn main() {
    let api_key = std::env::var("OPENROUTER_API_KEY").expect("OPENROUTER_API_KEY not set");

    let model: Model<OpenAICompletions> = Model {
        id: "anthropic/claude-3.5-sonnet".to_string(),
        name: "Claude 3.5 Sonnet (via OpenRouter)".to_string(),
        api: OpenAICompletions,
        provider: Provider::Custom("openrouter".to_string()),
        base_url: "https://openrouter.ai/api/v1/chat/completions".to_string(),
        reasoning: false,
        input: vec![InputType::Text],
        cost: ModelCost {
            input: 0.0,
            output: 0.0,
            cache_read: 0.0,
            cache_write: 0.0,
        },
        context_window: 200000,
        max_tokens: 8192,
        headers: None,
        compat: None,
    };

    println!("Testing OpenRouter Tool Calling");
    println!("Model: {}\n", model.id);

    // Define a weather tool
    let weather_tool = Tool {
        name: "get_weather".to_string(),
        description: "Get the current weather in a given location".to_string(),
        parameters: json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "The city and state, e.g. San Francisco, CA"
                },
                "unit": {
                    "type": "string",
                    "enum": ["celsius", "fahrenheit"],
                    "description": "The unit of temperature"
                }
            },
            "required": ["location"]
        }),
    };

    let context = Context {
        system_prompt: Some("You are a helpful assistant with access to weather data.".to_string()),
        messages: vec![alchemy::types::Message::User(UserMessage {
            content: UserContent::Text("What's the weather like in San Francisco?".to_string()),
            timestamp: 0,
        })],
        tools: Some(vec![weather_tool]),
    };

    let options = OpenAICompletionsOptions {
        api_key: Some(api_key),
        temperature: Some(0.7),
        max_tokens: Some(500),
        tool_choice: Some(ToolChoice::Auto),
        reasoning_effort: None,
        headers: None,
    };

    let mut stream = stream_openai_completions(&model, &context, options);

    let mut text_response = String::new();
    let mut tool_calls = Vec::new();

    println!("Streaming response...\n");

    while let Some(event) = stream.next().await {
        match event {
            AssistantMessageEvent::Start { .. } => {
                println!("🚀 Stream started");
            }
            AssistantMessageEvent::TextStart { .. } => {
                println!("📝 Text block started");
            }
            AssistantMessageEvent::TextDelta { delta, .. } => {
                print!("{}", delta);
                text_response.push_str(&delta);
            }
            AssistantMessageEvent::TextEnd { .. } => {
                println!("\n📝 Text block ended");
            }
            AssistantMessageEvent::ToolCallStart { .. } => {
                println!("🔧 Tool call started");
            }
            AssistantMessageEvent::ToolCallDelta { .. } => {
                print!(".");
            }
            AssistantMessageEvent::ToolCallEnd { tool_call, .. } => {
                println!("\n🔧 Tool call ended: {}", tool_call.name);
                println!(
                    "   Arguments: {}",
                    serde_json::to_string_pretty(&tool_call.arguments).unwrap()
                );
                tool_calls.push(tool_call);
            }
            AssistantMessageEvent::Done { reason, message } => {
                println!("\n✅ Done! Reason: {:?}", reason);
                println!("\nFinal message:");
                println!("  Content blocks: {}", message.content.len());
                println!("  Usage: {:?}", message.usage);
                break;
            }
            AssistantMessageEvent::Error { error, .. } => {
                eprintln!("\n❌ Error: {:?}", error);
                return;
            }
            _ => {}
        }
    }

    println!("\n═══════════════════════════════════════");
    println!("Summary:");
    println!(
        "  Text: {}",
        if text_response.is_empty() {
            "(none)"
        } else {
            &text_response
        }
    );
    println!("  Tool calls: {}", tool_calls.len());

    for (i, call) in tool_calls.iter().enumerate() {
        println!("\n  Tool #{}: {}", i + 1, call.name);
        println!("  ID: {}", call.id);
        println!(
            "  Args: {}",
            serde_json::to_string_pretty(&call.arguments).unwrap()
        );
    }
}
