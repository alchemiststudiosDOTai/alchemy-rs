# Kimi Cache Enablement Plan

## Goal

Enable Kimi prompt caching in this crate by moving the first-class Kimi runtime off the Anthropic-style Messages endpoint and onto the Kimi Code OpenAI-compatible chat completions endpoint.

## Confirmed Current State

- The crate's Kimi model is defined in `src/models/kimi.rs` with:
  - `base_url = "https://api.kimi.com/coding"`
  - `api = AnthropicMessages`
  - `id = "kimi-coding"`
- The runtime in `src/providers/kimi.rs` uses the shared Anthropic-style path with:
  - `messages_endpoint = "/v1/messages"`
- Local reproduction with `.env` loaded and `cargo run --example provider_probe kimi` showed repeated identical requests returning:
  - `cache_read = 0`
  - `cache_write = 0`
- Direct raw call to `POST https://api.kimi.com/coding/v1/chat/completions` showed:
  - `403 access_terminated_error` without a coding-agent `User-Agent`
  - `200 OK` with `User-Agent: KimiCLI/1.29.0`

## Confirmed Requirements For Caching

Based on `.artifacts/research/2026-04-04_19-16-00_kimi-code-caching-findings.md` and direct header probing:

1. Use `POST https://api.kimi.com/coding/v1/chat/completions`
2. Send `User-Agent: KimiCLI/<version>`
3. Send `prompt_cache_key` in the top-level request body
4. Reuse the same `prompt_cache_key` across requests
5. Parse cache usage from:
   - `cached_tokens`
   - `prompt_tokens_details.cached_tokens` when present

## Direct Raw Endpoint Notes

- Endpoint tested: `https://api.kimi.com/coding/v1/chat/completions`
- Required request headers for access:
  - `Authorization: Bearer <KIMI_API_KEY>`
  - `Content-Type: application/json`
  - `User-Agent: KimiCLI/1.29.0`
- Without the `User-Agent`, the endpoint returned:
  - `HTTP/2 403`
  - `error.type = "access_terminated_error"`
- With the `User-Agent`, the endpoint returned:
  - `HTTP/2 200`
- The cache key field name used by Kimi Code is:
  - `prompt_cache_key`
- Example value used during direct probing:
  - `codex-raw-log-session`
- Raw log artifact:
  - `.artifacts/execute/2026-04-05_14-00-30_kimi-chat-completions-raw.log`

## Implementation Plan

1. Rework the first-class Kimi model/helper in `src/models/kimi.rs`
   - Change Kimi from `Model<AnthropicMessages>` to `Model<OpenAICompletions>`
   - Point the base URL at the chat completions endpoint
   - Use the Kimi Code model id accepted by that endpoint

2. Rework provider dispatch in `src/stream/mod.rs`
   - Route `KnownProvider::Kimi` through the OpenAI-compatible completions runtime
   - Remove the current special-case Anthropic-style Kimi dispatch

3. Update Kimi provider wiring in `src/providers/kimi.rs`
   - Stop using the Anthropic-like shared runtime
   - Reuse the OpenAI-compatible shared runtime with Kimi-specific request customization

4. Add Kimi-specific request shaping
   - Always send `User-Agent: KimiCLI/<version>`
   - Map `options.session_id` to `prompt_cache_key`
   - Preserve any required auth and existing request options

5. Extend usage parsing in the OpenAI-compatible runtime
   - Read `cached_tokens`
   - Fall back to `prompt_tokens_details.cached_tokens` if needed
   - Map that value into crate `Usage.cache_read`

6. Review response normalization
   - Verify Kimi reasoning fields are still surfaced correctly on the OpenAI-compatible path
   - Check whether any existing Kimi-specific behavior depends on Anthropic-style event names

7. Update tests
   - Adjust model-type assertions for Kimi
   - Add or update unit tests for:
     - `prompt_cache_key` emission from `session_id`
     - `User-Agent` header injection
     - cache token parsing
   - Keep live tests ignored, but update them to use the new path

8. Update docs
   - `docs/providers/kimi.md`
   - `README.md`
   - Any examples that assume Anthropic-style Kimi behavior

## Validation Plan

1. Run targeted unit tests for Kimi and shared OpenAI-compatible runtime
2. Run `make quality-quick`
3. Run a live repeated-request probe against Kimi Code with a stable `session_id`
4. Confirm repeated passes show non-zero cached token usage on the chat completions path

## Risks

- Kimi reasoning/tool-call behavior may differ between the Anthropic-style and OpenAI-compatible paths
- Existing public typing for Kimi may be a breaking change if callers currently rely on `Model<AnthropicMessages>`
- Cache accounting may use different fields depending on the exact Kimi response shape
