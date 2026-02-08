use alchemy::stream;
use alchemy::types::{
    AssistantMessageEvent, Context, InputType, KnownProvider, Message, Model, ModelCost,
    OpenAICompletions, Provider, UserContent, UserMessage,
};
use futures::StreamExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = Model::<OpenAICompletions> {
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
    };

    let context = Context {
        system_prompt: Some("You are a helpful assistant.".to_string()),
        messages: vec![Message::User(UserMessage {
            content: UserContent::Text("What is 2 + 2?".to_string()),
            timestamp: 0,
        })],
        tools: None,
    };

    let mut stream = stream(&model, &context, None)?;

    while let Some(event) = stream.next().await {
        match event {
            AssistantMessageEvent::TextDelta { delta, .. } => print!("{}", delta),
            AssistantMessageEvent::Done { message, .. } => {
                println!("\n\nUsage: {} tokens", message.usage.total_tokens);
            }
            AssistantMessageEvent::Error { error, .. } => {
                eprintln!("Error: {:?}", error.error_message);
            }
            _ => {}
        }
    }

    Ok(())
}
