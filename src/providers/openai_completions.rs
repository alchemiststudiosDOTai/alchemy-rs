use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::json;

use super::shared::{
    apply_deferred_tool_calls, convert_messages, convert_tools, extract_reasoning,
    handle_reasoning_delta, handle_text_delta, map_stop_reason, prepare_openai_like_chunk,
    run_openai_like_stream, spawn_openai_like_stream, AssistantThinkingMode, CurrentBlock,
    OpenAiLikeMessageOptions, OpenAiLikeRequest, OpenAiLikeStreamChunk, SystemPromptRole,
};
#[cfg(test)]
use super::shared::{finish_current_block, initialize_output};
use crate::types::{
    AssistantMessage, AssistantMessageEventStream, Context, EventStreamSender, KnownProvider,
    MaxTokensField, Model, OpenAICompletions, OpenAICompletionsCompat, Provider,
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
    pub zai: Option<crate::types::ZaiChatCompletionsOptions>,
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
}

impl ResolvedCompat {
    /// Apply explicit model-specified overrides on top of detected defaults.
    fn with_overrides(detected: Self, explicit: &OpenAICompletionsCompat) -> Self {
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
        }
    }
}

/// Stream completions from an OpenAI-compatible API.
pub fn stream_openai_completions(
    model: &Model<OpenAICompletions>,
    context: &Context,
    options: OpenAICompletionsOptions,
) -> AssistantMessageEventStream {
    spawn_openai_like_stream(
        model,
        context,
        options,
        |model, context, options, output, sender| {
            Box::pin(run_stream(model, context, options, output, sender))
        },
    )
}

async fn run_stream(
    model: &Model<OpenAICompletions>,
    context: &Context,
    options: &OpenAICompletionsOptions,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
) -> Result<(), crate::Error> {
    let compat = resolve_compat(model);
    let params = build_params(model, context, options, &compat);

    run_openai_like_stream(
        OpenAiLikeRequest::new(model, options, &params),
        output,
        sender,
        |chunk, output, sender, current_block| {
            if let Some(chunk) = chunk {
                process_chunk(&chunk, output, sender, current_block);
            }
        },
    )
    .await
}

fn process_chunk(
    chunk: &OpenAiLikeStreamChunk,
    output: &mut AssistantMessage,
    sender: &mut EventStreamSender,
    current_block: &mut Option<CurrentBlock>,
) {
    let Some(prelude) =
        prepare_openai_like_chunk(chunk, output, sender, current_block, map_stop_reason)
    else {
        return;
    };

    if let Some(content) = prelude.delta.content.as_deref() {
        handle_text_delta(content, output, sender, current_block);
    }

    if let Some(reasoning) = extract_reasoning(prelude.delta) {
        handle_reasoning_delta(reasoning, output, sender, current_block);
    }

    apply_deferred_tool_calls(prelude, output, sender, current_block);
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

    let assistant_thinking_mode = if compat.requires_thinking_as_text {
        AssistantThinkingMode::PlainText
    } else {
        AssistantThinkingMode::Omit
    };

    let message_options = OpenAiLikeMessageOptions::openai_like(
        system_role,
        compat.requires_tool_result_name,
        assistant_thinking_mode,
    );

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
            params["reasoning_effort"] =
                serde_json::to_value(reasoning_effort).unwrap_or(json!("medium"));
        }
    }

    params
}

/// Detect compatibility settings from provider and base URL.
fn detect_compat(model: &Model<OpenAICompletions>) -> ResolvedCompat {
    let provider = &model.provider;
    let base_url = &model.base_url;

    let is_featherless = matches!(provider, Provider::Known(KnownProvider::Featherless))
        || base_url.contains("featherless.ai");

    let is_non_standard = matches!(
        provider,
        Provider::Known(KnownProvider::Cerebras)
            | Provider::Known(KnownProvider::Xai)
            | Provider::Known(KnownProvider::Mistral)
    ) || base_url.contains("cerebras.ai")
        || base_url.contains("api.x.ai")
        || base_url.contains("mistral.ai")
        || base_url.contains("chutes.ai");

    let use_max_tokens = is_featherless
        || matches!(provider, Provider::Known(KnownProvider::Mistral))
        || base_url.contains("mistral.ai")
        || base_url.contains("chutes.ai");

    let is_grok =
        matches!(provider, Provider::Known(KnownProvider::Xai)) || base_url.contains("api.x.ai");

    let is_mistral = matches!(provider, Provider::Known(KnownProvider::Mistral))
        || base_url.contains("mistral.ai");

    ResolvedCompat {
        supports_store: !is_non_standard,
        supports_developer_role: !is_non_standard,
        supports_reasoning_effort: !is_grok,
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
    }
}

