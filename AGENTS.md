# AGENTS.md

## Project Overview
- `alchemy-llm` is a Rust crate that exposes a unified LLM API with a streaming-first interface.
- Implemented runtime paths today are OpenAI-compatible Completions, MiniMax Completions, and z.ai Completions; other provider identities exist in the type layer but still return not-implemented errors from `src/stream/mod.rs`.
- The public crate surface is re-exported from `src/lib.rs`.

## Where To Start
- Start with `README.md` for current provider status, public examples, and developer commands.
- Read `src/lib.rs` to see the exported modules and top-level helpers.
- Read `src/stream/mod.rs` next; it is the main `stream(...)` / `complete(...)` dispatch layer.
- Read `docs/README.md`, then `docs/providers/architecture.md` for the provider implementation contract.
- There is no top-level `ARCHITECTURE.md`; use `docs/providers/architecture.md` as the architecture source of truth.

## Repository Map
- `Cargo.toml` - crate metadata, dependencies, and published package settings.
- `src/lib.rs` - public module wiring and re-exports.
- `src/models/` - model helper constructors (`featherless_model`, MiniMax helpers, z.ai helpers).
- `src/providers/` - provider runtimes, auth/env lookup, and provider-specific request handling.
- `src/providers/shared/` - shared OpenAI-like HTTP, message serialization, and stream block helpers.
- `src/stream/` - top-level dispatch and stream collection helpers.
- `src/types/` - canonical API, content, message, event, model, and usage types.
- `src/transform.rs` - cross-provider conversation transformation and tool-call replay handling.
- `src/utils/` - parsing, sanitization, validation, overflow, and think-tag helpers.
- `src/test_helpers/` - shared test utilities for inline module tests.
- `docs/` - compact docs index plus provider architecture / Featherless notes.
- `rules/` - ast-grep config and boundary rules.
- `scripts/harness.sh` - full validation harness used by local hooks.
- `.cargo-husky/hooks/pre-commit` - pre-commit gate, including large-file blocking and harness execution.
- `.env.example` - canonical env var names for provider credentials.

## Commands
- `cargo build` - build the crate.
- `cargo test --all-features` - run tests directly.
- `cargo fmt --all -- --check` - formatting check used by the harness.
- `cargo clippy --all-targets --all-features -- -D warnings` - strict lint pass.
- `cargo check --all-targets --all-features` - compile-check all targets/features.
- `make test` - wrapper for `cargo test --all-features`.
- `make quality-quick` - `fmt` + `clippy` + `check`.
- `make quality-full` - quick checks plus tests, complexity, duplicates, dead-code, ast-rules, and large-files.
- `make ast-rules` - run ast-grep against `rules/sgconfig.yml`.
- `make harness` - minimal-output full commit harness via `scripts/harness.sh`.
- `make duplicates` requires `polydup`; `make ast-rules` requires `sg`.

## Boundaries
- `src/types/**/*.rs` must not import `crate::providers` or `crate::stream` (`rules/checks/no-providers-import-in-types.yml`, `rules/checks/no-stream-import-in-types.yml`).
- `src/providers/**/*.rs` must not import `crate::stream`; import stream-related types from `crate::types` instead (`rules/checks/no-stream-import-in-providers.yml`).
- Put shared OpenAI-compatible behavior in `src/providers/shared/` before duplicating logic in provider files.
- Keep top-level dispatch decisions in `src/stream/mod.rs`; that file is where runtime support becomes user-visible.
- Tests are colocated under `src/**` with `#[cfg(test)]`; there is no top-level `tests/` directory in the current tree.

## Sources Of Truth
- `README.md` - public crate status and developer entrypoint.
- `docs/README.md` - docs index.
- `docs/providers/architecture.md` - provider replay / thinking normalization contract.
- `docs/providers/featherless.md` - first-class Featherless notes on the shared OpenAI-compatible path.
- `Cargo.toml` - package metadata and dependency set.
- `Makefile` - canonical local build / lint / quality commands.
- `scripts/harness.sh` - exact full validation sequence.
- `.cargo-husky/hooks/pre-commit` - local commit gate and large-file limit.
- `.cargo/config.toml` and `clippy.toml` - lint thresholds and rustflags.
- `rules/README.md` and `rules/checks/*.yml` - structural boundary enforcement.
- `src/providers/env.rs` and `.env.example` - canonical provider credential variable names.

## Change Guardrails
- Keep `src/lib.rs` exports coherent when adding, renaming, or removing modules/helpers.
- When adding provider auth support, update both `src/providers/env.rs` and `.env.example`, then adjust docs that mention that provider.
- Preserve the thinking/replay contract documented in `docs/providers/architecture.md`; same-provider replay fidelity is a repo-level rule.
- Do not document GitHub Actions paths as validation gates; this repository currently has no `.github/workflows/` directory.
- Newly added files over 500 KB are blocked by the pre-commit hook.

## Validation Checklist
- Recheck every path referenced above after edits.
- Run `make quality-quick` for normal code changes.
- Run `make harness` before handoff when changes touch runtime logic, rules, or shared infrastructure.
- If you change ast-grep rules, run `make ast-rules` explicitly.
- If you change provider auth or docs, verify env var names against both `.env.example` and `src/providers/env.rs`.
- Keep this file compact; link to source docs instead of copying detailed policy.

<!-- DARK_FOREST:START -->
## Dark Forest

This repository uses the Dark Forest workflow scaffold.

Read in this order:

- `.darkforest/index.md`
- `.darkforest/operations/README.md`
- the newest file in `.darkforest/operations/active/`, or `.darkforest/operations/operation-template.md` if no active operation exists
- `.darkforest/session-log.md`
- `.darkforest/inventory/work.json`

Routing:

- Use `dark-forest-bootstrap` only for first adoption or major rescoping.
- Use `dark-forest-lite` by default.
- Use `dark-forest-gates` when an invariant needs mechanical proof.
<!-- DARK_FOREST:END -->
