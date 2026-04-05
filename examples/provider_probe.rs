use std::env;
use std::time::Instant;

use alchemy_llm::types::{
    AnthropicMessages, AssistantMessage, AssistantMessageEvent, CacheOptions, Context, InputType,
    KnownProvider, Message, Model, ModelCost, OpenAICompletions, Provider, Usage, UserContent,
    UserMessage,
};
use alchemy_llm::{kimi_k2_5, minimax_m2_7, stream, OpenAICompletionsOptions};
use futures::StreamExt;

const DEFAULT_SYSTEM_PROMPT: &str = "You are concise. Follow the user exactly.";
const DEFAULT_PASS_COUNT: usize = 5;
const DEFAULT_MAX_TOKENS: u32 = 64;
const SHARED_PREFIX_LINES: usize = 128;
const SHARED_PREFIX_LINE: &str =
    "Shared provider probe prefix for repeated requests. Preserve this wording exactly.";
const KIMI_PASS_COUNT: usize = 10;
const KIMI_PREFIX_LINES: usize = 512;
const KIMI_PREFIX_LINE: &str =
    "Kimi cache probe stable prefix sentence for repeated requests. Preserve this wording exactly.";
const KIMI_CACHE_KEY: &str = "provider-probe-kimi-prefix-cache";

#[derive(Clone)]
struct ProbeScenario {
    system_prompt: String,
    prompt: String,
    pass_count: usize,
    max_tokens: u32,
    stable_prefix_chars: usize,
}

#[derive(Default)]
struct CacheSummary {
    cache_hit_passes: usize,
    cache_write_passes: usize,
    no_cache_passes: usize,
    total_elapsed_ms: u128,
}

#[tokio::main]
async fn main() -> alchemy_llm::Result<()> {
    let provider = env::args().nth(1).map(|value| value.to_lowercase());

    match provider.as_deref() {
        None | Some("all") => run_all_from_env().await,
        Some(name) => run_named_probe(name).await,
    }
}

async fn run_all_from_env() -> alchemy_llm::Result<()> {
    let mut ran_any = false;

    for provider in ["minimax", "kimi", "openrouter", "chutes"] {
        if is_configured(provider) {
            ran_any = true;
            run_named_probe(provider).await?;
        }
    }

    if ran_any {
        Ok(())
    } else {
        Err(alchemy_llm::Error::InvalidResponse(
            "no supported provider keys found in environment".to_string(),
        ))
    }
}

async fn run_named_probe(provider: &str) -> alchemy_llm::Result<()> {
    match provider {
        "minimax" => {
            run_probe(
                "minimax",
                minimax_m2_7(),
                shared_scenario("minimax ok"),
                None,
            )
            .await
        }
        "kimi" => run_probe("kimi", kimi_k2_5(), kimi_scenario("kimi ok"), None).await,
        "openrouter" => {
            run_probe(
                "openrouter",
                openrouter_model(),
                shared_scenario("openrouter ok"),
                None,
            )
            .await
        }
        "chutes" => {
            run_probe(
                "chutes",
                chutes_model(),
                shared_scenario("chutes ok"),
                Some(OpenAICompletionsOptions {
                    api_key: env::var("CHUTES_API_KEY").ok(),
                    ..Default::default()
                }),
            )
            .await
        }
        other => Err(alchemy_llm::Error::InvalidResponse(format!(
            "unknown provider probe: {other}"
        ))),
    }
}

fn shared_scenario(reply: &str) -> ProbeScenario {
    let stable_prefix = build_shared_prefix();

    ProbeScenario {
        system_prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
        prompt: format!("{stable_prefix}\n\nFinal instruction: Reply with exactly: {reply}"),
        pass_count: DEFAULT_PASS_COUNT,
        max_tokens: DEFAULT_MAX_TOKENS,
        stable_prefix_chars: stable_prefix.len(),
    }
}

fn kimi_scenario(reply: &str) -> ProbeScenario {
    let stable_prefix = build_repeated_prefix(KIMI_PREFIX_LINES, KIMI_PREFIX_LINE);

    ProbeScenario {
        system_prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
        prompt: format!("{stable_prefix}\n\nFinal instruction: Reply with exactly: {reply}"),
        pass_count: KIMI_PASS_COUNT,
        max_tokens: DEFAULT_MAX_TOKENS,
        stable_prefix_chars: stable_prefix.len(),
    }
}

