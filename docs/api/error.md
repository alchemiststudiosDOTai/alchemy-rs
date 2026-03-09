---
summary: "Centralized error types for the crate"
status: current
last_verified: 2026-03-09
---

# Error Handling

All crate errors flow through `src/error.rs`.

## Error Variants

- `NoApiKey`
- `RequestError`
- `ApiError`
- `Aborted`
- `InvalidResponse`
- `InvalidHeader`
- `InvalidJson`
- `ModelNotFound`
- `UnknownProvider`
- `UnknownApi`
- `ToolValidationFailed`
- `ToolNotFound`
- `ContextOverflow`

## Result Alias

`pub type Result<T> = std::result::Result<T, Error>;`

## Usage Guidance

- Prefer propagating errors with `?`.
- Treat `ContextOverflow`, tool validation, and provider HTTP errors as expected caller-facing failure modes.
- Avoid introducing stringly-typed parallel error channels outside this enum unless there is a clear contract reason.
