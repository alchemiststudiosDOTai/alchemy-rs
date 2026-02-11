mod event_stream;

pub use event_stream::{AssistantMessageEventStream, EventStreamSender};

use crate::error::{Error, Result};
use crate::providers::{get_env_api_key, stream_openai_completions, OpenAICompletionsOptions};
use crate::types::{AssistantMessage, Context, Model, OpenAICompletions};

/// Stream a completion from an OpenAI-compatible model.
///
/// This is the main entry point for streaming completions. It handles:
/// - API key resolution (from options or environment)
/// - Dispatching to the OpenAI completions provider
///
/// # Errors
///
/// Returns `Error::NoApiKey` if no API key is provided and none can be found
/// in the environment for the model's provider.
pub fn stream(
    model: &Model<OpenAICompletions>,
    context: &Context,
    options: Option<OpenAICompletionsOptions>,
) -> Result<AssistantMessageEventStream> {
    let api_key = options
        .as_ref()
        .and_then(|o| o.api_key.clone())
        .or_else(|| get_env_api_key(&model.provider));

    if api_key.is_none() {
        return Err(Error::NoApiKey(model.provider.to_string()));
    }

    let mut resolved_options = options.unwrap_or_default();
    if let Some(key) = api_key {
        resolved_options.api_key = Some(key);
    }

    Ok(stream_openai_completions(model, context, resolved_options))
}

/// Stream a completion and await the final result.
///
/// Convenience wrapper around [`stream`] that collects the stream
/// and returns the final [`AssistantMessage`].
pub async fn complete(
    model: &Model<OpenAICompletions>,
    context: &Context,
    options: Option<OpenAICompletionsOptions>,
) -> Result<AssistantMessage> {
    let s = stream(model, context, options)?;
    s.result().await
}
