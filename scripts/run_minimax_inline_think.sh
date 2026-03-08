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

: "${MINIMAX_API_KEY:?MINIMAX_API_KEY is required}"

export MINIMAX_INLINE_PROMPT="${MINIMAX_INLINE_PROMPT:-${1:-Think step by step, then answer: 1729 + 98}}"

cd "${REPO_ROOT}"
cargo run --example minimax_live_inline_think
