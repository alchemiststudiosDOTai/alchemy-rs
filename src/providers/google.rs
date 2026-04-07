use super::openai_completions::OpenAICompletionsOptions;
use super::shared::stream_google_like;
use crate::types::{AssistantMessageEventStream, Context, GoogleGenerativeAi, Model};

pub fn stream_google_generative_ai(
    model: &Model<GoogleGenerativeAi>,
    context: &Context,
    options: OpenAICompletionsOptions,
) -> AssistantMessageEventStream {
    stream_google_like(model, context, options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::google::gemini_2_5_flash;
    use crate::types::{
        Api, AssistantMessageEvent, Content, KnownProvider, Message, Provider, StopReason, Tool,
        UserContent, UserMessage,
    };
    use futures::StreamExt;
    use std::env;

    fn require_live_api_key() {
        assert!(
            env::var("GEMINI_API_KEY").is_ok(),
            "GEMINI_API_KEY must be set to run live Google tests"
        );
    }

    fn text_context(prompt: &str) -> Context {
        Context {
            system_prompt: Some("You are concise.".to_string()),
            messages: vec![Message::User(UserMessage {
                content: UserContent::Text(prompt.to_string()),
                timestamp: 0,
            })],
            tools: None,
        }
    }

    #[tokio::test]
    #[ignore = "live API test"]
    async fn live_google_complete_returns_typed_text_message() {
        require_live_api_key();

        let model = gemini_2_5_flash();
        let result = crate::complete(
            &model,
            &text_context("Reply with exactly: hello from gemini"),
            Some(OpenAICompletionsOptions {
                max_tokens: Some(64),
                ..Default::default()
            }),
        )
        .await
        .expect("google complete should succeed");

        assert_eq!(result.api, Api::GoogleGenerativeAi);
        assert_eq!(result.provider, Provider::Known(KnownProvider::Google));
        assert!(matches!(
            result.stop_reason,
            StopReason::Stop | StopReason::Length
        ));
        assert!(matches!(result.content.first(), Some(Content::Text { .. })));
    }

    #[tokio::test]
    #[ignore = "live API test"]
    async fn live_google_stream_emits_typed_text_events() {
        require_live_api_key();

        let model = gemini_2_5_flash();
        let mut stream = crate::stream(
            &model,
            &text_context("Say hello in one short sentence."),
            Some(OpenAICompletionsOptions {
                max_tokens: Some(64),
                ..Default::default()
            }),
        )
        .expect("google stream should start");

        let events: Vec<_> = stream.by_ref().collect().await;

        assert!(events.iter().any(|e| matches!(
            e,
            AssistantMessageEvent::Start { partial }
                if partial.provider == Provider::Known(KnownProvider::Google)
                    && partial.api == Api::GoogleGenerativeAi
        )));
        assert!(events
            .iter()
            .any(|e| matches!(e, AssistantMessageEvent::TextDelta { .. })));
        assert!(events.iter().any(|e| matches!(
            e,
            AssistantMessageEvent::Done { message, .. }
                if message.provider == Provider::Known(KnownProvider::Google)
        )));
    }

    #[tokio::test]
    #[ignore = "live API test"]
    async fn live_google_complete_returns_typed_tool_call() {
        require_live_api_key();

        let model = gemini_2_5_flash();
        let context = Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: UserContent::Text(
                    "Use the get_weather tool for Austin, TX. Do not answer directly.".to_string(),
                ),
                timestamp: 0,
            })],
            tools: Some(vec![Tool::new(
                "get_weather",
                "Get the weather for a city",
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        "city": {"type": "string"}
                    },
                    "required": ["city"]
                }),
            )]),
        };

        let result = crate::complete(
            &model,
            &context,
            Some(OpenAICompletionsOptions {
                max_tokens: Some(256),
                ..Default::default()
            }),
        )
        .await
        .expect("google tool call should succeed");

        assert_eq!(result.api, Api::GoogleGenerativeAi);
        assert_eq!(result.stop_reason, StopReason::ToolUse);
        assert!(matches!(
            result.content.first(),
            Some(Content::ToolCall { inner }) if inner.name == "get_weather"
        ));
    }
}
