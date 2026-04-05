use std::collections::HashMap;

use crate::cache::capability::ProviderCacheCapability;
use crate::types::CacheOptions;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CacheRequestValue {
    String(String),
}

impl CacheRequestValue {
    pub(crate) fn to_json_value(&self) -> serde_json::Value {
        match self {
            Self::String(value) => serde_json::Value::String(value.clone()),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CacheRequestMutations {
    pub headers: HashMap<String, String>,
    pub body_fields: HashMap<String, CacheRequestValue>,
    pub endpoint_override: Option<String>,
    pub model_override: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CacheRequestInput<'a> {
    pub base_url: &'a str,
    pub model_id: &'a str,
    pub cache: Option<&'a CacheOptions>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CacheRequestPreparation {
    pub endpoint: String,
    pub model_id: String,
    pub headers: HashMap<String, String>,
    pub body_fields: HashMap<String, CacheRequestValue>,
}

impl CacheRequestPreparation {
    pub(crate) fn from_input(
        input: CacheRequestInput<'_>,
        mutations: CacheRequestMutations,
    ) -> Self {
        Self {
            endpoint: mutations
                .endpoint_override
                .unwrap_or_else(|| input.base_url.to_string()),
            model_id: mutations
                .model_override
                .unwrap_or_else(|| input.model_id.to_string()),
            headers: mutations.headers,
            body_fields: mutations.body_fields,
        }
    }
}

pub(crate) fn prepare_cache_request(
    capability: Option<ProviderCacheCapability>,
    input: CacheRequestInput<'_>,
) -> Option<CacheRequestPreparation> {
    capability.map(|capability| capability.prepare_request(input))
}

pub(crate) fn apply_cache_request_preparation(
    params: &serde_json::Value,
    preparation: Option<&CacheRequestPreparation>,
) -> serde_json::Value {
    let Some(preparation) = preparation else {
        return params.clone();
    };

    let mut params = params.clone();
    params["model"] = serde_json::Value::String(preparation.model_id.clone());

    for (field, value) in &preparation.body_fields {
        params[field] = value.to_json_value();
    }

    params
}
