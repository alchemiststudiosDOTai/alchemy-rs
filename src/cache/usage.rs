#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct NormalizedCacheUsage {
    pub cache_read_tokens: u32,
    pub cache_write_tokens: u32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct OpenAiLikeCacheUsage {
    pub cache_read_input_tokens: Option<u32>,
    pub cache_creation_input_tokens: Option<u32>,
    pub prompt_cached_tokens: Option<u32>,
    pub prompt_cache_write_tokens: Option<u32>,
}

pub(crate) fn normalize_openai_like_cache_usage(
    usage: OpenAiLikeCacheUsage,
) -> NormalizedCacheUsage {
    NormalizedCacheUsage {
        cache_read_tokens: usage
            .cache_read_input_tokens
            .or(usage.prompt_cached_tokens)
            .unwrap_or(0),
        cache_write_tokens: usage
            .cache_creation_input_tokens
            .or(usage.prompt_cache_write_tokens)
            .unwrap_or(0),
    }
}

#[cfg(test)]
mod tests {
    use super::{normalize_openai_like_cache_usage, NormalizedCacheUsage, OpenAiLikeCacheUsage};

    #[test]
    fn normalize_openai_like_cache_usage_prefers_top_level_fields() {
        let usage = normalize_openai_like_cache_usage(OpenAiLikeCacheUsage {
            cache_read_input_tokens: Some(12),
            cache_creation_input_tokens: Some(8),
            prompt_cached_tokens: Some(3),
            prompt_cache_write_tokens: Some(2),
        });

        assert_eq!(
            usage,
            NormalizedCacheUsage {
                cache_read_tokens: 12,
                cache_write_tokens: 8,
            }
        );
    }
}
