# Phase 2: Base Provider Abstraction

**Status:** Not Started
**Estimated Files:** 3-4 Rust modules
**Source Reference:** `ai/src/stream.ts` (554 lines)

---

## Objective

Create the provider dispatch system that routes streaming requests to the correct provider implementation based on the model's API type.

---

## Prerequisites

- Phase 1 complete (EventStream, model registry)

---

## Architecture Overview

```
                    +------------------+
                    |  stream(model,   |
                    |  context, opts)  |
                    +--------+---------+
                             |
                    +--------v---------+
                    |  API Key Check   |
                    |  (env fallback)  |
                    +--------+---------+
                             |
              +--------------+--------------+
              |              |              |
     +--------v----+  +------v------+  +----v--------+
     | Anthropic   |  | OpenAI      |  | Google      |
     | Provider    |  | Provider    |  | Provider    |
     +-------------+  +-------------+  +-------------+
```

---

## Components to Implement

### 1. Provider Trait

**Rust Target:** `src/providers/mod.rs`

```rust
use async_trait::async_trait;
use crate::types::{Context, Model, StreamOptions};
use crate::stream::AssistantMessageEventStream;
use crate::Result;

#[async_trait]
pub trait Provider: Send + Sync {
    type Api: crate::types::ApiType;
    type Options: StreamOptions;

    fn stream(
        &self,
        model: &Model<Self::Api>,
        context: &Context,
        options: Self::Options,
    ) -> AssistantMessageEventStream;
}
```

---

### 2. Environment API Key Resolution

**Source:** `ai/src/stream.ts` lines 53-115

**Rust Target:** `src/providers/env.rs`

```rust
use crate::types::{KnownProvider, Provider};
use std::env;
use std::path::PathBuf;

/// Get API key for provider from known environment variables.
/// Returns None for OAuth-only providers.
pub fn get_env_api_key(provider: &Provider) -> Option<String> {
    match provider {
        Provider::Known(KnownProvider::Anthropic) => {
            env::var("ANTHROPIC_OAUTH_TOKEN")
                .or_else(|_| env::var("ANTHROPIC_API_KEY"))
                .ok()
        }
        Provider::Known(KnownProvider::OpenAI) => {
            env::var("OPENAI_API_KEY").ok()
        }
        Provider::Known(KnownProvider::Google) => {
            env::var("GEMINI_API_KEY").ok()
        }
        Provider::Known(KnownProvider::GoogleVertex) => {
            if has_vertex_adc_credentials() {
                Some("<authenticated>".to_string())
            } else {
                None
            }
        }
        Provider::Known(KnownProvider::AmazonBedrock) => {
            if has_bedrock_credentials() {
                Some("<authenticated>".to_string())
            } else {
                None
            }
        }
        Provider::Known(KnownProvider::Groq) => env::var("GROQ_API_KEY").ok(),
        Provider::Known(KnownProvider::Cerebras) => env::var("CEREBRAS_API_KEY").ok(),
        Provider::Known(KnownProvider::Xai) => env::var("XAI_API_KEY").ok(),
        Provider::Known(KnownProvider::OpenRouter) => env::var("OPENROUTER_API_KEY").ok(),
        Provider::Known(KnownProvider::VercelAiGateway) => env::var("AI_GATEWAY_API_KEY").ok(),
        Provider::Known(KnownProvider::Zai) => env::var("ZAI_API_KEY").ok(),
        Provider::Known(KnownProvider::Mistral) => env::var("MISTRAL_API_KEY").ok(),
        Provider::Known(KnownProvider::Minimax) => env::var("MINIMAX_API_KEY").ok(),
        Provider::Known(KnownProvider::MinimaxCn) => env::var("MINIMAX_CN_API_KEY").ok(),
        Provider::Custom(_) => None,
    }
}

fn has_vertex_adc_credentials() -> bool {
    // Check GOOGLE_APPLICATION_CREDENTIALS first
    if let Ok(path) = env::var("GOOGLE_APPLICATION_CREDENTIALS") {
        return PathBuf::from(path).exists();
    }

    // Fall back to default ADC path
    if let Some(home) = dirs::home_dir() {
        let adc_path = home
            .join(".config")
            .join("gcloud")
            .join("application_default_credentials.json");

        if adc_path.exists() {
            // Also need project and location
            let has_project = env::var("GOOGLE_CLOUD_PROJECT").is_ok()
                || env::var("GCLOUD_PROJECT").is_ok();
            let has_location = env::var("GOOGLE_CLOUD_LOCATION").is_ok();
            return has_project && has_location;
        }
    }
    false
}

fn has_bedrock_credentials() -> bool {
    env::var("AWS_PROFILE").is_ok()
        || (env::var("AWS_ACCESS_KEY_ID").is_ok() && env::var("AWS_SECRET_ACCESS_KEY").is_ok())
        || env::var("AWS_BEARER_TOKEN_BEDROCK").is_ok()
        || env::var("AWS_CONTAINER_CREDENTIALS_RELATIVE_URI").is_ok()
        || env::var("AWS_CONTAINER_CREDENTIALS_FULL_URI").is_ok()
        || env::var("AWS_WEB_IDENTITY_TOKEN_FILE").is_ok()
}
```

