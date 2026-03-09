---
summary: "Design principles for Alchemy's stream-first library surface"
status: current
last_verified: 2026-03-09
---

# Design

## Core Principles

- Stream first: the event stream is the primary abstraction, and convenience helpers sit on top of it.
- Shared contracts first: `types`, `error`, and message/event semantics should remain provider-agnostic where possible.
- Provider differences stay visible when they affect correctness.
- Reuse shared runtime code: OpenAI-like providers should converge in `src/providers/shared/*` rather than drift into copy-paste variants.
- Keep docs close to implementation and separate durable reference material from temporary plans.
