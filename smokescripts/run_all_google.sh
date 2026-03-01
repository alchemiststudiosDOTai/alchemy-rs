#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

bash "${SCRIPT_DIR}/run_google_streaming.sh"
echo
echo "----------------------------------------"
echo
bash "${SCRIPT_DIR}/run_google_tool_call.sh"
