//! Quick test of OpenRouter API

use alchemy_llm::providers::{stream_openai_completions, OpenAICompletionsOptions};
use alchemy_llm::types::{
    AssistantMessageEvent, Context, InputType, Model, ModelCost, OpenAICompletions, Provider,
    UserContent, UserMessage,
};
use futures::StreamExt;

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

    println!("Testing OpenRouter with: {}", model.id);
    println!("Base URL: {}\n", model.base_url);

    let context = Context {
        system_prompt: Some("You are a helpful assistant.".to_string()),
        messages: vec![alchemy_llm::types::Message::User(UserMessage {
            content: UserContent::Text("Say hello in one sentence!".to_string()),
            timestamp: 0,
        })],
        tools: None,
    };

    let options = OpenAICompletionsOptions {
        api_key: Some(api_key),
        temperature: Some(0.7),
        max_tokens: Some(100),
        tool_choice: None,
        reasoning_effort: None,
        headers: None,
    };

    let mut stream = stream_openai_completions(&model, &context, options);

    let mut response = String::new();

    while let Some(event) = stream.next().await {
        match event {
            AssistantMessageEvent::TextDelta { delta, .. } => {
                print!("{}", delta);
                response.push_str(&delta);
            }
            AssistantMessageEvent::Done { reason, .. } => {
                println!("\n\nDone! Reason: {:?}", reason);
            }
            AssistantMessageEvent::Error { error, .. } => {
                eprintln!("\nError: {:?}", error);
                return;
            }
            _ => {}
        }
    }

    println!("\n\nFull response: {}", response);
}
