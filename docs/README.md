---
summary: "Documentation index for provider architecture and future crate guides"
read_when:
  - You want the starting point for project documentation
  - You are adding a new provider implementation
  - You need the unified thinking/replay contract
---

# Alchemy Documentation Index

## Start Here

- [providers/architecture.md](./providers/architecture.md) - Provider implementation contract for unified thinking, replay fidelity, and stream normalization
- [providers/featherless.md](./providers/featherless.md) - Featherless first-class provider notes for the shared OpenAI-compatible path

## Notes

This docs tree is being rebuilt incrementally on `main`.

The provider architecture document is the source of truth for how new providers should:

- normalize reasoning into `Content::Thinking`
- preserve same-provider replay fidelity
- use shared stream block helpers
- handle provider-native thought signatures when required
