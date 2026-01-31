pub use crate::types::{AssistantMessageEventStream, EventStreamSender};

use crate::error::{Error, Result};
use crate::providers::{get_env_api_key, stream_openai_completions, OpenAICompletionsOptions};
use crate::types::{Api, AssistantMessage, Context, Model, OpenAICompletions};

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
    TApi: crate::types::ApiType,
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
            // SAFETY: We know the model has OpenAICompletions API type
            // This is a type-level guarantee from the match
            let model_ptr = model as *const Model<TApi> as *const Model<OpenAICompletions>;
            let openai_model = unsafe { &*model_ptr };
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
    TApi: crate::types::ApiType,
{
    let s = stream(model, context, options)?;
    s.result().await
}
