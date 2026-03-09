---
summary: "Reliability expectations and failure modes for the library"
status: current
last_verified: 2026-03-09
---

# Reliability

Alchemy is a library crate, so reliability is mostly about correct request construction, stream parsing, and stable shared contracts rather than service uptime.

## Main Failure Surfaces

- Missing or incorrect API keys
- Provider HTTP failures or malformed responses
- Stream chunk parsing errors
- Tool-call validation failures
- Context overflow or replay incompatibility when switching models/providers

## Current Controls

- Shared `Error` / `Result` contract in [api/error.md](./api/error.md)
- Type-based contracts in `src/types/*`
- `cargo fmt`, `cargo check`, `cargo clippy`, and `cargo test`
- `make ast-rules` for import and boundary enforcement

## Gaps

- There is no hosted-runtime runbook because this repo ships a library, not a service.
- Reliability notes are strongest for the first-class providers documented here.
