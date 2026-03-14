use serde::Deserialize;
use serde_json::json;

use super::openai_completions::OpenAICompletionsOptions;
use super::shared::{
    finish_current_block, handle_reasoning_delta, handle_text_delta, initialize_output,
    merge_headers, process_sse_stream_no_done, push_stream_done, push_stream_error,
    send_streaming_request, CurrentBlock, ReasoningDelta,
};
use crate::types::{
    Api, AssistantMessage, AssistantMessageEvent, AssistantMessageEventStream, Content, Context,
    Cost, EventStreamSender, GoogleGenerativeAi, InputType, Message, Model, StopReason,
    ToolResultContent, Usage, UserContent, UserContentBlock,
};

const GOOGLE_API_KEY_HEADER: &str = "x-goog-api-key";
const STREAM_GENERATE_CONTENT: &str = "streamGenerateContent";
const SSE_ALT_PARAM: &str = "alt=sse";
const THOUGHT_SIGNATURE: &str = "thought";
const TOOL_CALL_ID_PREFIX: &str = "google_tc_";

pub fn stream_google_generative_ai(
    model: &Model<GoogleGenerativeAi>,
    context: &Context,
    options: OpenAICompletionsOptions,
) -> AssistantMessageEventStream {
    let (stream, sender) = AssistantMessageEventStream::new();
    let model = model.clone();
    let context = context.clone();
    tokio::spawn(async move { run_stream(model, context, options, sender).await });
    stream
}

async fn run_stream(
    model: Model<GoogleGenerativeAi>,
    context: Context,
    options: OpenAICompletionsOptions,
    mut sender: EventStreamSender,
) {
    let mut output = initialize_output(
        Api::GoogleGenerativeAi,
        model.provider.clone(),
        model.id.clone(),
    );
    if let Err(e) = run_stream_inner(&model, &context, &options, &mut output, &mut sender).await {
        push_stream_error(&mut output, &mut sender, e);
    }
}

async fn run_stream_inner(
    model: &Model<GoogleGenerativeAi>,
    context: &Context,
    options: &OpenAICompletionsOptions,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
) -> Result<(), crate::Error> {
    let api_key = options
        .api_key
        .as_ref()
        .ok_or_else(|| crate::Error::NoApiKey(model.provider.to_string()))?;

    let client = build_client(api_key, model, options)?;
    let url = format!(
        "{}/models/{}:{STREAM_GENERATE_CONTENT}?{SSE_ALT_PARAM}",
        model.base_url, model.id
    );
    let params = build_params(model, context, options);
    let response = send_streaming_request(&client, &url, &params).await?;

    sender.push(AssistantMessageEvent::Start {
        partial: output.clone(),
    });

    let mut current_block: Option<CurrentBlock> = None;
    let mut tool_call_index: u32 = 0;

    process_sse_stream_no_done::<StreamChunk, _>(response, |chunk| {
        process_chunk(
            &chunk,
            output,
            sender,
            &mut current_block,
            &mut tool_call_index,
        );
    })
    .await?;

    finish_current_block(&mut current_block, output, sender);
    push_stream_done(output, sender);
    Ok(())
}

fn build_client(
    api_key: &str,
    model: &Model<GoogleGenerativeAi>,
    options: &OpenAICompletionsOptions,
) -> Result<reqwest::Client, crate::Error> {
    use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};

    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        HeaderName::from_static(GOOGLE_API_KEY_HEADER),
        HeaderValue::from_str(api_key).map_err(|e| crate::Error::InvalidHeader(e.to_string()))?,
    );

    merge_headers(&mut headers, model.headers.as_ref());
    merge_headers(&mut headers, options.headers.as_ref());

    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(crate::Error::from)
}

