---
title: "provider-cache-capability-layer – QA Report"
phase: QA
date: "2026-04-06 15:48:33"
owner: "Codex"
parent_execute: ".artifacts/execute/2026-04-06_15-36-36_provider-cache-capability-layer.md"
git_commit_at_qa: "7a57ada"
tags: [qa, provider-cache-capability-layer]
---

## Summary

| Metric | Count |
|--------|-------|
| Files reviewed | 10 |
| Functions reviewed | 8 |
| CRITICAL findings | 0 |
| WARNING findings | 3 |
| INFO findings | 1 |
| PASS (no issues) | 5 |

## Scope Note

`memory-bank/execute/` does not exist in this repository. QA scope was taken from the execute artifact at `.artifacts/execute/2026-04-06_15-36-36_provider-cache-capability-layer.md`, which is the only recorded execute log for this work.

## Changed Areas Reviewed

### File: `src/types/options.rs`

| Function/Class | Lines | Status |
|----------------|-------|--------|
| `StreamOptions` / `CacheOptions` / option structs | L4-71 | PASS |

Notes:
- Public cache contract is coherent.
- `CacheOptions` is re-exported and serialized as expected.

### File: `src/cache/capability.rs`

| Function/Class | Lines | Status |
|----------------|-------|--------|
| `cache_capability_for()` | L15-20 | PASS |
| `kimi_request_mutations()` | L34-56 | PASS |

Notes:
- Kimi endpoint override, model override, `User-Agent`, and `prompt_cache_key` mapping are localized to the cache layer as intended.

### File: `src/cache/request.rs`

| Function/Class | Lines | Status |
|----------------|-------|--------|
| request preparation helpers | inspected via execute scope | PASS |

Notes:
- Request mutation application preserves the public `model.id` while rewriting the transport payload.

### File: `src/cache/usage.rs`

| Function/Class | Lines | Status |
|----------------|-------|--------|
| `normalize_openai_like_cache_usage()` | inspected via execute scope | PASS |

Notes:
- Normalization order is explicit and matches the acceptance tests for `cached_tokens` fallback fields.

### File: `src/providers/shared/http.rs`

| Function/Class | Lines | Status |
|----------------|-------|--------|
| `merge_header_layers()` | L45-55 | WARNING |

#### Findings for `merge_header_layers()`

| Severity | Category | Finding | Recommendation |
|----------|----------|---------|----------------|
| WARNING | Contracts / Data Flow | Cache-layer headers are merged before request headers, and the test at L91-110 explicitly locks in that caller-provided headers override cache headers. For Kimi, this means a user can replace `User-Agent: KimiCLI/1.29.0` injected by the cache capability and silently break the provider contract or disable cache behavior. | Preserve mandatory capability-owned headers, or explicitly reject conflicting caller headers for providers that require transport-specific values. |

### File: `src/providers/kimi.rs`

| Function/Class | Lines | Status |
|----------------|-------|--------|
| `build_request()` | L81-106 | PASS |
| `process_chunk()` | L147-175 | PASS |

Notes:
- Runtime routing preserves the public `Model<AnthropicMessages>` contract while shifting transport details under the cache layer.
- No control-flow or usage-normalization regressions were identified in the request-building or chunk-processing path itself.

### File: `src/stream/mod.rs`

| Function/Class | Lines | Status |
|----------------|-------|--------|
| `stream()` Kimi dispatch branch | inspected via execute scope | PASS |

Notes:
- Dispatch ownership remains in `src/stream/mod.rs` as required by the plan.

### File: `src/models/kimi.rs`

| Function/Class | Lines | Status |
|----------------|-------|--------|
| `kimi_k2_5()` | inspected via execute scope | PASS |

Notes:
- Public model typing is unchanged and consistent with the documented crate boundary.

### File: `examples/provider_probe.rs`

| Function/Class | Lines | Status |
|----------------|-------|--------|
| `run_single_pass()` / `handle_stream_event()` | L239-355 | WARNING |
| `build_request_options()` | L272-285 | WARNING |
| Kimi cache-key constants | L18-22 | INFO |

#### Findings for `run_single_pass()` / `handle_stream_event()`

| Severity | Category | Finding | Recommendation |
|----------|----------|---------|----------------|
| WARNING | Contracts / Edge Cases | The probe declares success on any `Done` event and returns only `message.usage` at L348, without checking the final assistant text or stop reason. In live validation on April 6, 2026, the probe reported a completed pass while the observed final text was only `kimi`, not the requested `kimi ok`. This makes the example a false-positive cache probe for behavioral correctness. | Validate final message content and stop reason before counting a pass as successful, especially when the prompt requires an exact reply. |

