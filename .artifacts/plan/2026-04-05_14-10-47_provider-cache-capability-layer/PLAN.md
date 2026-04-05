---
title: "provider cache capability layer implementation plan"
link: "provider-cache-capability-layer-plan"
type: implementation_plan
ontological_relations:
  - relates_to: [[2026-04-04_19-16-00_kimi-code-caching-findings]]
tags: [plan, cache, kimi, providers, coding]
uuid: "20858211-da99-4379-8971-8c8ccb8b80db"
created_at: "2026-04-05T19:10:47Z"
parent_research: ".artifacts/research/2026-04-04_19-16-00_kimi-code-caching-findings.md"
git_commit_at_plan: "69e7184"
---

## Goal

- Build a new central cache capability layer that owns unified cache request shaping and normalized cache usage, then wire Kimi into it as the first provider while keeping Kimi publicly typed as `Model<AnthropicMessages>`.
- Out of scope: adding a compatibility shim for `session_id`, migrating additional providers in this change, deployment work, or broad non-cache refactors.

## Scope & Assumptions

- IN scope:
  - replace the old public cache/session surface with a new cache abstraction
  - add a central cache module that provider runtimes call into
  - keep dispatch ownership in `src/stream/mod.rs`
  - keep transport ownership inside provider runtimes
  - preserve Kimi's public model helper as `Model<AnthropicMessages>`
  - route Kimi's runtime implementation to the caching-capable chat completions path internally
  - normalize cache usage in one canonical layer
- OUT of scope:
  - keeping `session_id` as a fallback alias
  - migrating Anthropic, MiniMax, z.ai, or OpenAI providers to use the new cache layer in this change
  - redesigning non-cache parts of the event model
  - changing the crate-wide top-level `stream(...)` signature
- Assumptions:
  - Kimi caching requires `POST /coding/v1/chat/completions`
  - Kimi caching requires `User-Agent: KimiCLI/<version>`
  - Kimi cache identity is carried by top-level `prompt_cache_key`
  - Kimi public type remains `AnthropicMessages` for this MVP even though the runtime transport changes underneath
  - the new cache layer is the source of truth for cache key mapping, header injection, and cache usage extraction

## Deliverables

- New central cache capability module under `src/cache/`
- New public cache options type replacing `session_id`
- Kimi provider runtime wired through the cache capability layer
- Shared usage normalization moved behind the cache abstraction
- Developer docs updated for the new cache contract and Kimi MVP path

## Readiness

- Preconditions:
  - research doc exists at `.artifacts/research/2026-04-04_19-16-00_kimi-code-caching-findings.md`
  - raw endpoint evidence exists at `.artifacts/execute/2026-04-05_14-00-30_kimi-chat-completions-raw.log`
  - current Kimi runtime entrypoint exists in `src/providers/kimi.rs`
  - current public cache-adjacent field exists in `src/types/options.rs` and is unused in provider code
- Existing uncommitted files observed at planning time:
  - `.artifacts/execute/2026-04-05_14-00-30_kimi-chat-completions-raw.log`
  - `plan.md`

## Milestones

- M1: Public cache contract and central cache module skeleton
- M2: Shared request/usage integration hooks
- M3: Kimi runtime migration onto the new cache layer
- M4: Cleanup, docs, and validation hooks

## Ticket Index

<!-- TICKET_INDEX:START -->

| Task | Title | Ticket |
|---|---|---|
| T001 | Replace legacy session identity with a public cache contract | [tickets/T001.md](tickets/T001.md) |
| T002 | Create the central cache capability layer | [tickets/T002.md](tickets/T002.md) |
| T003 | Thread cache capability hooks through shared request and usage plumbing | [tickets/T003.md](tickets/T003.md) |
| T004 | Migrate the Kimi runtime onto the cache capability layer while preserving the public Anthropic model | [tickets/T004.md](tickets/T004.md) |
| T005 | Remove obsolete cache logic and codify the new pattern in developer docs | [tickets/T005.md](tickets/T005.md) |

