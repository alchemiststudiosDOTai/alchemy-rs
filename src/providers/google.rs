use std::collections::HashMap;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, CONTENT_TYPE};
use serde::Deserialize;
use serde_json::json;

use super::openai_completions::{OpenAICompletionsOptions, ToolChoice};
use super::shared::{
    finish_current_block, handle_reasoning_delta, handle_text_delta, initialize_output,
    process_sse_stream_no_done, push_stream_done, push_stream_error, CurrentBlock, ReasoningDelta,
};
use crate::stream::{AssistantMessageEventStream, EventStreamSender};
use crate::types::{
    Api, AssistantMessage, AssistantMessageEvent, Content, Context, Cost, GoogleGenerativeAi,
    InputType, Message, Model, StopReason, ToolCall, ToolResultContent, Usage, UserContent,
    UserContentBlock,
};

const GOOGLE_API_KEY_HEADER: &str = "x-goog-api-key";
const STREAM_GENERATE_CONTENT_METHOD: &str = "streamGenerateContent";
const SSE_ALT_PARAM: &str = "alt=sse";
const THOUGHT_SIGNATURE: &str = "thought";
const GOOGLE_TOOL_CALL_ID_PREFIX: &str = "google_tc_";

/// Stream completions from Google Generative AI (Gemini) API.
pub fn stream_google_generative_ai(
    model: &Model<GoogleGenerativeAi>,
    context: &Context,
    options: OpenAICompletionsOptions,
) -> AssistantMessageEventStream {
    let (stream, sender) = AssistantMessageEventStream::new();

    let model = model.clone();
    let context = context.clone();

    tokio::spawn(async move {
        run_stream(model, context, options, sender).await;
    });

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

    if let Err(error) = run_stream_inner(&model, &context, &options, &mut output, &mut sender).await
    {
        push_stream_error(&mut output, &mut sender, error);
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

    let client =
        build_google_http_client(api_key, model.headers.as_ref(), options.headers.as_ref())?;
    let params = build_params(model, context, options);
    let url = build_streaming_url(&model.base_url, &model.id);

    let response = client.post(&url).json(&params).send().await?;

    if !response.status().is_success() {
        let status_code = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(crate::Error::ApiError {
            status_code,
            message: body,
        });
    }

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

fn build_google_http_client(
    api_key: &str,
    model_headers: Option<&HashMap<String, String>>,
    extra_headers: Option<&HashMap<String, String>>,
) -> Result<reqwest::Client, crate::Error> {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    headers.insert(
        HeaderName::from_static(GOOGLE_API_KEY_HEADER),
        HeaderValue::from_str(api_key).map_err(|e| crate::Error::InvalidHeader(e.to_string()))?,
    );

    super::shared::merge_headers(&mut headers, model_headers);
    super::shared::merge_headers(&mut headers, extra_headers);

    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .map_err(crate::Error::from)
}

fn build_streaming_url(base_url: &str, model_id: &str) -> String {
    format!("{base_url}/models/{model_id}:{STREAM_GENERATE_CONTENT_METHOD}?{SSE_ALT_PARAM}")
}

fn build_params(
    model: &Model<GoogleGenerativeAi>,
    context: &Context,
    options: &OpenAICompletionsOptions,
) -> serde_json::Value {
    let mut params = json!({
        "contents": convert_google_messages(model, context),
    });

    if let Some(system_prompt) = &context.system_prompt {
        params["systemInstruction"] = json!({
            "parts": [{ "text": system_prompt }]
        });
    }

    let mut generation_config = serde_json::Map::new();

    if let Some(temperature) = options.temperature {
        generation_config.insert("temperature".to_string(), json!(temperature));
    }

    if let Some(max_tokens) = options.max_tokens {
        generation_config.insert("maxOutputTokens".to_string(), json!(max_tokens));
    }

    if !generation_config.is_empty() {
        params["generationConfig"] = serde_json::Value::Object(generation_config);
    }

    if model.reasoning {
        params["thinkingConfig"] = json!({ "includeThoughts": true });
    }

    if let Some(tools) = &context.tools {
        params["tools"] = convert_google_tools(tools);
    }

    if let Some(tool_choice) = &options.tool_choice {
        let mode = match tool_choice {
            ToolChoice::None => "NONE",
            ToolChoice::Required => "ANY",
            ToolChoice::Auto => "AUTO",
            ToolChoice::Function { .. } => "AUTO",
        };
        params["toolConfig"] = json!({
            "functionCallingConfig": { "mode": mode }
        });
    }

    params
}

fn convert_google_messages(
    model: &Model<GoogleGenerativeAi>,
    context: &Context,
) -> serde_json::Value {
    let mut contents: Vec<serde_json::Value> = Vec::new();

    for message in &context.messages {
        match message {
            Message::User(user) => {
                contents.push(convert_google_user_message(model, user));
            }
            Message::Assistant(assistant) => {
                if let Some(converted) = convert_google_assistant_message(assistant) {
                    contents.push(converted);
                }
            }
            Message::ToolResult(result) => {
                contents.push(convert_google_tool_result(result));
            }
        }
    }

    json!(contents)
}

fn convert_google_user_message(
    model: &Model<GoogleGenerativeAi>,
    user: &crate::types::UserMessage,
) -> serde_json::Value {
    let parts = match &user.content {
        UserContent::Text(text) => vec![json!({ "text": text })],
        UserContent::Multi(blocks) => blocks
            .iter()
            .filter_map(|block| match block {
                UserContentBlock::Text(text) => Some(json!({ "text": text.text })),
                UserContentBlock::Image(image) if model.input.contains(&InputType::Image) => {
                    Some(json!({
                        "inlineData": {
                            "mimeType": image.mime_type,
                            "data": image.to_base64()
                        }
                    }))
                }
                UserContentBlock::Image(_) => None,
            })
            .collect(),
    };

    json!({
        "role": "user",
        "parts": parts
    })
}

fn convert_google_assistant_message(assistant: &AssistantMessage) -> Option<serde_json::Value> {
    let mut parts: Vec<serde_json::Value> = Vec::new();

    for content in &assistant.content {
        match content {
            Content::Text { inner } if !inner.text.is_empty() => {
                parts.push(json!({ "text": inner.text }));
            }
            Content::ToolCall { inner } => {
                parts.push(json!({
                    "functionCall": {
                        "name": inner.name,
                        "args": inner.arguments
                    }
                }));
            }
            // Thinking content is omitted on replay — Google manages thinking internally
            Content::Thinking { .. } | Content::Text { .. } | Content::Image { .. } => {}
        }
    }

    if parts.is_empty() {
        return None;
    }

    Some(json!({
        "role": "model",
        "parts": parts
    }))
}

fn convert_google_tool_result(result: &crate::types::ToolResultMessage) -> serde_json::Value {
    let text_content = result
        .content
        .iter()
        .filter_map(|item| match item {
            ToolResultContent::Text(text) => Some(text.text.clone()),
            ToolResultContent::Image(_) => None,
        })
        .collect::<Vec<_>>()
        .join("\n");

    let response = if result.is_error {
        json!({ "error": text_content })
    } else {
        json!({ "result": text_content })
    };

    json!({
        "role": "user",
        "parts": [{
            "functionResponse": {
                "name": result.tool_name,
                "response": response
            }
        }]
    })
}

fn convert_google_tools(tools: &[crate::types::Tool]) -> serde_json::Value {
    let function_declarations: Vec<serde_json::Value> = tools
        .iter()
        .map(|tool| {
            json!({
                "name": tool.name,
                "description": tool.description,
                "parameters": tool.parameters
            })
        })
        .collect();

    json!([{
        "functionDeclarations": function_declarations
    }])
}

// -- Deserialization structs --

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StreamChunk {
    candidates: Option<Vec<Candidate>>,
    usage_metadata: Option<GoogleUsageMetadata>,
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
    function_call: Option<GoogleFunctionCall>,
}

#[derive(Debug, Deserialize)]
struct GoogleFunctionCall {
    name: String,
    args: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GoogleUsageMetadata {
    prompt_token_count: Option<u32>,
    candidates_token_count: Option<u32>,
    total_token_count: Option<u32>,
    #[serde(rename = "thoughtsTokenCount")]
    _thoughts_token_count: Option<u32>,
    cached_content_token_count: Option<u32>,
}

// -- Chunk processing --

fn process_chunk(
    chunk: &StreamChunk,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
    current_block: &mut Option<CurrentBlock>,
    tool_call_index: &mut u32,
) {
    if let Some(usage) = &chunk.usage_metadata {
        update_google_usage(usage, output);
    }

    let Some(candidate) = chunk.candidates.as_ref().and_then(|c| c.first()) else {
        return;
    };

    if let Some(reason) = &candidate.finish_reason {
        output.stop_reason = map_google_stop_reason(reason);
    }

    let Some(content) = &candidate.content else {
        return;
    };

    let Some(parts) = &content.parts else {
        return;
    };

    let mut has_function_call = false;

    for part in parts {
        if let Some(function_call) = &part.function_call {
            has_function_call = true;
            handle_google_tool_call(
                function_call,
                output,
                sender,
                current_block,
                tool_call_index,
            );
            continue;
        }

        if let Some(text) = &part.text {
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

    // If we saw function calls and stop reason wasn't already set to an error,
    // ensure stop_reason is ToolUse
    if has_function_call && output.stop_reason == StopReason::Stop {
        output.stop_reason = StopReason::ToolUse;
    }
}

fn handle_google_tool_call(
    function_call: &GoogleFunctionCall,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
    current_block: &mut Option<CurrentBlock>,
    tool_call_index: &mut u32,
) {
    finish_current_block(current_block, output, sender);

    let id = format!("{GOOGLE_TOOL_CALL_ID_PREFIX}{tool_call_index}");
    *tool_call_index += 1;

    output.content.push(Content::tool_call(
        id.clone(),
        function_call.name.clone(),
        function_call.args.clone(),
    ));

    let content_index = output.content.len() - 1;

    sender.push(AssistantMessageEvent::ToolCallStart {
        content_index,
        partial: output.clone(),
    });

    sender.push(AssistantMessageEvent::ToolCallEnd {
        content_index,
        tool_call: ToolCall {
            id: id.into(),
            name: function_call.name.clone(),
            arguments: function_call.args.clone(),
            thought_signature: None,
        },
        partial: output.clone(),
    });
}

fn update_google_usage(usage: &GoogleUsageMetadata, output: &mut AssistantMessage) {
    let input_tokens = usage.prompt_token_count.unwrap_or(0);
    let output_tokens = usage.candidates_token_count.unwrap_or(0);
    let total_tokens = usage
        .total_token_count
        .unwrap_or(input_tokens + output_tokens);
    let cache_read = usage.cached_content_token_count.unwrap_or(0);

    output.usage = Usage {
        input: input_tokens,
        output: output_tokens,
        cache_read,
        cache_write: 0,
        total_tokens,
        cost: Cost::default(),
    };
}

fn map_google_stop_reason(reason: &str) -> StopReason {
    match reason {
        "STOP" => StopReason::Stop,
        "MAX_TOKENS" => StopReason::Length,
        "SAFETY" | "RECITATION" | "OTHER" => StopReason::Error,
        _ => StopReason::Stop,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        AssistantMessageEvent, InputType, KnownProvider, ModelCost, Provider, StopReason, Tool,
        Usage, UserContent, UserMessage,
    };
    use futures::executor::block_on;
    use futures::StreamExt;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    const TEST_BASE_URL: &str = "https://generativelanguage.googleapis.com/v1beta";

    fn make_model(reasoning: bool) -> Model<GoogleGenerativeAi> {
        Model {
            id: "gemini-2.5-flash-latest".to_string(),
            name: "Gemini 2.5 Flash".to_string(),
            api: GoogleGenerativeAi,
            provider: Provider::Known(KnownProvider::Google),
            base_url: TEST_BASE_URL.to_string(),
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

    fn make_output_message() -> AssistantMessage {
        AssistantMessage {
            content: vec![],
            api: Api::GoogleGenerativeAi,
            provider: Provider::Known(KnownProvider::Google),
            model: "gemini-2.5-flash-latest".to_string(),
            usage: Usage::default(),
            stop_reason: StopReason::Stop,
            error_message: None,
            timestamp: 0,
        }
    }

    fn process_chunks_for_test(
        chunks: Vec<StreamChunk>,
    ) -> (Vec<AssistantMessageEvent>, AssistantMessage) {
        let (mut stream, mut sender) = AssistantMessageEventStream::new();
        let mut output = make_output_message();
        let mut current_block = None;
        let mut tool_call_index: u32 = 0;

        for chunk in chunks {
            process_chunk(
                &chunk,
                &mut output,
                &mut sender,
                &mut current_block,
                &mut tool_call_index,
            );
        }

        finish_current_block(&mut current_block, &mut output, &mut sender);

        drop(sender);

        let events = block_on(async move { stream.by_ref().collect::<Vec<_>>().await });
        (events, output)
    }

    #[test]
    fn build_params_constructs_google_request_format() {
        let model = make_model(false);
        let context = make_context();
        let options = OpenAICompletionsOptions {
            temperature: Some(0.7),
            max_tokens: Some(1024),
            ..OpenAICompletionsOptions::default()
        };

        let params = build_params(&model, &context, &options);

        // systemInstruction
        assert_eq!(
            params["systemInstruction"]["parts"][0]["text"],
            "You are concise"
        );

        // contents
        assert_eq!(params["contents"][0]["role"], "user");
        assert_eq!(params["contents"][0]["parts"][0]["text"], "Hello");

        // generationConfig
        assert_eq!(params["generationConfig"]["temperature"], 0.7);
        assert_eq!(params["generationConfig"]["maxOutputTokens"], 1024);

        // No thinkingConfig for non-reasoning model
        assert!(params.get("thinkingConfig").is_none());
    }

    #[test]
    fn build_params_includes_thinking_config_for_reasoning_model() {
        let model = make_model(true);
        let context = make_context();
        let options = OpenAICompletionsOptions::default();

        let params = build_params(&model, &context, &options);

        assert_eq!(params["thinkingConfig"]["includeThoughts"], true);
    }

    #[test]
    fn build_params_omits_thinking_config_for_non_reasoning_model() {
        let model = make_model(false);
        let context = make_context();
        let options = OpenAICompletionsOptions::default();

        let params = build_params(&model, &context, &options);

        assert!(params.get("thinkingConfig").is_none());
    }

    #[test]
    fn build_params_converts_tools_to_function_declarations() {
        let model = make_model(false);
        let context = Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: UserContent::Text("test".to_string()),
                timestamp: 0,
            })],
            tools: Some(vec![Tool::new(
                "get_weather",
                "Get weather",
                serde_json::json!({
                    "type": "object",
                    "properties": { "city": { "type": "string" } }
                }),
            )]),
        };
        let options = OpenAICompletionsOptions::default();

        let params = build_params(&model, &context, &options);

        let func_decls = &params["tools"][0]["functionDeclarations"];
        assert_eq!(func_decls[0]["name"], "get_weather");
        assert_eq!(func_decls[0]["description"], "Get weather");
        assert_eq!(func_decls[0]["parameters"]["type"], "object");
    }

    #[test]
    fn process_chunk_maps_text_part_to_text_events() {
        let chunk: StreamChunk = serde_json::from_value(json!({
            "candidates": [{
                "content": {
                    "parts": [{ "text": "Hello world" }]
                }
            }]
        }))
        .expect("valid text chunk");

        let (events, output) = process_chunks_for_test(vec![chunk]);

        assert!(matches!(events[0], AssistantMessageEvent::TextStart { .. }));
        assert!(matches!(events[1], AssistantMessageEvent::TextDelta { .. }));
        assert!(matches!(events[2], AssistantMessageEvent::TextEnd { .. }));

        match &output.content[0] {
            Content::Text { inner } => {
                assert_eq!(inner.text, "Hello world");
            }
            _ => panic!("expected text content"),
        }
    }

    #[test]
    fn process_chunk_maps_thought_part_to_thinking_events() {
        let chunk: StreamChunk = serde_json::from_value(json!({
            "candidates": [{
                "content": {
                    "parts": [{ "text": "Let me think...", "thought": true }]
                }
            }]
        }))
        .expect("valid thought chunk");

        let (events, output) = process_chunks_for_test(vec![chunk]);

        assert!(matches!(
            events[0],
            AssistantMessageEvent::ThinkingStart { .. }
        ));
        assert!(matches!(
            events[1],
            AssistantMessageEvent::ThinkingDelta { .. }
        ));
        assert!(matches!(
            events[2],
            AssistantMessageEvent::ThinkingEnd { .. }
        ));

        match &output.content[0] {
            Content::Thinking { inner } => {
                assert_eq!(inner.thinking, "Let me think...");
                assert_eq!(inner.thinking_signature.as_deref(), Some(THOUGHT_SIGNATURE));
            }
            _ => panic!("expected thinking content"),
        }
    }

    #[test]
    fn process_chunk_maps_function_call_to_tool_call_events() {
        let chunk: StreamChunk = serde_json::from_value(json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "functionCall": {
                            "name": "get_weather",
                            "args": { "city": "Tokyo" }
                        }
                    }]
                }
            }]
        }))
        .expect("valid function call chunk");

        let (events, output) = process_chunks_for_test(vec![chunk]);

        assert!(matches!(
            events[0],
            AssistantMessageEvent::ToolCallStart { .. }
        ));
        assert!(matches!(
            events[1],
            AssistantMessageEvent::ToolCallEnd { .. }
        ));

        assert_eq!(output.stop_reason, StopReason::ToolUse);

        match &output.content[0] {
            Content::ToolCall { inner } => {
                assert_eq!(inner.id.as_str(), "google_tc_0");
                assert_eq!(inner.name, "get_weather");
                assert_eq!(inner.arguments, json!({ "city": "Tokyo" }));
            }
            _ => panic!("expected tool call content"),
        }
    }

    #[test]
    fn process_chunk_maps_google_finish_reasons() {
        assert_eq!(map_google_stop_reason("STOP"), StopReason::Stop);
        assert_eq!(map_google_stop_reason("MAX_TOKENS"), StopReason::Length);
        assert_eq!(map_google_stop_reason("SAFETY"), StopReason::Error);
        assert_eq!(map_google_stop_reason("RECITATION"), StopReason::Error);
        assert_eq!(map_google_stop_reason("OTHER"), StopReason::Error);
        assert_eq!(map_google_stop_reason("unknown"), StopReason::Stop);
    }

    #[test]
    fn update_google_usage_maps_fields_correctly() {
        let usage = GoogleUsageMetadata {
            prompt_token_count: Some(100),
            candidates_token_count: Some(50),
            total_token_count: Some(160),
            _thoughts_token_count: Some(10),
            cached_content_token_count: Some(20),
        };

        let mut output = make_output_message();
        update_google_usage(&usage, &mut output);

        assert_eq!(output.usage.input, 100);
        assert_eq!(output.usage.output, 50);
        assert_eq!(output.usage.total_tokens, 160);
        assert_eq!(output.usage.cache_read, 20);
    }

    #[test]
    fn build_streaming_url_constructs_correct_path() {
        let url = build_streaming_url(TEST_BASE_URL, "gemini-2.5-flash-latest");
        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/models/gemini-2.5-flash-latest:streamGenerateContent?alt=sse"
        );
    }

    #[test]
    fn convert_google_assistant_omits_thinking_on_replay() {
        let assistant = AssistantMessage {
            content: vec![
                Content::thinking("internal reasoning"),
                Content::text("visible answer"),
            ],
            api: Api::GoogleGenerativeAi,
            provider: Provider::Known(KnownProvider::Google),
            model: "gemini-2.5-flash-latest".to_string(),
            usage: Usage::default(),
            stop_reason: StopReason::Stop,
            error_message: None,
            timestamp: 0,
        };

        let converted = convert_google_assistant_message(&assistant).expect("should convert");
        let parts = converted["parts"]
            .as_array()
            .expect("parts should be array");

        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0]["text"], "visible answer");
    }

    async fn spawn_sse_server(body: String) -> String {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test listener");
        let address = listener.local_addr().expect("listener address");

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept request");

            let mut request_buffer = [0_u8; 4096];
            let _ = socket.read(&mut request_buffer).await;

            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body,
            );

            socket
                .write_all(response.as_bytes())
                .await
                .expect("write response");
        });

        format!("http://{}", address)
    }

    #[tokio::test]
    async fn stream_google_generative_ai_final_message_shape() {
        let sse_body = concat!(
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"thinking...\",\"thought\":true}]}}]}\n\n",
            "data: {\"candidates\":[{\"content\":{\"parts\":[{\"text\":\"answer\"}]},\"finishReason\":\"STOP\"}]}\n\n",
            "data: {\"usageMetadata\":{\"promptTokenCount\":10,\"candidatesTokenCount\":5,\"totalTokenCount\":15}}\n\n"
        )
        .to_string();

        let server_url = spawn_sse_server(sse_body).await;

        let mut model = make_model(true);
        model.base_url = server_url;

        let options = OpenAICompletionsOptions {
            api_key: Some("test-key".to_string()),
            ..OpenAICompletionsOptions::default()
        };

        let stream = stream_google_generative_ai(&model, &make_context(), options);
        let result = stream.result().await.expect("stream result");

        assert_eq!(result.api, Api::GoogleGenerativeAi);
        assert_eq!(result.provider, Provider::Known(KnownProvider::Google));
        assert_eq!(result.model, "gemini-2.5-flash-latest");
        assert_eq!(result.stop_reason, StopReason::Stop);
        assert_eq!(result.usage.total_tokens, 15);
    }
}
