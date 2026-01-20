# Alchemy

Alchemy is a unified LLM API abstraction layer in Rust, ported from `@mariozechner/pi-ai` TypeScript package. It supports 8+ providers (Anthropic, OpenAI, Google, AWS Bedrock, Mistral, xAI, Groq, Cerebras, OpenRouter) through 4 main API interfaces.

```
alchemy/src/         Rust crate - the port target
ai/src/              TypeScript source - reference implementation
memory-bank/plan/    Phase planning documents
memory-bank/research/ Research and mapping docs
```

## Design Philosophy

**One-to-one port, not a rewrite.** Match TypeScript behavior exactly before optimizing.

- **Fidelity:** Port logic faithfully. Rust idioms yes, but behavior must match.
- **Streaming First:** All providers use async streams. No blocking.
- **Type Safety:** Leverage Rust's type system. No `any` equivalents (`Value` only at boundaries).
- **Zero OAuth:** API-key auth only. OAuth providers excluded from scope.

## Workflow Rules

- Never begin coding until the objective is **explicitly defined**. If unclear, ask questions.
- Reference the TypeScript source before implementing. File paths in `memory-bank/plan/`.
- Small, focused diffs only. Commit frequently.
- Run `cargo check` and `cargo clippy` before committing.
- Never ignore warnings. Do not suppress, downgrade, or change defaults to silence them; fix the underlying issue.

## Code Style & Typing

- Enforce `cargo fmt` and `cargo clippy` before PRs.
- Use explicit types. Avoid `impl Trait` in return position unless necessary.
- `#[allow(...)]` only with strong justification in comment.
- Flatten nested conditionals by returning early.
- If it is never executed, remove it.
- Normalize symmetries: identical things look identical, different things look different.
- Keep a variable's declaration and first use adjacent.
- Extract sub-expressions into well-named variables to record intent.
- Replace magic numbers with constants that broadcast meaning.
- Never use magic literals; `const` preferred.

## Error Handling

- Fail fast, fail loud. No silent `unwrap_or_default()`.
- Use `Result<T, Error>` everywhere. `?` operator for propagation.
- Minimize branching: every `if`/`match` must be justified.
- Custom `Error` enum with `thiserror`. No stringly-typed errors.

## Dependencies

- Avoid new dependencies unless widely used and maintained.
- Prefer `tokio` ecosystem for async.
- Check crate download counts and last update before adding.

## Scope & Maintenance

- Delete dead code (never `#[allow(dead_code)]` guard it).
- Backward compatibility only if low maintenance cost.

---

## Quality Gates

### Gate 0: No Shims

Never use shims. Fix the interface.

- If 19 files use an interface and new work has a problem, the interface is not the problem.
- If only two files use it and it's giving issues, rewrite it correctly.
- 99% of the time we don't need a unique interface.

### Gate 1: Coupling and Cohesion (Constantine)

**High cohesion** = everything in a module serves one purpose
**Low coupling** = modules don't need to know each other's internals

- **What single responsibility does this module have?** One sentence or split it.
- **Can I change this module without changing others?** If no, coupling problem.
- **Dependencies flow one direction:** `stream` -> `providers` -> `types` -> `utils`. Never backward.
- **How many dots?** `model.provider.config.headers.get("x")` = coupling smell. Max 2 dots.

**Project Structure:**

```
src/
  lib.rs              Public API exports
  error.rs            Error types only
  types/              Core type definitions
  stream/             EventStream, dispatch
  providers/          Provider implementations
    anthropic/
    openai/
    google/
    bedrock/
  models/             Model registry
  utils/              Helpers (validation, overflow, json_parse)
  transform.rs        Cross-provider message transformation
```

### Gate 2: Dependency Direction (Martin)

```
stream -> providers -> types -> utils
                ↓
        infrastructure (reqwest, aws-sdk)
```

- Inner layers know nothing about outer layers.
- `types/` never imports from `providers/`.
- `providers/` never imports from `stream/` (except types).
- Infrastructure (HTTP clients, SDKs) are plugins, not core.

**Wrong:**

```rust
// In providers/anthropic.rs
use crate::stream::dispatch;  // provider reaching into stream layer
```

**Right:**

```rust
// Stream layer calls providers, not the reverse
// providers/anthropic.rs only uses types and utils
use crate::types::{Model, Context, AssistantMessage};
```

### Gate 3: Design by Contract (Meyer)

Every function has:

- **Preconditions** - what must be true before calling
- **Postconditions** - what will be true after
- **Invariants** - what's always true

**Wrong:**

```rust
fn get_model(id: &str) -> Option<&Model> {
    MODELS.get(id)  // silent None, caller doesn't know why
}
```

**Right:**

```rust
fn get_model(id: &str) -> Result<&Model, Error> {
    MODELS.get(id).ok_or_else(|| Error::ModelNotFound {
        model_id: id.to_string(),
    })
}
```

### Gate 4: Match TypeScript Behavior

Before marking a component done:

1. Read the TypeScript source completely
2. List all edge cases and error conditions
3. Verify Rust handles them identically
4. Add tests that verify parity

**Example checklist for Anthropic provider:**

- [ ] Tool call ID normalization: `^[a-zA-Z0-9_-]{1,64}$`
- [ ] Thinking signature preservation for replay
- [ ] Prompt caching via `cache_control`
- [ ] Stop reason mapping matches exactly
- [ ] Usage calculation includes cache tokens

---

## Memory Bank Directory

```
memory-bank/
  research/           Codebase analysis and mapping
  plan/               Phase implementation plans
```

### Phase Documents

- `phase-1-core-types.md` - Types, EventStream, model registry
- `phase-2-provider-abstraction.md` - Dispatch, env keys, options
- `phase-3-anthropic-provider.md` - Reference provider implementation
- `phase-4-additional-providers.md` - OpenAI, Google, Bedrock
- `phase-5-cross-provider-features.md` - Transform, validation, overflow

### Continuous Learning

Dump bugs, smells, issues here as encountered. Raw is fine.

Format: `[date] [type] description`

Types: bug, smell, pattern, lesson, antipattern

---

## Current Status

**Types:** ~85% complete in `alchemy/src/types/`
**Phase 1 Remaining:** EventStream (#1), Model Registry (#2)
**Tests:** None yet - build as we go
**Docs:** Phase plans complete, inline docs as we implement
