use serde_json::json;

use crate::types::{
    ApiType, Content, Context, InputType, Message, Model, Tool, ToolResultContent, UserContent,
    UserContentBlock, UserMessage,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SystemPromptRole {
    System,
    Developer,
}

impl SystemPromptRole {
    fn as_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Developer => "developer",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct OpenAiLikeMessageOptions {
    pub system_role: SystemPromptRole,
    pub requires_tool_result_name: bool,
}

pub(crate) fn convert_messages<TApi: ApiType>(
    model: &Model<TApi>,
    context: &Context,
    options: &OpenAiLikeMessageOptions,
) -> serde_json::Value {
    let mut messages = Vec::new();

    push_system_prompt(&mut messages, context, options.system_role);

    for message in &context.messages {
        match message {
            Message::User(user) => messages.push(convert_user_message(model, user)),
            Message::Assistant(assistant) => {
                if let Some(converted) = convert_assistant_message(assistant) {
                    messages.push(converted);
                }
            }
            Message::ToolResult(result) => {
                messages.push(convert_tool_result(
                    result,
                    options.requires_tool_result_name,
                ));
            }
        }
    }

    json!(messages)
}

fn push_system_prompt(
    messages: &mut Vec<serde_json::Value>,
    context: &Context,
    role: SystemPromptRole,
) {
    let Some(system_prompt) = &context.system_prompt else {
        return;
    };

    messages.push(json!({
        "role": role.as_str(),
        "content": system_prompt,
    }));
}

fn convert_user_message<TApi: ApiType>(
    model: &Model<TApi>,
    user: &UserMessage,
) -> serde_json::Value {
    let content = user_content_to_json(model, &user.content);

    json!({
        "role": "user",
        "content": content,
    })
}

fn user_content_to_json<TApi: ApiType>(
    model: &Model<TApi>,
    content: &UserContent,
) -> serde_json::Value {
    match content {
        UserContent::Text(text) => json!(text),
        UserContent::Multi(blocks) => {
            let parts: Vec<serde_json::Value> = blocks
                .iter()
                .filter_map(|block| match block {
                    UserContentBlock::Text(text) => Some(json!({
                        "type": "text",
                        "text": text.text,
                    })),
                    UserContentBlock::Image(image) if model.input.contains(&InputType::Image) => {
                        Some(json!({
                            "type": "image_url",
                            "image_url": {
                                "url": format!(
                                    "data:{};base64,{}",
                                    image.mime_type,
                                    image.to_base64(),
                                )
                            }
                        }))
                    }
                    UserContentBlock::Image(_) => None,
                })
                .collect();

            json!(parts)
        }
    }
}

fn convert_assistant_message(
    assistant: &crate::types::AssistantMessage,
) -> Option<serde_json::Value> {
    let mut message = json!({
        "role": "assistant",
    });

    let text_parts = assistant_text_parts(assistant);
    if !text_parts.is_empty() {
        message["content"] = json!(text_parts
            .iter()
            .map(|text| json!({ "type": "text", "text": text }))
            .collect::<Vec<_>>());
    }

    let tool_calls = assistant_tool_calls(assistant);
    if !tool_calls.is_empty() {
        message["tool_calls"] = json!(tool_calls);
    }

    if message.get("content").is_none() && message.get("tool_calls").is_none() {
        return None;
    }

    Some(message)
}

fn assistant_text_parts(assistant: &crate::types::AssistantMessage) -> Vec<String> {
    assistant
        .content
        .iter()
        .filter_map(|content| match content {
            Content::Text { inner } if !inner.text.is_empty() => Some(inner.text.clone()),
            _ => None,
        })
        .collect()
}

fn assistant_tool_calls(assistant: &crate::types::AssistantMessage) -> Vec<serde_json::Value> {
    assistant
        .content
        .iter()
        .filter_map(|content| match content {
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
        .collect()
}

fn convert_tool_result(
    result: &crate::types::ToolResultMessage,
    requires_tool_result_name: bool,
) -> serde_json::Value {
    let content = result
        .content
        .iter()
        .filter_map(|item| match item {
            ToolResultContent::Text(text) => Some(text.text.clone()),
            ToolResultContent::Image(_) => None,
        })
        .collect::<Vec<_>>()
        .join("\n");

    let mut message = json!({
        "role": "tool",
        "tool_call_id": result.tool_call_id,
        "content": content,
    });

    if requires_tool_result_name {
        message["name"] = json!(result.tool_name);
    }

    message
}

pub(crate) fn convert_tools(tools: &[Tool]) -> serde_json::Value {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        Api, AssistantMessage, Cost, KnownProvider, ModelCost, OpenAICompletions, Provider,
        StopReason, Usage,
    };

    fn make_model(input: Vec<InputType>) -> Model<OpenAICompletions> {
        Model {
            id: "test-model".to_string(),
            name: "Test model".to_string(),
            api: OpenAICompletions,
            provider: Provider::Known(KnownProvider::OpenAI),
            base_url: "https://example.com/v1/chat/completions".to_string(),
            reasoning: false,
            input,
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

    fn make_assistant_message() -> AssistantMessage {
        AssistantMessage {
            content: vec![Content::text("hello")],
            api: Api::OpenAICompletions,
            provider: Provider::Known(KnownProvider::OpenAI),
            model: "test-model".to_string(),
            usage: Usage {
                input: 0,
                output: 0,
                cache_read: 0,
                cache_write: 0,
                total_tokens: 0,
                cost: Cost::default(),
            },
            stop_reason: StopReason::Stop,
            error_message: None,
            timestamp: 0,
        }
    }

    #[test]
    fn convert_messages_uses_system_role_option() {
        let model = make_model(vec![InputType::Text]);
        let context = Context {
            system_prompt: Some("sys".to_string()),
            messages: vec![],
            tools: None,
        };

        let params = convert_messages(
            &model,
            &context,
            &OpenAiLikeMessageOptions {
                system_role: SystemPromptRole::Developer,
                requires_tool_result_name: false,
            },
        );

        assert_eq!(params[0]["role"], "developer");
        assert_eq!(params[0]["content"], "sys");
    }

    #[test]
    fn convert_messages_drops_images_for_text_only_model() {
        let model = make_model(vec![InputType::Text]);
        let context = Context {
            system_prompt: None,
            messages: vec![Message::User(UserMessage {
                content: UserContent::Multi(vec![UserContentBlock::Image(
                    crate::types::ImageContent {
                        data: vec![1, 2, 3],
                        mime_type: "image/png".to_string(),
                    },
                )]),
                timestamp: 0,
            })],
            tools: None,
        };

        let params = convert_messages(
            &model,
            &context,
            &OpenAiLikeMessageOptions {
                system_role: SystemPromptRole::System,
                requires_tool_result_name: false,
            },
        );

        assert_eq!(params[0]["content"], serde_json::json!([]));
    }

    #[test]
    fn convert_messages_includes_assistant_text_content() {
        let model = make_model(vec![InputType::Text]);
        let context = Context {
            system_prompt: None,
            messages: vec![Message::Assistant(make_assistant_message())],
            tools: None,
        };

        let params = convert_messages(
            &model,
            &context,
            &OpenAiLikeMessageOptions {
                system_role: SystemPromptRole::System,
                requires_tool_result_name: false,
            },
        );

        assert_eq!(params[0]["role"], "assistant");
        assert_eq!(params[0]["content"][0]["text"], "hello");
    }
}
