# Research – stream() Unsafe Pointer Cast Soundness Issue

**Date:** 2026-02-10
**Owner:** Research sub-agents
**Phase:** Research

## Goal

Investigate the soundness of the unsafe raw pointer cast in `stream()` that converts `Model<TApi>` to `Model<OpenAICompletions>` (and similar types) based # Alchemy

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
on a runtime match on the `Api` enum.

## Findings

### The Issue Location

**File:** `src/stream/mod.rs`
**Lines:** 48-58

```rust
match api {
    Api::OpenAICompletions => {
        // SAFETY: We know the model has OpenAICompletions API type
        // This is a type-level guarantee from the match
        let model_ptr = model as *const Model<TApi> as *const Model<OpenAICompletions>;
        let openai_model = unsafe { &*model_ptr };
        Ok(stream_openai_completions(
            openai_model,
            context,
            resolved_options,
        ))
    }
    // ... other arms
}
```

### The Soundness Gap

The SAFETY comment claims a "type-level guarantee from the match," but **this is incorrect**. The match is on the **runtime** `Api` enum value returned by `model.api.api()`, not the **compile-time** type parameter `TApi`.

```rust
pub fn stream<TApi>(
    model: &Model<TApi>,  // Any TApi: ApiType is accepted here
    context: &Context,
    options: Option<OpenAICompletionsOptions>,
) -> Result<AssistantMessageEventStream>
where
    TApi: crate::types::ApiType,
{
    let api = model.api.api();  // Runtime enum value

    match api {  // Runtime dispatch, NOT type-level
        Api::OpenAICompletions => {
            // UNSAFE: Assumes TApi == OpenAICompletions, but this was only
            // checked at runtime, not enforced at compile time
            let model_ptr = model as *const Model<TApi> as *const Model<OpenAICompletions>;
            ...
        }
    }
}
```

### Why This Is Undefined Behavior

**File:** `src/types/model.rs`
**Lines:** 16-29

The `Model<TApi>` struct has a generic `compat` field:

```rust
pub struct Model<TApi: ApiType> {
    pub id: String,
    pub name: String,
    pub api: TApi,
    pub provider: Provider,
    pub base_url: String,
    pub reasoning: bool,
    pub input: Vec<InputType>,
    pub cost: ModelCost,
    pub context_window: u32,
    pub max_tokens: u32,
    pub headers: Option<HashMap<String, String>>,
    pub compat: Option<TApi::Compat>,  // Different type for each TApi!
}
```

The `compat` field has type `Option<TApi::Compat>`, where `Compat` is an associated type. For different API types, this is a **different concrete type**:

| Type | Compat Type |
|------|-------------|
| `Model<OpenAICompletions>` | `Option<OpenAICompletionsCompat>` |
| `Model<AnthropicMessages>` | `Option<NoCompat>` |
| `Model<OpenAIResponses>` | `Option<OpenAIResponsesCompat>` |

Casting `*const Model<AnthropicMessages>` to `*const Model<OpenAICompletions>` interprets `Option<NoCompat>` memory as `Option<OpenAICompletionsCompat>`. This is **undefined behavior** in Rust because the types may have different layouts.

### The Attack Vector (Theoretical)

While all current `ApiType` implementations have `api()` methods that return constant values, Rust's type system does not enforce this:

```rust
// A malicious or buggy ApiType implementation could do:
struct EvilApi;

impl ApiType for EvilApi {
    type Compat = NoCompat;

    fn api(&self) -> Api {
        Api::OpenAICompletions  // LIES! Returns wrong variant
    }
}

// Now construct:
let evil_model = Model {
    id: "evil".to_string(),
    api: EvilApi,  // Type is EvilApi
    compat: None::<NoCompat>,
    // ...
};

// stream() would:
// 1. Call evil_model.api.api() -> Api::OpenAICompletions
// 2. Match OpenAICompletions arm
// 3. Cast Model<EvilApi> to Model<OpenAICompletions>
// 4. Access compat as Option<OpenAICompletionsCompat> when it's actually Option<NoCompat>
// 5. UB!
```

### Type Architecture Summary

```
┌─────────────────────────────────────────────────────────────────┐
│                     COMPILE TIME                                │
│  Model<TApi> where TApi: ApiType                               │
│  └── compat: Option<TApi::Compat>  ← Different for each TApi   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              │ stream(model) - accepts any TApi
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                     RUNTIME                                     │
│  api = model.api.api() -> Api enum                             │
│  match api {                                                    │
│      Api::OpenAICompletions => {                               │
│          // Runtime check passed, but TApi still unknown!       │
│          cast Model<TApi> to Model<OpenAICompletions>  ← UB    │
│      }                                                          │
│  }                                                              │
└─────────────────────────────────────────────────────────────────┘
```

## Key Patterns / Solutions Found

### Current Pattern (Unsound)

The code attempts to bridge compile-time types with runtime dispatch via unsafe pointer casts. This is a common anti-pattern when:

1. You have a generic type `Model<T>` that varies by an associated type
2. You need to dispatch at runtime based on a value
3. You "know" the type matches and use pointer casts to "prove" it to the compiler

### Potential Fixes

1. **Avoid the cast entirely** - Use trait objects or enum dispatch instead of generic specialization
2. **Enforce at type level** - Use sealed traits to prevent external `ApiType` implementations
3. **Validate before cast** - Use `std::any::TypeId` to verify `TApi` matches before casting
4. **Redesign the API** - Store `compat` in a type-erased form (e.g., `Box<dyn Any>`) when runtime dispatch is needed

## Knowledge Gaps

- Has this UB been triggered in practice? (likely not due to current `ApiType` implementations)
- Why was unsafe code chosen over enum dispatch or trait objects?
- Are miri tests run that would catch this UB?

## References

- `src/stream/mod.rs:48-58` - The unsafe cast
- `src/types/model.rs:16-29` - Model struct with generic compat field
- `src/types/api.rs:163-166` - ApiType trait definition
- `src/types/model.rs:53-95` - ApiType implementations

## Additional Search

```bash
# Find all unsafe pointer casts in the codebase
grep -rn "as \*const Model" src/

# Find all ApiType implementations
grep -rn "impl ApiType for" src/
```

---

**Summary:** The pointer cast in `stream()` is unsound because:
1. The match on `Api` enum is a runtime check, not a type-level guarantee
2. The `compat` field has different types for different `TApi`, making the transmute UB
3. A malicious `ApiType` implementation could trigger this UB
