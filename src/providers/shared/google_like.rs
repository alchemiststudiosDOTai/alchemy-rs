use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use serde::Deserialize;
use serde_json::json;

use super::openai_like_runtime::{process_sse_stream, send_streaming_request};
use super::{
    finish_current_block, handle_reasoning_delta, handle_text_delta, handle_tool_calls,
    initialize_output, merge_headers, push_stream_done, push_stream_error, CurrentBlock,
    OpenAiLikeFunctionDelta, OpenAiLikeToolCallDelta, ReasoningDelta,
};
use crate::types::{
    Api, AssistantMessage, AssistantMessageEvent, AssistantMessageEventStream, Content, Context,
    EventStreamSender, GoogleGenerativeAi, InputType, Message, Model, StopReason, Tool,
    ToolResultContent, Usage, UserContent, UserContentBlock,
};

const THINKING_SIGNATURE: &str = "google_thought";

pub(crate) fn stream_google_like(
    model: &Model<GoogleGenerativeAi>,
    context: &Context,
    options: crate::providers::OpenAICompletionsOptions,
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
    options: crate::providers::OpenAICompletionsOptions,
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
    options: &crate::providers::OpenAICompletionsOptions,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
) -> Result<(), crate::Error> {
    let api_key = options
        .api_key
        .as_ref()
        .ok_or_else(|| crate::Error::NoApiKey(model.provider.to_string()))?;

    let url = format!(
        "{}/models/{}:streamGenerateContent?alt=sse&key={}",
        model.base_url, model.id, api_key
    );
    let client = build_client(model, options)?;
    let params = build_params(model, context, options);
    let response = send_streaming_request(&client, &url, &params).await?;

    sender.push(AssistantMessageEvent::Start {
        partial: output.clone(),
    });

    let mut current_block: Option<CurrentBlock> = None;

    process_sse_stream::<StreamChunk, _>(response, |chunk| {
        process_chunk(&chunk, output, sender, &mut current_block);
    })
    .await?;

    finish_current_block(&mut current_block, output, sender);
    push_stream_done(output, sender);
    Ok(())
}

fn build_client(
    model: &Model<GoogleGenerativeAi>,
    options: &crate::providers::OpenAICompletionsOptions,
) -> Result<reqwest::Client, crate::Error> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    merge_headers(&mut headers, model.headers.as_ref());
    merge_headers(&mut headers, options.headers.as_ref());
    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(crate::Error::from)
}

pub(crate) fn build_params(
    model: &Model<GoogleGenerativeAi>,
    context: &Context,
    options: &crate::providers::OpenAICompletionsOptions,
) -> serde_json::Value {
    let mut params = json!({ "contents": convert_contents(model, context) });

    if let Some(sys) = &context.system_prompt {
        params["systemInstruction"] = json!({ "parts": [{ "text": sys }] });
    }

    let mut gen_config = serde_json::Map::new();
    if let Some(max) = options.max_tokens {
        gen_config.insert("maxOutputTokens".into(), json!(max));
    }
    if let Some(t) = options.temperature {
        gen_config.insert("temperature".into(), json!(t));
    }
    if !gen_config.is_empty() {
        params["generationConfig"] = serde_json::Value::Object(gen_config);
    }

    if model.reasoning {
        params["thinkingConfig"] = json!({ "thinkingBudget": 8192 });
    }

    if let Some(tools) = &context.tools {
        params["tools"] = convert_tools(tools);
    }

    if let Some(tool_choice) = &options.tool_choice {
        let v = serde_json::to_value(tool_choice).unwrap_or(json!("auto"));
        let mode = match v.as_str().unwrap_or("auto") {
            "none" => "NONE",
            "required" => "ANY",
            _ => "AUTO",
        };
        params["toolConfig"] = json!({ "functionCallingConfig": { "mode": mode } });
    }

    params
}

