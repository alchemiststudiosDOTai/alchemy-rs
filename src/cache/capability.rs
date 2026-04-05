use crate::cache::request::{
    CacheRequestInput, CacheRequestMutations, CacheRequestPreparation, CacheRequestValue,
};
use crate::types::{KnownProvider, Provider};

const KIMI_CHAT_COMPLETIONS_PATH: &str = "/v1/chat/completions";
const KIMI_TRANSPORT_MODEL_ID: &str = "kimi-for-coding";
const KIMI_CLI_USER_AGENT: &str = "KimiCLI/1.29.0";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderCacheCapability {
    Kimi,
}

pub(crate) fn cache_capability_for(provider: &Provider) -> Option<ProviderCacheCapability> {
    match provider {
        Provider::Known(KnownProvider::Kimi) => Some(ProviderCacheCapability::Kimi),
        _ => None,
    }
}

impl ProviderCacheCapability {
    pub(crate) fn prepare_request(self, input: CacheRequestInput<'_>) -> CacheRequestPreparation {
        CacheRequestPreparation::from_input(input, self.request_mutations(input))
    }

    pub(crate) fn request_mutations(self, input: CacheRequestInput<'_>) -> CacheRequestMutations {
        match self {
            Self::Kimi => kimi_request_mutations(input),
        }
    }
}

fn kimi_request_mutations(input: CacheRequestInput<'_>) -> CacheRequestMutations {
    let mut mutations = CacheRequestMutations {
        endpoint_override: Some(format!(
            "{}{}",
            input.base_url.trim_end_matches('/'),
            KIMI_CHAT_COMPLETIONS_PATH
        )),
        model_override: Some(KIMI_TRANSPORT_MODEL_ID.to_string()),
        ..CacheRequestMutations::default()
    };

    mutations
        .headers
        .insert("User-Agent".to_string(), KIMI_CLI_USER_AGENT.to_string());

    if let Some(cache) = input.cache {
        mutations.body_fields.insert(
            "prompt_cache_key".to_string(),
            CacheRequestValue::String(cache.key.clone()),
        );
    }

    mutations
}

#[cfg(test)]
mod tests {
    use super::{cache_capability_for, ProviderCacheCapability, KIMI_CLI_USER_AGENT};
    use crate::cache::request::{CacheRequestInput, CacheRequestValue};
    use crate::types::{CacheOptions, KnownProvider, Provider};

    #[test]
    fn kimi_cache_capability_returns_request_mutations() {
        let capability = cache_capability_for(&Provider::Known(KnownProvider::Kimi))
            .expect("kimi capability should exist");
        let preparation = capability.prepare_request(CacheRequestInput {
            base_url: "https://api.kimi.com/coding",
            model_id: "kimi-coding",
            cache: Some(&CacheOptions {
                key: "cache-key".to_string(),
            }),
        });

        assert_eq!(
            preparation.endpoint,
            "https://api.kimi.com/coding/v1/chat/completions"
        );
        assert_eq!(preparation.model_id, "kimi-for-coding");
        assert_eq!(
            preparation.headers.get("User-Agent").map(String::as_str),
            Some(KIMI_CLI_USER_AGENT)
        );
        assert_eq!(
            preparation.body_fields.get("prompt_cache_key"),
            Some(&CacheRequestValue::String("cache-key".to_string()))
        );
    }

    #[test]
    fn non_kimi_provider_has_no_cache_capability() {
        let capability = cache_capability_for(&Provider::Known(KnownProvider::OpenAI));
        assert!(capability.is_none());
    }

    #[test]
    fn kimi_cache_capability_preserves_cacheless_request_shape() {
        let capability = ProviderCacheCapability::Kimi;
        let preparation = capability.prepare_request(CacheRequestInput {
            base_url: "https://api.kimi.com/coding/",
            model_id: "kimi-coding",
            cache: None,
        });

        assert_eq!(
            preparation.endpoint,
            "https://api.kimi.com/coding/v1/chat/completions"
        );
        assert_eq!(preparation.model_id, "kimi-for-coding");
        assert!(preparation.body_fields.is_empty());
    }
}
