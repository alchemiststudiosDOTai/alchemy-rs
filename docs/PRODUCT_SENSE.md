---
summary: "Who Alchemy is for and what tradeoffs the crate currently optimizes for"
status: current
last_verified: 2026-03-09
---

# Product Sense

Alchemy targets Rust developers who need one crate surface across multiple LLM vendors without giving up streaming semantics or typed message/event contracts.

## What It Optimizes For

- A consistent crate API for multi-provider integrations
- Streaming-centric usage rather than one-shot wrappers
- First-class implementations where provider behavior diverges materially
- Cross-provider conversation replay and tool-call normalization

## Non-Goals

- Hosted-service guarantees or SLAs
- Perfectly even documentation depth across every provider in the marketing list
- Hiding every vendor-specific option behind a lowest-common-denominator API