---

### 3. Main Stream Dispatcher

**Source:** `ai/src/stream.ts` lines 117-166

**Rust Target:** `src/stream/dispatch.rs`

```rust
use crate::types::{Api, AnyModel, Context};
use crate::stream::AssistantMessageEventStream;
use crate::providers::env::get_env_api_key;
use crate::error::{Error, Result};

/// Stream a completion from any model.
///
/// Dispatches to the appropriate provider based on the model's API type.
pub fn stream(
    model: &AnyModel,
    context: &Context,
    options: impl Into<StreamOptionsAny>,
) -> Result<AssistantMessageEventStream> {
    let options = options.into();

    // Get API key (options override, then env fallback)
    let api_key = options.api_key()
        .map(String::from)
        .or_else(|| get_env_api_key(model.provider()));

    // Some providers don't need API keys (Vertex, Bedrock use ADC/IAM)
    let needs_api_key = !matches!(
        model.api(),
        Api::GoogleVertex | Api::BedrockConverseStream
    );

    if needs_api_key && api_key.is_none() {
        return Err(Error::NoApiKey(model.provider().to_string()));
    }

    match model {
        AnyModel::Anthropic(m) => {
            let opts = options.into_anthropic(api_key)?;
            Ok(crate::providers::anthropic::stream(m, context, opts))
        }
        AnyModel::Bedrock(m) => {
            let opts = options.into_bedrock()?;
            Ok(crate::providers::bedrock::stream(m, context, opts))
        }
        AnyModel::OpenAICompletions(m) => {
            let opts = options.into_openai_completions(api_key)?;
            Ok(crate::providers::openai::completions::stream(m, context, opts))
        }
        AnyModel::OpenAIResponses(m) => {
            let opts = options.into_openai_responses(api_key)?;
            Ok(crate::providers::openai::responses::stream(m, context, opts))
        }
        AnyModel::GoogleGenerativeAi(m) => {
            let opts = options.into_google(api_key)?;
            Ok(crate::providers::google::stream(m, context, opts))
        }
        AnyModel::GoogleVertex(m) => {
            let opts = options.into_google_vertex()?;
            Ok(crate::providers::google::vertex::stream(m, context, opts))
        }
    }
}

/// Convenience function: stream and await the final result.
pub async fn complete(
    model: &AnyModel,
    context: &Context,
    options: impl Into<StreamOptionsAny>,
) -> Result<AssistantMessage> {
    let s = stream(model, context, options)?;
    Ok(s.result().await?)
}
```

---

### 4. Simple Options Mapping

**Source:** `ai/src/stream.ts` lines 177-453 (mapOptionsForApi)

**Purpose:** Convert `SimpleStreamOptions` to provider-specific options with:
- Thinking budget calculations
- Max tokens adjustments
- Provider-specific reasoning effort mapping

**Rust Target:** `src/stream/simple.rs`

```rust
use crate::types::{
    AnyModel, Api, Context, SimpleStreamOptions, ThinkingLevel, ThinkingBudgets,
};
use crate::stream::AssistantMessageEventStream;
use crate::Result;

/// Stream with simplified options that work across all providers.
pub fn stream_simple(
    model: &AnyModel,
    context: &Context,
    options: Option<SimpleStreamOptions>,
) -> Result<AssistantMessageEventStream> {
    let options = options.unwrap_or_default();
    let provider_options = map_options_for_api(model, &options);
    crate::stream::stream(model, context, provider_options)
}

fn map_options_for_api(
    model: &AnyModel,
    options: &SimpleStreamOptions,
) -> StreamOptionsAny {
    let base_max_tokens = options.max_tokens
        .unwrap_or_else(|| model.max_tokens().min(32000));

    match model.api() {
        Api::AnthropicMessages => {
            map_anthropic_options(model, options, base_max_tokens)
        }
        Api::BedrockConverseStream => {
            map_bedrock_options(model, options, base_max_tokens)
        }
        Api::OpenAICompletions => {
            map_openai_completions_options(model, options)
        }
        Api::OpenAIResponses => {
            map_openai_responses_options(model, options)
        }
        Api::GoogleGenerativeAi => {
            map_google_options(model, options)
        }
        Api::GoogleVertex => {
            map_google_vertex_options(model, options)
        }
    }
}

/// Adjust max_tokens to account for thinking budget.
/// APIs like Anthropic require max_tokens > thinking.budget_tokens.
fn adjust_max_tokens_for_thinking(
    base_max_tokens: u32,
    model_max_tokens: u32,
    reasoning_level: ThinkingLevel,
    custom_budgets: Option<&ThinkingBudgets>,
) -> (u32, u32) {
    let default_budgets = ThinkingBudgets {
        minimal: Some(1024),
        low: Some(2048),
        medium: Some(8192),
        high: Some(16384),
    };

    let level = clamp_reasoning(reasoning_level);
    let mut thinking_budget = custom_budgets
        .and_then(|b| b.get(level))
        .or_else(|| default_budgets.get(level))
        .unwrap_or(8192);

    let min_output_tokens = 1024;
    let max_tokens = (base_max_tokens + thinking_budget).min(model_max_tokens);

    // If not enough room, reduce thinking budget
    if max_tokens <= thinking_budget {
        thinking_budget = max_tokens.saturating_sub(min_output_tokens);
    }

    (max_tokens, thinking_budget)
}

/// Clamp xhigh to high for providers that don't support it.
fn clamp_reasoning(level: ThinkingLevel) -> ThinkingLevel {
    match level {
        ThinkingLevel::Xhigh => ThinkingLevel::High,
        other => other,
    }
}
```

