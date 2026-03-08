#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

bash "${SCRIPT_DIR}/run_minimax_reasoning_split.sh"
echo
echo "----------------------------------------"
echo
bash "${SCRIPT_DIR}/run_minimax_inline_think.sh"
echo
echo "----------------------------------------"
echo
bash "${SCRIPT_DIR}/run_minimax_usage_chunk.sh"