<!-- TICKET_INDEX:END -->

## Work Breakdown (Tasks)

### T001: Replace legacy session identity with a public cache contract

**Summary**: Introduce a new public cache options type and remove `session_id` from the public options contract so cache identity is modeled explicitly from day one.

**Owner**: core

**Estimate**: 2h

**Dependencies**: none

**Target milestone**: M1

**Acceptance test**: `cargo test cache_options_replace_session_id_in_public_options`

**Files/modules touched**:
- src/types/options.rs
- src/types/mod.rs
- src/providers/openai_completions.rs
- src/lib.rs

**Steps**:
1. Add a new public cache options type in `src/types/options.rs` with a required stable cache key field and room for future provider-neutral expansion.
2. Remove `session_id` from `StreamOptions`, `BaseStreamOptions`, and `SimpleStreamOptions`.
3. Add the new cache options field to `OpenAICompletionsOptions`, because that is the current public request-options surface used by `stream(...)` and `complete(...)`.
4. Re-export the new public type from `src/types/mod.rs` and any required top-level surface in `src/lib.rs`.
5. Add a focused unit test proving the new cache field is the only public cache/session identity field exposed by options.

### T002: Create the central cache capability layer

**Summary**: Add a new internal cache module that owns provider-specific cache request mutations and normalized cache usage extraction without becoming a second dispatcher.

**Owner**: core

**Estimate**: 4h

**Dependencies**: T001

**Target milestone**: M1

**Acceptance test**: `cargo test kimi_cache_capability_returns_request_mutations`

**Files/modules touched**:
- src/cache/mod.rs
- src/cache/capability.rs
- src/cache/request.rs
- src/cache/usage.rs
- src/lib.rs

**Steps**:
1. Create `src/cache/mod.rs` as the central cache entrypoint and organize request-side and usage-side helpers under explicit submodules.
2. Define an internal provider capability contract that can express:
   - request header injection
   - top-level request body mutations
   - optional endpoint override
   - model-id override when transport requires it
3. Define a normalized cache usage extraction API that returns crate-compatible read/write token counts from provider response shapes.
4. Add a Kimi capability implementation as the first concrete provider-backed cache capability.
5. Keep the module internal to the crate for now; only the public options type should be exposed.

### T003: Thread cache capability hooks through shared request and usage plumbing

**Summary**: Integrate the central cache layer with the shared request-building and usage-parsing paths so provider runtimes can delegate cache-specific work to one place.

**Owner**: core

**Estimate**: 5h

**Dependencies**: T002

**Target milestone**: M2

**Acceptance test**: `cargo test cache_usage_normalizer_reads_openai_cached_token_fields`

**Files/modules touched**:
- src/providers/shared/openai_like_runtime.rs
- src/providers/shared/http.rs
- src/providers/shared/stream_blocks.rs
- src/providers/openai_completions.rs
- src/cache/request.rs
- src/cache/usage.rs

**Steps**:
1. Add a cache-aware request preparation seam so provider runtimes can apply cache capability mutations before the HTTP request is sent.
2. Support merging cache-provided headers with model headers and request headers in a deterministic order.
3. Support cache-provided request body fields without hardcoding provider names into the shared OpenAI-like runtime.
4. Move or wrap cache token extraction so normalized cache usage is owned by the new cache layer rather than scattered helper logic.
5. Add a focused test covering `cached_tokens` and `prompt_tokens_details.cached_tokens` normalization.

### T004: Migrate the Kimi runtime onto the cache capability layer while preserving the public Anthropic model

**Summary**: Keep `kimi_k2_5()` publicly typed as `Model<AnthropicMessages>`, but replace the runtime implementation so `stream_kimi_messages(...)` uses the cache-capable chat completions transport through the new cache layer.

**Owner**: core

**Estimate**: 6h

**Dependencies**: T003

**Target milestone**: M3

**Acceptance test**: `cargo test kimi_runtime_builds_chat_completions_request_with_cache_contract`

