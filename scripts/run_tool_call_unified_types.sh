#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

if [[ -f "${REPO_ROOT}/.env" ]]; then
  set -a
  # shellcheck disable=SC1091
  source "${REPO_ROOT}/.env"
  set +a
fi

: "${OPENROUTER_API_KEY:?OPENROUTER_API_KEY is required}"
: "${MINIMAX_API_KEY:?MINIMAX_API_KEY is required}"
: "${CHUTES_API_KEY:?CHUTES_API_KEY is required}"

# Default to full typed response output for this smoke.
export TOOL_SMOKE_TYPES_ONLY="${TOOL_SMOKE_TYPES_ONLY:-0}"
export TOOL_SMOKE_FULL_TYPED_RESPONSE="${TOOL_SMOKE_FULL_TYPED_RESPONSE:-1}"

cd "${REPO_ROOT}"
cargo run --example tool_call_unified_types_smoke
