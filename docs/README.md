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

## Utilities

- [utils/transform.md](./utils/transform.md) - Cross-provider conversation transformation

## Latest Documented Merge (main)

The most recent merge to `main` introduced first-class MiniMax support:

- New API type: `Api::MinimaxCompletions`
- New provider implementation: `src/providers/minimax.rs`
- Built-in MiniMax model constructors under `src/models/minimax.rs`
- Streaming reasoning support via `reasoning_split` and `<think>` fallback parsing
- New MiniMax live examples and smoke scripts

See the full guide: [providers/minimax.md](./providers/minimax.md).
