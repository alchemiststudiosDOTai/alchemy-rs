---
summary: "Documentation index for Alchemy crate guides, provider docs, and API references"
read_when:
  - You need a starting point for project documentation
  - You want to find provider-specific usage guides
  - You want to understand the latest documented feature set
---

# Alchemy Documentation Index

## Start Here

- [../README.md](../README.md) - Project overview, installation, and quick start
- [api/lib.md](./api/lib.md) - Public API exports from `src/lib.rs`
- [api/error.md](./api/error.md) - Error and `Result` contract

## Provider Guides

- [providers/minimax.md](./providers/minimax.md) - First-class MiniMax provider (global + CN)
- [providers/zai.md](./providers/zai.md) - First-class z.ai GLM provider

## Utilities

- [utils/transform.md](./utils/transform.md) - Cross-provider conversation transformation

## Latest Release (0.1.7)

The latest published crate release focuses on shared OpenAI-like runtime consolidation:

- Deduplicated request/stream orchestration helpers across OpenAI-compatible, MiniMax, and z.ai providers
- Refactored shared stream block handling for interleaved reasoning/content/tool-call sequences
- Reduced duplicate test and enum string-mapping boilerplate while preserving behavior

For release details, see [../CHANGELOG.md](../CHANGELOG.md#017---2026-03-05).
