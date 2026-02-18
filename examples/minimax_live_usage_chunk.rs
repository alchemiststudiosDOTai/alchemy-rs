use alchemy_llm::stream;
use alchemy_llm::types::{Content, Context, Message, UserContent, UserMessage};
use alchemy_llm::{minimax_m2_5, OpenAICompletionsOptions};

fn default_prompt() -> String {
    std::env::var("MINIMAX_USAGE_PROMPT").unwrap_or_else(|_| "Reply with exactly: OK".to_string())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let api_key = std::env::var("MINIMAX_API_KEY").map_err(|_| "MINIMAX_API_KEY not set")?;

    let mut model = minimax_m2_5();
    model.reasoning = false;
    let context = Context {
        system_prompt: Some("You are terse.".to_string()),
        messages: vec![Message::User(UserMessage {
            content: UserContent::Text(default_prompt()),
            timestamp: 0,
        })],
        tools: None,
    };

    let options = Some(OpenAICompletionsOptions {
        api_key: Some(api_key),
        temperature: Some(0.8),
        max_tokens: Some(1024),
        ..OpenAICompletionsOptions::default()
    });

    let message = stream(&model, &context, options)?.result().await?;

    println!("== MiniMax final message ==");
    println!("api: {:?}", message.api);
    println!("provider: {:?}", message.provider);
    println!("model: {}", message.model);
    println!("stop_reason: {:?}", message.stop_reason);

    println!("\ncontent blocks:");
    for (index, block) in message.content.iter().enumerate() {
        match block {
            Content::Thinking { inner } => {
                println!(
                    "  [{index}] thinking len={} sig={:?}",
                    inner.thinking.len(),
                    inner.thinking_signature
                );
            }
            Content::Text { inner } => {
                println!("  [{index}] text: {}", inner.text);
            }
            Content::ToolCall { inner } => {
                println!("  [{index}] tool_call: {} {}", inner.id, inner.name);
            }
            Content::Image { inner } => {
                println!("  [{index}] image bytes={}", inner.data.len());
            }
        }
    }

    println!("\nusage:");
    println!("  input={}", message.usage.input);
    println!("  output={}", message.usage.output);
    println!("  cache_read={}", message.usage.cache_read);
    println!("  cache_write={}", message.usage.cache_write);
    println!("  total={}", message.usage.total_tokens);

    Ok(())
}
