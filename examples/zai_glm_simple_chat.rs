use alchemy_llm::types::{
    AssistantMessageEvent, Context, Message, UserContent, UserMessage, ZaiChatCompletionsOptions,
    ZaiThinking, ZaiThinkingType,
};
use alchemy_llm::{glm_4_7, stream, OpenAICompletionsOptions};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("ZAI_API_KEY").map_err(|_| "ZAI_API_KEY not set")?;

    let model = glm_4_7();
    let context = Context {
        system_prompt: Some("You are concise and practical.".to_string()),
        messages: vec![Message::User(UserMessage {
            content: UserContent::Text(
                "Explain Rust ownership in two short sentences.".to_string(),
            ),
            timestamp: 0,
        })],
        tools: None,
    };

    let options = Some(OpenAICompletionsOptions {
        api_key: Some(api_key),
        temperature: Some(0.6),
        max_tokens: Some(512),
        zai: Some(ZaiChatCompletionsOptions {
            thinking: Some(ZaiThinking {
                kind: ZaiThinkingType::Enabled,
                clear_thinking: Some(false),
            }),
            ..ZaiChatCompletionsOptions::default()
        }),
        ..OpenAICompletionsOptions::default()
    });

    let mut event_stream = stream(&model, &context, options)?;

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
