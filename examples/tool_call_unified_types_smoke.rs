use alchemy_llm::providers::openai_completions::ToolChoice;
use alchemy_llm::types::{
    AssistantMessageEvent, Context, InputType, KnownProvider, Message, Model, ModelCost,
    OpenAICompletions, Provider, TextContent, Tool, ToolCall, ToolCallId, ToolResultContent,
    ToolResultMessage, UserContent, UserMessage,
};
use alchemy_llm::{minimax_m2_5, stream, OpenAICompletionsOptions};
use futures::StreamExt;
use serde_json::json;

const DEFAULT_PROMPT: &str =
    "Call get_weather exactly once for Tokyo. Do not provide a final answer before the tool call.";

fn build_tool() -> Tool {
    Tool::new(
        "get_weather",
        "Get weather by location",
        json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "City name"
                }
            },
            "required": ["location"]
        }),
    )
}

fn build_context() -> Context {
    let prompt = std::env::var("TOOL_SMOKE_PROMPT").unwrap_or_else(|_| DEFAULT_PROMPT.to_string());

    Context {
        system_prompt: Some(
            "You are a strict function-calling assistant. Always call a tool when available."
                .to_string(),
        ),
        messages: vec![Message::User(UserMessage {
            content: UserContent::Text(prompt),
            timestamp: 0,
        })],
        tools: Some(vec![build_tool()]),
    }
}

fn make_options(api_key: String) -> OpenAICompletionsOptions {
    OpenAICompletionsOptions {
        api_key: Some(api_key),
        temperature: Some(0.2),
        max_tokens: Some(256),
        tool_choice: Some(ToolChoice::Required),
        reasoning_effort: None,
        headers: None,
    }
}

fn assert_unified_tool_call_id_type(id: &ToolCallId) {
    let _ = id;
}

fn value_type_name<T>(_value: &T) -> &'static str {
    std::any::type_name::<T>()
}

fn prove_unified_type(label: &str, tool_call: &ToolCall) -> Result<(), Box<dyn std::error::Error>> {
    assert_unified_tool_call_id_type(&tool_call.id);

    let tool_result = ToolResultMessage {
        tool_call_id: tool_call.id.clone(),
        tool_name: tool_call.name.clone(),
        content: vec![ToolResultContent::Text(TextContent {
            text: "Smoke tool result placeholder".to_string(),
            text_signature: None,
        })],
        details: None,
        is_error: false,
        timestamp: 0,
    };

    assert_unified_tool_call_id_type(&tool_result.tool_call_id);

    println!(
        "[{label}] type(tool_call.id) = {}",
        value_type_name(&tool_call.id)
    );
    println!(
        "[{label}] type(tool_result.tool_call_id) = {}",
        value_type_name(&tool_result.tool_call_id)
    );
    println!(
        "[{label}] id_equal_after_copy = {}",
        tool_result.tool_call_id == tool_call.id
    );

    let serialized = serde_json::to_string(&tool_result)?;
    println!("[{label}] serialized_tool_result = {serialized}");

    Ok(())
}

async fn run_provider_smoke<TApi>(
    label: &str,
    model: &Model<TApi>,
    api_key: String,
) -> Result<(), Box<dyn std::error::Error>>
where
    TApi: alchemy_llm::types::ApiType,
{
    println!("\n=== {label} ===");
    println!("model: {}", model.id);
    println!("provider: {}", model.provider);
    println!("base_url: {}", model.base_url);

    let mut stream = stream(model, &build_context(), Some(make_options(api_key)))?;
    let mut saw_tool_call = false;

    while let Some(event) = stream.next().await {
        match event {
            AssistantMessageEvent::ToolCallStart { content_index, .. } => {
                println!("[{label}] tool_call_start block={content_index}");
            }
            AssistantMessageEvent::ToolCallDelta { delta, .. } => {
                println!("[{label}] tool_call_delta bytes={}", delta.len());
            }
            AssistantMessageEvent::ToolCallEnd { tool_call, .. } => {
                saw_tool_call = true;
                println!(
                    "[{label}] tool_call_end name={} id={} empty_id={}",
                    tool_call.name,
                    tool_call.id,
                    tool_call.id.is_empty()
                );
                println!("[{label}] tool_call_args={}", tool_call.arguments);
                prove_unified_type(label, &tool_call)?;
            }
            AssistantMessageEvent::Done { reason, message } => {
                println!(
                    "[{label}] done reason={reason:?} stop_reason={:?}",
                    message.stop_reason
                );
                break;
            }
            AssistantMessageEvent::Error { error, .. } => {
                let message = error
                    .error_message
                    .unwrap_or_else(|| "Unknown provider error".to_string());
                return Err(format!("[{label}] stream error: {message}").into());
            }
            _ => {}
        }
    }

    if !saw_tool_call {
        return Err(format!("[{label}] no tool call observed").into());
    }

    Ok(())
}

fn openrouter_model() -> Model<OpenAICompletions> {
    let openrouter_model_id = std::env::var("OPENROUTER_MODEL")
        .unwrap_or_else(|_| "anthropic/claude-3.5-sonnet".to_string());

    Model {
        id: openrouter_model_id,
        name: "OpenRouter tool smoke model".to_string(),
        api: OpenAICompletions,
        provider: Provider::Known(KnownProvider::OpenRouter),
        base_url: "https://openrouter.ai/api/v1/chat/completions".to_string(),
        reasoning: false,
        input: vec![InputType::Text],
        cost: ModelCost {
            input: 0.0,
            output: 0.0,
            cache_read: 0.0,
            cache_write: 0.0,
        },
        context_window: 200_000,
        max_tokens: 8_192,
        headers: None,
        compat: None,
    }
}

fn chutes_model() -> Model<OpenAICompletions> {
    let chutes_model_id = std::env::var("CHUTES_MODEL")
        .unwrap_or_else(|_| "deepseek-ai/DeepSeek-V3-0324".to_string());

    Model {
        id: chutes_model_id,
        name: "Chutes tool smoke model".to_string(),
        api: OpenAICompletions,
        provider: Provider::Custom("chutes".to_string()),
        base_url: "https://llm.chutes.ai/v1/chat/completions".to_string(),
        reasoning: false,
        input: vec![InputType::Text],
        cost: ModelCost {
            input: 0.0,
            output: 0.0,
            cache_read: 0.0,
            cache_write: 0.0,
        },
        context_window: 128_000,
        max_tokens: 4_096,
        headers: None,
        compat: None,
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let openrouter_api_key =
        std::env::var("OPENROUTER_API_KEY").map_err(|_| "OPENROUTER_API_KEY not set")?;
    let minimax_api_key =
        std::env::var("MINIMAX_API_KEY").map_err(|_| "MINIMAX_API_KEY not set")?;
    let chutes_api_key = std::env::var("CHUTES_API_KEY").map_err(|_| "CHUTES_API_KEY not set")?;

    run_provider_smoke("openrouter", &openrouter_model(), openrouter_api_key).await?;

    let mut minimax_model = minimax_m2_5();
    minimax_model.reasoning = false;
    run_provider_smoke("minimax", &minimax_model, minimax_api_key).await?;

    run_provider_smoke("chutes", &chutes_model(), chutes_api_key).await?;

    println!("\nAll provider tool-call smoke checks passed.");
    Ok(())
}
