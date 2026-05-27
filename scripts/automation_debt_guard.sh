#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REG="${1:-$ROOT_DIR/docs/AUTOMATION_DEBT_REGISTER.md}"

if [[ ! -f "$REG" ]]; then
  echo "automation debt guard: missing register: $REG" >&2
  exit 2
fi

open_count="$(awk '
  BEGIN{open=0}
  /^Status:[[:space:]]*active/ {active=1}
  /^Status:[[:space:]]*`open`/ {open++}
  END{print open}
' "$REG")"

if [[ "$open_count" =~ ^[0-9]+$ ]] && [[ "$open_count" -gt 0 ]]; then
  echo "automation debt guard: FAIL unresolved_items=$open_count register=$REG" >&2
  exit 1
fi

echo "automation debt guard: PASS register=$REG"