#### Findings for `build_request_options()`

| Severity | Category | Finding | Recommendation |
|----------|----------|---------|----------------|
| WARNING | State / Idempotency | The Kimi probe always uses the fixed key `provider-probe-kimi-prefix-cache` at L22 and L280-281. Cache state therefore leaks across separate invocations, so a supposedly cold pass can already be warm from an earlier run. That happened during live validation on April 6, 2026: a later Kimi run showed `cache_hit_passes=10` because pass 1 reused an already-warmed key. | Make the probe key configurable or derive a fresh namespace per run when the goal is to observe cold-to-warm behavior. |
| INFO | Testability | `examples/provider_probe.rs` has no automated tests, so regressions in its validation logic are only caught manually. | Add example-level assertions or a small test harness if the probe remains a relied-on validation tool. |

### File: `README.md`, `docs/providers/architecture.md`, `docs/providers/kimi.md`

| Function/Class | Lines | Status |
|----------------|-------|--------|
| cache-contract documentation | sampled against execute scope | PASS |

Notes:
- Documentation aligns with the current implementation: public cache options, Kimi `prompt_cache_key`, and the `KimiCLI/1.29.0` transport header are all documented.

## Test Coverage Analysis

| Function / Area | Has Tests | Evidence | Missing Cases |
|-----------------|-----------|----------|---------------|
| Public cache options contract | Yes | `cargo test cache_options_replace_session_id_in_public_options` | No test for backward-compat migration behavior beyond serialization shape |
| Kimi cache capability request mutations | Yes | `cargo test kimi_cache_capability_returns_request_mutations` | No test for conflicting caller headers overriding mandatory cache headers |
| Cache usage normalization | Yes | `cargo test cache_usage_normalizer_reads_openai_cached_token_fields` | No test for mixed top-level and nested cache-write precedence beyond current fields |
| Kimi request building | Yes | `cargo test kimi_runtime_builds_chat_completions_request_with_cache_contract` | No test for missing capability path because `build_request()` panics on absent capability |
| Header precedence | Yes | `cargo test merge_header_layers_applies_deterministic_precedence` | Current test codifies the risky override behavior instead of guarding against it |
| Provider probe example | No | `rg -n "#\\[test\\]|#\\[tokio::test\\]" examples/provider_probe.rs` returned no matches | Exact-output validation, cold-cache isolation, and failure-mode assertions |

## Contract/API Verification

| Surface | Schema/Contract Match | Breaking Changes |
|---------|-----------------------|------------------|
| Public request options (`CacheOptions`, `OpenAICompletionsOptions.cache`) | Yes | Intended removal of legacy `session_id` surface |
| Kimi runtime transport rewrite | Yes | No public API break; transport details remain internal |
| Kimi provider docs | Yes | None identified in docs |
| Probe example as validation tool | Partial | It exercises the new cache surface, but it does not reliably prove cold-start behavior or exact response correctness |

## Static Analysis Summary

| Tool | Result |
|------|--------|
| Existing repo harness | Previously passed per execute log |
| Targeted Rust test | `cargo test merge_header_layers_applies_deterministic_precedence` passed |
| Live validation | `cargo run --example provider_probe kimi` and `cargo run --example provider_probe minimax` both returned cache-read signals; Kimi also demonstrated cross-run warm-cache contamination due to the fixed cache key |

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation Status |
|------|------------|--------|-------------------|
| Caller overrides mandatory Kimi `User-Agent` header and breaks cache transport | Medium | High | Not mitigated |
| Probe reports success while model output is truncated or incorrect | High | Medium | Not mitigated |
| Kimi “cold” probe starts warm because cache key is reused across runs | High | Low | Not mitigated |

## Recommendations Summary

### Must Fix (CRITICAL)

None.

### Should Fix (WARNING)
 
1. Protect provider-mandated cache headers from being overridden by user-supplied request headers in the shared HTTP merge path.
2. Make the probe validate final assistant output and stop reason instead of treating any `Done` event as success.
3. Stop using a single hard-coded Kimi cache key for all runs when the goal is to observe cold-to-warm cache behavior.

### Observations (INFO)

1. The implementation itself is structurally coherent: cache request shaping, transport overrides, usage normalization,
and Kimi dispatch all align with the plan and acceptance tests. 
