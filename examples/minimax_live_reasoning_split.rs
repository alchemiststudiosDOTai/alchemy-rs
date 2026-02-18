use alchemy_llm::stream;
use alchemy_llm::types::{AssistantMessageEvent, Context, Message, UserContent, UserMessage};
use alchemy_llm::{minimax_m2_5, OpenAICompletionsOptions};
use futures::StreamExt;

fn prompt_from_env() -> String {
    std::env::var("MINIMAX_PROMPT").unwrap_or_else(|_| {
        "Explain why Rust ownership prevents data races in one paragraph.".to_string()
    })
}

fn api_key_from_env() -> Result<String, std::env::VarError> {
    std::env::var("MINIMAX_API_KEY")
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = api_key_from_env().map_err(|_| "MINIMAX_API_KEY not set")?;
    let prompt = prompt_from_env();

    let model = minimax_m2_5();
    let context = Context {
        system_prompt: Some("You are a concise technical assistant.".to_string()),
        messages: vec![Message::User(UserMessage {
            content: UserContent::Text(prompt),
            timestamp: 0,
        })],
        tools: None,
    };

    let options = Some(OpenAICompletionsOptions {
        api_key: Some(api_key),
        temperature: Some(0.7),
        max_tokens: Some(768),
        ..OpenAICompletionsOptions::default()
    });

    let mut event_stream = stream(&model, &context, options)?;

    println!("== MiniMax reasoning_split stream ==");

    while let Some(event) = event_stream.next().await {
        match event {
            AssistantMessageEvent::ThinkingStart { .. } => println!("\n[thinking:start]"),
            AssistantMessageEvent::ThinkingDelta { delta, .. } => print!("{delta}"),
            AssistantMessageEvent::ThinkingEnd { .. } => println!("\n[thinking:end]"),
            AssistantMessageEvent::TextStart { .. } => println!("\n[text:start]"),
            AssistantMessageEvent::TextDelta { delta, .. } => print!("{delta}"),
            AssistantMessageEvent::TextEnd { .. } => println!("\n[text:end]"),
            AssistantMessageEvent::Done { message, .. } => {
                println!("\n== done ==");
                println!("model: {}", message.model);
                println!("stop_reason: {:?}", message.stop_reason);
                println!(
                    "usage: input={} output={} total={}",
                    message.usage.input, message.usage.output, message.usage.total_tokens
                );
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
