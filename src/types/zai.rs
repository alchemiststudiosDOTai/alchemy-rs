use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZaiChatCompletionsOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub do_sample: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<[String; 1]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ZaiResponseFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ZaiThinking>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaiResponseFormat {
    #[serde(rename = "type")]
    pub kind: ZaiResponseFormatType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_schema: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ZaiResponseFormatType {
    Text,
    JsonObject,
    JsonSchema,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZaiThinking {
    #[serde(rename = "type")]
    pub kind: ZaiThinkingType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clear_thinking: Option<bool>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ZaiThinkingType {
    Enabled,
    Disabled,
}

#[cfg(test)]
mod tests {
    use super::{
        ZaiChatCompletionsOptions, ZaiResponseFormat, ZaiResponseFormatType, ZaiThinking,
        ZaiThinkingType,
    };
    use serde_json::json;

    #[test]
    fn zai_options_default_omits_all_fields() {
        let options = ZaiChatCompletionsOptions::default();
        let serialized = serde_json::to_value(&options).expect("serialize default options");

        assert_eq!(serialized, json!({}));
    }

    #[test]
    fn zai_options_serialize_with_expected_contract_shape() {
        let options = ZaiChatCompletionsOptions {
            do_sample: Some(true),
            top_p: Some(0.8),
            max_tokens: Some(4096),
            stop: Some(["stop-here".to_string()]),
            tool_stream: Some(true),
            request_id: Some("request-123".to_string()),
            user_id: Some("user-456".to_string()),
            response_format: Some(ZaiResponseFormat {
                kind: ZaiResponseFormatType::JsonSchema,
                json_schema: Some(json!({"type": "object"})),
            }),
            thinking: Some(ZaiThinking {
                kind: ZaiThinkingType::Enabled,
                clear_thinking: Some(false),
            }),
        };

        let serialized = serde_json::to_value(&options).expect("serialize populated options");

        assert_eq!(
            serialized,
            json!({
                "do_sample": true,
                "top_p": 0.8,
                "max_tokens": 4096,
                "stop": ["stop-here"],
                "tool_stream": true,
                "request_id": "request-123",
                "user_id": "user-456",
                "response_format": {
                    "type": "json_schema",
                    "json_schema": {
                        "type": "object"
                    }
                },
                "thinking": {
                    "type": "enabled",
                    "clear_thinking": false
                }
            })
        );
    }
}
