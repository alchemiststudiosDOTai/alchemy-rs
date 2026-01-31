# Dependency Directions

This project uses a **one-way dependency flow** to keep layers clean.

## Layer Order (top → bottom)

```
stream → providers → types → utils
```

**Rule:** A module may depend on modules **to its right** only.

## Examples

✅ **Allowed** (left depends on right)
- `stream` → `providers`
- `providers` → `types`
- `types` → `utils`

❌ **Not allowed** (right depends on left)
- `providers` → `stream`
- `types` → `providers`
- `utils` → `types`

## What an Arrow Means

In all dependency maps here, an arrow `A → B` means:

- **`A` imports `B`** using a `use` statement.
- So **`A` depends on `B`**.

## Why This Rule Exists

- Keeps higher-level orchestration from leaking into lower-level utilities.
- Prevents circular dependencies.
- Makes it safe to change lower layers without touching higher ones.
