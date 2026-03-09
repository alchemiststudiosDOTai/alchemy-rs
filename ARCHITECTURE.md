# Architecture

## Repository Shape

Alchemy is a library crate, not a hosted service. The repository's main job is to expose a consistent Rust API for multiple LLM providers while preserving provider-specific streaming behavior where correctness depends on it.

## Layer Map

```text
src/
  lib.rs                 public exports
  error.rs               shared Error / Result contract
  models/                built-in model constructors and metadata
  providers/             provider implementations and auth helpers
  providers/shared/      shared OpenAI-like request and stream runtime
  stream/                provider-agnostic dispatch entry points
  transform.rs           cross-provider conversation normalization
  types/                 canonical event, message, tool, usage, and model types
  utils/                 reusable validation, parsing, and sanitization helpers
rules/                   ast-grep architecture boundary checks
docs/                    human-authored system-of-record docs
```

## Main Flow

1. Callers build a `Model`, `Context`, and optional request options.
2. `stream::stream()` dispatches by `Api` to a concrete provider path.
3. The provider builds the request, resolves credentials, opens the stream, and converts raw chunks into shared `AssistantMessageEvent` values.
4. `src/providers/shared/*` centralizes OpenAI-like request and stream handling used by multiple providers.
5. `stream::complete()` folds the event stream into a final `AssistantMessage`.
6. `transform_messages*()` rewrites prior conversation history when callers switch providers or model families.

## Module Responsibilities

- `src/models/*`
  Built-in constructor and metadata layer only.
- `src/providers/*`
  Provider-specific request building, auth resolution, response parsing, and event emission.
- `src/providers/shared/*`
  Shared OpenAI-like transport and stream helpers. Extend this layer before duplicating logic.
- `src/stream/*`
  Public dispatch entry points and result assembly.
- `src/types/*`
  Canonical crate-wide contracts. Changes here affect every provider.
- `src/transform.rs`
  Compatibility layer for replaying conversation history across providers.
- `src/utils/*`
  Generic helpers that should stay independent of specific provider business logic.

## Current First-Class Paths

The current code and exports show first-class streaming support for:

- OpenAI-compatible completions
- MiniMax completions
- z.ai completions

The README lists a broader provider set, but the detailed docs surface is currently thinner than that list. Use [docs/providers/index.md](./docs/providers/index.md) as the accurate guide to what is documented in depth.

## Quality Boundaries

- `Makefile` is the intended command entry point for formatting, linting, tests, and architecture scans.
- `rules/` contains ast-grep boundary checks.
- Provider and type changes should usually be validated with both Rust checks and the architecture rules.
