---
summary: "Detailed quality gates for code review and architecture decisions"
read_when:
  - Reviewing or writing new code for the project
  - Making architectural decisions about module boundaries
  - Porting a new provider from the TypeScript source
---

# Quality Gates

## Gate 0: No Shims

Never use shims. Fix the interface.

- If 19 files use an interface and new work has a problem, the interface is not the problem.
- If only two files use it and it's giving issues, rewrite it correctly.
- 99% of the time we don't need a unique interface.

## Gate 1: Coupling and Cohesion (Constantine)

**High cohesion** = everything in a module serves one purpose
**Low coupling** = modules don't need to know each other's internals

- **What single responsibility does this module have?** One sentence or split it.
- **Can I change this module without changing others?** If no, coupling problem.
- **Dependencies flow one direction:** `stream` -> `providers` -> `types` -> `utils`. Never backward.
- **How many dots?** `model.provider.config.headers.get("x")` = coupling smell. Max 2 dots.

## Gate 2: Dependency Direction (Martin)

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

## Gate 3: Design by Contract (Meyer)

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

## Gate 4: Match TypeScript Behavior

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
