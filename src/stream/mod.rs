pub use crate::types::{AssistantMessageEventStream, EventStreamSender};

use crate::error::{Error, Result};
use crate::providers::{
    get_env_api_key, stream_anthropic_messages, stream_kimi_messages, stream_minimax_completions,
    stream_openai_completions, stream_zai_completions, OpenAICompletionsOptions,
};
use crate::types::{
    AnthropicMessages, ApiType, AssistantMessage, Context, KnownProvider, MinimaxCompletions,
    Model, OpenAICompletions, Provider, ZaiCompletions,
};

/// An API marker type with an implemented streaming runtime.
///
/// `stream()` and `complete()` dispatch through this trait, so a model is
/// only streamable when its API family has a provider implementation —
/// unsupported APIs are rejected at compile time.
pub trait StreamableApi: ApiType + Sized {
    fn stream(
        model: &Model<Self>,
        context: &Context,
        options: OpenAICompletionsOptions,
    ) -> AssistantMessageEventStream;
}

impl StreamableApi for OpenAICompletions {
    fn stream(
        model: &Model<Self>,
        context: &Context,
        options: OpenAICompletionsOptions,
    ) -> AssistantMessageEventStream {
        stream_openai_completions(model, context, options)
    }
}

impl StreamableApi for MinimaxCompletions {
    fn stream(
        model: &Model<Self>,
        context: &Context,
        options: OpenAICompletionsOptions,
    ) -> AssistantMessageEventStream {
        stream_minimax_completions(model, context, options)
    }
}

impl StreamableApi for ZaiCompletions {
    fn stream(
        model: &Model<Self>,
        context: &Context,
        options: OpenAICompletionsOptions,
    ) -> AssistantMessageEventStream {
        stream_zai_completions(model, context, options)
    }
}

impl StreamableApi for AnthropicMessages {
    fn stream(
        model: &Model<Self>,
        context: &Context,
        options: OpenAICompletionsOptions,
    ) -> AssistantMessageEventStream {
        if matches!(model.provider, Provider::Known(KnownProvider::Kimi)) {
            stream_kimi_messages(model, context, options)
        } else {
            stream_anthropic_messages(model, context, options)
        }
    }
}

/// Stream a completion from a model.
///
/// This is the main entry point for streaming completions. It resolves the
/// API key (from options or environment) and dispatches to the provider
/// runtime for the model's API type.
///
/// # Errors
///
/// Returns `Error::NoApiKey` if no API key is provided and none can be found
/// in the environment for the model's provider.
pub fn stream<TApi: StreamableApi>(
    model: &Model<TApi>,
    context: &Context,
    options: Option<OpenAICompletionsOptions>,
) -> Result<AssistantMessageEventStream> {
    let mut options = options.unwrap_or_default();

    if options.api_key.is_none() {
        options.api_key = get_env_api_key(&model.provider);
    }

    if options.api_key.is_none() {
        return Err(Error::NoApiKey(model.provider.to_string()));
    }

    Ok(TApi::stream(model, context, options))
}

/// Stream a completion and await the final result.
///
/// This is a convenience wrapper around `stream()` that collects the stream
/// and returns the final `AssistantMessage`.
pub async fn complete<TApi: StreamableApi>(
    model: &Model<TApi>,
    context: &Context,
    options: Option<OpenAICompletionsOptions>,
) -> Result<AssistantMessage> {
    let s = stream(model, context, options)?;
    s.result().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        Api, InputType, KnownProvider, ModelCost, Provider, StopReason, StopReasonError,
        StopReasonSuccess, ZaiCompletions,
    };
    use tokio::time::{timeout, Duration};

    fn minimax_test_model(base_url: &str) -> Model<MinimaxCompletions> {
        Model {
            id: "MiniMax-M2.5".to_string(),
            name: "MiniMax M2.5".to_string(),
            api: MinimaxCompletions,
            provider: Provider::Known(KnownProvider::Minimax),
            base_url: base_url.to_string(),
            reasoning: true,
            input: vec![InputType::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 204_800,
            max_tokens: 16_384,
            headers: None,
            compat: None,
        }
    }

    fn zai_test_model(base_url: &str) -> Model<ZaiCompletions> {
        Model {
            id: "glm-4.7".to_string(),
            name: "GLM 4.7".to_string(),
            api: ZaiCompletions,
            provider: Provider::Known(KnownProvider::Zai),
            base_url: base_url.to_string(),
            reasoning: true,
            input: vec![InputType::Text],
            cost: ModelCost {
                input: 0.0,
                output: 0.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
            context_window: 200_000,
            max_tokens: 128_000,
            headers: None,
            compat: None,
        }
    }

    fn featherless_test_model(base_url: &str) -> Model<OpenAICompletions> {
        Model {
            id: "moonshotai/Kimi-K2.5".to_string(),
            name: "Kimi K2.5".to_string(),
            api: OpenAICompletions,
            provider: Provider::Known(KnownProvider::Featherless),
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
            max_tokens: 16_384,
            headers: None,
            compat: None,
        }
    }

    async fn assert_dispatches_to_provider<TApi: StreamableApi>(
        model: Model<TApi>,
        expected_api: Api,
    ) {
        let context = Context::default();
        let options = Some(OpenAICompletionsOptions {
            api_key: Some("test-key".to_string()),
            ..OpenAICompletionsOptions::default()
        });

        let stream = stream(&model, &context, options).expect("dispatch should succeed");
        let result = timeout(Duration::from_secs(5), stream.result())
            .await
            .expect("stream should finish quickly")
            .expect("stream result should be returned");

        assert_eq!(result.api, expected_api);
        assert_eq!(result.stop_reason, StopReason::Error);
    }

    #[tokio::test]
    async fn stream_dispatches_to_minimax_provider() {
        let model = minimax_test_model("http://127.0.0.1:1/v1/chat/completions");
        assert_dispatches_to_provider(model, Api::MinimaxCompletions).await;
    }

    #[tokio::test]
    async fn stream_dispatches_to_zai_provider() {
        let model = zai_test_model("http://127.0.0.1:1/api/paas/v4/chat/completions");
        assert_dispatches_to_provider(model, Api::ZaiCompletions).await;
    }

    fn anthropic_test_model(base_url: &str) -> Model<AnthropicMessages> {
        Model {
            id: "claude-sonnet-4-6".to_string(),
            name: "Claude Sonnet 4.6".to_string(),
            api: AnthropicMessages,
            provider: Provider::Known(KnownProvider::Anthropic),
            base_url: base_url.to_string(),
            reasoning: true,
            input: vec![InputType::Text, InputType::Image],
            cost: ModelCost {
                input: 0.003,
                output: 0.015,
                cache_read: 0.0003,
                cache_write: 0.00375,
            },
            context_window: 200_000,
            max_tokens: 64_000,
            headers: None,
            compat: None,
        }
    }

    #[tokio::test]
    async fn stream_dispatches_to_anthropic_provider() {
        let model = anthropic_test_model("http://127.0.0.1:1");
        assert_dispatches_to_provider(model, Api::AnthropicMessages).await;
    }

    #[tokio::test]
    async fn stream_dispatches_featherless_through_openai_completions_provider() {
        let model = featherless_test_model("http://127.0.0.1:1/v1/chat/completions");
        assert_dispatches_to_provider(model, Api::OpenAICompletions).await;
    }

    #[test]
    fn stop_reason_conversion_contract_is_unchanged() {
        assert_eq!(StopReason::from(StopReasonSuccess::Stop), StopReason::Stop);
        assert_eq!(StopReason::from(StopReasonError::Error), StopReason::Error);
    }
}
