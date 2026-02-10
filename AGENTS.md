# Alchemy

Unified LLM API abstraction layer in Rust, ported from `@mariozechner/pi-ai` TypeScript package. Supports 8+ providers (Anthropic, OpenAI, Google, AWS Bedrock, Mistral, xAI, Groq, Cerebras, OpenRouter).

## Project Structure

```
src/                 Rust crate (the port target)
ai/src/              TypeScript source (reference implementation)
docs/                Architecture and API docs
memory-bank/         Planning and research documents
```

## Design Philosophy

**One-to-one port, not a rewrite.** Match TypeScript behavior exactly before optimizing.

- Port logic faithfully. Rust idioms yes, but behavior must match.
- All providers use async streams. No blocking.
- `Value` only at serialization boundaries. No `any` equivalents.
- API-key auth only. No OAuth.

## Workflow

- Never begin coding until the objective is explicitly defined. Ask if unclear.
- Reference the TypeScript source before implementing.
- Run `cargo check` and `cargo clippy` before committing. Fix all warnings.
- No `#[allow(dead_code)]` or `#[allow(...)]` without strong justification.

## Error Handling

- `Result<T, Error>` everywhere. `?` for propagation.
- Custom `Error` enum with `thiserror`. No stringly-typed errors.
- Fail fast. No silent `unwrap_or_default()`.

## Dependency Direction

```
stream -> providers -> types -> utils
                ↓
        infrastructure (reqwest, aws-sdk)
```

Inner layers never import from outer layers. Infrastructure is pluggable.

## Quality Gates

See `docs/quality-gates.md` for detailed gates (no shims, coupling/cohesion, dependency direction, design by contract, TypeScript parity).

## Deployment

Published to crates.io as [`alchemy-llm`](https://crates.io/crates/alchemy-llm).

```bash
cargo publish --dry-run && cargo publish
```
