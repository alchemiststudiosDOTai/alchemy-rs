use std::collections::HashMap;

use crate::types::CacheOptions;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CacheRequestValue {
    String(String),
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
