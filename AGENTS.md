# Agent Guide

## Purpose

Alchemy is a Rust crate for streaming LLM interactions through a shared, provider-agnostic API. This file is the navigation layer: keep it short, use it to route work, and treat the deeper docs under `docs/` as the system of record.

## Non-Negotiables

- Start with [README.md](./README.md), [ARCHITECTURE.md](./ARCHITECTURE.md), and [docs/README.md](./docs/README.md) before changing behavior.
- Preserve the stream-first public surface: `stream()`, `complete()`, and the shared event/message/type contracts are the product core.
- Keep provider-specific request and stream parsing in `src/providers/*`. Shared OpenAI-like behavior belongs in `src/providers/shared/*`, not duplicated per provider.
- Treat `src/types/*` and `src/error.rs` as cross-provider contracts. Any change there is library-wide and should update docs.
- Keep built-in model metadata in `src/models/*`, dispatch in `src/stream/*`, and cross-provider replay logic in `src/transform.rs`.
- Update durable docs when stable behavior changes. Add new docs to the nearest index immediately.
- Put temporary execution detail under [docs/exec-plans/active/](./docs/exec-plans/active/index.md), archive finished work under [docs/exec-plans/completed/](./docs/exec-plans/completed/index.md), and keep durable debt in [docs/exec-plans/tech-debt-tracker.md](./docs/exec-plans/tech-debt-tracker.md).
- Keep generated outputs under [docs/generated/](./docs/generated/index.md). Do not mix generated material into hand-authored reference docs.
- For substantive Rust changes, run `cargo fmt`, `cargo check --all-targets --all-features`, `cargo clippy --all-targets --all-features -- -D warnings`, and `cargo test --all-features`. Run `make ast-rules` when module boundaries or imports change.

## Reading Order

1. [README.md](./README.md)
2. [ARCHITECTURE.md](./ARCHITECTURE.md)
3. [docs/README.md](./docs/README.md)
4. The area index for the subsystem you are touching

## Task Routing

- Public API and exports:
  [docs/api/index.md](./docs/api/index.md), [docs/api/lib.md](./docs/api/lib.md), `src/lib.rs`
- Errors and shared contracts:
  [docs/api/error.md](./docs/api/error.md), `src/error.rs`, `src/types/*`
- Provider implementations and model constructors:
  [docs/providers/index.md](./docs/providers/index.md), `src/providers/*`, `src/models/*`
- Stream dispatch and cross-provider replay:
  [ARCHITECTURE.md](./ARCHITECTURE.md), [docs/utils/index.md](./docs/utils/index.md), `src/stream/*`, `src/transform.rs`
- Design, reliability, security, and quality expectations:
  [docs/DESIGN.md](./docs/DESIGN.md), [docs/RELIABILITY.md](./docs/RELIABILITY.md), [docs/SECURITY.md](./docs/SECURITY.md), [docs/QUALITY_SCORE.md](./docs/QUALITY_SCORE.md), [rules/README.md](./rules/README.md)
- Planning and documentation hygiene:
  [docs/PLANS.md](./docs/PLANS.md), [docs/exec-plans/index.md](./docs/exec-plans/index.md)

## Maintenance Rules

- Update the deeper source-of-truth doc first; update this file only when routing or repo-wide rules change.
- Keep this file skimmable. Do not turn it into a changelog or architecture dump.
- Use explicit status markers such as `current`, `draft`, `needs review`, or `generated` in indexes.
- Mark uncertainty directly instead of preserving stale claims.
- Current known gap: this checkout does not include the examples/scripts inventory referenced in older README copy. Treat [docs/QUALITY_SCORE.md](./docs/QUALITY_SCORE.md) and [docs/exec-plans/tech-debt-tracker.md](./docs/exec-plans/tech-debt-tracker.md) as the current record of those gaps.
