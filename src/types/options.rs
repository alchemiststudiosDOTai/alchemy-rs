use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub trait StreamOptions: Send + Sync {
    fn temperature(&self) -> Option<f64> {
        None
    }
    fn max_tokens(&self) -> Option<u32> {
        None
    }
    fn api_key(&self) -> Option<&str> {
        None
    }
    fn cache(&self) -> Option<&CacheOptions> {
        None
    }
    fn headers(&self) -> Option<&HashMap<String, String>> {
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[non_exhaustive]
pub struct CacheOptions {
    pub key: String,
}

impl CacheOptions {
    pub fn new(key: impl Into<String>) -> Self {
        Self { key: key.into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimpleStreamOptions {
    #[serde(flatten)]
    pub base: BaseStreamOptions,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<ThinkingLevel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_budgets: Option<ThinkingBudgets>,
}

impl StreamOptions for SimpleStreamOptions {
    fn temperature(&self) -> Option<f64> {
        self.base.temperature
    }
    fn max_tokens(&self) -> Option<u32> {
        self.base.max_tokens
    }
    fn api_key(&self) -> Option<&str> {
        self.base.api_key.as_deref()
    }
    fn cache(&self) -> Option<&CacheOptions> {
        self.base.cache.as_ref()
    }
    fn headers(&self) -> Option<&HashMap<String, String>> {
        self.base.headers.as_ref()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaseStreamOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<CacheOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
}

impl StreamOptions for BaseStreamOptions {}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingLevel {
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingBudgets {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimal: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub low: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub medium: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::{BaseStreamOptions, CacheOptions, SimpleStreamOptions};
    use crate::providers::OpenAICompletionsOptions;
    use serde_json::json;

    #[test]
    fn cache_options_replace_session_id_in_public_options() {
        let cache = CacheOptions {
            key: "cache-key".to_string(),
        };

        let base_json = serde_json::to_value(BaseStreamOptions {
            temperature: None,
            max_tokens: None,
            api_key: None,
            cache: Some(cache.clone()),
            headers: None,
        })
        .expect("base options serialize");
        assert_eq!(base_json["cache"], json!({ "key": "cache-key" }));
        assert!(base_json.get("session_id").is_none());

        let simple_json = serde_json::to_value(SimpleStreamOptions {
            base: BaseStreamOptions {
                temperature: None,
                max_tokens: None,
                api_key: None,
                cache: Some(cache.clone()),
                headers: None,
            },
            reasoning: None,
            thinking_budgets: None,
        })
        .expect("simple options serialize");
        assert_eq!(simple_json["cache"], json!({ "key": "cache-key" }));
        assert!(simple_json.get("session_id").is_none());

        let options = OpenAICompletionsOptions {
            cache: Some(cache),
            ..OpenAICompletionsOptions::default()
        };
        assert_eq!(
            options.cache.as_ref().map(|cache| cache.key.as_str()),
            Some("cache-key")
        );
    }
}