**Files/modules touched**:
- src/providers/kimi.rs
- src/providers/mod.rs
- src/stream/mod.rs
- src/models/kimi.rs
- src/cache/capability.rs
- src/cache/request.rs

**Steps**:
1. Replace the current Anthropic-like Kimi runtime implementation in `src/providers/kimi.rs` with a Kimi-specific runtime that delegates request shaping to the cache capability layer.
2. Internally target `https://api.kimi.com/coding/v1/chat/completions` for Kimi cache-capable requests while preserving the public `Model<AnthropicMessages>` helper.
3. Have the Kimi capability inject `User-Agent: KimiCLI/<version>` and map the new public cache key onto top-level `prompt_cache_key`.
4. Normalize Kimi model identity if the runtime must translate `kimi-coding` to the transport-accepted model id.
5. Preserve text, reasoning, and tool-call behavior at the crate boundary so the public event contract does not regress.

### T005: Remove obsolete cache logic and codify the new pattern in developer docs

**Summary**: Delete old cache/session assumptions, document the new cache pattern, and update the local probe example so future providers can follow the same module shape.

**Owner**: core

**Estimate**: 3h

**Dependencies**: T004

**Target milestone**: M4

**Acceptance test**: `rg -n "prompt_cache_key|User-Agent: KimiCLI|cache options" README.md docs/providers/kimi.md docs/providers/architecture.md examples/provider_probe.rs`

**Files/modules touched**:
- src/types/options.rs
- docs/providers/kimi.md
- docs/providers/architecture.md
- README.md
- examples/provider_probe.rs

**Steps**:
1. Remove any remaining legacy `session_id` references or stale comments that describe the old cache/session approach.
2. Document the new central cache abstraction and its ownership boundaries in `docs/providers/architecture.md`.
3. Update `docs/providers/kimi.md` to describe the new cache contract, including `prompt_cache_key` mapping and `User-Agent` requirements.
4. Update `README.md` and `examples/provider_probe.rs` so the Kimi probe exercises the new public cache options shape rather than legacy assumptions.
5. Keep the docs focused on developer implementation and provider integration, not end-user marketing.

## Risks & Mitigations

- Public type/transport mismatch for Kimi:
  - Mitigation: keep the mismatch isolated to `src/providers/kimi.rs` and document it explicitly in the cache capability contract.
- Hard cutover risk from removing `session_id`:
  - Mitigation: do the public options replacement first and update all in-repo call sites in the same change.
- Shared runtime pollution:
  - Mitigation: keep provider-specific logic in the cache capability module rather than branching on provider names throughout shared helpers.
- Kimi response-shape drift:
  - Mitigation: centralize cache usage extraction and add narrow tests around Kimi/OpenAI-like usage fields.
- Reasoning/tool-call regression during transport migration:
  - Mitigation: keep Kimi-specific runtime tests focused on crate boundary behavior, not just request shape.

## Test Strategy

- Add one focused unit test per task.
- Prefer offline request-shaping and usage-normalization tests for T001-T004.
- Reserve live Kimi validation for the implementation follow-up after the code path is in place.
- Use the existing provider probe only as a developer validation hook, not as the primary proof in unit tests.

## References

- `.artifacts/research/2026-04-04_19-16-00_kimi-code-caching-findings.md:1`
- `.artifacts/execute/2026-04-05_14-00-30_kimi-chat-completions-raw.log:1`
- `src/types/options.rs:4`
- `src/providers/openai_completions.rs:22`
- `src/stream/mod.rs:20`
- `src/providers/kimi.rs:1`
- `src/providers/shared/openai_like_runtime.rs:11`
- `src/providers/shared/stream_blocks.rs:56`
- `src/models/kimi.rs:3`

## Final Gate

- **Output summary**: `.artifacts/plan/2026-04-05_14-10-47_provider-cache-capability-layer/`, 4 milestones, 5 tickets
- **Next step**: proceed to execute-phase with `.artifacts/plan/2026-04-05_14-10-47_provider-cache-capability-layer/PLAN.md`
