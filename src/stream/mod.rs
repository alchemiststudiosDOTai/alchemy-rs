mod event_stream;

pub use event_stream::{AssistantMessageEventStream, EventStreamSender};

use crate::error::{Error, Result};
use crate::providers::{get_env_api_key, stream_openai_completions, OpenAICompletionsOptions};
use crate::types::{Api, AssistantMessage, Context, Model, OpenAICompletions};
use std::any::Any;

/// Stream a completion from an OpenAI-compatible model.
///
/// This is the main entry point for streaming completions. It handles:
/// - API key resolution (from options or environment)
/// - Dispatching to the correct provider based on the model's API type
///
/// # Errors
///
/// Returns `Error::NoApiKey` if no API key is provided and none can be found
/// in the environment for the model's provider.
pub fn stream<TApi>(
    model: &Model<TApi>,
    context: &Context,
    options: Option<OpenAICompletionsOptions>,
) -> Result<AssistantMessageEventStream>
where
    TApi: crate::types::ApiType + 'static,
{
    let api = model.api.api();

    // Get API key from options or environment
    let api_key = options
        .as_ref()
        .and_then(|o| o.api_key.clone())
        .or_else(|| get_env_api_key(&model.provider));

    // Check if API key is required
    let needs_api_key = !matches!(api, Api::GoogleVertex | Api::BedrockConverseStream);

    if needs_api_key && api_key.is_none() {
        return Err(Error::NoApiKey(model.provider.to_string()));
    }

    // Build options with resolved API key
    let mut resolved_options = options.unwrap_or_default();
    if let Some(key) = api_key {
        resolved_options.api_key = Some(key);
    }

    match api {
        Api::OpenAICompletions => {
            // `api` is a runtime value. Ensure the compile-time type parameter `TApi`
            // actually is `OpenAICompletions` before proceeding.
            let openai_model = (model as &dyn Any)
                .downcast_ref::<Model<OpenAICompletions>>()
                .ok_or_else(|| {
                    Error::InvalidResponse(
                        "Model/api type mismatch: api() returned openai-completions, but model is not Model<OpenAICompletions>".to_string(),
                    )
                })?;

            Ok(stream_openai_completions(
                openai_model,
                context,
                resolved_options,
            ))
        }
        Api::AnthropicMessages => Err(Error::InvalidResponse(
            "Anthropic provider not yet implemented".to_string(),
        )),
        Api::BedrockConverseStream => Err(Error::InvalidResponse(
            "Bedrock provider not yet implemented".to_string(),
        )),
        Api::OpenAIResponses => Err(Error::InvalidResponse(
            "OpenAI Responses provider not yet implemented".to_string(),
        )),
        Api::GoogleGenerativeAi => Err(Error::InvalidResponse(
            "Google Generative AI provider not yet implemented".to_string(),
        )),
        Api::GoogleVertex => Err(Error::InvalidResponse(
            "Google Vertex provider not yet implemented".to_string(),
        )),
    }
}

/// Stream a completion and await the final result.
///
/// This is a convenience wrapper around `stream()` that collects the stream
/// and returns the final `AssistantMessage`.
pub async fn complete<TApi>(
    model: &Model<TApi>,
    context: &Context,
    options: Option<OpenAICompletionsOptions>,
) -> Result<AssistantMessage>
where
    TApi: crate::types::ApiType + 'static,
{
    let s = stream(model, context, options)?;
    s.result().await
}
