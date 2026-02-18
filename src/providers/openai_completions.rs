use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::shared::{
    build_http_client, convert_messages, convert_tools, finish_current_block,
    handle_reasoning_delta, handle_text_delta, handle_tool_calls, initialize_output,
    map_stop_reason, process_sse_stream, push_stream_done, push_stream_error,
    send_streaming_request, update_usage_from_chunk, CurrentBlock, OpenAiLikeMessageOptions,
    OpenAiLikeStreamUsage, OpenAiLikeToolCallDelta, ReasoningDelta, SystemPromptRole,
};
use crate::stream::{AssistantMessageEventStream, EventStreamSender};
use crate::types::{
    Api, AssistantMessage, AssistantMessageEvent, Context, KnownProvider, MaxTokensField, Model,
    OpenAICompletions, OpenAICompletionsCompat, Provider, ThinkingFormat,
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

impl From<(ResolvedCompat, &OpenAICompletionsCompat)> for ResolvedCompat {
    fn from((detected, explicit): (ResolvedCompat, &OpenAICompletionsCompat)) -> Self {
        Self {
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
}

/// Stream completions from an OpenAI-compatible API.
pub fn stream_openai_completions(
    model: &Model<OpenAICompletions>,
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
    model: Model<OpenAICompletions>,
    context: Context,
    options: OpenAICompletionsOptions,
    mut sender: EventStreamSender,
) {
    let mut output = initialize_output(
        Api::OpenAICompletions,
        model.provider.clone(),
        model.id.clone(),
    );

    if let Err(error) = run_stream_inner(&model, &context, &options, &mut output, &mut sender).await
    {
        push_stream_error(&mut output, &mut sender, error);
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

    let response = send_streaming_request(&client, &model.base_url, &params).await?;

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

const REASONING_CONTENT_FIELD: &str = "reasoning_content";
const REASONING_FIELD: &str = "reasoning";
const REASONING_TEXT_FIELD: &str = "reasoning_text";

fn process_chunk(
    chunk: &StreamChunk,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
    current_block: &mut Option<CurrentBlock>,
) {
    if let Some(usage) = &chunk.usage {
        update_usage_from_chunk(usage, output);
    }

    let Some(choice) = chunk.choices.first() else {
        return;
    };

    if let Some(reason) = &choice.finish_reason {
        output.stop_reason = map_stop_reason(reason);
    }

    let Some(delta) = &choice.delta else {
        return;
    };

    if let Some(content) = delta.content.as_deref() {
        handle_text_delta(content, output, sender, current_block);
    }

    if let Some(reasoning) = extract_reasoning(delta) {
        handle_reasoning_delta(reasoning, output, sender, current_block);
    }

    if let Some(tool_calls) = &delta.tool_calls {
        handle_tool_calls(tool_calls, output, sender, current_block);
    }
}

fn extract_reasoning(delta: &StreamDelta) -> Option<ReasoningDelta<'_>> {
    if let Some(text) = delta.reasoning_content.as_deref() {
        return Some(ReasoningDelta {
            text,
            signature: REASONING_CONTENT_FIELD,
        });
    }

    if let Some(text) = delta.reasoning.as_deref() {
        return Some(ReasoningDelta {
            text,
            signature: REASONING_FIELD,
        });
    }

    delta.reasoning_text.as_deref().map(|text| ReasoningDelta {
        text,
        signature: REASONING_TEXT_FIELD,
    })
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

    let system_role = if model.reasoning && compat.supports_developer_role {
        SystemPromptRole::Developer
    } else {
        SystemPromptRole::System
    };

    let message_options = OpenAiLikeMessageOptions {
        system_role,
        requires_tool_result_name: compat.requires_tool_result_name,
    };

    params["messages"] = convert_messages(model, context, &message_options);

    if compat.supports_usage_in_streaming {
        params["stream_options"] = json!({ "include_usage": true });
    }

    if compat.supports_store {
        params["store"] = json!(false);
    }

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

    if let Some(temperature) = options.temperature {
        params["temperature"] = json!(temperature);
    }

    if let Some(tools) = &context.tools {
        params["tools"] = convert_tools(tools);
    }

    if let Some(tool_choice) = &options.tool_choice {
        params["tool_choice"] = serde_json::to_value(tool_choice).unwrap_or(json!("auto"));
    }

    if model.reasoning && compat.supports_reasoning_effort {
        if let Some(reasoning_effort) = &options.reasoning_effort {
            if compat.thinking_format == ThinkingFormat::Zai {
                params["thinking"] = json!({ "type": "enabled" });
            } else {
                params["reasoning_effort"] =
                    serde_json::to_value(reasoning_effort).unwrap_or(json!("medium"));
            }
        }
    }

    params
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

    match model.compat.as_ref() {
        Some(explicit) => ResolvedCompat::from((detected, explicit)),
        None => detected,
    }
}

#[derive(Debug, Deserialize)]
struct StreamChunk {
    #[serde(default)]
    choices: Vec<StreamChoice>,
    usage: Option<OpenAiLikeStreamUsage>,
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
    tool_calls: Option<Vec<OpenAiLikeToolCallDelta>>,
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
            context_window: 128_000,
            max_tokens: 4_096,
            headers: None,
            compat: None,
        }
    }

    #[test]
    fn detect_compat_for_openai_defaults() {
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
    fn detect_compat_for_mistral_defaults() {
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
