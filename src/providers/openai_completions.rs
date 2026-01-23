use std::collections::HashMap;

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::shared::{build_http_client, unix_timestamp_millis};
use crate::stream::{AssistantMessageEventStream, EventStreamSender};
use crate::types::{
    Api, AssistantMessage, AssistantMessageEvent, Content, Context, KnownProvider, MaxTokensField,
    Model, OpenAICompletions, Provider, StopReason, StopReasonError, StopReasonSuccess,
    ThinkingFormat, Tool, ToolCall, Usage,
};

/// Options for OpenAI completions streaming.
#[derive(Debug, Clone, Default)]
pub struct OpenAICompletionsOptions {
    pub api_key: Option<String>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub tool_choice: Option<ToolChoice>,
    pub reasoning_effort: Option<ReasoningEffort>,
    pub headers: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolChoice {
    Auto,
    None,
    Required,
    #[serde(rename = "function")]
    Function {
        name: String,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
}

/// Resolved compatibility settings with all fields set.
#[derive(Debug, Clone)]
struct ResolvedCompat {
    supports_store: bool,
    supports_developer_role: bool,
    supports_reasoning_effort: bool,
    supports_usage_in_streaming: bool,
    max_tokens_field: MaxTokensField,
    requires_tool_result_name: bool,
    requires_assistant_after_tool_result: bool,
    requires_thinking_as_text: bool,
    requires_mistral_tool_ids: bool,
    thinking_format: ThinkingFormat,
}

/// Stream completions from an OpenAI-compatible API.
pub fn stream_openai_completions(
    model: &Model<OpenAICompletions>,
    context: &Context,
    options: OpenAICompletionsOptions,
) -> AssistantMessageEventStream {
    let (stream, sender) = AssistantMessageEventStream::new();

    // Clone what we need for the async task
    let model = model.clone();
    let context = context.clone();

    tokio::spawn(async move {
        run_stream(model, context, options, sender).await;
    });

    stream
}

async fn run_stream(
    model: Model<OpenAICompletions>,
    context: Context,
    options: OpenAICompletionsOptions,
    mut sender: EventStreamSender,
) {
    let mut output = AssistantMessage {
        content: vec![],
        api: Api::OpenAICompletions,
        provider: model.provider.clone(),
        model: model.id.clone(),
        usage: Usage::default(),
        stop_reason: StopReason::Stop,
        error_message: None,
        timestamp: unix_timestamp_millis(),
    };

    let result = run_stream_inner(&model, &context, &options, &mut output, &mut sender).await;

    if let Err(e) = result {
        output.stop_reason = StopReason::Error;
        output.error_message = Some(e.to_string());
        sender.push(AssistantMessageEvent::Error {
            reason: StopReasonError::Error,
            error: output,
        });
    }
}

async fn run_stream_inner(
    model: &Model<OpenAICompletions>,
    context: &Context,
    options: &OpenAICompletionsOptions,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
) -> Result<(), crate::Error> {
    let api_key = options
        .api_key
        .as_ref()
        .ok_or_else(|| crate::Error::NoApiKey(model.provider.to_string()))?;

    let compat = resolve_compat(model);
    let client = build_http_client(api_key, model.headers.as_ref(), options.headers.as_ref())?;
    let params = build_params(model, context, options, &compat);

    let response = client.post(&model.base_url).json(&params).send().await?;

    if !response.status().is_success() {
        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default();
        return Err(crate::Error::ApiError {
            status_code: status,
            message: body,
        });
    }

    sender.push(AssistantMessageEvent::Start {
        partial: output.clone(),
    });

    // Track current content block state
    let mut current_block: Option<CurrentBlock> = None;

    // Process SSE stream
    let mut stream = response.bytes_stream();
    let mut buffer = String::new();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        buffer.push_str(&String::from_utf8_lossy(&chunk));

        // Process complete SSE lines
        while let Some(line_end) = buffer.find('\n') {
            let line = buffer[..line_end].trim().to_string();
            buffer = buffer[line_end + 1..].to_string();

            if line.is_empty() || line.starts_with(':') {
                continue;
            }

            if let Some(data) = line.strip_prefix("data: ") {
                if data == "[DONE]" {
                    break;
                }

                if let Ok(chunk) = serde_json::from_str::<StreamChunk>(data) {
                    process_chunk(&chunk, output, sender, &mut current_block);
                }
            }
        }
    }

    // Finish any pending block
    finish_current_block(&mut current_block, output, sender);

    // Send done event
    sender.push(AssistantMessageEvent::Done {
        reason: match output.stop_reason {
            StopReason::Stop => StopReasonSuccess::Stop,
            StopReason::Length => StopReasonSuccess::Length,
            StopReason::ToolUse => StopReasonSuccess::ToolUse,
            _ => StopReasonSuccess::Stop,
        },
        message: output.clone(),
    });

    Ok(())
}

#[derive(Debug)]
enum CurrentBlock {
    Text {
        text: String,
    },
    Thinking {
        thinking: String,
        signature: String,
    },
    ToolCall {
        id: String,
        name: String,
        partial_args: String,
    },
}

fn process_chunk(
    chunk: &StreamChunk,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
    current_block: &mut Option<CurrentBlock>,
) {
    // Handle usage
    if let Some(usage) = &chunk.usage {
        let cached_tokens = usage
            .prompt_tokens_details
            .as_ref()
            .and_then(|d| d.cached_tokens)
            .unwrap_or(0);
        let reasoning_tokens = usage
            .completion_tokens_details
            .as_ref()
            .and_then(|d| d.reasoning_tokens)
            .unwrap_or(0);
        let input = usage.prompt_tokens.saturating_sub(cached_tokens);
        let output_tokens = usage.completion_tokens + reasoning_tokens;

        output.usage = Usage {
            input,
            output: output_tokens,
            cache_read: cached_tokens,
            cache_write: 0,
            total_tokens: input + output_tokens + cached_tokens,
            ..Default::default()
        };
    }

    // Process choice
    let Some(choice) = chunk.choices.first() else {
        return;
    };

    // Update stop reason
    if let Some(reason) = &choice.finish_reason {
        output.stop_reason = map_stop_reason(reason);
    }

    let Some(delta) = &choice.delta else {
        return;
    };

    // Handle text content
    if let Some(content) = &delta.content {
        if !content.is_empty() {
            match current_block {
                Some(CurrentBlock::Text { text }) => {
                    text.push_str(content);
                    sender.push(AssistantMessageEvent::TextDelta {
                        content_index: output.content.len().saturating_sub(1),
                        delta: content.clone(),
                        partial: output.clone(),
                    });
                }
                _ => {
                    finish_current_block(current_block, output, sender);
                    *current_block = Some(CurrentBlock::Text {
                        text: content.clone(),
                    });
                    output.content.push(Content::text(""));
                    sender.push(AssistantMessageEvent::TextStart {
                        content_index: output.content.len() - 1,
                        partial: output.clone(),
                    });
                    sender.push(AssistantMessageEvent::TextDelta {
                        content_index: output.content.len() - 1,
                        delta: content.clone(),
                        partial: output.clone(),
                    });
                }
            }
        }
    }

    // Handle reasoning content (various field names)
    let reasoning = delta
        .reasoning_content
        .as_ref()
        .or(delta.reasoning.as_ref())
        .or(delta.reasoning_text.as_ref());

    if let Some(reasoning_text) = reasoning {
        if !reasoning_text.is_empty() {
            let signature = if delta.reasoning_content.is_some() {
                "reasoning_content"
            } else if delta.reasoning.is_some() {
                "reasoning"
            } else {
                "reasoning_text"
            };

            match current_block {
                Some(CurrentBlock::Thinking { thinking, .. }) => {
                    thinking.push_str(reasoning_text);
                    sender.push(AssistantMessageEvent::ThinkingDelta {
                        content_index: output.content.len().saturating_sub(1),
                        delta: reasoning_text.clone(),
                        partial: output.clone(),
                    });
                }
                _ => {
                    finish_current_block(current_block, output, sender);
                    *current_block = Some(CurrentBlock::Thinking {
                        thinking: reasoning_text.clone(),
                        signature: signature.to_string(),
                    });
                    output.content.push(Content::thinking(""));
                    sender.push(AssistantMessageEvent::ThinkingStart {
                        content_index: output.content.len() - 1,
                        partial: output.clone(),
                    });
                    sender.push(AssistantMessageEvent::ThinkingDelta {
                        content_index: output.content.len() - 1,
                        delta: reasoning_text.clone(),
                        partial: output.clone(),
                    });
                }
            }
        }
    }

    // Handle tool calls
    if let Some(tool_calls) = &delta.tool_calls {
        for tc in tool_calls {
            let should_start_new = match current_block {
                Some(CurrentBlock::ToolCall { id, .. }) => {
                    tc.id.as_ref().is_some_and(|new_id| new_id != id)
                }
                _ => true,
            };

            if should_start_new {
                finish_current_block(current_block, output, sender);
                *current_block = Some(CurrentBlock::ToolCall {
                    id: tc.id.clone().unwrap_or_default(),
                    name: tc
                        .function
                        .as_ref()
                        .and_then(|f| f.name.clone())
                        .unwrap_or_default(),
                    partial_args: String::new(),
                });
                output.content.push(Content::tool_call(
                    tc.id.clone().unwrap_or_default(),
                    tc.function
                        .as_ref()
                        .and_then(|f| f.name.clone())
                        .unwrap_or_default(),
                    serde_json::Value::Object(serde_json::Map::new()),
                ));
                sender.push(AssistantMessageEvent::ToolCallStart {
                    content_index: output.content.len() - 1,
                    partial: output.clone(),
                });
            }

            if let Some(CurrentBlock::ToolCall {
                id,
                name,
                partial_args,
            }) = current_block
            {
                if let Some(new_id) = &tc.id {
                    *id = new_id.clone();
                }
                if let Some(f) = &tc.function {
                    if let Some(n) = &f.name {
                        *name = n.clone();
                    }
                    if let Some(args) = &f.arguments {
                        partial_args.push_str(args);
                        sender.push(AssistantMessageEvent::ToolCallDelta {
                            content_index: output.content.len() - 1,
                            delta: args.clone(),
                            partial: output.clone(),
                        });
                    }
                }
            }
        }
    }
}

fn finish_current_block(
    current_block: &mut Option<CurrentBlock>,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
) {
    let Some(block) = current_block.take() else {
        return;
    };

    let content_index = output.content.len().saturating_sub(1);

    match block {
        CurrentBlock::Text { text } => {
            // Update the content in output
            if let Some(Content::Text { inner }) = output.content.get_mut(content_index) {
                inner.text = text.clone();
            }
            sender.push(AssistantMessageEvent::TextEnd {
                content_index,
                content: text,
                partial: output.clone(),
            });
        }
        CurrentBlock::Thinking {
            thinking,
            signature,
        } => {
            // Update the content in output
            if let Some(Content::Thinking { inner }) = output.content.get_mut(content_index) {
                inner.thinking = thinking.clone();
                inner.thinking_signature = Some(signature);
            }
            sender.push(AssistantMessageEvent::ThinkingEnd {
                content_index,
                content: thinking,
                partial: output.clone(),
            });
        }
        CurrentBlock::ToolCall {
            id,
            name,
            partial_args,
        } => {
            let arguments: serde_json::Value = serde_json::from_str(&partial_args)
                .unwrap_or_else(|_| serde_json::Value::Object(serde_json::Map::new()));

            // Update the content in output
            if let Some(Content::ToolCall { inner }) = output.content.get_mut(content_index) {
                inner.id = id.clone();
                inner.name = name.clone();
                inner.arguments = arguments.clone();
            }

            sender.push(AssistantMessageEvent::ToolCallEnd {
                content_index,
                tool_call: ToolCall {
                    id,
                    name,
                    arguments,
                    thought_signature: None,
                },
                partial: output.clone(),
            });
        }
    }
}

fn map_stop_reason(reason: &str) -> StopReason {
    match reason {
        "stop" => StopReason::Stop,
        "length" => StopReason::Length,
        "function_call" | "tool_calls" => StopReason::ToolUse,
        "content_filter" => StopReason::Error,
        _ => StopReason::Stop,
    }
}

fn build_params(
    model: &Model<OpenAICompletions>,
    context: &Context,
    options: &OpenAICompletionsOptions,
    compat: &ResolvedCompat,
) -> serde_json::Value {
    let mut params = json!({
        "model": model.id,
        "stream": true,
    });

    // Add messages
    let messages = convert_messages(model, context, compat);
    params["messages"] = messages;

    // Stream options
    if compat.supports_usage_in_streaming {
        params["stream_options"] = json!({ "include_usage": true });
    }

    // Store option
    if compat.supports_store {
        params["store"] = json!(false);
    }

    // Max tokens
    if let Some(max_tokens) = options.max_tokens {
        match compat.max_tokens_field {
            MaxTokensField::MaxTokens => {
                params["max_tokens"] = json!(max_tokens);
            }
            MaxTokensField::MaxCompletionTokens => {
                params["max_completion_tokens"] = json!(max_tokens);
            }
        }
    }

    // Temperature
    if let Some(temp) = options.temperature {
        params["temperature"] = json!(temp);
    }

    // Tools
    if let Some(tools) = &context.tools {
        params["tools"] = convert_tools(tools);
    }

    // Tool choice
    if let Some(tc) = &options.tool_choice {
        params["tool_choice"] = serde_json::to_value(tc).unwrap_or(json!("auto"));
    }

    // Reasoning effort
    if let Some(effort) = &options.reasoning_effort {
        if model.reasoning && compat.supports_reasoning_effort {
            if compat.thinking_format == ThinkingFormat::Zai {
                params["thinking"] = json!({ "type": "enabled" });
            } else {
                params["reasoning_effort"] =
                    serde_json::to_value(effort).unwrap_or(json!("medium"));
            }
        }
    }

    params
}

fn convert_messages(
    model: &Model<OpenAICompletions>,
    context: &Context,
    compat: &ResolvedCompat,
) -> serde_json::Value {
    let mut messages = Vec::new();

    // System prompt
    if let Some(system) = &context.system_prompt {
        let role = if model.reasoning && compat.supports_developer_role {
            "developer"
        } else {
            "system"
        };
        messages.push(json!({
            "role": role,
            "content": system,
        }));
    }

    // Convert messages
    for msg in &context.messages {
        match msg {
            crate::types::Message::User(user) => {
                let content = match &user.content {
                    crate::types::UserContent::Text(text) => json!(text),
                    crate::types::UserContent::Multi(blocks) => {
                        let parts: Vec<serde_json::Value> = blocks
                            .iter()
                            .filter_map(|block| match block {
                                crate::types::UserContentBlock::Text(t) => {
                                    Some(json!({ "type": "text", "text": t.text }))
                                }
                                crate::types::UserContentBlock::Image(img) => {
                                    if model.input.contains(&crate::types::InputType::Image) {
                                        Some(json!({
                                            "type": "image_url",
                                            "image_url": {
                                                "url": format!("data:{};base64,{}", img.mime_type, img.to_base64())
                                            }
                                        }))
                                    } else {
                                        None
                                    }
                                }
                            })
                            .collect();
                        json!(parts)
                    }
                };
                messages.push(json!({
                    "role": "user",
                    "content": content,
                }));
            }
            crate::types::Message::Assistant(assistant) => {
                let mut msg = json!({
                    "role": "assistant",
                });

                // Collect text content
                let text_parts: Vec<String> = assistant
                    .content
                    .iter()
                    .filter_map(|c| match c {
                        Content::Text { inner } if !inner.text.is_empty() => {
                            Some(inner.text.clone())
                        }
                        _ => None,
                    })
                    .collect();

                if !text_parts.is_empty() {
                    msg["content"] = json!(text_parts
                        .iter()
                        .map(|t| json!({ "type": "text", "text": t }))
                        .collect::<Vec<_>>());
                }

                // Collect tool calls
                let tool_calls: Vec<serde_json::Value> = assistant
                    .content
                    .iter()
                    .filter_map(|c| match c {
                        Content::ToolCall { inner } => Some(json!({
                            "id": inner.id,
                            "type": "function",
                            "function": {
                                "name": inner.name,
                                "arguments": inner.arguments.to_string(),
                            }
                        })),
                        _ => None,
                    })
                    .collect();

                if !tool_calls.is_empty() {
                    msg["tool_calls"] = json!(tool_calls);
                }

                // Skip empty assistant messages
                if msg.get("content").is_none() && msg.get("tool_calls").is_none() {
                    continue;
                }

                messages.push(msg);
            }
            crate::types::Message::ToolResult(result) => {
                let mut msg = json!({
                    "role": "tool",
                    "tool_call_id": result.tool_call_id,
                    "content": result.content.iter()
                        .filter_map(|c| match c {
                            crate::types::message::ToolResultContent::Text(t) => Some(t.text.clone()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join("\n"),
                });

                if compat.requires_tool_result_name {
                    msg["name"] = json!(result.tool_name);
                }

                messages.push(msg);
            }
        }
    }

    json!(messages)
}

fn convert_tools(tools: &[Tool]) -> serde_json::Value {
    let converted: Vec<serde_json::Value> = tools
        .iter()
        .map(|tool| {
            json!({
                "type": "function",
                "function": {
                    "name": tool.name,
                    "description": tool.description,
                    "parameters": tool.parameters,
                    "strict": false,
                }
            })
        })
        .collect();
    json!(converted)
}

/// Detect compatibility settings from provider and base URL.
fn detect_compat(model: &Model<OpenAICompletions>) -> ResolvedCompat {
    let provider = &model.provider;
    let base_url = &model.base_url;

    let is_zai =
        matches!(provider, Provider::Known(KnownProvider::Zai)) || base_url.contains("api.z.ai");

    let is_non_standard = matches!(
        provider,
        Provider::Known(KnownProvider::Cerebras)
            | Provider::Known(KnownProvider::Xai)
            | Provider::Known(KnownProvider::Mistral)
            | Provider::Known(KnownProvider::Zai)
    ) || base_url.contains("cerebras.ai")
        || base_url.contains("api.x.ai")
        || base_url.contains("mistral.ai")
        || base_url.contains("chutes.ai")
        || is_zai;

    let use_max_tokens = matches!(provider, Provider::Known(KnownProvider::Mistral))
        || base_url.contains("mistral.ai")
        || base_url.contains("chutes.ai");

    let is_grok =
        matches!(provider, Provider::Known(KnownProvider::Xai)) || base_url.contains("api.x.ai");

    let is_mistral = matches!(provider, Provider::Known(KnownProvider::Mistral))
        || base_url.contains("mistral.ai");

    ResolvedCompat {
        supports_store: !is_non_standard,
        supports_developer_role: !is_non_standard,
        supports_reasoning_effort: !is_grok && !is_zai,
        supports_usage_in_streaming: true,
        max_tokens_field: if use_max_tokens {
            MaxTokensField::MaxTokens
        } else {
            MaxTokensField::MaxCompletionTokens
        },
        requires_tool_result_name: is_mistral,
        requires_assistant_after_tool_result: false,
        requires_thinking_as_text: is_mistral,
        requires_mistral_tool_ids: is_mistral,
        thinking_format: if is_zai {
            ThinkingFormat::Zai
        } else {
            ThinkingFormat::Openai
        },
    }
}

/// Get resolved compatibility settings, merging detected with model-specified.
fn resolve_compat(model: &Model<OpenAICompletions>) -> ResolvedCompat {
    let detected = detect_compat(model);

    let Some(explicit) = &model.compat else {
        return detected;
    };

    ResolvedCompat {
        supports_store: explicit.supports_store.unwrap_or(detected.supports_store),
        supports_developer_role: explicit
            .supports_developer_role
            .unwrap_or(detected.supports_developer_role),
        supports_reasoning_effort: explicit
            .supports_reasoning_effort
            .unwrap_or(detected.supports_reasoning_effort),
        supports_usage_in_streaming: explicit
            .supports_usage_in_streaming
            .unwrap_or(detected.supports_usage_in_streaming),
        max_tokens_field: explicit
            .max_tokens_field
            .unwrap_or(detected.max_tokens_field),
        requires_tool_result_name: explicit
            .requires_tool_result_name
            .unwrap_or(detected.requires_tool_result_name),
        requires_assistant_after_tool_result: explicit
            .requires_assistant_after_tool_result
            .unwrap_or(detected.requires_assistant_after_tool_result),
        requires_thinking_as_text: explicit
            .requires_thinking_as_text
            .unwrap_or(detected.requires_thinking_as_text),
        requires_mistral_tool_ids: explicit
            .requires_mistral_tool_ids
            .unwrap_or(detected.requires_mistral_tool_ids),
        thinking_format: explicit.thinking_format.unwrap_or(detected.thinking_format),
    }
}

// SSE Response types

#[derive(Debug, Deserialize)]
struct StreamChunk {
    #[serde(default)]
    choices: Vec<StreamChoice>,
    usage: Option<StreamUsage>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    delta: Option<StreamDelta>,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamDelta {
    content: Option<String>,
    reasoning_content: Option<String>,
    reasoning: Option<String>,
    reasoning_text: Option<String>,
    tool_calls: Option<Vec<StreamToolCall>>,
}

#[derive(Debug, Deserialize)]
struct StreamToolCall {
    id: Option<String>,
    function: Option<StreamFunction>,
}

#[derive(Debug, Deserialize)]
struct StreamFunction {
    name: Option<String>,
    arguments: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    prompt_tokens_details: Option<PromptTokensDetails>,
    completion_tokens_details: Option<CompletionTokensDetails>,
}

#[derive(Debug, Deserialize)]
struct PromptTokensDetails {
    cached_tokens: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct CompletionTokensDetails {
    reasoning_tokens: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{InputType, ModelCost};

    fn make_test_model(
        id: &str,
        name: &str,
        provider: KnownProvider,
        base_url: &str,
    ) -> Model<OpenAICompletions> {
        Model {
            id: id.to_string(),
            name: name.to_string(),
            api: OpenAICompletions,
            provider: Provider::Known(provider),
            base_url: base_url.to_string(),
            reasoning: false,
            input: vec![InputType::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 128000,
            max_tokens: 4096,
            headers: None,
            compat: None,
        }
    }

    #[test]
    fn test_map_stop_reason() {
        assert_eq!(map_stop_reason("stop"), StopReason::Stop);
        assert_eq!(map_stop_reason("length"), StopReason::Length);
        assert_eq!(map_stop_reason("tool_calls"), StopReason::ToolUse);
        assert_eq!(map_stop_reason("function_call"), StopReason::ToolUse);
        assert_eq!(map_stop_reason("content_filter"), StopReason::Error);
        assert_eq!(map_stop_reason("unknown"), StopReason::Stop);
    }

    #[test]
    fn test_detect_compat_openai() {
        let model = make_test_model(
            "gpt-4",
            "GPT-4",
            KnownProvider::OpenAI,
            "https://api.openai.com/v1/chat/completions",
        );

        let compat = detect_compat(&model);
        assert!(compat.supports_store);
        assert!(compat.supports_developer_role);
        assert!(compat.supports_reasoning_effort);
        assert_eq!(compat.max_tokens_field, MaxTokensField::MaxCompletionTokens);
        assert!(!compat.requires_mistral_tool_ids);
    }

    #[test]
    fn test_detect_compat_mistral() {
        let model = make_test_model(
            "mistral-large",
            "Mistral Large",
            KnownProvider::Mistral,
            "https://api.mistral.ai/v1/chat/completions",
        );

        let compat = detect_compat(&model);
        assert!(!compat.supports_store);
        assert!(!compat.supports_developer_role);
        assert_eq!(compat.max_tokens_field, MaxTokensField::MaxTokens);
        assert!(compat.requires_mistral_tool_ids);
        assert!(compat.requires_tool_result_name);
    }
}
