use alchemy_llm::providers::openai_completions::ToolChoice;
use alchemy_llm::types::{
    AssistantMessage, AssistantMessageEvent, Context, InputType, KnownProvider, Message, Model,
    ModelCost, OpenAICompletions, Provider, TextContent, Tool, ToolCall, ToolResultContent,
    ToolResultMessage, UserContent, UserMessage,
};
use alchemy_llm::{stream, OpenAICompletionsOptions};
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

async fn stream_message(
    model: &Model<OpenAICompletions>,
    context: &Context,
    options: Option<OpenAICompletionsOptions>,
) -> Result<(AssistantMessage, Vec<ToolCall>), Box<dyn std::error::Error>> {
    let mut stream = stream(model, context, options)?;

    let mut tool_calls: Vec<ToolCall> = Vec::new();

    while let Some(event) = stream.next().await {
        match event {
            AssistantMessageEvent::TextDelta { delta, .. } => print!("{}", delta),
            AssistantMessageEvent::ToolCallEnd { tool_call, .. } => {
                println!("\n[Tool call: {}({})]", tool_call.name, tool_call.arguments);
                tool_calls.push(tool_call);
            }
            AssistantMessageEvent::Done { message, .. } => return Ok((message, tool_calls)),
            AssistantMessageEvent::Error { error, .. } => {
                let message = error
                    .error_message
                    .clone()
                    .unwrap_or_else(|| "Unknown error".to_string());
                return Err(message.into());
            }
            _ => {}
        }
    }

    Err("Stream ended without result".into())
}

fn tool_result_text(
    tool_call_id: String,
    tool_name: String,
    text: String,
    is_error: bool,
) -> ToolResultMessage {
    ToolResultMessage {
        tool_call_id,
        tool_name,
        content: vec![ToolResultContent::Text(TextContent {
            text,
            text_signature: None,
        })],
        details: None,
        is_error,
        timestamp: 0,
    }
}

fn dispatch_tool_call(call: ToolCall) -> ToolResultMessage {
    let ToolCall {
        id,
        name,
        arguments,
        thought_signature: _,
    } = call;

    if name == "get_weather" {
        let location = arguments.get("location").and_then(|v| v.as_str());

        return match location {
            Some(location) => tool_result_text(
                id,
                name,
                format!("Sunny, 22°C (location: {location})"),
                false,
            ),
            None => tool_result_text(
                id,
                name,
                "Tool call missing required argument: location".to_string(),
                true,
            ),
        };
    }

    let text = format!("Unknown tool: {name}");
    tool_result_text(id, name, text, true)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = get_model();
    let tool = get_weather_tool();

    let mut context = Context {
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

    let mut step: u32 = 1;

    loop {
        println!("Request {step}...\n");

        let (assistant_msg, tool_calls) =
            stream_message(&model, &context, Some(options.clone())).await?;

        if tool_calls.is_empty() {
            println!(
                "\n\nTotal tokens used: {}",
                assistant_msg.usage.total_tokens
            );
            return Ok(());
        }

        context.messages.push(Message::Assistant(assistant_msg));

        for tool_call in tool_calls {
            println!(
                "\n[Running tool: {}({})]",
                tool_call.name, tool_call.arguments
            );

            let tool_result = dispatch_tool_call(tool_call);
            context.messages.push(Message::ToolResult(tool_result));
        }

        println!("\n\nContinuing after tool results...\n");
        step += 1;
    }
}