fn convert_contents(
    model: &Model<GoogleGenerativeAi>,
    context: &Context,
) -> Vec<serde_json::Value> {
    context
        .messages
        .iter()
        .filter_map(|m| match m {
            Message::User(u) => Some(json!({
                "role": "user",
                "parts": convert_user_parts(model, &u.content),
            })),
            Message::Assistant(a) => {
                let parts = convert_assistant_parts(a);
                (!parts.is_empty()).then(|| json!({ "role": "model", "parts": parts }))
            }
            Message::ToolResult(r) => {
                let text = r
                    .content
                    .iter()
                    .filter_map(|c| match c {
                        ToolResultContent::Text(t) => Some(t.text.clone()),
                        ToolResultContent::Image(_) => None,
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                Some(json!({
                    "role": "user",
                    "parts": [{ "functionResponse": {
                        "name": r.tool_name,
                        "response": { "result": text }
                    }}]
                }))
            }
        })
        .collect()
}

fn convert_user_parts(
    model: &Model<GoogleGenerativeAi>,
    content: &UserContent,
) -> Vec<serde_json::Value> {
    match content {
        UserContent::Text(text) => vec![json!({ "text": text })],
        UserContent::Multi(blocks) => blocks
            .iter()
            .filter_map(|b| match b {
                UserContentBlock::Text(t) => Some(json!({ "text": t.text })),
                UserContentBlock::Image(img) if model.input.contains(&InputType::Image) => Some(
                    json!({ "inlineData": { "mimeType": img.mime_type, "data": img.to_base64() } }),
                ),
                UserContentBlock::Image(_) => None,
            })
            .collect(),
    }
}

fn convert_assistant_parts(assistant: &AssistantMessage) -> Vec<serde_json::Value> {
    assistant
        .content
        .iter()
        .filter_map(|c| match c {
            Content::Thinking { inner } if !inner.thinking.is_empty() => {
                Some(json!({ "thought": true, "text": inner.thinking }))
            }
            Content::Text { inner } if !inner.text.is_empty() => {
                Some(json!({ "text": inner.text }))
            }
            Content::ToolCall { inner } => {
                Some(json!({ "functionCall": { "name": inner.name, "args": inner.arguments } }))
            }
            _ => None,
        })
        .collect()
}

fn convert_tools(tools: &[Tool]) -> serde_json::Value {
    let decls: Vec<_> = tools
        .iter()
        .map(|t| {
            json!({
                "name": t.name,
                "description": t.description,
                "parameters": t.parameters,
            })
        })
        .collect();
    json!([{ "functionDeclarations": decls }])
}

fn process_chunk(
    chunk: &StreamChunk,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
    current_block: &mut Option<CurrentBlock>,
) {
    if let Some(u) = &chunk.usage_metadata {
        let input = u.prompt_token_count.unwrap_or(0);
        let out = u.candidates_token_count.unwrap_or(0);
        let thoughts = u.thoughts_token_count.unwrap_or(0);
        output.usage = Usage {
            input,
            output: out,
            cache_read: u.cached_content_token_count.unwrap_or(0),
            cache_write: 0,
            total_tokens: u.total_token_count.unwrap_or(input + out + thoughts),
            cost: crate::types::Cost::default(),
        };
    }

    let Some(candidate) = chunk.candidates.as_ref().and_then(|c| c.first()) else {
        return;
    };
    if let Some(reason) = &candidate.finish_reason {
        output.stop_reason = map_stop_reason(reason);
    }

    let Some(content) = &candidate.content else {
        return;
    };

    for part in &content.parts {
        if let Some(text) = &part.text {
            if part.thought.unwrap_or(false) {
                handle_reasoning_delta(
                    ReasoningDelta {
                        text,
                        signature: THINKING_SIGNATURE,
                    },
                    output,
                    sender,
                    current_block,
                );
            } else {
                handle_text_delta(text, output, sender, current_block);
            }
        }

        if let Some(fc) = &part.function_call {
            let args_str = fc
                .args
                .as_ref()
                .map(|a| serde_json::to_string(a).unwrap_or_default())
                .unwrap_or_default();
            let tool_deltas = vec![OpenAiLikeToolCallDelta {
                id: Some(format!("call_{}", fc.name)),
                function: Some(OpenAiLikeFunctionDelta {
                    name: Some(fc.name.clone()),
                    arguments: Some(args_str),
                }),
            }];
            handle_tool_calls(&tool_deltas, output, sender, current_block);
            output.stop_reason = StopReason::ToolUse;
        }
    }
}

fn map_stop_reason(reason: &str) -> StopReason {
    match reason {
        "STOP" => StopReason::Stop,
        "MAX_TOKENS" => StopReason::Length,
        "SAFETY" | "RECITATION" | "BLOCKLIST" | "PROHIBITED_CONTENT" | "SPII" => StopReason::Error,
        _ => StopReason::Stop,
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StreamChunk {
    candidates: Option<Vec<Candidate>>,
    usage_metadata: Option<UsageMeta>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Candidate {
    content: Option<CandidateContent>,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CandidateContent {
    parts: Vec<Part>,
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
    args: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UsageMeta {
    prompt_token_count: Option<u32>,
    candidates_token_count: Option<u32>,
    total_token_count: Option<u32>,
    cached_content_token_count: Option<u32>,
    thoughts_token_count: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::OpenAICompletionsOptions;
    use crate::types::{
        AssistantMessageEvent, KnownProvider, ModelCost, Provider, TextContent, ToolResultMessage,
        UserMessage,
    };
    use futures::executor::block_on;
    use futures::StreamExt;

    fn make_model(reasoning: bool) -> Model<GoogleGenerativeAi> {
        Model {
            id: "gemini-2.5-flash".to_string(),
            name: "Gemini 2.5 Flash".to_string(),
            api: GoogleGenerativeAi,
            provider: Provider::Known(KnownProvider::Google),
            base_url: "https://generativelanguage.googleapis.com/v1beta".to_string(),
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
            system_prompt: Some("You are concise".to_string()),
            messages: vec![Message::User(UserMessage {
                content: UserContent::Text("Hello".to_string()),
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
            model: "gemini-2.5-flash".to_string(),
            usage: Usage::default(),
            stop_reason: StopReason::Stop,
            error_message: None,
            timestamp: 0,
        }
    }

    fn run_chunks(chunks: Vec<StreamChunk>) -> (Vec<AssistantMessageEvent>, AssistantMessage) {
        let (mut stream, mut sender) = AssistantMessageEventStream::new();
        let mut output = make_output();
        let mut block = None;
        for c in chunks {
            process_chunk(&c, &mut output, &mut sender, &mut block);
        }
        finish_current_block(&mut block, &mut output, &mut sender);
        drop(sender);
        let events = block_on(async move { stream.by_ref().collect::<Vec<_>>().await });
        (events, output)
    }

    #[test]
    fn build_params_sets_system_instruction() {
        let p = build_params(&make_model(true), &make_context(), &Default::default());
        assert_eq!(
            p["systemInstruction"]["parts"][0]["text"],
            "You are concise"
        );
    }

    #[test]
    fn build_params_uses_user_role_and_parts() {
        let p = build_params(&make_model(false), &make_context(), &Default::default());
        assert_eq!(p["contents"][0]["role"], "user");
        assert_eq!(p["contents"][0]["parts"][0]["text"], "Hello");
    }

    #[test]
    fn build_params_adds_thinking_config_for_reasoning() {
        let p = build_params(&make_model(true), &make_context(), &Default::default());
        assert_eq!(p["thinkingConfig"]["thinkingBudget"], 8192);
    }

    #[test]
    fn build_params_omits_thinking_config_for_non_reasoning() {
        let p = build_params(&make_model(false), &make_context(), &Default::default());
        assert!(p.get("thinkingConfig").is_none());
    }

    #[test]
    fn build_params_includes_generation_config() {
        let p = build_params(
            &make_model(false),
            &make_context(),
            &OpenAICompletionsOptions {
                temperature: Some(0.7),
                max_tokens: Some(1024),
                ..Default::default()
            },
        );
        assert_eq!(p["generationConfig"]["temperature"], 0.7);
        assert_eq!(p["generationConfig"]["maxOutputTokens"], 1024);
    }

    #[test]
    fn build_params_replays_thinking_with_thought_flag() {
        let assistant = AssistantMessage {
            content: vec![Content::thinking("step"), Content::text("answer")],
            ..make_output()
        };
        let ctx = Context {
            system_prompt: None,
            messages: vec![Message::Assistant(assistant)],
            tools: None,
        };
        let p = build_params(&make_model(true), &ctx, &Default::default());
        assert_eq!(p["contents"][0]["role"], "model");
        assert_eq!(p["contents"][0]["parts"][0]["thought"], true);
        assert_eq!(p["contents"][0]["parts"][0]["text"], "step");
        assert_eq!(p["contents"][0]["parts"][1]["text"], "answer");
    }

    #[test]
    fn build_params_converts_tool_result_to_function_response() {
        let ctx = Context {
            system_prompt: None,
            messages: vec![Message::ToolResult(ToolResultMessage {
                tool_call_id: "call_get_weather".into(),
                tool_name: "get_weather".to_string(),
                content: vec![ToolResultContent::Text(TextContent {
                    text: "sunny".to_string(),
                    text_signature: None,
                })],
                details: None,
                is_error: false,
                timestamp: 0,
            })],
            tools: None,
        };
        let p = build_params(&make_model(false), &ctx, &Default::default());
        assert_eq!(
            p["contents"][0]["parts"][0]["functionResponse"]["name"],
            "get_weather"
        );
    }

    #[test]
    fn text_chunk_emits_text_events() {
        let chunk: StreamChunk = serde_json::from_value(json!({
            "candidates": [{ "content": { "parts": [{ "text": "hi" }], "role": "model" } }]
        }))
        .unwrap();
        let (events, output) = run_chunks(vec![chunk]);
        assert!(matches!(events[0], AssistantMessageEvent::TextStart { .. }));
        assert!(matches!(&output.content[0], Content::Text { inner } if inner.text == "hi"));
    }

    #[test]
    fn thought_chunk_emits_thinking_events() {
        let chunk: StreamChunk = serde_json::from_value(json!({
            "candidates": [{ "content": { "parts": [{ "text": "hmm", "thought": true }], "role": "model" } }]
        }))
        .unwrap();
        let (events, output) = run_chunks(vec![chunk]);
        assert!(matches!(
            events[0],
            AssistantMessageEvent::ThinkingStart { .. }
        ));
        assert!(
            matches!(&output.content[0], Content::Thinking { inner } if inner.thinking == "hmm")
        );
    }

    #[test]
    fn function_call_chunk_emits_tool_call() {
        let chunk: StreamChunk = serde_json::from_value(json!({
            "candidates": [{ "content": { "parts": [{
                "functionCall": { "name": "get_weather", "args": { "city": "Tokyo" } }
            }], "role": "model" } }]
        }))
        .unwrap();
        let (_events, output) = run_chunks(vec![chunk]);
        assert_eq!(output.stop_reason, StopReason::ToolUse);
        assert!(
            matches!(&output.content[0], Content::ToolCall { inner } if inner.name == "get_weather")
        );
    }

    #[test]
    fn usage_metadata_updates_output() {
        let chunk: StreamChunk = serde_json::from_value(json!({
            "candidates": [{ "content": { "parts": [{ "text": "hi" }] }, "finishReason": "STOP" }],
            "usageMetadata": { "promptTokenCount": 10, "candidatesTokenCount": 5, "totalTokenCount": 15 }
        }))
        .unwrap();
        let (_, output) = run_chunks(vec![chunk]);
        assert_eq!(output.usage.input, 10);
        assert_eq!(output.usage.output, 5);
        assert_eq!(output.usage.total_tokens, 15);
    }

    #[test]
    fn stop_reasons_map_correctly() {
        let mut output = make_output();
        let mut sender = AssistantMessageEventStream::new().1;
        let mut block = None;

        let chunk: StreamChunk = serde_json::from_value(json!({
            "candidates": [{ "content": { "parts": [{ "text": "x" }] }, "finishReason": "MAX_TOKENS" }]
        }))
        .unwrap();
        process_chunk(&chunk, &mut output, &mut sender, &mut block);
        assert_eq!(output.stop_reason, StopReason::Length);
    }

    #[test]
    fn convert_tools_uses_function_declarations() {
        let tools = vec![Tool::new("calc", "Calculate", json!({"type": "object"}))];
        let result = convert_tools(&tools);
        assert_eq!(result[0]["functionDeclarations"][0]["name"], "calc");
    }
}