fn build_params(
    model: &Model<GoogleGenerativeAi>,
    context: &Context,
    options: &OpenAICompletionsOptions,
) -> serde_json::Value {
    let mut params = json!({ "contents": convert_messages(model, context) });

    if let Some(sys) = &context.system_prompt {
        params["systemInstruction"] = json!({ "parts": [{ "text": sys }] });
    }

    let mut gen_config = serde_json::Map::new();
    if let Some(t) = options.temperature {
        gen_config.insert("temperature".into(), json!(t));
    }
    if let Some(max) = options.max_tokens {
        gen_config.insert("maxOutputTokens".into(), json!(max));
    }
    if model.reasoning {
        gen_config.insert("thinkingConfig".into(), json!({ "includeThoughts": true }));
    }
    if !gen_config.is_empty() {
        params["generationConfig"] = serde_json::Value::Object(gen_config);
    }

    if let Some(tools) = &context.tools {
        let decls: Vec<_> = tools
            .iter()
            .map(|t| {
                json!({
                    "name": t.name, "description": t.description, "parameters": t.parameters,
                })
            })
            .collect();
        params["tools"] = json!([{ "functionDeclarations": decls }]);
    }

    if let Some(tc) = &options.tool_choice {
        use super::openai_completions::ToolChoice;
        match tc {
            ToolChoice::Function { name } => {
                params["toolConfig"] = json!({
                    "functionCallingConfig": {
                        "mode": "ANY",
                        "allowedFunctionNames": [name],
                    }
                });
            }
            _ => {
                let mode = match tc {
                    ToolChoice::None => "NONE",
                    ToolChoice::Required => "ANY",
                    ToolChoice::Auto => "AUTO",
                    ToolChoice::Function { .. } => unreachable!(),
                };
                params["toolConfig"] = json!({ "functionCallingConfig": { "mode": mode } });
            }
        }
    }

    params
}

fn convert_messages(
    model: &Model<GoogleGenerativeAi>,
    context: &Context,
) -> Vec<serde_json::Value> {
    context
        .messages
        .iter()
        .filter_map(|m| match m {
            Message::User(u) => Some(convert_user_message(model, u)),
            Message::Assistant(a) => convert_assistant_message(a),
            Message::ToolResult(r) => Some(convert_tool_result(r)),
        })
        .collect()
}

fn convert_user_message(
    model: &Model<GoogleGenerativeAi>,
    user: &crate::types::UserMessage,
) -> serde_json::Value {
    let parts = match &user.content {
        UserContent::Text(text) => vec![json!({ "text": text })],
        UserContent::Multi(blocks) => blocks
            .iter()
            .filter_map(|b| match b {
                UserContentBlock::Text(t) => Some(json!({ "text": t.text })),
                UserContentBlock::Image(img) if model.input.contains(&InputType::Image) => {
                    Some(json!({
                        "inlineData": { "mimeType": img.mime_type, "data": img.to_base64() }
                    }))
                }
                UserContentBlock::Image(_) => None,
            })
            .collect(),
    };
    json!({ "role": "user", "parts": parts })
}

fn convert_assistant_message(assistant: &AssistantMessage) -> Option<serde_json::Value> {
    let parts: Vec<_> = assistant
        .content
        .iter()
        .filter_map(|c| match c {
            Content::Thinking { inner } if !inner.thinking.is_empty() => {
                Some(json!({ "text": inner.thinking, "thought": true }))
            }
            Content::Text { inner } if !inner.text.is_empty() => {
                Some(json!({ "text": inner.text }))
            }
            Content::ToolCall { inner } => {
                Some(json!({ "functionCall": { "name": inner.name, "args": inner.arguments } }))
            }
            _ => None,
        })
        .collect();
    (!parts.is_empty()).then(|| json!({ "role": "model", "parts": parts }))
}

