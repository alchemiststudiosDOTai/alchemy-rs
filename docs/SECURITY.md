---
summary: "Security boundaries for API keys, provider calls, and documentation hygiene"
status: current
last_verified: 2026-03-09
---

# Security

## Sensitive Surfaces

- Provider API keys loaded from environment variables or option structs
- Outbound HTTP requests to third-party LLM providers
- Tool-call payloads that may contain untrusted model-generated JSON

## Expectations

- Keep secrets in `.env` or the runtime environment; never hardcode credentials in source or docs.
- Treat provider responses as untrusted input until parsed and validated.
- Keep tool-call validation aligned with the shared type system and helper utilities.
- Keep generated or copied external material out of core reference docs unless clearly labeled.

## Unknowns

- There is no formal threat model checked into the repo yet.
- Secret scanning and dependency-audit policy are not documented in-repo today.