/// Get resolved compatibility settings, merging detected with model-specified.
fn resolve_compat(model: &Model<OpenAICompletions>) -> ResolvedCompat {
    let detected = detect_compat(model);

    match model.compat.as_ref() {
        Some(explicit) => ResolvedCompat::with_overrides(detected, explicit),
        None => detected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{
        assert_final_message_shape, build_final_message_shape_chunks, make_test_model,
        ExpectedFinalMessageShape,
    };
    use crate::types::{
        Api, Context, MaxTokensField, Message, StopReason, UserContent, UserMessage,
    };
    use futures::executor::block_on;
    use futures::StreamExt;
    use serde_json::json;

    fn make_model(provider: KnownProvider, id: &str, base_url: &str) -> Model<OpenAICompletions> {
        make_test_model(OpenAICompletions, provider, id, base_url, false)
    }

    fn process_chunks_for_test(
        model: &Model<OpenAICompletions>,
        chunks: Vec<OpenAiLikeStreamChunk>,
    ) -> AssistantMessage {
        let (mut stream, mut sender) = AssistantMessageEventStream::new();
        let mut output = initialize_output(
            Api::OpenAICompletions,
            model.provider.clone(),
            model.id.clone(),
        );
        let mut current_block = None;

        for chunk in chunks {
            process_chunk(&chunk, &mut output, &mut sender, &mut current_block);
        }

        finish_current_block(&mut current_block, &mut output, &mut sender);
        drop(sender);

        let _events = block_on(async move { stream.by_ref().collect::<Vec<_>>().await });
        output
    }

    #[test]
    fn detect_compat_for_openai_defaults() {
        let model = make_model(
            KnownProvider::OpenAI,
            "gpt-4",
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
        let model = make_model(
            KnownProvider::Mistral,
            "mistral-large",
            "https://api.mistral.ai/v1/chat/completions",
        );

        let compat = detect_compat(&model);
        assert!(!compat.supports_store);
        assert!(!compat.supports_developer_role);
        assert_eq!(compat.max_tokens_field, MaxTokensField::MaxTokens);
        assert!(compat.requires_mistral_tool_ids);
        assert!(compat.requires_tool_result_name);
    }

    #[test]
    fn detect_compat_for_featherless_defaults() {
        let model = make_model(
            KnownProvider::Featherless,
            "moonshotai/Kimi-K2.5",
            "https://api.featherless.ai/v1/chat/completions",
        );

        let compat = detect_compat(&model);
        assert!(compat.supports_store);
        assert!(compat.supports_developer_role);
        assert!(compat.supports_reasoning_effort);
        assert!(compat.supports_usage_in_streaming);
        assert_eq!(compat.max_tokens_field, MaxTokensField::MaxTokens);
        assert!(!compat.requires_mistral_tool_ids);
    }

    #[test]
    fn build_params_uses_max_tokens_for_featherless() {
        let model = make_model(
            KnownProvider::Featherless,
            "moonshotai/Kimi-K2.5",
            "https://api.featherless.ai/v1/chat/completions",
        );
        let context = Context {
            system_prompt: Some("Be concise.".to_string()),
            messages: vec![Message::User(UserMessage {
                content: UserContent::Text("Reply with ok".to_string()),
                timestamp: 0,
            })],
            tools: None,
        };
        let options = OpenAICompletionsOptions {
            max_tokens: Some(128),
            ..OpenAICompletionsOptions::default()
        };

        let params = build_params(&model, &context, &options, &resolve_compat(&model));

        assert_eq!(params["model"], json!("moonshotai/Kimi-K2.5"));
        assert_eq!(params["max_tokens"], json!(128));
        assert!(params.get("max_completion_tokens").is_none());
        assert_eq!(params["store"], json!(false));
        assert_eq!(params["stream_options"]["include_usage"], json!(true));
        assert_eq!(params["messages"][0]["role"], json!("system"));
    }

    #[test]
    fn stream_featherless_openai_compatible_runtime_returns_expected_message_shape() {
        let model = make_model(
            KnownProvider::Featherless,
            "moonshotai/Kimi-K2.5",
            "https://api.featherless.ai/v1/chat/completions",
        );
        let chunks = build_final_message_shape_chunks(json!({
            "reasoning": "thinking"
        }));
        let result = process_chunks_for_test(&model, chunks);

        assert_final_message_shape(
            &result,
            ExpectedFinalMessageShape {
                api: Api::OpenAICompletions,
                provider: Provider::Known(KnownProvider::Featherless),
                model: "moonshotai/Kimi-K2.5",
                stop_reason: StopReason::Stop,
                total_tokens: 15,
            },
        );
    }
}