fn convert_tool_result(result: &crate::types::ToolResultMessage) -> serde_json::Value {
    let text = result
        .content
        .iter()
        .filter_map(|c| match c {
            ToolResultContent::Text(t) => Some(t.text.clone()),
            ToolResultContent::Image(_) => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    let response = if result.is_error {
        json!({ "error": text })
    } else {
        json!({ "result": text })
    };
    json!({ "role": "user", "parts": [{ "functionResponse": { "name": result.tool_name, "response": response } }] })
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StreamChunk {
    candidates: Option<Vec<Candidate>>,
    usage_metadata: Option<UsageMetadata>,
    prompt_feedback: Option<PromptFeedback>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PromptFeedback {
    block_reason: Option<String>,
    safety_ratings: Option<Vec<SafetyRating>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SafetyRating {
    category: Option<String>,
    probability: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Candidate {
    content: Option<CandidateContent>,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CandidateContent {
    parts: Option<Vec<Part>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Part {
    text: Option<String>,
    thought: Option<bool>,
    function_call: Option<FunctionCall>,
}

#[derive(Debug, Deserialize)]
struct FunctionCall {
    name: String,
    args: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageMetadata {
    prompt_token_count: Option<u32>,
    candidates_token_count: Option<u32>,
    total_token_count: Option<u32>,
    cached_content_token_count: Option<u32>,
}

fn process_chunk(
    chunk: &StreamChunk,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
    current_block: &mut Option<CurrentBlock>,
    tool_call_index: &mut u32,
) {
    if let Some(usage) = &chunk.usage_metadata {
        let input = usage.prompt_token_count.unwrap_or(0);
        let out = usage.candidates_token_count.unwrap_or(0);
        output.usage = Usage {
            input,
            output: out,
            cache_read: usage.cached_content_token_count.unwrap_or(0),
            cache_write: 0,
            total_tokens: usage.total_token_count.unwrap_or(input + out),
            cost: Cost::default(),
        };
    }

    if handle_prompt_feedback(chunk, output) {
        return;
    }

    let Some(candidate) = chunk.candidates.as_ref().and_then(|c| c.first()) else {
        return;
    };

    if let Some(reason) = &candidate.finish_reason {
        output.stop_reason = map_stop_reason(reason);
    }

    let Some(parts) = candidate.content.as_ref().and_then(|c| c.parts.as_ref()) else {
        return;
    };

    let mut saw_function_call = false;
    for part in parts {
        if let Some(fc) = &part.function_call {
            saw_function_call = true;
            finish_current_block(current_block, output, sender);
            let id = format!("{TOOL_CALL_ID_PREFIX}{tool_call_index}");
            *tool_call_index += 1;
            output.content.push(Content::tool_call(
                id.clone(),
                fc.name.clone(),
                fc.args.clone(),
            ));
            let content_index = output.content.len() - 1;
            sender.push(AssistantMessageEvent::ToolCallStart {
                content_index,
                partial: output.clone(),
            });
            sender.push(AssistantMessageEvent::ToolCallEnd {
                content_index,
                tool_call: crate::types::ToolCall {
                    id: id.into(),
                    name: fc.name.clone(),
                    arguments: fc.args.clone(),
                    thought_signature: None,
                },
                partial: output.clone(),
            });
        } else if let Some(text) = &part.text {
            if part.thought == Some(true) {
                handle_reasoning_delta(
                    ReasoningDelta {
                        text,
                        signature: THOUGHT_SIGNATURE,
                    },
                    output,
                    sender,
                    current_block,
                );
            } else {
                handle_text_delta(text, output, sender, current_block);
            }
        }
    }

    if saw_function_call && output.stop_reason == StopReason::Stop {
        output.stop_reason = StopReason::ToolUse;
    }
}

fn handle_prompt_feedback(chunk: &StreamChunk, output: &mut AssistantMessage) -> bool {
    let Some(feedback) = &chunk.prompt_feedback else {
        return false;
    };
    let Some(reason) = &feedback.block_reason else {
        return false;
    };
    output.stop_reason = StopReason::Error;
    let ratings_info = feedback
        .safety_ratings
        .as_ref()
        .map(|ratings| {
            ratings
                .iter()
                .map(|r| {
                    format!(
                        "{}:{}",
                        r.category.as_deref().unwrap_or("unknown"),
                        r.probability.as_deref().unwrap_or("unknown")
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    output.error_message = Some(if ratings_info.is_empty() {
        format!("Prompt blocked: {reason}")
    } else {
        format!("Prompt blocked: {reason} (safety ratings: {ratings_info})")
    });
    true
}

fn map_stop_reason(reason: &str) -> StopReason {
    match reason {
        "STOP" => StopReason::Stop,
        "MAX_TOKENS" => StopReason::Length,
        "SAFETY"
        | "RECITATION"
        | "OTHER"
        | "BLOCKLIST"
        | "PROHIBITED_CONTENT"
        | "SPII"
        | "MALFORMED_FUNCTION_CALL"
        | "UNEXPECTED_TOOL_CALL"
        | "TOO_MANY_TOOL_CALLS"
        | "MISSING_THOUGHT_SIGNATURE" => StopReason::Error,
        _ => StopReason::Error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        AssistantMessageEvent, KnownProvider, ModelCost, Provider, Tool, UserMessage,
    };
    use futures::executor::block_on;
    use futures::StreamExt;

    fn make_model(reasoning: bool) -> Model<GoogleGenerativeAi> {
        Model {
            id: "gemini-2.5-flash".into(),
            name: "Gemini 2.5 Flash".into(),
            api: GoogleGenerativeAi,
            provider: Provider::Known(KnownProvider::Google),
            base_url: "https://generativelanguage.googleapis.com/v1beta".into(),
            reasoning,
            input: vec![InputType::Text, InputType::Image],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 1_048_576,
            max_tokens: 65_536,
            headers: None,
            compat: None,
        }
    }

    fn make_context() -> Context {
        Context {
            system_prompt: Some("You are concise".into()),
            messages: vec![Message::User(UserMessage {
                content: UserContent::Text("Hello".into()),
                timestamp: 0,
            })],
            tools: None,
        }
    }

    fn make_output() -> AssistantMessage {
        AssistantMessage {
            content: vec![],
            api: Api::GoogleGenerativeAi,
            provider: Provider::Known(KnownProvider::Google),
            model: "gemini-2.5-flash".into(),
            usage: Usage::default(),
            stop_reason: StopReason::Stop,
            error_message: None,
            timestamp: 0,
        }
    }

    fn process_chunks(chunks: Vec<StreamChunk>) -> (Vec<AssistantMessageEvent>, AssistantMessage) {
        let (mut stream, mut sender) = AssistantMessageEventStream::new();
        let mut output = make_output();
        let mut current_block = None;
        let mut tool_call_index: u32 = 0;
        for chunk in &chunks {
            process_chunk(
                chunk,
                &mut output,
                &mut sender,
                &mut current_block,
                &mut tool_call_index,
            );
        }
        finish_current_block(&mut current_block, &mut output, &mut sender);
        drop(sender);
        let events = block_on(async { stream.by_ref().collect::<Vec<_>>().await });
        (events, output)
    }

    #[test]
    fn build_params_basic_format() {
        let params = build_params(
            &make_model(false),
            &make_context(),
            &OpenAICompletionsOptions {
                temperature: Some(0.7),
                max_tokens: Some(1024),
                ..Default::default()
            },
        );
        assert_eq!(
            params["systemInstruction"]["parts"][0]["text"],
            "You are concise"
        );
        assert_eq!(params["contents"][0]["role"], "user");
        assert_eq!(params["generationConfig"]["temperature"], 0.7);
        assert_eq!(params["generationConfig"]["maxOutputTokens"], 1024);
    }

    #[test]
    fn build_params_thinking_config() {
        let params = build_params(&make_model(true), &make_context(), &Default::default());
        assert_eq!(
            params["generationConfig"]["thinkingConfig"]["includeThoughts"],
            true
        );
    }

    #[test]
    fn build_params_no_thinking_for_non_reasoning() {
        let params = build_params(&make_model(false), &make_context(), &Default::default());
        assert!(params
            .get("generationConfig")
            .and_then(|gc| gc.get("thinkingConfig"))
            .is_none());
    }

    #[test]
    fn build_params_tools() {
        let ctx = Context {
            system_prompt: None,
            tools: Some(vec![Tool::new(
                "get_weather",
                "Get weather",
                json!({"type": "object"}),
            )]),
            messages: vec![Message::User(UserMessage {
                content: UserContent::Text("t".into()),
                timestamp: 0,
            })],
        };
        let params = build_params(&make_model(false), &ctx, &Default::default());
        assert_eq!(
            params["tools"][0]["functionDeclarations"][0]["name"],
            "get_weather"
        );
    }

    #[test]
    fn text_chunk_emits_text_events() {
        let chunk: StreamChunk = serde_json::from_value(json!({
            "candidates": [{ "content": { "parts": [{ "text": "Hello world" }] } }]
        }))
        .unwrap();
        let (events, output) = process_chunks(vec![chunk]);
        assert!(matches!(events[0], AssistantMessageEvent::TextStart { .. }));
        assert!(
            matches!(&output.content[0], Content::Text { inner } if inner.text == "Hello world")
        );
    }

    #[test]
    fn thought_chunk_emits_thinking_events() {
        let chunk: StreamChunk = serde_json::from_value(json!({
            "candidates": [{ "content": { "parts": [{ "text": "Let me think...", "thought": true }] } }]
        })).unwrap();
        let (events, output) = process_chunks(vec![chunk]);
        assert!(matches!(
            events[0],
            AssistantMessageEvent::ThinkingStart { .. }
        ));
        assert!(
            matches!(&output.content[0], Content::Thinking { inner } if inner.thinking == "Let me think...")
        );
    }

    #[test]
    fn function_call_emits_tool_call_events() {
        let chunk: StreamChunk = serde_json::from_value(json!({
            "candidates": [{ "content": { "parts": [{ "functionCall": { "name": "get_weather", "args": { "city": "Tokyo" } } }] } }]
        })).unwrap();
        let (events, output) = process_chunks(vec![chunk]);
        assert!(matches!(
            events[0],
            AssistantMessageEvent::ToolCallStart { .. }
        ));
        assert!(matches!(
            events[1],
            AssistantMessageEvent::ToolCallEnd { .. }
        ));
        assert_eq!(output.stop_reason, StopReason::ToolUse);
        assert!(
            matches!(&output.content[0], Content::ToolCall { inner } if inner.name == "get_weather")
        );
    }

    #[test]
    fn stop_reason_mapping() {
        assert_eq!(map_stop_reason("STOP"), StopReason::Stop);
        assert_eq!(map_stop_reason("MAX_TOKENS"), StopReason::Length);
        assert_eq!(map_stop_reason("SAFETY"), StopReason::Error);
        assert_eq!(map_stop_reason("RECITATION"), StopReason::Error);
    }

    #[test]
    fn usage_metadata_maps_correctly() {
        let chunk: StreamChunk = serde_json::from_value(json!({
            "usageMetadata": { "promptTokenCount": 100, "candidatesTokenCount": 50, "totalTokenCount": 160, "cachedContentTokenCount": 20 }
        })).unwrap();
        let (_, output) = process_chunks(vec![chunk]);
        assert_eq!(output.usage.input, 100);
        assert_eq!(output.usage.output, 50);
        assert_eq!(output.usage.total_tokens, 160);
        assert_eq!(output.usage.cache_read, 20);
    }

    #[test]
    fn assistant_replay_preserves_thinking() {
        let assistant = AssistantMessage {
            content: vec![Content::thinking("reasoning"), Content::text("answer")],
            api: Api::GoogleGenerativeAi,
            provider: Provider::Known(KnownProvider::Google),
            model: "gemini-2.5-flash".into(),
            usage: Usage::default(),
            stop_reason: StopReason::Stop,
            error_message: None,
            timestamp: 0,
        };
        let converted = convert_assistant_message(&assistant).unwrap();
        let parts = converted["parts"].as_array().unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0]["text"], "reasoning");
        assert_eq!(parts[0]["thought"], true);
        assert_eq!(parts[1]["text"], "answer");
        assert!(parts[1].get("thought").is_none());
    }
}
