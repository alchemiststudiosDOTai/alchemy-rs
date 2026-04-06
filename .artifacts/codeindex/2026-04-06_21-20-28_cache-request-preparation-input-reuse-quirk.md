---
title: "cache request preparation input reuse quirk"
link: "cache-request-preparation-input-reuse-quirk"
type: code_index
ontological_relations:
  - relates_to: [[provider-cache-capability-layer-plan]]
  - relates_to: [[provider-cache-capability-layer-execute]]
tags: [codeindex, cache, request-shaping, kimi]
uuid: "A6A5EE7D-96F0-41D4-A87A-634483D5F95A"
created_at: "2026-04-06T21:20:28Z"
owner: "tuna"
---

# Summary

`CacheRequestPreparation::from_input(input, self.request_mutations(input))` in `src/cache/kimi_cache_capability.rs` looks redundant at first glance because `input` appears twice. It is a deliberate two-phase shape, not a bug.

# Files

- `src/cache/kimi_cache_capability.rs`
- `src/cache/request.rs`

# Quirk

The same `CacheRequestInput` value is passed twice:

1. `self.request_mutations(input)`
   - computes provider-specific deltas only
   - examples for Kimi:
     - `endpoint_override`
     - `model_override`
     - `User-Agent`
     - `prompt_cache_key`

2. `CacheRequestPreparation::from_input(input, mutations)`
   - materializes the final owned request shape
   - resolves fallback behavior:
     - `endpoint = endpoint_override || input.base_url`
     - `model_id = model_override || input.model_id`
     - carries forward `headers`
     - carries forward `body_fields`

# Why It Works

- `CacheRequestInput<'_>` is `Copy`, so reusing it is cheap.
- `request_mutations(...)` answers "what changes?"
- `from_input(...)` answers "what is the final request after applying changes?"

# Mental Model

```text
input
-> compute mutations from input
-> merge mutations onto input defaults
-> final CacheRequestPreparation
```

# Notes

- This is a separation-of-concerns quirk, not duplicated work.
- It keeps provider translation distinct from final request materialization.