fn build_shared_prefix() -> String {
    build_repeated_prefix(SHARED_PREFIX_LINES, SHARED_PREFIX_LINE)
}

fn build_repeated_prefix(lines: usize, line: &str) -> String {
    (0..lines).map(|_| line).collect::<Vec<_>>().join("\n")
}

fn is_configured(provider: &str) -> bool {
    match provider {
        "minimax" => env::var("MINIMAX_API_KEY").is_ok(),
        "kimi" => env::var("KIMI_API_KEY").is_ok(),
        "openrouter" => env::var("OPENROUTER_API_KEY").is_ok(),
        "chutes" => env::var("CHUTES_API_KEY").is_ok(),
        _ => false,
    }
}

fn zero_cost() -> ModelCost {
    ModelCost {
        input: 0.0,
        output: 0.0,
        cache_read: 0.0,
        cache_write: 0.0,
    }
}

fn openrouter_model() -> Model<OpenAICompletions> {
    Model::<OpenAICompletions> {
        id: "openai/gpt-4o-mini".to_string(),
        name: "OpenRouter GPT-4o Mini".to_string(),
        api: OpenAICompletions,
        provider: Provider::Known(KnownProvider::OpenRouter),
        base_url: "https://openrouter.ai/api/v1/chat/completions".to_string(),
        reasoning: false,
        input: vec![InputType::Text],
        cost: zero_cost(),
        context_window: 128_000,
        max_tokens: 16_384,
        headers: None,
        compat: None,
    }
}

fn chutes_model() -> Model<OpenAICompletions> {
    Model::<OpenAICompletions> {
        id: "openai/gpt-oss-20b-TEE".to_string(),
        name: "Chutes GPT-OSS 20B".to_string(),
        api: OpenAICompletions,
        provider: Provider::Custom("chutes".to_string()),
        base_url: "https://llm.chutes.ai/v1/chat/completions".to_string(),
        reasoning: false,
        input: vec![InputType::Text],
        cost: zero_cost(),
        context_window: 131_072,
        max_tokens: 131_072,
        headers: None,
        compat: None,
    }
}

async fn run_probe<TApi: alchemy_llm::types::ApiType>(
    label: &str,
    model: Model<TApi>,
    scenario: ProbeScenario,
    options: Option<OpenAICompletionsOptions>,
) -> alchemy_llm::Result<()> {
    println!("== provider={label} ==");
    println!("model.name={}", model.name);
    println!("model.id={}", model.id);
    println!("model.base_url={}", model.base_url);
    println!("passes={}", scenario.pass_count);
    println!("prompt_chars={}", scenario.prompt.len());
    println!("stable_prefix_chars={}", scenario.stable_prefix_chars);

    let mut previous_usage: Option<Usage> = None;
    let mut summary = CacheSummary::default();

    for pass in 1..=scenario.pass_count {
        println!("-- pass={pass} --");
        let (usage, elapsed_ms) = run_single_pass(&model, &scenario, options.clone()).await?;
        summary.total_elapsed_ms += elapsed_ms;
        print_cache_status(&usage);
        println!("[timing] elapsed_ms={elapsed_ms}");
        update_cache_summary(&mut summary, &usage);

        if let Some(previous) = &previous_usage {
            println!(
                "[cache-compare] prev_read={} prev_write={} curr_read={} curr_write={}",
                previous.cache_read, previous.cache_write, usage.cache_read, usage.cache_write
            );
        } else {
            println!(
                "[cache-compare] prev_read=n/a prev_write=n/a curr_read={} curr_write={}",
                usage.cache_read, usage.cache_write
            );
        }

        previous_usage = Some(usage);
    }

    print_cache_summary(&summary, scenario.pass_count);
    println!();
    Ok(())
}

async fn run_single_pass<TApi: alchemy_llm::types::ApiType>(
    model: &Model<TApi>,
    scenario: &ProbeScenario,
    options: Option<OpenAICompletionsOptions>,
) -> alchemy_llm::Result<(Usage, u128)> {
    let started_at = Instant::now();
    let context = build_context(scenario);
    let request_options = build_request_options(&model.provider, options, scenario.max_tokens);

    let mut stream = stream(model, &context, request_options)?;

    while let Some(event) = stream.next().await {
        if let Some(result) = handle_stream_event(event) {
            return result.map(|usage| (usage, started_at.elapsed().as_millis()));
        }
    }

    Err(alchemy_llm::Error::InvalidResponse(
        "stream ended without Done event".to_string(),
    ))
}