---

## File Structure After Phase 2

```
alchemy/
  src/
    lib.rs              [UPDATE: add providers, stream mods]
    providers/
      mod.rs            [NEW: Provider trait, re-exports]
      env.rs            [NEW: API key resolution]
    stream/
      mod.rs            [UPDATE: add dispatch]
      event_stream.rs   [Phase 1]
      dispatch.rs       [NEW: main stream function]
      simple.rs         [NEW: simplified options]
```

---

## Provider Stub Structure

Each provider will be a separate module. Phase 2 creates stubs:

```rust
// src/providers/anthropic.rs (stub)
use crate::types::{Context, Model, model::AnthropicMessages};
use crate::stream::AssistantMessageEventStream;

pub struct AnthropicOptions {
    pub api_key: String,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub thinking_enabled: bool,
    pub thinking_budget_tokens: Option<u32>,
    // ...
}

pub fn stream(
    model: &Model<AnthropicMessages>,
    context: &Context,
    options: AnthropicOptions,
) -> AssistantMessageEventStream {
    todo!("Implement in Phase 3")
}
```

---

## Dependencies to Add

```toml
[dependencies]
dirs = "5.0"      # For home directory lookup
once_cell = "1.19" # Already needed from Phase 1
```

---

## Acceptance Criteria

- [ ] `get_env_api_key` returns correct keys for all providers
- [ ] `stream()` dispatches to correct provider based on model API
- [ ] `stream()` returns `Error::NoApiKey` when key missing and required
- [ ] `stream_simple()` maps options correctly for each API
- [ ] Thinking budget calculations match TypeScript implementation
- [ ] Provider stubs compile (return `todo!()`)
- [ ] `cargo test` passes
- [ ] `cargo clippy` clean

---

## Testing Strategy

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_get_env_api_key_anthropic() {
        env::set_var("ANTHROPIC_API_KEY", "test-key");
        let key = get_env_api_key(&Provider::Known(KnownProvider::Anthropic));
        assert_eq!(key, Some("test-key".to_string()));
        env::remove_var("ANTHROPIC_API_KEY");
    }

    #[test]
    fn test_anthropic_oauth_takes_precedence() {
        env::set_var("ANTHROPIC_API_KEY", "api-key");
        env::set_var("ANTHROPIC_OAUTH_TOKEN", "oauth-token");
        let key = get_env_api_key(&Provider::Known(KnownProvider::Anthropic));
        assert_eq!(key, Some("oauth-token".to_string()));
        env::remove_var("ANTHROPIC_API_KEY");
        env::remove_var("ANTHROPIC_OAUTH_TOKEN");
    }

    #[test]
    fn test_adjust_max_tokens_for_thinking() {
        let (max, budget) = adjust_max_tokens_for_thinking(
            8000,   // base_max_tokens
            32000,  // model_max_tokens
            ThinkingLevel::Medium,
            None,
        );
        // 8000 + 8192 = 16192
        assert_eq!(max, 16192);
        assert_eq!(budget, 8192);
    }

    #[test]
    fn test_adjust_max_tokens_capped() {
        let (max, budget) = adjust_max_tokens_for_thinking(
            30000,  // base_max_tokens
            32000,  // model_max_tokens
            ThinkingLevel::High,
            None,
        );
        // 30000 + 16384 would exceed 32000, so capped
        assert_eq!(max, 32000);
        // Budget reduced: 32000 - 30000 = 2000, but min output is 1024
        // Actually: max_tokens (32000) - min_output (1024) = 30976 thinking budget
        // But we want: max_tokens <= thinking_budget check
        // 32000 > 16384, so budget stays 16384
        assert_eq!(budget, 16384);
    }
}
```

---

## Notes

- The TypeScript code has OAuth providers (google-gemini-cli, openai-codex-responses) - these are **excluded** from port
- `github-copilot` provider uses different env vars (GH_TOKEN etc) - not porting OAuth providers
- Vertex and Bedrock use ADC/IAM credentials, not API keys - special handling
