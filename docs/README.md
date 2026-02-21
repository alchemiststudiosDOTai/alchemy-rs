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

## Latest Release (0.1.5)

The latest published crate release adds first-class tool-call ID typing and cross-provider smoke coverage:

- New canonical `ToolCallId` type (`src/types/tool_call_id.rs`)
- `ToolCall.id` and `ToolResultMessage.tool_call_id` now use `ToolCallId`
- Unified cross-provider smoke flow for OpenRouter + MiniMax + Chutes
- Full typed stream/event output in `smokescripts/run_tool_call_unified_types.sh`

For release details, see [../CHANGELOG.md](../CHANGELOG.md#015---2026-02-21).

For MiniMax-specific documentation from the previous release train, see [providers/minimax.md](./providers/minimax.md).