fn build_context(scenario: &ProbeScenario) -> Context {
    Context {
        system_prompt: Some(scenario.system_prompt.clone()),
        messages: vec![Message::User(UserMessage {
            content: UserContent::Text(scenario.prompt.clone()),
            timestamp: 0,
        })],
        tools: None,
    }
}

fn build_request_options(
    provider: &Provider,
    options: Option<OpenAICompletionsOptions>,
    max_tokens: u32,
) -> Option<OpenAICompletionsOptions> {
    let mut options = options.unwrap_or_default();
    options.max_tokens = Some(max_tokens);

    if matches!(provider, Provider::Known(KnownProvider::Kimi)) {
        options.cache = Some(CacheOptions::new(KIMI_CACHE_KEY));
    }

    Some(options)
}

fn handle_stream_event(event: AssistantMessageEvent) -> Option<alchemy_llm::Result<Usage>> {
    match event {
        AssistantMessageEvent::Start { partial } => {
            print_start_event(&partial);
            None
        }
        AssistantMessageEvent::TextStart { content_index, .. } => {
            println!("[text-start] index={content_index}");
            None
        }
        AssistantMessageEvent::TextDelta { delta, .. } => {
            println!("[text-delta] {delta}");
            None
        }
        AssistantMessageEvent::TextEnd {
            content_index,
            content,
            ..
        } => {
            println!("[text-end] index={content_index} content={content}");
            None
        }
        AssistantMessageEvent::ThinkingStart { content_index, .. } => {
            println!("[thinking-start] index={content_index}");
            None
        }
        AssistantMessageEvent::ThinkingDelta { delta, .. } => {
            println!("[thinking-delta] {delta}");
            None
        }
        AssistantMessageEvent::ThinkingEnd {
            content_index,
            content,
            ..
        } => {
            println!("[thinking-end] index={content_index} content={content}");
            None
        }
        AssistantMessageEvent::ToolCallStart { content_index, .. } => {
            println!("[tool-call-start] index={content_index}");
            None
        }
        AssistantMessageEvent::ToolCallDelta {
            content_index,
            delta,
            ..
        } => {
            println!("[tool-call-delta] index={content_index} delta={delta}");
            None
        }
        AssistantMessageEvent::ToolCallEnd {
            content_index,
            tool_call,
            ..
        } => {
            println!(
                "[tool-call-end] index={} name={} args={}",
                content_index, tool_call.name, tool_call.arguments
            );
            None
        }
        AssistantMessageEvent::Done { message, .. } => Some(Ok(message.usage)),
        AssistantMessageEvent::Error { error, .. } => {
            let message = error
                .error_message
                .unwrap_or_else(|| "stream error".to_string());
            Some(Err(alchemy_llm::Error::InvalidResponse(message)))
        }
    }
}

fn print_start_event(partial: &AssistantMessage) {
    println!(
        "[start] api={:?} provider={} model={}",
        partial.api, partial.provider, partial.model
    );
}

fn print_cache_status(usage: &Usage) {
    println!(
        "[cache-pass] hit={} write={} read_tokens={} write_tokens={}",
        usage.cache_read > 0,
        usage.cache_write > 0,
        usage.cache_read,
        usage.cache_write
    );
}

fn update_cache_summary(summary: &mut CacheSummary, usage: &Usage) {
    if usage.cache_read > 0 {
        summary.cache_hit_passes += 1;
    }

    if usage.cache_write > 0 {
        summary.cache_write_passes += 1;
    }

    if usage.cache_read == 0 && usage.cache_write == 0 {
        summary.no_cache_passes += 1;
    }
}

fn print_cache_summary(summary: &CacheSummary, pass_count: usize) {
    println!("== cache summary ==");
    println!("total_passes={pass_count}");
    println!("cache_hit_passes={}", summary.cache_hit_passes);
    println!("cache_write_passes={}", summary.cache_write_passes);
    println!("no_cache_passes={}", summary.no_cache_passes);
    println!("total_elapsed_ms={}", summary.total_elapsed_ms);
    println!(
        "avg_elapsed_ms={}",
        summary.total_elapsed_ms / pass_count.max(1) as u128
    );
}

#[allow(dead_code)]
fn _assert_model_types() {
    let _: Model<AnthropicMessages> = kimi_k2_5();
    let _: Model<OpenAICompletions> = openrouter_model();
}
