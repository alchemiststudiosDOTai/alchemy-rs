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

export MINIMAX_PROMPT="${MINIMAX_PROMPT:-${1:-Explain Rust lifetimes with one concrete example.}}"

cd "${REPO_ROOT}"
cargo run --example minimax_live_reasoning_split
